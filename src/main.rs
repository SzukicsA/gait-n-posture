// main.rs
// Windows-friendly BLE scanner that resolves names via GAP/0x2A00 if missing.

use btleplug::api::{
    Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter,};
use btleplug::platform::{Manager as ManagerStruct, Peripheral};
use futures::stream::StreamExt;
use std::collections::HashMap;
use tokio::time::{sleep, timeout, Duration};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🪟 Windows-Optimized Bluetooth Scanner");
    println!("OS: {}", std::env::consts::OS);
    println!("{}", "=".repeat(50));

    let manager = ManagerStruct::new().await?;
    println!("✅ Manager loaded");

    let adapters = manager.adapters().await?;
    let adapter = adapters
        .into_iter()
        .nth(0)
        .ok_or("No Bluetooth adapter found")?;

    println!("🎯 Using Bluetooth adapter");

    // Empty filter = scan all services (BLE only; btleplug doesn't see Classic).
    let scan_filter = ScanFilter { services: vec![] };

    let mut events = adapter.events().await?;

    println!("🔍 Starting scan (connectable devices will be prioritized)...");
    println!("💡 Tip: Put your phone in the Bluetooth settings screen for better name ads\n");

    // We'll loop a few "rounds" to give Windows time to populate properties.
    let adapter_clone = adapter.clone();
    let handle = tokio::spawn(async move {
        let mut device_info: HashMap<String, (String, bool, u32)> = HashMap::new(); // addr -> (name, printed, attempts)
        let mut attempted_gap_name: HashMap<String, bool> = HashMap::new(); // addr -> already tried GAP read
        let mut scan_round = 1;

        loop {
            println!("🔄 Scan Round {}", scan_round);

            let _ = adapter_clone.stop_scan().await;
            sleep(Duration::from_millis(400)).await;
            if let Err(e) = adapter_clone.start_scan(scan_filter.clone()).await {
                eprintln!("❌ Error restarting scan: {e:?}");
                break;
            }

            let round_timeout = Duration::from_secs(15);
            let round_start = std::time::Instant::now();

            while round_start.elapsed() < round_timeout {
                match timeout(Duration::from_millis(200), events.next()).await {
                    Ok(Some(event)) => {
                        let (id, is_discovery) = match &event {
                            CentralEvent::DeviceDiscovered(id) => (id.clone(), true),
                            CentralEvent::DeviceUpdated(id) => (id.clone(), false),
                            _ => continue,
                        };

                        let Ok(peripheral) = adapter_clone.peripheral(&id).await else { continue };

                        // Pull advertised properties if available
                        let props_opt = peripheral.properties().await.ok().flatten();
                        if props_opt.is_none() {
                            continue;
                        }
                        let props = props_opt.unwrap();

                        let address = props.address.to_string();
                        // Heuristic "is_connectable": either the device reports connectable or has an RSSI (active ad)
                        let connectable = props.rssi.is_some();

                        if !connectable && props.local_name.is_none() && props.services.is_empty() {
                            // Likely a non-connectable beacon; skip for "pair-ready" discovery.
                            continue;
                        }

                        // 1) Try advertising name
                        let mut name = props.local_name.clone().unwrap_or_else(|| "Unknown".into());

                        // 2) Fallback: if name is unknown, try to read GAP Device Name (0x1800/0x2A00)
                        if name == "Unknown" && connectable && !attempted_gap_name.contains_key(&address)
                        {
                            attempted_gap_name.insert(address.clone(), true);

                            // Do the fallback with a timeout so we don't hang
                            match timeout(
                                Duration::from_secs(5),
                                resolve_name_via_gap_name(&peripheral),
                            )
                            .await
                            {
                                Ok(Some(gap_name)) if !gap_name.is_empty() => {
                                    name = gap_name;
                                }
                                _ => {
                                    // Ignore failures (many devices hide name until paired)
                                }
                            }
                        }

                        // Update our bookkeeping
                        let (old_name, was_printed, attempt_count) = device_info
                            .get(&address)
                            .cloned()
                            .unwrap_or(("Unknown".to_string(), false, 0));
                        let new_attempts = attempt_count + 1;

                        let should_print = match (&old_name[..], &name[..]) {
                            ("Unknown", n) if n != "Unknown" => true,             // resolved now
                            (old, new) if old != new => true,                     // changed
                            ("Unknown", "Unknown") if !was_printed => true,       // first sighting
                            _ => false,
                        };

                        device_info.insert(address.clone(), (name.clone(), was_printed || should_print, new_attempts));

                        if should_print {
                            let event_type = if is_discovery { "🔍 Discovered" } else { "🏷️ Updated" };
                            println!("{event_type}: {name} ({})", props.address);
                            if let Some(rssi) = props.rssi {
                                println!("   📶 Signal: {} dBm", rssi);
                            }
                            if !props.services.is_empty() {
                                println!("   🔧 Services (adv): {}", props.services.len());
                            }
                            println!();
                        }
                    }
                    Ok(None) => {
                        println!("📡 Event stream ended");
                        break;
                    }
                    Err(_) => {
                        // per-event timeout; continue polling
                        continue;
                    }
                }
            }

            scan_round += 1;
            if scan_round > 4 {
                println!("🏁 Completed {} scanning rounds", scan_round - 1);
                break;
            }
            println!("⏸️ Round complete, short pause...\n");
            sleep(Duration::from_secs(2)).await;
        }

        // (Optional) Summary
        let total = device_info.len();
        let named = device_info.values().filter(|(n, _, _)| n != "Unknown").count();
        println!("📊 Summary: total={}, named={}, unknown={}", total, named, total - named);
        if named > 0 {
            println!("🏷️ Named devices:");
            for (addr, (n, _, attempts)) in device_info.iter() {
                if n != "Unknown" {
                    println!("   • {} ({}) - {} attempts", n, addr, attempts);
                }
            }
        }
    });

    // Let it run a while or until Ctrl+C
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\n🛑 Stopping scan...");
        }
        _ = sleep(Duration::from_secs(90)) => {
            println!("\n⏰ Scan timeout after 90 seconds");
        }
    }

    adapter.stop_scan().await?;
    handle.abort();

    println!("✅ Scan complete!");
    Ok(())
}

/// Cross-platform name resolution:
/// 1) use advertising `local_name` if present
/// 2) else connect → discover → read GAP Device Name (0x1800/0x2A00), then disconnect
async fn resolve_name_via_gap_name(p: &Peripheral) -> Option<String> {
    // Check if we already have a name in properties (advert)
    if let Ok(Some(props)) = p.properties().await {
        if let Some(n) = props.local_name {
            if !n.is_empty() {
                return Some(n);
            }
        }
    }

    // Connect only if needed, disconnect after if we connected
    let already_connected = p.is_connected().await.unwrap_or(false);
    if !already_connected {
        if let Err(e) = p.connect().await {
            eprintln!("[{}] connect failed: {:?}", p.id(), e);
            return None;
        }
    }

    // Populate GATT DB
    let _ = p.discover_services().await;

    // defining Uuid generic access (0x1800) and device name characteristics (0x2A00)
    let GAP_SERVICE: Uuid = Uuid::parse_str("00001800-0000-1000-8000-00805F9B34FB").unwrap();
    let DEVICE_NAME_CHAR: Uuid = Uuid::parse_str("00002A00-0000-1000-8000-00805F9B34FB").unwrap();
    

    // Find GAP service (0x1800) and read Device Name (0x2A00)
    let maybe_name = p
        .services()
        .iter()
        .find(|s| s.uuid == GAP_SERVICE)
        .and_then(|s| {
            s.characteristics
                .iter()
                .find(|c| c.uuid == DEVICE_NAME_CHAR)
                .cloned()
        })
        .and_then(|c: Characteristic| futures::executor::block_on(async {
            p.read(&c).await.ok()
        }))
        .and_then(|bytes| String::from_utf8(bytes).ok());

    if !already_connected {
        let _ = p.disconnect().await;
    }

    maybe_name
}

