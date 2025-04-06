use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json;
use crate::{Blockchain, Block, Transaction};

pub struct Network {
    bc: Arc<Mutex<Blockchain>>,
    addr: String,
    peers: Vec<String>,
}

impl Network {
    pub fn new(bc: Arc<Mutex<Blockchain>>, addr: String, peers: Vec<String>) -> Self {
        Network { bc, addr, peers }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        println!("P2P server listening on {}", self.addr);

        // Connect to peers
        for peer in &self.peers {
            self.connect_to_peer(peer).await;
        }

        // Handle incoming connections
        while let Ok((stream, addr)) = listener.accept().await {
            let bc = Arc::clone(&self.bc);
            tokio::spawn(async move {
                Self::handle_connection(stream, addr, bc).await;
            });
        }
        Ok(())
    }

    async fn connect_to_peer(&self, peer: &str) {
        match TcpStream::connect(peer).await {
            Ok(mut stream) => {
                println!("Connected to peer {}", peer);
                let chain = self.bc.lock().unwrap().get_chain();
                let msg = serde_json::to_string(&chain).unwrap();
                stream.write_all(msg.as_bytes()).await.unwrap();
            }
            Err(e) => println!("Failed to connect to {}: {}", peer, e),
        }
    }

    async fn handle_connection(mut stream: TcpStream, addr: std::net::SocketAddr, bc: Arc<Mutex<Blockchain>>) {
        let mut buffer = [0; 4096];
        match stream.read(&mut buffer).await {
            Ok(n) if n > 0 => {
                let msg = String::from_utf8_lossy(&buffer[..n]);
                println!("Received from {}: {}", addr, msg.len());

                // Try parsing as a chain (sync) or single block/tx
                if let Ok(chain) = serde_json::from_str::<Vec<Block>>(&msg) {
                    let mut bc = bc.lock().unwrap();
                    if chain.len() > bc.chain.len() {
                        println!("Syncing chain from peer {}", addr);
                        bc.chain = chain.into_iter().collect();
                    }
                } else if let Ok(block) = serde_json::from_str::<Block>(&msg) {
                    let mut bc = bc.lock().unwrap();
                    if block.index == bc.chain.back().unwrap().index + 1 &&
                       block.previous_hash == bc.chain.back().unwrap().hash {
                        bc.apply_block(block);
                        println!("Applied block from peer {}", addr);
                    }
                } else if let Ok(tx) = serde_json::from_str::<Transaction>(&msg) {
                    let mut bc = bc.lock().unwrap();
                    bc.add_transaction(tx);
                    println!("Added tx from peer {}", addr);
                }

                stream.write_all(b"ACK").await.unwrap();
            }
            Ok(_) => println!("Empty message from {}", addr),
            Err(e) => println!("Error reading from {}: {}", addr, e),
        }
    }

    pub async fn broadcast_block(&self, block: Block) {
        let msg = serde_json::to_string(&block).unwrap();
        for peer in &self.peers {
            if let Ok(mut stream) = TcpStream::connect(peer).await {
                stream.write_all(msg.as_bytes()).await.unwrap();
            }
        }
    }

    pub async fn broadcast_tx(&self, tx: Transaction) {
        let msg = serde_json::to_string(&tx).unwrap();
        for peer in &self.peers {
            if let Ok(mut stream) = TcpStream::connect(peer).await {
                stream.write_all(msg.as_bytes()).await.unwrap();
            }
        }
    }
}
