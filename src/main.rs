use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use chrono::Utc;
use rand::Rng;
use ed25519_dalek::{PublicKey, Signature, Signer, Verifier, Keypair};
use rand::rngs::OsRng;
use hex;

use warp::Filter;

mod network;
use network::Network;

const TOTAL_SUPPLY: u64 = 1_000_000_000 * 10_u64.pow(8);
const FEE: u64 = 5_000;
const BLOCK_TIME: u64 = 5;
const NODE_ADDR: &str = "127.0.0.1:7878";
const API_ADDR: &str = "127.0.0.1:8000";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Transaction {
    sender: String,
    receiver: String,
    amount: u64,
    fee: u64,
    nonce: u64,  // Added for replay protection
    signature: String,
    timestamp: i64,
    public_key: String,
}

impl Transaction {
    fn hash(&self) -> Vec<u8> {
        let tx_data = format!(
            "{}{}{}{}{}{}",
            self.sender, self.receiver, self.amount, self.fee, self.nonce, self.timestamp
        );
        let mut hasher = Sha256::new();
        hasher.update(tx_data);
        hasher.finalize().to_vec()
    }

    fn verify_signature(&self) -> bool {
        let pub_bytes = match hex::decode(&self.public_key) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let public_key = match PublicKey::from_bytes(&pub_bytes) {
            Ok(pk) => pk,
            Err(_) => return false,
        };
        let sig_bytes = match hex::decode(&self.signature) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let signature = match Signature::from_bytes(&sig_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };
        public_key.verify(&self.hash(), &signature).is_ok()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Block {
    index: u64,
    timestamp: i64,
    transactions: Vec<Transaction>,
    previous_hash: String,
    hash: String,
    validator: String,
}

#[derive(Clone)]
struct Blockchain {
    chain: VecDeque<Block>,
    balances: HashMap<String, u64>,
    pending_txs: Vec<Transaction>,
    stakes: HashMap<String, u64>,
    nonces: HashMap<String, u64>,  // Track expected nonce per address for replay protection
}

impl Blockchain {
    fn new() -> Self {
        let mut bc = Blockchain {
            chain: VecDeque::new(),
            balances: HashMap::new(),
            pending_txs: Vec::new(),
            stakes: HashMap::new(),
            nonces: HashMap::new(),
        };
        bc.create_genesis();
        bc
    }

    fn create_genesis(&mut self) {
        let genesis = Block {
            index: 0,
            timestamp: Utc::now().timestamp(),
            transactions: vec![],
            previous_hash: "0".repeat(64),
            hash: String::new(),
            validator: "genesis_validator".to_string(),
        };
        let hash = Self::hash_block(&genesis);
        let genesis = Block { hash, ..genesis };
        self.chain.push_back(genesis);
        self.balances.insert("treasury".to_string(), TOTAL_SUPPLY);
    }

    fn hash_block(block: &Block) -> String {
        let input = format!(
            "{}{}{}{}",
            block.index,
            block.timestamp,
            serde_json::to_string(&block.transactions).unwrap(),
            block.previous_hash
        );
        let mut hasher = Sha256::new();
        hasher.update(input);
        hex::encode(hasher.finalize())
    }

    // Validate chain consistency (used for internal audits)
    fn validate_chain(&self) -> bool {
        let mut previous_hash = "0".repeat(64);
        for block in self.chain.iter() {
            if block.previous_hash != previous_hash {
                return false;
            }
            let computed_hash = Self::hash_block(block);
            if block.hash != computed_hash {
                return false;
            }
            previous_hash = block.hash.clone();
        }
        true
    }

    fn add_transaction(&mut self, tx: Transaction) -> bool {
        // Verify signature first
        if !tx.verify_signature() {
            println!("Rejected tx: invalid signature from {}", tx.sender);
            return false;
        }

        // Replay protection: check nonce
        let expected_nonce = self.nonces.entry(tx.sender.clone()).or_insert(0);
        if tx.nonce != *expected_nonce {
            println!(
                "Rejected tx: incorrect nonce for {}. Expected {}, got {}",
                tx.sender, *expected_nonce, tx.nonce
            );
            return false;
        }

        let sender_bal = self.balances.get(&tx.sender).unwrap_or(&0);
        if *sender_bal >= tx.amount + tx.fee {
            self.pending_txs.push(tx.clone());
            *expected_nonce += 1;
            true
        } else {
            println!("Transaction failed: insufficient balance for {}", tx.sender);
            false
        }
    }

    fn select_validator(&self) -> String {
        let total_stake: u64 = self.stakes.values().sum();
        if total_stake == 0 {
            return "default_validator".to_string();
        }
        let mut rng = rand::thread_rng();
        let pick = rng.gen_range(0..total_stake);
        let mut cumulative = 0;
        for (addr, stake) in &self.stakes {
            cumulative += stake;
            if pick < cumulative {
                return addr.clone();
            }
        }
        "default_validator".to_string()
    }

    fn create_block(&mut self) -> Option<Block> {
        if self.pending_txs.is_empty() {
            return None;
        }
        let previous_block = self.chain.back().unwrap();
        let validator = self.select_validator();
        let txs: Vec<Transaction> = self.pending_txs.drain(..).collect();

        let block = Block {
            index: previous_block.index + 1,
            timestamp: Utc::now().timestamp(),
            transactions: txs,
            previous_hash: previous_block.hash.clone(),
            hash: String::new(),
            validator,
        };
        let hash = Self::hash_block(&block);
        Some(Block { hash, ..block })
    }

    fn apply_block(&mut self, block: Block) {
        for tx in &block.transactions {
            if !tx.verify_signature() {
                println!("Invalid tx in block {}: signature failed", block.index);
                continue;
            }

            let sender = tx.sender.clone();
            let receiver = tx.receiver.clone();
            let amount = tx.amount;
            let fee = tx.fee;

            let sender_bal = self.balances.entry(sender.clone()).or_insert(0);
            if *sender_bal >= amount + fee {
                *sender_bal -= amount + fee;
                *self.balances.entry(receiver).or_insert(0) += amount;
                *self.balances.entry(block.validator.clone()).or_insert(0) += fee;
            } else {
                println!("Skipped tx from {} due to low balance", sender);
            }
        }
        self.chain.push_back(block);
    }

    fn get_chain(&self) -> Vec<Block> {
        self.chain.iter().cloned().collect()
    }

    fn get_balance(&self, address: &str) -> u64 {
        *self.balances.get(address).unwrap_or(&0)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bc = Arc::new(Mutex::new(Blockchain::new()));
    println!("Cacia (CC) node running at {}", NODE_ADDR);

    // Initialize blockchain with sample stakes and balances
    {
        let mut bc_locked = bc.lock().unwrap();
        bc_locked.stakes.insert("validator1".to_string(), 1_000 * 10_u64.pow(8));
        bc_locked.balances.insert("user1".to_string(), 1_000 * 10_u64.pow(8));
        bc_locked.balances.insert("user2".to_string(), 100 * 10_u64.pow(8));
    }

    let network = Network::new(
        Arc::clone(&bc),
        NODE_ADDR.to_string(),
        vec![
            "127.0.0.1:7879".to_string(),
            "127.0.0.1:7880".to_string(),
        ],
    );

    // Spawn network task for broadcasting blocks and transactions
    let network_clone = network.clone();
    tokio::spawn(async move {
        network_clone.run().await.unwrap();
    });

    // Simulated transaction creation with keypair (example)
    {
        let mut csprng = OsRng {};
        let keypair = Keypair::generate(&mut csprng);
        let pubkey_hex = hex::encode(keypair.public.to_bytes());

        let mut tx = Transaction {
            sender: "user1".to_string(),
            receiver: "user2".to_string(),
            amount: 10 * 10_u64.pow(8),
            fee: FEE,
            nonce: 0, // first transaction nonce
            signature: "".to_string(),
            timestamp: Utc::now().timestamp(),
            public_key: pubkey_hex.clone(),
        };

        let sig = keypair.sign(&tx.hash());
        tx.signature = hex::encode(sig.to_bytes());

        let mut bc_locked = bc.lock().unwrap();
        if bc_locked.add_transaction(tx.clone()) {
            network.broadcast_tx(tx).await;
        }
    }

    // Auto block creation task
    let bc_clone = Arc::clone(&bc);
    let network_clone = network.clone();
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(BLOCK_TIME)).await;
            let mut bc_locked = bc_clone.lock().unwrap();
            if let Some(block) = bc_locked.create_block() {
                bc_locked.apply_block(block.clone());
                println!(
                    "New block [{}] by {} | txs: {}",
                    block.index,
                    block.validator,
                    block.transactions.len()
                );
                network_clone.broadcast_block(block).await;
            }
        }
    });

    // Run REST API server concurrently (provides endpoints for chain queries, balance, and transaction submission)
    let bc_for_api = Arc::clone(&bc);
    tokio::spawn(async move {
        run_api(bc_for_api).await;
    });

    // Keep main thread alive indefinitely
    loop {
        sleep(Duration::from_secs(60)).await;
    }
}

// REST API using warp
async fn run_api(bc: Arc<Mutex<Blockchain>>) {
    // GET /chain - returns blockchain
    let chain_route = warp::path("chain")
        .and(warp::get())
        .and(with_blockchain(bc.clone()))
        .and_then(handle_get_chain);

    // GET /balance/{address} - returns balance for an address
    let balance_route = warp::path!("balance" / String)
        .and(warp::get())
        .and(with_blockchain(bc.clone()))
        .and_then(handle_get_balance);

    // POST /tx - accepts a transaction in JSON format
    let tx_route = warp::path("tx")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_blockchain(bc.clone()))
        .and_then(handle_post_tx);

    let routes = chain_route.or(balance_route).or(tx_route);
    println!("API running on {}", API_ADDR);
    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}

fn with_blockchain(
    bc: Arc<Mutex<Blockchain>>,
) -> impl Filter<Extract = (Arc<Mutex<Blockchain>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || bc.clone())
}

async fn handle_get_chain(
    bc: Arc<Mutex<Blockchain>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let bc_locked = bc.lock().unwrap();
    let chain = bc_locked.get_chain();
    Ok(warp::reply::json(&chain))
}

async fn handle_get_balance(
    address: String,
    bc: Arc<Mutex<Blockchain>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let bc_locked = bc.lock().unwrap();
    let balance = bc_locked.get_balance(&address);
    Ok(warp::reply::json(&balance))
}

async fn handle_post_tx(
    tx: Transaction,
    bc: Arc<Mutex<Blockchain>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut bc_locked = bc.lock().unwrap();
    if bc_locked.add_transaction(tx.clone()) {
        Ok(warp::reply::json(&"Transaction accepted"))
    } else {
        Ok(warp::reply::json(&"Transaction rejected"))
    }
}
