use clap::{App, Arg, SubCommand};
use tokio;
use cacia_gui::run_gui;  // Import the GUI function

#[tokio::main]
async fn main() {
    let matches = App::new("Cacia (CC) CLI and GUI")
        .version("0.1.0")
        .author("Zone-crypto-ZNE")
        .about("Cacia cryptocurrency command-line and GUI tool")
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
            SubCommand::with_name("gui")
                .about("Launch the Cacia GUI"),
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
        ("gui", _) => {
            run_gui(); // Launch the GUI when "gui" command is used
        }
        _ => eprintln!("Invalid subcommand."),
    }
}

async fn check_balance(wallet: &str) {
    let balance = get_balance_from_file(wallet).await;
    println!("Balance for wallet {}: {} CC", wallet, balance);
}

async fn send_transaction(from: &str, to: &str, amount: &str) {
    println!("Sending {} CC from {} to {}", amount, from, to);
}

async fn get_balance_from_file(wallet: &str) -> u64 {
    let path = format!("./wallets/{}.balance", wallet);
    if let Ok(contents) = std::fs::read_to_string(path) {
        contents.trim().parse().unwrap_or(0)
    } else {
        0
    }
}
