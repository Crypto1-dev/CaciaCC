use cacia::{Network, Node};

fn main() {
    let launch_mode = std::env::var("LAUNCH_MODE").unwrap_or_else(|_| "testnet".to_string());

    if launch_mode == "testnet" {
        println!("Launching Cacia node in TESTNET mode...");
    } else {
        println!("Unknown launch mode. Defaulting to TESTNET.");
    }

    // Configure the node for single-node mode
    let network = Network::new_single_node("127.0.0.1:7878"); // Local address, no P2P required

    let node = Node::new(network);
    
    // Start the node
    node.start();

    // Perform additional solo testnet initialization tasks if necessary
    println!("Solo Testnet node running on 127.0.0.1:7878");

    // Wait for the node to run (you could add more logic for interactions here)
    std::thread::sleep(std::time::Duration::from_secs(60));
}
