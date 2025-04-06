use std::process::{Command, exit};

fn main() {
    println!("Cacia (CC) Mainnet Launch Initiated");

    // Make sure to run a clean build
    if let Err(e) = clean_and_build() {
        eprintln!("Error during build: {}", e);
        exit(1);
    }

    // Set mainnet parameters
    let mainnet_params = [
        ("NODE_ADDR", "127.0.0.1:7878"),
        ("API_ADDR", "127.0.0.1:8000"),
        ("TOTAL_SUPPLY", "1000000000000000"), // 1 million CC, adjust as needed
        ("FEE", "5000"), // Transaction fee in smallest units
    ];

    // Execute node with mainnet parameters
    if let Err(e) = start_mainnet_node(&mainnet_params) {
        eprintln!("Error starting mainnet node: {}", e);
        exit(1);
    }

    println!("Cacia (CC) Mainnet successfully launched!");
}

fn clean_and_build() -> Result<(), String> {
    // Run cargo clean to remove old builds
    let clean_output = Command::new("cargo")
        .arg("clean")
        .output()
        .map_err(|e| format!("Failed to clean project: {}", e))?;

    if !clean_output.status.success() {
        return Err(format!("Cargo clean failed: {:?}", clean_output));
    }

    // Run cargo build to compile the mainnet
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

fn start_mainnet_node(params: &[(&str, &str)]) -> Result<(), String> {
    // Set environment variables for the mainnet parameters
    for &(key, value) in params.iter() {
        std::env::set_var(key, value);
    }

    // Run the mainnet node with the appropriate environment variables
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
