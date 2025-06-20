// 🔧 Import packages. first comes module and then 
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::Manager as ManagerStruct;
use tokio::time::{sleep, Duration};
use std::io::{self, Write};
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
    // after getting the adapter this function scans for available devices
    adapter.start_scan(ScanFilter::default()).await.unwrap(); // Scan for bluetooth devices
   // for i in (1..=30).rev() {
   //     println!("...{}s", i);
        sleep(Duration::from_secs(10)).await;
   // }

    //Now print a list of devices
        // device information
        let name_char_uuid = Uuid::parse_str("00002a00-0000-1000-8000-00805f9b34fb").unwrap();
    
        let peripherals = adapter.peripherals().await.unwrap();

        for peripheral in peripherals.iter() {
            let Some(properties) = peripheral.properties().await.unwrap() else{
                continue;
            };
            
            // collects information on devices
            let address = peripheral.address();

            // get advertised name
            let mut name_display = "(no advertised name)".to_string();

            // skip devices that can't be connected to_string
            if let Some(name) = &properties.local_name {
                    name_display = name.clone();
                }

            println!(
                "Found device: {} [{}]",
                name_display,
                address
                );
        }
}

