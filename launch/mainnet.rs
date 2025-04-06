use cacia::{Network, Node};

fn main() {
    let launch_mode = std::env::var("LAUNCH_MODE").unwrap_or_else(|_| "mainnet".to_string());

    if launch_mode == "mainnet" {
        println!("Launching Cacia node in MAINNET mode...");
    } else {
        println!("Unknown launch mode. Defaulting to MAINNET.");
    }

    // Set the network configuration to mainnet (mainnet peers, genesis block, etc.)
    let network = Network::new("mainnet"); // Replace with mainnet configuration

    let node = Node::new(network);

    // Start the node
    node.start();

    println!("Mainnet node running on 127.0.0.1:7878");

    // Wait for the node to run (you can adjust the sleep duration)
    std::thread::sleep(std::time::Duration::from_secs(60));
}
