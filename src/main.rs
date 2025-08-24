
// 🔧 Import packages
use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter, CentralEvent};
use btleplug::platform::Manager as ManagerStruct;
use futures::stream::StreamExt;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 📦 Create a new instance of the Bluetooth manager (platform-specific)
    let manager = ManagerStruct::new().await?;
    println!("Manager loaded");

    // 🔍 Get the first available Bluetooth adapter
    let adapters = manager.adapters().await?;
    let adapter = adapters
        .into_iter()
        .nth(0)
        .ok_or("No Bluetooth adapter found")?;
    
    println!("Adapter found!");

    // Get event stream before starting scan
    let mut events = adapter.events().await?;
    
    // Start scanning for devices
    adapter.start_scan(ScanFilter::default()).await?;
    println!("Scanning for Bluetooth devices... Press Ctrl+C to stop");

    // Clone adapter for use in the spawned task
    let adapter_clone = adapter.clone();
    
    // Spawn a task to handle discovered devices
    let handle = tokio::spawn(async move {
        let mut device_info: HashMap<String, (String, bool)> = HashMap::new(); // (address -> (name, printed))
        let mut update_count: HashMap<String, u32> = HashMap::new(); // Track update attempts
        
        while let Some(event) = events.next().await {
            // Determine event type before matching to avoid borrowing issues
            let (id, is_discovery) = match &event {
                CentralEvent::DeviceDiscovered(id) => (id.clone(), true),
                CentralEvent::DeviceUpdated(id) => (id.clone(), false),
                _ => continue, // Skip other event types
            };
            
            if let Ok(peripheral) = adapter_clone.peripheral(&id).await {
                if let Ok(Some(properties)) = peripheral.properties().await {
                    let address = properties.address.to_string();
                    let name = properties
                        .local_name
                        .unwrap_or_else(|| "Unknown".to_string());
                    
                    // Count updates for this device
                    let updates = update_count.entry(address.clone()).or_insert(0);
                    if !is_discovery {
                        *updates += 1;
                    }
                    
                    // Check if this is a new device or if the name has changed from "Unknown"
                    let should_print = match device_info.get(&address) {
                        None => {
                            // New device - always print
                            device_info.insert(address.clone(), (name.clone(), true));
                            true
                        }
                        Some((old_name, _printed)) => {
                            if old_name == "Unknown" && name != "Unknown" {
                                // Device name was resolved from Unknown to actual name
                                device_info.insert(address.clone(), (name.clone(), true));
                                true
                            } else if old_name != &name {
                                // Device name changed to something different
                                device_info.insert(address.clone(), (name.clone(), true));
                                true
                            } else {
                                // No change in name
                                false
                            }
                        }
                    };
                    
                    if should_print {
                        let event_type = if is_discovery {
                            "🔍 Discovered"
                        } else {
                            "📝 Name resolved"
                        };
                        
                        println!("{}: {} ({})", event_type, name, properties.address);
                        
                        // Show additional info if available
                        if let Some(rssi) = properties.rssi {
                            println!("   📶 Signal strength: {} dBm", rssi);
                        }
                        if !properties.services.is_empty() {
                            println!("   🔧 Services: {} available", properties.services.len());
                        }
                        
                        // Show how many updates we've seen for devices still showing as "Unknown"
                        if name == "Unknown" && *updates > 5 {
                            println!("   ⏱️  Still unknown after {} updates (likely privacy-enabled device)", updates);
                        }
                        
                        println!(); // Empty line for readability
                    }
                }
            }
        }
    });

    // Wait for Ctrl+C signal
    tokio::signal::ctrl_c().await?;
    println!("\nShutting down...");
    
    // Stop scanning and abort the task
    adapter.stop_scan().await?;
    handle.abort();
    
    Ok(())
}
