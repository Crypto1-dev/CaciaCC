use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};
use chrono::Utc;
use rand::rngs::OsRng;
use ed25519_dalek::{Keypair, Signer};
use hex;

mod main; // Assumes your main.rs exposes Blockchain and Transaction types.
use main::{Blockchain, Transaction};

mod network;
use network::Network;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Launching Cacia (CC) coin for testing...");

    // Initialize the blockchain and set up initial stakes and balances.
    let bc = Arc::new(Mutex::new(Blockchain::new()));
    {
        let mut bc_locked = bc.lock().unwrap();
        bc_locked.stakes.insert("validator1".to_string(), 1_000 * 10_u64.pow(8));
        bc_locked.balances.insert("user1".to_string(), 1_000 * 10_u64.pow(8));
        bc_locked.balances.insert("user2".to_string(), 100 * 10_u64.pow(8));
    }

    // Set up the network node with a known address and some peer addresses.
    let node_addr = "127.0.0.1:7878".to_string();
    let peers = vec!["127.0.0.1:7879".to_string(), "127.0.0.1:7880".to_string()];
    let network = Network::new(Arc::clone(&bc), node_addr, peers);

    // Spawn the network task.
    let network_clone = network.clone();
    tokio::spawn(async move {
        network_clone.run().await.unwrap();
    });

    // Simulate a transaction: Create a sample transaction from user1 to user2.
    {
        let mut csprng = OsRng {};
        let keypair = Keypair::generate(&mut csprng);
        let pubkey_hex = hex::encode(keypair.public.to_bytes());

        let mut tx = Transaction {
            sender: "user1".to_string(),
            receiver: "user2".to_string(),
            amount: 10 * 10_u64.pow(8),
            fee: 5000,
            nonce: 0, // First transaction for user1
            signature: "".to_string(),
            timestamp: Utc::now().timestamp(),
            public_key: pubkey_hex.clone(),
        };

        let sig = keypair.sign(&tx.hash());
        tx.signature = hex::encode(sig.to_bytes());

        let mut bc_locked = bc.lock().unwrap();
        if bc_locked.add_transaction(tx.clone()) {
            println!("Sample transaction added.");
            network.broadcast_tx(tx).await;
        } else {
            println!("Sample transaction failed to add.");
        }
    }

    // Spawn a task for auto block creation every BLOCK_TIME seconds.
    let bc_clone = Arc::clone(&bc);
    let network_clone = network.clone();
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(5)).await; // Using 5 seconds for testing; adjust as needed.
            let mut bc_locked = bc_clone.lock().unwrap();
            if let Some(block) = bc_locked.create_block() {
                bc_locked.apply_block(block.clone());
                println!(
                    "New block created: index {} by {} ({} txs)",
                    block.index,
                    block.validator,
                    block.transactions.len()
                );
                network_clone.broadcast_block(block).await;
            }
        }
    });

    // Spawn the REST API server (using Warp) to expose endpoints for testing.
    let bc_for_api = Arc::clone(&bc);
    tokio::spawn(async move {
        run_api(bc_for_api).await;
    });

    println!("Cacia (CC) coin is running for testing. Press Ctrl+C to stop.");
    // Keep the main task alive.
    loop {
        sleep(Duration::from_secs(60)).await;
    }
}

async fn run_api(bc: Arc<Mutex<Blockchain>>) {
    use warp::Filter;

    // GET /chain - Returns the entire blockchain.
    let chain_route = warp::path("chain")
        .and(warp::get())
        .and(with_blockchain(Arc::clone(&bc)))
        .and_then(handle_get_chain);

    // GET /balance/{address} - Returns the balance for a specific address.
    let balance_route = warp::path!("balance" / String)
        .and(warp::get())
        .and(with_blockchain(Arc::clone(&bc)))
        .and_then(handle_get_balance);

    // POST /tx - Accepts a transaction in JSON format.
    let tx_route = warp::path("tx")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_blockchain(Arc::clone(&bc)))
        .and_then(handle_post_tx);

    let routes = chain_route.or(balance_route).or(tx_route);
    println!("REST API running on 127.0.0.1:8000");
    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}

fn with_blockchain(
    bc: Arc<Mutex<Blockchain>>,
) -> impl warp::Filter<Extract = (Arc<Mutex<Blockchain>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || Arc::clone(&bc))
}

async fn handle_get_chain(bc: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
    let chain = bc.lock().unwrap().get_chain();
    Ok(warp::reply::json(&chain))
}

async fn handle_get_balance(address: String, bc: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
    let balance = bc.lock().unwrap().get_balance(&address);
    Ok(warp::reply::json(&balance))
}

async fn handle_post_tx(tx: Transaction, bc: Arc<Mutex<Blockchain>>) -> Result<impl warp::Reply, warp::Rejection> {
    let mut bc_locked = bc.lock().unwrap();
    if bc_locked.add_transaction(tx.clone()) {
        Ok(warp::reply::json(&"Transaction accepted"))
    } else {
        Ok(warp::reply::json(&"Transaction rejected"))
    }
}
