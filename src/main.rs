// 🔧 Import packages. first comes module and then 
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::Manager as ManagerStruct;
use tokio::time::{sleep, Duration};
// use std::io::{self, Write};
// use uuid::Uuid;

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
        sleep(Duration::from_secs(30)).await;
    // }

    //Now print a list of devices
        // device information
        // let name_char_uuid = Uuid::parse_str("00002a00-0000-1000-8000-00805f9b34fb").unwrap();
    
        // look and list devices to a list that have been scanned beforehand
        let peripherals = adapter.peripherals().await.unwrap(); // await mean wait until done with the operation and unwrap mean extract results

        for peripheral in peripherals.iter() {
        // collects information on devices
        let address = peripheral.address();

            if let Err(e) = peripheral.connect().await {
                eprintln!("Could not connect to device {}: {:?}", address, e);
                continue;
            }

            if let Err(e) = peripheral.discover_services().await {
                eprintln!("Could not discover services: {:?}", e);
                continue;
            }

            let services = peripheral.services();

            for services in services {
                println!("  services; {}", services.uuid);
            }

            if let Err(e) = peripheral.disconnect().await {
                eprintln!("could not disconnect: {:?}", e);
            }

            let Some(properties) = peripheral.properties().await.unwrap() else{
                continue;
            };
            
            // get advertised name
            let name_display = match &properties.local_name{
                Some(name) => name.clone(),
                None => "(no name in advertisement)".to_string(),
            };

            println!("Found devices: {} [{}]", name_display, address);
            
            // skip devices that can't be connected to_string
            //if let Some(name) = &properties.local_name {
            //        name_display = name.clone();
            //    }

            // println!(
            //          "Found device: {} [{}]",
            //          name_display,
            //          address
            //          );

       }
}




 
