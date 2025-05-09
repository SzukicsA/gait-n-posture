use btleplug::platform::{Adapter, Manager};                         // for Bluetooth manager
use tokio::time::{sleep, Duration};                                 // for pause/wait

fn start() {
    println!("Hello, tester!");
}

#[tokio::main] 
async fn main() {                                                   // call the function to open tokio 
    let manager = match Manager::new().await {                      // the function with a check if it loads?
        Ok(m) => {                                                  // Message whether it loaded? 
            println!("Manager loaded");                            
            m
        },
        Err(e) => {
            eprintln!("Failed loading manager: {:?}",               // Message whether it didn't load?

            e);
            return;

        }
    };

    let adapters_result = manager.adapters().await;

    let adapters = match adapters_result {
    Ok(list) => {
        // get first adapter
        match list.intro_inter().nth(0) {
            Some(adapter) => {
                println!("Adapter found!");
                adapter
            },
            None => {
                eprintln!("No adapters found!");
                return;
            }            
        }
    },
    Err(e) => {
        eprintln!("Failed to get adapters: {:?}",
            e);
    }
};


