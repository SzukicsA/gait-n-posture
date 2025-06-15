// 🔧 Import the Manager trait, which provides the .adapters() method.
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
    for i in (1..=30).rev() {
        println!("...{}s", i);
        sleep(Duration::from_secs(1)).await;
    }

    //Now print a list of devices
        // device information
        let name_char_uuid = Uuid::parse_str("00002a00-0000-1000-8000-00805f9b34fb").unwrap();
    
        let peripherals = adapter.peripherals().await.unwrap();

        let mut valid_devices = vec![];

        for peripheral in peripherals.iter() {
            let Some(props) = peripheral.properties().await.unwrap() else{
                continue;
            };
        }
            
            // collects information on devices
            let address = peripheral.address();

            // get advertised name
            let mut name_display = "(no advertised name)".to_string();

            // skip devices that can't be connected to
            if let Some(props) = &properties {
                if let Some(name) = &props.local_name {
                    name_display = name.clone();
                }
            }

            println!(
                "[{}] Name: {}, Address: {}",
                valid_devices.len(),
                name_display,
                address
                );

            let mut gatt_name = None;

            if name_display == "(no advertised name)" {
                if let Ok(_) = peripheral.connect().await {
                    if let Ok(_) = peripheral.discover_services().await {
                        for service in peripheral.services() {
                            for characteristic in &service.characteristics {
                                if characteristic.uuid == name_char_uuid {
                                    if let Ok(name_data) = peripheral.read(characteristic).await {
                                        gatt_name = Some(String::from_utf8_lossy(&name_data).to_string());
                                    }
                                }
                            }
                        }
                    }
                    // Keep connection until deliberately broken
                    loop{
                        print!("Type 'd' to disconnect, 'q' to quit: ");
                        io::stdout().flush().unwrap();
                        let mut cmd = String::new();
                        io::stdin().read_line(&mut cmd).unwrap();
                        match cmd.trim() {
                            "d" => {
                                peripheral.disconnect().await.unwrap();
                                println!("Disconnected");
                                break;
                            }
                            "q" => break,
                            _ => println!("Unknown command"),
                        }
                    }
                }
            }

            let final_name = if name_display == "(no adcertised name)" {
                gatt_name
                    .map(|n| format!("GATT: {}", n))
                    .unwrap_or_else(|| "(unknown)".to_string())
            } else {
                format!("ADV: {}", name_display)
            };

            println!(
                "[{}] Name: {}, Address: {}",
                valid_devices.len(),
                final_name,
                address);
            valid_devices.push(peripheral.clone());

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

