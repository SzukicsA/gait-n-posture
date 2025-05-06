fn main() {
    println!("Hello, bluetooth!");
}

use tokio::main;            //load the package for bluetooth 

async main() {              //call the function to open the 
    println!("tokio loaded")
}

use btleplug::platform      //load bluetooth manager to connect with devices
Manager::new() {            //call the function to open the 
    println!("Manager loaded")
}

