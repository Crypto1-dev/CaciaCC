use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json;
use crate::{Blockchain, Block, Transaction};

#[derive(Clone)]
pub struct Network {
    pub bc: Arc<Mutex<Blockchain>>,
    pub addr: String,
    pub peers: Vec<String>,
}

impl Network {
    pub fn new(bc: Arc<Mutex<Blockchain>>, addr: String, peers: Vec<String>) -> Self {
        Self { bc, addr, peers }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        println!("P2P server listening on {}", self.addr);

        // Connect to known peers
        for peer in &self.peers {
            self.connect_to_peer(peer).await;
        }

        // Accept incoming connections continuously
        loop {
            let (stream, addr) = listener.accept().await?;
            let bc_clone = Arc::clone(&self.bc);
            tokio::spawn(async move {
                Self::handle_connection(stream, addr, bc_clone).await;
            });
        }
    }

    async fn connect_to_peer(&self, peer: &str) {
        match TcpStream::connect(peer).await {
            Ok(mut stream) => {
                println!("Connected to peer {}", peer);
                let chain = self.bc.lock().unwrap().get_chain();
                let msg = serde_json::to_string(&chain).unwrap();
                if let Err(e) = stream.write_all(msg.as_bytes()).await {
                    println!("Error sending chain to {}: {}", peer, e);
                }
            }
            Err(e) => println!("Failed to connect to peer {}: {}", peer, e),
        }
    }

    async fn handle_connection(mut stream: TcpStream, addr: std::net::SocketAddr, bc: Arc<Mutex<Blockchain>>) {
        let mut buffer = [0u8; 4096];
        match stream.read(&mut buffer).await {
            Ok(n) if n > 0 => {
                let msg = String::from_utf8_lossy(&buffer[..n]).to_string();
                println!("Received {} bytes from {}", n, addr);

                // Attempt to deserialize as a chain first
                if let Ok(chain) = serde_json::from_str::<Vec<Block>>(&msg) {
                    let mut bc_lock = bc.lock().unwrap();
                    if chain.len() > bc_lock.chain.len() {
                        println!("Syncing chain from peer {}", addr);
                        // Assume incoming chain is valid; ideally, run validate_chain() here
                        bc_lock.chain = chain.into_iter().collect();
                    }
                }
                // Attempt to deserialize as a block
                else if let Ok(block) = serde_json::from_str::<Block>(&msg) {
                    let mut bc_lock = bc.lock().unwrap();
                    if block.index == bc_lock.chain.back().unwrap().index + 1 &&
                       block.previous_hash == bc_lock.chain.back().unwrap().hash {
                        bc_lock.apply_block(block);
                        println!("Applied block from peer {}", addr);
                    }
                }
                // Attempt to deserialize as a transaction
                else if let Ok(tx) = serde_json::from_str::<Transaction>(&msg) {
                    let mut bc_lock = bc.lock().unwrap();
                    if bc_lock.add_transaction(tx) {
                        println!("Added transaction from peer {}", addr);
                    }
                } else {
                    println!("Unrecognized message format from peer {}", addr);
                }
                let _ = stream.write_all(b"ACK").await;
            }
            Ok(_) => println!("Received empty message from {}", addr),
            Err(e) => println!("Error reading from {}: {}", addr, e),
        }
    }

    pub async fn broadcast_block(&self, block: Block) {
        let msg = serde_json::to_string(&block).unwrap();
        for peer in &self.peers {
            if let Ok(mut stream) = TcpStream::connect(peer).await {
                if let Err(e) = stream.write_all(msg.as_bytes()).await {
                    println!("Error broadcasting block to {}: {}", peer, e);
                }
            } else {
                println!("Could not connect to peer {}", peer);
            }
        }
    }

    pub async fn broadcast_tx(&self, tx: Transaction) {
        let msg = serde_json::to_string(&tx).unwrap();
        for peer in &self.peers {
            if let Ok(mut stream) = TcpStream::connect(peer).await {
                if let Err(e) = stream.write_all(msg.as_bytes()).await {
                    println!("Error broadcasting transaction to {}: {}", peer, e);
                }
            } else {
                println!("Could not connect to peer {}", peer);
            }
        }
    }
}
