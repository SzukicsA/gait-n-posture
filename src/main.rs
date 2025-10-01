use btleplug::api::Manager as _;
use anyhow::Result;
use btleplug::api::{Central, Peripheral as _, ScanFilter, Characteristic};
use btleplug::platform::{Manager as ManagerStruct, Peripheral, Adapter};
use uuid::Uuid;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    let manager = ManagerStruct::new().await?;
    let adapter = manager.adapters().await?.into_iter().next().ok_or_else(|| anyhow::anyhow!("no adapter"))?;

    // ✅ call the finder INSIDE main, after you have `adapter`
    let maybe = find_device_by_name(&adapter, "A's S20+", 90).await?;
    match maybe {
        Some(p) => println!("✅ Device Found : {:?}", p.id()),
        None => println!("❌ Not found within timeout"),
    }

    Ok(())
}

pub async fn find_device_by_name(adapter: &Adapter, target_name: &str, timeout_s: u64)
    -> Result<Option<Peripheral>>
{
    adapter.start_scan(ScanFilter { services: vec![] }).await?;
    sleep(Duration::from_secs(2)).await; // warm-up

    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_s);
    while std::time::Instant::now() < deadline {
        for p in adapter.peripherals().await? {
            if let Some(props) = p.properties().await? {
                if let Some(n) = &props.local_name {
                    if n == target_name { let _ = adapter.stop_scan().await; return Ok(Some(p)); }
                }
            }
            if let Some(n) = resolve_name_via_gap_name(&p).await {
                if n == target_name { let _ = adapter.stop_scan().await; return Ok(Some(p)); }
            }
        }
        sleep(Duration::from_millis(800)).await;
    }

    let _ = adapter.stop_scan().await;
    Ok(None)
}

pub async fn resolve_name_via_gap_name(p: &Peripheral) -> Option<String> {
    if let Ok(Some(props)) = p.properties().await {
        if let Some(n) = props.local_name { if !n.is_empty() { return Some(n); } }
    }
    let already = p.is_connected().await.ok()?;
    if !already && p.connect().await.is_err() { return None; }
    let _ = p.discover_services().await;

    let gap  = Uuid::parse_str("00001800-0000-1000-8000-00805F9B34FB").ok()?;
    let devn = Uuid::parse_str("00002A00-0000-1000-8000-00805F9B34FB").ok()?;

    let name = p.services().iter()
        .find(|s| s.uuid == gap)
        .and_then(|s| s.characteristics.iter().find(|c| c.uuid == devn).cloned())
        .and_then(|c: Characteristic| futures::executor::block_on(async { p.read(&c).await.ok() }))
        .and_then(|bytes| String::from_utf8(bytes).ok());

    if !already { let _ = p.disconnect().await; }
    name
}
