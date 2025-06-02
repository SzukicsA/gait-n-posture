
use btleplug::api::{Manager as ManagerTrait, Peripheral, ScanFilter};
use btleplug::platform::{Manager as ManagerStruct};
use btleplug::api::Central;
use tokio::time::{Duration, sleep};
use std::io::{self, Write};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let manager = match ManagerStruct::new().await {
        Ok(m) => {
            println!("Manager loaded");
            m
        }
        Err(e) => {
            eprintln!("Failed loading manager: {:?}", e);
            return;
        }
    };

    let adapter = match manager.adapters().await {
        Ok(mut adapters) => match adapters.pop() {
            Some(a) => {
                println!("Adapter found!");
                a
            }
            None => {
                eprintln!("No adapters found!");
                return;
            }
        },
        Err(e) => {
            eprintln!("Failed to get adapters: {:?}", e);
            return;
        }
    };

    adapter.start_scan(ScanFilter::default()).await.unwrap();
    println!("Scanning for 10 seconds...");
    sleep(Duration::from_secs(10)).await;

    let name_char_uuid = Uuid::parse_str("00002a00-0000-1000-8000-00805f9b34fb").unwrap();
    let peripherals = adapter.peripherals().await.unwrap();

    let mut valid_devices = vec![];

    println!("Discovered devices:");
    for (i, peripheral) in peripherals.iter().enumerate() {
        let properties = peripheral.properties().await.unwrap();
        let address = peripheral.address();

        let adv_name = properties
            .as_ref()
            .and_then(|p| p.local_name.clone());

        let mut gatt_name = None;

        // Try to get GATT name only if no advertised name
        if adv_name.is_none() {
            if peripheral.connect().await.is_ok() {
                if peripheral.discover_services().await.is_ok() {
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
                let _ = peripheral.disconnect().await;
            }
        }

        let final_name = adv_name
            .map(|n| format!("ADV: {}", n))
            .or_else(|| gatt_name.map(|n| format!("GATT: {}", n)))
            .unwrap_or("(unknown)".to_string());

        println!("[{}] Name: {}, Address: {}", valid_devices.len(), final_name, address);
        valid_devices.push(peripheral.clone());
    }

    // Device selection
    print!("Enter a number to connect with a device or 'q' to quit: ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        eprintln!("Failed to read input.");
        return;
    }

    let trimmed = input.trim();
    if trimmed == "q" {
        println!("Exiting.");
        return;
    }

    let selected: usize = match trimmed.parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("Invalid input.");
            return;
        }
    };

    if selected >= valid_devices.len() {
        eprintln!("Invalid selection.");
        return;
    }

    let peripheral = &valid_devices[selected];
    println!("Connecting to device...");
    if peripheral.connect().await.is_err() {
        eprintln!("Failed to connect.");
        return;
    }

    println!("Connected to {}", peripheral.address());

    // Keep connection until user chooses to disconnect
    loop {
        print!("Type 'd' to disconnect, 'q' to quit: ");
        io::stdout().flush().unwrap();
        let mut cmd = String::new();
        io::stdin().read_line(&mut cmd).unwrap();
        match cmd.trim() {
            "d" => {
                peripheral.disconnect().await.unwrap();
                println!("Disconnected.");
                break;
            }
            "q" => break,
            _ => println!("Unknown command."),
        }
    }
}
