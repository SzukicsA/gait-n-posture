// 🔧 Import packages. first comes module and then 
use btleplug::api::{Central, CharPropFlags, Characteristic, Manager as _, Peripheral, ScanFilter, CentralEvent};
use btleplug::platform::Manager as ManagerStruct;
use tokio::time::{sleep, Duration};
use futures::stream::StreamExt;
// use std::io::{self, Write};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    // 📦 Create a new instance of the Bluetooth manager (platform-specific).
    // It gives you access to the available Bluetooth adapters on your machine.
    let manager = match ManagerStruct::new().await {
        Ok(m) => {
            println!("Manager loaded"); // ✅ Successfully created the manager
            m
        }
        Err(e) => {
            // ❌ Could not create the manager (e.g., Bluetooth not available)
            eprintln!("Failed loading manager: {:?}", e);
            return; // 🚪 Exit early since we can't continue
        }
    };

    // 🔍 Ask the manager to list all available Bluetooth adapters (async call).
    let adapters_result = manager.adapters().await;

    // 🔁 Handle the result: either a list of adapters, or an error.
    let adapter = match adapters_result {
        Ok(list) => {
            // 📦 Get the first available adapter (most systems only have one).
            match list.into_iter().nth(0) {
                Some(adapter) => {
                    println!("Adapter found!"); // ✅ We got an adapter to work with
                    adapter // 🎯 Store this adapter in the adapters variable
                }
                None => {
                    eprintln!("No adapters found!"); // ❌ No adapter was found (unexpected)
                    return;
                }
            }
        }
        Err(e) => {
            // ❌ Failed to fetch the list of adapters (OS or hardware issue)
            eprintln!("Failed to get adapters: {:?}", e);
            return;
        }
    };

let mut events = adapter.events().await.unwrap();  
    // after getting the adapter this function scans for available devices
    adapter.start_scan(ScanFilter::default()).await.unwrap(); // Scan for bluetooth discover_services
    sleep(Duration::from_secs(30)).await;

    //Now print a list of devices
        // device information
        //let name_char_uuid = Uuid::parse_str("00002a00-0000-1000-8000-00805f9b34fb").unwrap();
    
        // look and list devices to a list that have been scanned beforehand
        let peripherals = adapter.peripherals().await.unwrap(); // await mean wait until done with the operation and unwrap mean extract results

tokio::spawn(async move {  
    while let Some(event) = events.next().await {  
        match event {  
            CentralEvent::DeviceDiscovered(id) => {  
                if let Ok(peripheral) = adapter.peripheral(&id).await {  
                    if let Ok(Some(properties)) = peripheral.properties().await {  
                        let name = properties.local_name.unwrap_or("Unknown".to_string());  
                        println!("Discovered: {} ({})", name, properties.address);  
                    }  
                }  
            }  
            CentralEvent::DeviceUpdated(id) => {  
                // Device properties updated - might now have a name  
                if let Ok(peripheral) = adapter.peripheral(&id).await {  
                    if let Ok(Some(properties)) = peripheral.properties().await {  
                        if let Some(name) = properties.local_name {  
                            println!("Updated: {} ({})", name, properties.address);  
                        }  
                    }  
                }  
            }  
            _ => {}  
        }  
    }  
});


}

