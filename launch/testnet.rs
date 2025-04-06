use std::process::{Command, exit};
use std::env;

fn main() {
    println!("Testnet Launch Initiated");

    // Choose between testnet or mainnet mode
    let mode = get_mode();

    // Clean and build the project before launching
    if let Err(e) = clean_and_build() {
        eprintln!("Error during build: {}", e);
        exit(1);
    }

    // Set parameters based on the mode
    let params = match mode.as_str() {
        "testnet" => get_testnet_params(),
        "mainnet" => get_mainnet_params(),
        _ => {
            eprintln!("Invalid mode. Choose 'testnet' or 'mainnet'.");
            exit(1);
        }
    };

    // Start the node with the chosen parameters
    if let Err(e) = start_node(&params) {
        eprintln!("Error starting node: {}", e);
        exit(1);
    }

    println!("Node successfully launched in {} mode!", mode);
}

// Get the mode (testnet or mainnet) from environment variables or user input
fn get_mode() -> String {
    // Default to "testnet" if not set in environment
    if let Ok(mode) = env::var("LAUNCH_MODE") {
        return mode;
    }

    // If LAUNCH_MODE is not set, ask the user
    println!("Please choose launch mode: (testnet/mainnet)");
    let mut mode = String::new();
    std::io::stdin().read_line(&mut mode).unwrap();
    mode.trim().to_string()
}

// Clean and build the project to ensure everything is up-to-date
fn clean_and_build() -> Result<(), String> {
    let clean_output = Command::new("cargo")
        .arg("clean")
        .output()
        .map_err(|e| format!("Failed to clean project: {}", e))?;

    if !clean_output.status.success() {
        return Err(format!("Cargo clean failed: {:?}", clean_output));
    }

    let build_output = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .output()
        .map_err(|e| format!("Failed to build project: {}", e))?;

    if !build_output.status.success() {
        return Err(format!("Cargo build failed: {:?}", build_output));
    }

    Ok(())
}

// Get testnet parameters
fn get_testnet_params() -> Vec<(&'static str, &'static str)> {
    vec![
        ("NODE_ADDR", "127.0.0.1:7878"),  // Local testnet node address
        ("API_ADDR", "127.0.0.1:8000"),   // API endpoint
        ("TOTAL_SUPPLY", "1000000000"),    // Lower supply for testing
        ("FEE", "1000"),                   // Lower fees for testnet transactions
        ("BLOCK_TIME", "10"),              // Block time in seconds
    ]
}

// Get mainnet parameters
fn get_mainnet_params() -> Vec<(&'static str, &'static str)> {
    vec![
        ("NODE_ADDR", "127.0.0.1:7878"),  // Local mainnet node address
        ("API_ADDR", "127.0.0.1:8000"),   // API endpoint
        ("TOTAL_SUPPLY", "1000000000000000"),  // Larger supply for mainnet
        ("FEE", "5000"),                   // Standard fee for transactions
        ("BLOCK_TIME", "5"),               // Faster block time for mainnet
    ]
}

// Start the node with the provided parameters
fn start_node(params: &[(&str, &str)]) -> Result<(), String> {
    // Set environment variables for the chosen mode
    for &(key, value) in params.iter() {
        env::set_var(key, value);
    }

    // Run the node with the correct parameters (in testnet or mainnet mode)
    let node_output = Command::new("cargo")
        .arg("run")
        .arg("--release")
        .output()
        .map_err(|e| format!("Failed to start node: {}", e))?;

    if !node_output.status.success() {
        return Err(format!("Node failed to start: {:?}", node_output));
    }

    Ok(())
}

