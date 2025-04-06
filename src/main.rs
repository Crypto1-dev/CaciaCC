use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use chrono::Utc;
use rand::Rng;
mod network;
use network::Network;

const TOTAL_SUPPLY: u64 = 1_000_000_000 * 10_u64.pow(8); // 1B CC (8 decimals)
const FEE: u64 = 5_000; // 0.005 CC (8 decimals)
const BLOCK_TIME: u64 = 5; // 5 seconds
const NODE_ADDR: &str = "127.0.0.1:7878";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Transaction {
    sender: String,
    receiver: String,
    amount: u64,
    fee: u64,
    signature: String, // Placeholder until ECDSA
    timestamp: i64,
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

    fn add_transaction(&mut self, tx: Transaction) -> bool {
        let sender_bal = self.balances.get(&tx.sender).unwrap_or(&0);
        if *sender_bal >= tx.amount + tx.fee {
            self.pending_txs.push(tx.clone());
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
        let block = Block {
            index: previous_block.index + 1,
            timestamp: Utc::now().timestamp(),
            transactions: self.pending_txs.drain(..).collect(),
            previous_hash: previous_block.hash.clone(),
            hash: String::new(),
            validator,
        };
        let hash = Self::hash_block(&block);
        Some(Block { hash, ..block })
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

    fn get_chain(&self) -> Vec<Block> {
        self.chain.iter().cloned().collect()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bc = Arc::new(Mutex::new(Blockchain::new()));
    println!("Cacia (CC) node starting on {}", NODE_ADDR);

    // Initial setup
    {
        let mut bc = bc.lock().unwrap();
        bc.stakes.insert("validator1".to_string(), 1000 * 10_u64.pow(8));
        bc.stakes.insert("validator2".to_string(), 500 * 10_u64.pow(8));
        bc.balances.insert("user1".to_string(), 100 * 10_u64.pow(8));
        bc.balances.insert("user2".to_string(), 50 * 10_u64.pow(8));
    }

    let network = Network::new(Arc::clone(&bc), NODE_ADDR.to_string(), vec![
        "127.0.0.1:7879".to_string(),
        "127.0.0.1:7880".to_string(),
    ]);

    // Test transaction
    {
        let mut bc = bc.lock().unwrap();
        let tx = Transaction {
            sender: "user1".to_string(),
            receiver: "user2".to_string(),
            amount: 10 * 10_u64.pow(8), // 10 CC
            fee: FEE,
            signature: "placeholder_sig".to_string(),
            timestamp: Utc::now().timestamp(),
        };
        if bc.add_transaction(tx) {
            network.broadcast_tx(tx).await;
        }
    }

    // Block creation task
    let bc_clone = Arc::clone(&bc);
    let network_clone = network.clone();
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(BLOCK_TIME)).await;
            let mut bc = bc_clone.lock().unwrap();
            if let Some(block) = bc.create_block() {
                bc.apply_block(block.clone());
                println!("New block: {} (txs: {})", block.hash, block.transactions.len());
                network_clone.broadcast_block(block).await;
            }
        }
    });

    network.run().await?;
    Ok(())
}
