use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use secp256k1::{SecretKey, PublicKey, Secp256k1, Message};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use chrono::Utc;
use rand::Rng;

const TOTAL_SUPPLY: u64 = 1_000_000_000 * 10_u64.pow(8); // 1B CC (8 decimals)
const FEE: u64 = 5_000; // 0.005 CC (8 decimals)
const BLOCK_TIME: u64 = 5; // 5 seconds

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Transaction {
    sender: String, // Public key (hex)
    receiver: String, // Public key (hex)
    amount: u64, // In smallest unit (10^8 CC)
    fee: u64,
    signature: String, // Hex-encoded
    timestamp: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Block {
    index: u64,
    timestamp: i64,
    transactions: Vec<Transaction>,
    previous_hash: String,
    hash: String,
    validator: String, // Public key of staker
}

#[derive(Clone)]
struct Blockchain {
    chain: VecDeque<Block>,
    balances: HashMap<String, u64>,
    pending_txs: Vec<Transaction>,
    stakes: HashMap<String, u64>, // Address -> staked amount
}

impl Blockchain {
    fn new() -> Self {
        let mut bc = Blockchain {
            chain: VecDeque::new(),
            balances: HashMap::new(),
            pending_txs: Vec::new(),
            stakes: HashMap::new(),
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
        // Distribute initial supply to a "treasury" address (simplified)
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

    fn add_transaction(&mut self, tx: Transaction) {
        self.pending_txs.push(tx);
    }

    fn select_validator(&self) -> String {
        // Simple PoS: pick validator with highest stake (weighted random in real impl)
        self.stakes.iter()
            .max_by_key(|&(_, &stake)| stake)
            .map(|(addr, _)| addr.clone())
            .unwrap_or("default_validator".to_string())
    }

    fn create_block(&mut self) -> Block {
        let previous_block = self.chain.back().unwrap();
        let validator = self.select_validator();
        let block = Block {
            index: previous_block.index + 1,
            timestamp: Utc::now().timestamp(),
            transactions: self.pending_txs.drain(..).collect(),
            previous_hash: previous_block.hash.clone(),
            hash: String::new(),
            validator,
        };
        let hash = Self::hash_block(&block);
        Block { hash, ..block }
    }

    fn apply_block(&mut self, block: Block) {
        for tx in &block.transactions {
            let sender = tx.sender.clone();
            let receiver = tx.receiver.clone();
            let amount = tx.amount;
            let fee = tx.fee;

            let sender_bal = self.balances.entry(sender.clone()).or_insert(0);
            if *sender_bal >= amount + fee {
                *sender_bal -= amount + fee;
                *self.balances.entry(receiver).or_insert(0) += amount;
                *self.balances.entry(block.validator.clone()).or_insert(0) += fee;
            }
        }
        self.chain.push_back(block);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bc = Arc::new(Mutex::new(Blockchain::new()));
    let listener = TcpListener::bind("127.0.0.1:7878").await?;
    println!("Cacia (CC) node running on 127.0.0.1:7878");

    // Simulate staking for testing
    {
        let mut bc = bc.lock().unwrap();
        bc.stakes.insert("validator1".to_string(), 1000 * 10_u64.pow(8));
    }

    // Spawn block creation task
    let bc_clone = Arc::clone(&bc);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(BLOCK_TIME)).await;
            let mut bc = bc_clone.lock().unwrap();
            let block = bc.create_block();
            bc.apply_block(block.clone());
            println!("New block: {}", block.hash);
        }
    });

    // P2P server
    while let Ok((stream, addr)) = listener.accept().await {
        let bc = Arc::clone(&bc);
        tokio::spawn(handle_connection(stream, addr, bc));
    }

    Ok(())
}

async fn handle_connection(mut stream: TcpStream, addr: SocketAddr, bc: Arc<Mutex<Blockchain>>) {
    let mut buffer = [0; 1024];
    match stream.read(&mut buffer).await {
        Ok(n) if n > 0 => {
            let msg = String::from_utf8_lossy(&buffer[..n]);
            println!("Received from {}: {}", addr, msg);
            // Handle incoming txs or blocks (simplified)
            stream.write_all(b"ACK").await.unwrap();
        }
        _ => println!("Connection closed by {}", addr),
    }
}
