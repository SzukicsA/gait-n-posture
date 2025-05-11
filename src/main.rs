// 🔧 Import the Manager trait, which provides the `.adapters()` method.
use btleplug::api::{Manager as ManagerTrait, ScanFilter};

// 🔧 Import the platform-specific Manager struct and Adapter type.
// These are used to create a Bluetooth manager instance and represent an adapter (like a USB dongle or built-in BT).
use btleplug::platform::{Adapter, Manager as ManagerStruct};


// 🔧 Import the ScanFilter function
use btleplug::api:ScanFilter;

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
    let adapters = match adapters_result {
        Ok(list) => {
            // 📦 Get the first available adapter (most systems only have one).
            match list.into_iter().nth(0) {
                Some(adapter) => {
                    println!("Adapter found!"); // ✅ We got an adapter to work with
                    adapter, // 🎯 Store this adapter in the `adapters` variable
                    adapter.start_scan(ScanFilter::default()).await.unwrap(); // Scan for bluetooth devices
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
}

