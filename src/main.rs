// 🔧 Import the Manager trait, which provides the `.adapters()` method.
use btleplug::api::{Characteristic, Manager as ManagerTrait, Peripheral, ScanFilter};

// 🔧 Import the platform-specific Manager struct and Adapter type.
// These are used to create a Bluetooth manager instance and represent an adapter (like a USB dongle or built-in BT).
use btleplug::platform::{Adapter, Manager as ManagerStruct};

// Central trait required to scan 
use btleplug::api::Central;
use tokio::select;

// import to allow interactive input
use std::io::{self, Write};
use std::string;

// import plug to connect to devices
// use btleplug::api::Peripheral;

// ⏱ Import sleep and Duration to pause the program later (e.g., while scanning for devices).
use tokio::time::{Duration, sleep};

// 🚀 This marks the asynchronous main function, run inside the Tokio async runtime.
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
                    adapter // 🎯 Store this adapter in the `adapters` variable
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
    println!("Scanning for 10 seconds");
    sleep(Duration::from_secs(10)).await;

    //Now print a list of devices
    let peripherals = adapter.peripherals().await.unwrap();
        for (i, peripheral) in peripherals.iter().enumerate() {
            let properties = peripheral.properties().await.unwrap();
            let address    = peripheral.address();
            let name       = properties
                .as_ref()
                .and_then(|p| p.local_name.clone())
                .unwrap_or("(unknown)".to_string());
            println!("[{}] Device: {}, Address: {}", i, name, address);
        };
        // 
        print!("Enter choose device to connect to (number):");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let selected: usize = input.trim().parse().unwrap();

        // Connect to selected device
        let peripheral = &peripherals[selected];
        peripheral.connect().await.unwrap();
        println!("Connected!");
        
        let connected = peripheral.is_connected().await.unwrap();
        println!("Connected? {}", connected);

        // device information
        use uuid::Uuid;

        // collecting additional information on devices
        peripheral.discover_services().await.unwrap();
        let name_char_uuid = Uuid::parse_str("00002a00-0000-1000-8000-00805f9b34fb").unwrap();

        for service in peripheral.services() {
            for characteristic in &service.characteristics {
                if characteristic.uuid == name_char_uuid {
                    let name_data = peripheral.read(characteristic).await.unwrap();
                    let name_string = String::from_utf8_lossy(&name_data);
                    println!("💡 Device name (via GATT): {}", name_string);
                }
            }
        }
}
