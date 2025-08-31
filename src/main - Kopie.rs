// 🔧 Import packages
use btleplug::api::{Central, Manager as _, Peripheral, Peripheral as _, ScanFilter, CentralEvent, Characteristic, UUID};
use btleplug::platform::Manager as ManagerStruct;
use futures::stream::StreamExt;
use std::collections::HashMap;
use tokio::time::{sleep, Duration, timeout};

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

    // 🔧 OPTIMIZATION 1: Filter for discoverable/pairable devices only
    let scan_filter = ScanFilter {
        services: vec![], // Empty means scan for all services
    };
    
    let mut events = adapter.events().await?;
    
    // 🔧 OPTIMIZATION 2: Start with a short burst scan, then restart
    println!("🔍 Starting scan for pairable/discoverable devices only...");
    println!("💡 Tip: Make sure your phone is in Bluetooth settings or pairing mode\n");

    let adapter_clone = adapter.clone();
    
    let handle = tokio::spawn(async move {
        let mut device_info: HashMap<String, (String, bool, u32)> = HashMap::new(); // (address -> (name, printed, attempt_count))
        let mut connection_attempts: HashMap<String, bool> = HashMap::new();
        let mut scan_round = 1;
        
        // 🔧 OPTIMIZATION 3: Multiple scan cycles with different approaches
        loop {
            println!("🔄 Scan Round {} - Using different discovery strategy", scan_round);
            
            // Restart scan with new parameters
            if let Err(e) = adapter_clone.stop_scan().await {
                println!("⚠️ Warning stopping previous scan: {:?}", e);
            }
            
            // Brief pause between scans
            sleep(Duration::from_millis(500)).await;
            
            // Restart scan with pairable-device filter
            if let Err(e) = adapter_clone.start_scan(scan_filter.clone()).await {
                println!("❌ Error restarting scan: {:?}", e);
                break;
            }
            
            // 🔧 OPTIMIZATION 4: Process events with timeout per round
            let round_timeout = Duration::from_secs(15);
            let round_start = std::time::Instant::now();
            
            while round_start.elapsed() < round_timeout {
                // Use a shorter timeout for individual events
                match timeout(Duration::from_millis(100), events.next()).await {
                    Ok(Some(event)) => {
                        let (id, is_discovery) = match &event {
                            CentralEvent::DeviceDiscovered(id) => (id.clone(), true),
                            CentralEvent::DeviceUpdated(id) => (id.clone(), false),
                            _ => continue,
                        };
                        
                        if let Ok(peripheral) = adapter_clone.peripheral(&id).await {
                            if let Ok(Some(properties)) = peripheral.properties().await {
                                // 🔧 Filter for pairable devices only
                                let is_connectable = peripheral.is_connected().await.unwrap_or(false) || 
                                                   properties.rssi.is_some(); // Devices with RSSI are usually discoverable
                                
                                // Skip devices that don't seem pairable/discoverable
                                if !is_connectable && properties.local_name.is_none() && properties.services.is_empty() {
                                    continue;
                                }
                                let address = properties.address.to_string();
                                let name = properties.local_name.unwrap_or_else(|| "Unknown".to_string());
                                
                                let (old_name, was_printed, attempt_count) = device_info
                                    .get(&address)
                                    .cloned()
                                    .unwrap_or(("Unknown".to_string(), false, 0));
                                
                                let new_attempt_count = attempt_count + 1;
                                
                                let should_print = match (&old_name[..], &name[..]) {
                                    ("Unknown", n) if n != "Unknown" => {
                                        // Name was resolved!
                                        device_info.insert(address.clone(), (name.clone(), true, new_attempt_count));
                                        true
                                    }
                                    (old, new) if old != new => {
                                        // Name changed
                                        device_info.insert(address.clone(), (name.clone(), true, new_attempt_count));
                                        true
                                    }
                                    ("Unknown", "Unknown") if !was_printed && new_attempt_count == 1 => {
                                        // First time seeing this unknown device
                                        device_info.insert(address.clone(), (name.clone(), true, new_attempt_count));
                                        true
                                    }
                                    _ => {
                                        // Update attempt count but don't print
                                        device_info.insert(address.clone(), (name.clone(), was_printed, new_attempt_count));
                                        false
                                    }
                                };
                                
                                if should_print {
                                    let event_type = if is_discovery {
                                        "🔍 Discovered"
                                    } else {
                                        "🏷️ Name resolved"
                                    };
                                    
                                    println!("{}: {} ({})", event_type, name, properties.address);
                                    
                                    if let Some(rssi) = properties.rssi {
                                        println!("   📶 Signal: {} dBm", rssi);
                                    }
                                    
                                    if !properties.services.is_empty() {
                                        println!("   🔧 Services: {}", properties.services.len());
                                    }
                                    
                                    // 🔧 OPTIMIZATION 5: Try to get more info for unknown devices
                                    if name == "Unknown" && new_attempt_count <= 3 {
                                        println!("   🔍 Attempting to get more device info...");
                                        
                                        // Try to connect briefly to get more info (risky but might work)
                                        if !connection_attempts.contains_key(&address) {
                                            connection_attempts.insert(address.clone(), true);
                                            
                                            let peripheral_clone = peripheral.clone();
                                            let addr_clone = address.clone();
                                            
                                            // Spawn a quick connection attempt
                                            tokio::spawn(async move {
                                                match timeout(Duration::from_secs(3), peripheral_clone.connect()).await {
                                                    Ok(Ok(())) => {
                                                        println!("   ✅ Quick connect successful for {}", addr_clone);
                                                        
                                                        // Try to discover services quickly
                                                        match timeout(Duration::from_secs(2), peripheral_clone.discover_services()).await {
                                                            Ok(Ok(())) => {
                                                                let services = peripheral_clone.services();
                                                                if !services.is_empty() {
                                                                    println!("   🔧 Found {} services via connection", services.len());
                                                                    
                                                                    // Look for common service UUIDs to identify device type
                                                                    for service in services.iter().take(3) {
                                                                        let uuid_str = service.uuid.to_string();
                                                                        let device_type = match uuid_str.as_str() {
                                                                            "0000180f-0000-1000-8000-00805f9b34fb" => "Battery Service",
                                                                            "0000180a-0000-1000-8000-00805f9b34fb" => "Device Info",
                                                                            "00001812-0000-1000-8000-00805f9b34fb" => "HID Device",
                                                                            "0000110b-0000-1000-8000-00805f9b34fb" => "Audio Sink",
                                                                            "0000110e-0000-1000-8000-00805f9b34fb" => "A/V Remote",
                                                                            _ => &uuid_str[..8],
                                                                        };
                                                                        println!("      • {}: {}", device_type, uuid_str);
                                                                    }
                                                                }
                                                            }
                                                            _ => {
                                                                // Service discovery failed or timed out
                                                            }
                                                        }
                                                        
                                                        // Disconnect quickly
                                                        let _ = peripheral_clone.disconnect().await;
                                                    }
                                                    _ => {
                                                        // Connection failed, that's normal for many devices
                                                    }
                                                }
                                            });
                                        }
                                    }
                                    
                                    // Show progress for persistent unknowns
                                    if name == "Unknown" && new_attempt_count > 5 {
                                        println!("   ⏱️ {} attempts, still unknown (privacy device)", new_attempt_count);
                                    }
                                    
                                    println!();
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        // Stream ended
                        println!("📡 Event stream ended");
                        break;
                    }
                    Err(_) => {
                        // Timeout - continue to next round
                        continue;
                    }
                }
            }
            
            scan_round += 1;
            
            // 🔧 OPTIMIZATION 6: Stop after several rounds or if interrupted
            if scan_round > 4 {
                println!("🏁 Completed {} scanning rounds", scan_round - 1);
                break;
            }
            
            println!("⏸️ Round {} complete, brief pause before next round...\n", scan_round - 1);
            sleep(Duration::from_secs(2)).await;
        }
        
        // Final summary
        let total_devices = device_info.len();
        let named_devices = device_info.values().filter(|(name, _, _)| name != "Unknown").count();
        
        println!("📊 Final Summary:");
        println!("   Total devices found: {}", total_devices);
        println!("   Named devices: {}", named_devices);
        println!("   Unknown devices: {}", total_devices - named_devices);
        
        if named_devices > 0 {
            println!("\n🏷️ Named devices found:");
            for (addr, (name, _, attempts)) in device_info.iter() {
                if name != "Unknown" {
                    println!("   • {} ({}) - {} attempts", name, addr, attempts);
                }
            }
        }
    });

    // Run for a longer time to allow multiple scan rounds
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
    
    println!("✅ Windows-optimized scan complete!");
    Ok(())
}

