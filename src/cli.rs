use clap::{App, Arg, SubCommand};
use tokio;
use std::{fs, io::Write};
use ed25519_dalek::{Keypair, PublicKey, SecretKey};
use rand::rngs::OsRng;

#[tokio::main]
async fn main() {
    let matches = App::new("Cacia (CC) CLI")
        .version("0.1.0")
        .author("Zone-crypto-ZNE")
        .about("Cacia cryptocurrency command-line tool")
        .subcommand(
            SubCommand::with_name("balance")
                .about("Check the balance of a Cacia wallet")
                .arg(Arg::with_name("wallet")
                    .help("The wallet address or username")
                    .required(true)
                    .index(1)),
        )
        .subcommand(
            SubCommand::with_name("send")
                .about("Send Cacia to another wallet")
                .arg(Arg::with_name("from")
                    .help("The sending wallet address")
                    .required(true)
                    .index(1))
                .arg(Arg::with_name("to")
                    .help("The receiving wallet address")
                    .required(true)
                    .index(2))
                .arg(Arg::with_name("amount")
                    .help("Amount of Cacia to send")
                    .required(true)
                    .index(3)),
        )
        .subcommand(
            SubCommand::with_name("create_account")
                .about("Create a new Cacia wallet account")
                .arg(Arg::with_name("wallet_name")
                    .help("The name to assign to the wallet")
                    .required(true)
                    .index(1)),
        )
        .get_matches();

    match matches.subcommand() {
        ("balance", Some(sub_matches)) => {
            let wallet = sub_matches.value_of("wallet").unwrap();
            check_balance(wallet).await;
        }
        ("send", Some(sub_matches)) => {
            let from_wallet = sub_matches.value_of("from").unwrap();
            let to_wallet = sub_matches.value_of("to").unwrap();
            let amount = sub_matches.value_of("amount").unwrap();
            send_transaction(from_wallet, to_wallet, amount).await;
        }
        ("create_account", Some(sub_matches)) => {
            let wallet_name = sub_matches.value_of("wallet_name").unwrap();
            create_account(wallet_name).await;
        }
        _ => eprintln!("Invalid subcommand."),
    }
}

async fn check_balance(wallet: &str) {
    // Placeholder balance check (example: read from a file)
    let balance = get_balance_from_file(wallet).await;
    println!("Balance for wallet {}: {} CC", wallet, balance);
}

async fn send_transaction(from: &str, to: &str, amount: &str) {
    println!("Sending {} CC from {} to {}", amount, from, to);
    let result = process_transaction(from, to, amount).await;
    match result {
        Ok(_) => println!("Transaction sent successfully!"),
        Err(err) => eprintln!("Error sending transaction: {}", err),
    }
}

async fn get_balance_from_file(wallet: &str) -> u64 {
    // Example: Fetch balance from a file (you can replace this with actual blockchain integration)
    let path = format!("./wallets/{}.balance", wallet);
    if let Ok(contents) = fs::read_to_string(path) {
        contents.trim().parse().unwrap_or(0)
    } else {
        0
    }
}

async fn process_transaction(from: &str, to: &str, amount: &str) -> Result<(), String> {
    // Placeholder for actual transaction processing logic
    if amount.parse::<u64>().is_err() {
        return Err("Invalid amount.".to_string());
    }
    Ok(())
}

async fn create_account(wallet_name: &str) {
    // Generate the keypair
    let mut rng = OsRng;
    let keypair = Keypair::generate(&mut rng);

    // Extract the public and private keys
    let private_key: &SecretKey = &keypair.secret;
    let public_key: PublicKey = keypair.public;

    // Convert the public key to hex string
    let public_key_hex = hex::encode(public_key.as_bytes());
    let private_key_hex = hex::encode(private_key.to_bytes());

    // Create a wallet directory if it doesn't exist
    fs::create_dir_all("./wallets").expect("Failed to create wallet directory");

    // Save the keys to files
    let public_key_path = format!("./wallets/{}_public.key", wallet_name);
    let private_key_path = format!("./wallets/{}_private.key", wallet_name);

    // Write the public and private keys to files
    let mut public_file = fs::File::create(public_key_path).expect("Failed to create public key file");
    let mut private_file = fs::File::create(private_key_path).expect("Failed to create private key file");

    public_file.write_all(public_key_hex.as_bytes()).expect("Failed to write public key");
    private_file.write_all(private_key_hex.as_bytes()).expect("Failed to write private key");

    println!("Account created successfully!");
    println!("Public Key: {}", public_key_hex);
    println!("Private Key: {}", private_key_hex);
    println!("Keys saved to ./wallets/{}_public.key and ./wallets/{}_private.key", wallet_name, wallet_name);
}
