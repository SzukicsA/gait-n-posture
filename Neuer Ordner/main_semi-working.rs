// Cargo.toml (for reference)
// [package]
// name = "gatt_name_scanner"
// version = "0.1.0"
// edition = "2021"
//
// [dependencies]
// anyhow = "1"
// tokio = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
// btleplug = "0.11"     // or your current version
// uuid = "1"
// futures = "0.3"
// tracing = "0.1"
// tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }

use anyhow::{bail, Context, Result};
use btleplug::api::{
    Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::{info, warn, error, debug};
use tracing_subscriber::EnvFilter;
use futures::stream::StreamExt;
use uuid::Uuid;

// ---- OS-tuned timeouts ------------------------------------------------------
#[cfg(target_os = "windows")]
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
#[cfg(not(target_os = "windows"))]
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(target_os = "windows")]
const DISCOVER_TIMEOUT: Duration = Duration::from_secs(6);
#[cfg(not(target_os = "windows"))]
const DISCOVER_TIMEOUT: Duration = Duration::from_secs(4);

#[cfg(target_os = "windows")]
const READ_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(not(target_os = "windows"))]
const READ_TIMEOUT: Duration = Duration::from_secs(3);

// ---- GAP service + Device Name characteristic -------------------------------
const GAP_SERVICE: Uuid = Uuid::from_u128(0x00001800_0000_1000_8000_00805F9B34FB);
const DEVICE_NAME_CHAR: Uuid = Uuid::from_u128(0x00002A00_0000_1000_8000_00805F9B34FB);

// serialize connects so BlueZ doesn't get cranky
const MAX_PARALLEL_CONNECTS: usize = 1;

// how long to scan before we give up (press Ctrl+C to quit earlier)
const SCAN_WARMUP: Duration = Duration::from_secs(5);

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging();

    // 1) Get an adapter
    let manager = Manager::new().await.context("create btle manager")?;
    let adapters = manager.adapters().await.context("list adapters")?;
    let adapter = adapters
        .into_iter()
        .next()
        .context("no bluetooth adapters found")?;

    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()) // respects RUST_LOG
        .try_init();


    info!("using adapter");

    // 2) Start scan
    adapter
        .start_scan(ScanFilter::default())
        .await
        .context("start scan")?;
    info!("scanning… warming up for {SCAN_WARMUP:?}");
    sleep(SCAN_WARMUP).await;

    // 3) Subscribe to events
    let mut events = adapter.events().await.context("events stream")?;

    // Track which device IDs we already attempted to resolve
    let mut seen: HashSet<String> = HashSet::new();

    // 4) Event loop
    while let Some(evt) = events.next().await {
        match evt {
            CentralEvent::DeviceDiscovered(id)
            | CentralEvent::DeviceUpdated(id)
            | CentralEvent::ManufacturerDataAdvertisement { id, .. }
            | CentralEvent::ServiceDataAdvertisement { id, .. } => {
                // ensure we pull the latest Peripheral handle
                if let Ok(p) = adapter.peripheral(&id).await {
                    if handle_device(&adapter, &p, &mut seen).await? {
                        // handled (logged)
                    }
                }
            }
            CentralEvent::DeviceConnected(id) => {
                info!("connected: {id:?}");
            }
            CentralEvent::DeviceDisconnected(id) => {
                info!("disconnected: {id:?}");
            }
            _ => {}
        }
    }

    Ok(())
}

/// For each peripheral we see/updated, try to print a name. Prefer advertisement name.
/// If missing, attempt a serialized GATT connect → read 0x2A00 Device Name.
/// Returns true if we logged something.
async fn handle_device(adapter: &Adapter, p: &Peripheral, seen: &mut HashSet<String>) -> Result<bool> {
    let id = p.id();
    let id_s = format!("{id:?}");

    // de-dup frequent updates
    if !seen.insert(id_s.clone()) {
        // seen this device id before; still allow events but avoid spamming connects
        // only log advertisement name if newly available
    }

    let props = p.properties().await?;
    let adv_name = props
        .as_ref()
        .and_then(|pp| pp.local_name.clone())
        .filter(|s| !s.trim().is_empty());

    if let Some(name) = adv_name {
        info!("adv name: {name}  ({id_s})  rssi={}", props.and_then(|pp| pp.rssi).unwrap_or_default());
        return Ok(true);
    }

    // No advertised name — try via GATT, but serialize connects
    static SEMAPHORE: tokio::sync::OnceCell<tokio::sync::Semaphore> = tokio::sync::OnceCell::const_new();
    let sem = SEMAPHORE
        .get_or_init(|| async { tokio::sync::Semaphore::new(MAX_PARALLEL_CONNECTS) })
        .await;

    let permit = sem.acquire().await.unwrap();
    let name_via_gatt = get_device_name_via_gatt(p).await?;
    drop(permit);

    if let Some(name) = name_via_gatt {
        info!("gatt name: {name}  ({id_s})");
        Ok(true)
    } else {
        info!("no name (adv/gatt) for {id_s}");
        Ok(true)
    }
}

/// Read the Device Name (0x2A00) via GATT. No nested executors, all async with timeouts.
/// Connects only if needed and disconnects only if it connected here.
async fn get_device_name_via_gatt(p: &Peripheral) -> Result<Option<String>> {
    let already_connected = p.is_connected().await.unwrap_or(false);
    let mut connected_here = false;

    if !already_connected {
        match timeout(CONNECT_TIMEOUT, p.connect()).await {
            Ok(Ok(())) => connected_here = true,
            _ => return Ok(None), // timeout or error
        }
    }

    // Some backends need an explicit discover to populate services/chars
    let _ = timeout(DISCOVER_TIMEOUT, p.discover_services()).await;

    // Try to locate Device Name characteristic in the cache
    let maybe_char = p
        .characteristics()
        .into_iter()
        .find(|c| c.uuid == DEVICE_NAME_CHAR);

    let name = if let Some(ch) = maybe_char {
        match timeout(READ_TIMEOUT, p.read(&ch)).await {
            Ok(Ok(bytes)) => String::from_utf8(bytes).ok(),
            _ => None,
        }
    } else {
        // If not in char cache, try to find GAP service and then the char
        let has_gap = p.services().iter().any(|s| s.uuid == GAP_SERVICE);
        if has_gap {
            // services are known but char missing; some stacks only expose after read attempt
            None
        } else {
            // no services visible; give up quietly
            None
        }
    };

    if connected_here {
        let _ = timeout(Duration::from_secs(3), p.disconnect()).await;
    }

    Ok(name)
}

// ---------- tiny logging helper ----------
fn setup_logging() {
    use tracing_subscriber::EnvFilter;
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init();
}

#[allow(unused_macros)]
macro_rules! info {
    ($($t:tt)*) => { tracing::info!($($t)*) }
}

