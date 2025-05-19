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
        // device information
        use uuid::Uuid;
        let name_char_uuid = Uuid::parse_str("00002a00-0000-1000-8000-00805f9b34fb").unwrap();
    
        let peripherals = adapter.peripherals().await.unwrap();
        let mut valid_devices = vec![];

        for (i, peripheral) in peripherals.iter().enumerate() {
            let properties = peripheral.properties().await.unwrap();

            // collects information on devices
            let address = peripheral.address();

            // get advertised name
            let adv_name = properties
                .as_ref()
                .and_then(|p| p.local_name.clone())
                .unwrap_or("(none)".to_string());

            let mut gatt_name = "(unavailable)".to_string();

            if let Ok(_) = peripheral.connect().await {
                if let Ok(_) = peripheral.discover_services().await {
                    for service in peripheral.services() {
                        for characteristic in &service.characteristics {
                            if characteristic.uuid == name_char_uuid {
                                if let Ok(name_data) = peripheral.read(characteristic).await {
                                    gatt_name = String::from_utf8_lossy(&name_data).to_string();
                                }
                            }
                        }
                    }
                }
                // disconnect after reading
                let _ = peripheral.disconnect().await;
            }

            println!(
                "[{}] Adv Name: {} GATT Name: {} Address: {}",
                valid_devices.len(),
                adv_name,
                gatt_name,
                address
                );

            // save for selection
            valid_devices.push(peripheral.clone());
        };
        // 
        print!("Enter a number to connect with a device or 'q' to quit:");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            eprintln!("Failed to read input.");
            return;
        }

        let trimmed = input.trim();
        if trimmed == "q" {
            println!("Existing");
            return;
        }

        let selected: usize = match trimmed.parse() {
            Ok(n) => n,
            Err(_) => {
                eprintln!("Invalid input");
                return;
            }

        };

        // Error message is selection is invalid
        if selected >= valid_devices.len() {
            eprintln!("Invalid selection.");
            return;
        }

        // Connect to selected device
        let peripheral = &valid_devices[selected];
        peripheral.connect().await.unwrap();
        println!("Connected!");
        
        let connected = peripheral.is_connected().await.unwrap();
        println!("Connected? {}", connected);
}

