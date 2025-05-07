use btleplug::platform::Manager;                        // for Bluetooth manager
use tokio::time::{sleep, Duration};                     // for pause/wait

fn start() {
    println!("Hello, tester!");
}

#[tokio::main] 
async fn main() {                                                                                                        // call the function to open tokio 
    let manager = match Manager::new().await {                                                                   // the function with a check if it loads?
        Ok(m) => {                                      // Message whether it loaded? 
            println!("Manager loaded");                            
            m
        },
        Err(e) => {
            eprintln!("Failed loading manager: {:?}",  // Message whether it didn't load?

            e);
            return;

        }
    };
}




