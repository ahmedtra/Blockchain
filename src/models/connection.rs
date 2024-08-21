use super::blockchain::Blockchain;
use super::block::Block;

use tokio::io::{AsyncWriteExt, AsyncReadExt};
use serde_json;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::Sender;

use std::sync::Arc;
use tokio::sync::Mutex;



pub async fn connect_and_sync(peer: String, blockchain: Arc<Mutex<Blockchain>>) {
    if let Ok(mut stream) = TcpStream::connect(peer.clone()).await {
        println!("Connected to peer: {}", peer);
        
        // Request the blockchain
        let request = "GET_BLOCKCHAIN";
        stream.write_all(request.as_bytes()).await.unwrap();

        // Read the blockchain from the peer
        let mut buffer = vec![0; 4096];
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8(buffer[..n].to_vec()).unwrap();

        let peer_blockchain: Blockchain = serde_json::from_str(&response).unwrap();

        // Replace the local blockchain if the peer's blockchain is longer
        let mut local_blockchain = blockchain.lock().await;
        if peer_blockchain.chain.len() > local_blockchain.chain.len() {
            println!("Replacing local blockchain with peer's blockchain");
            *local_blockchain = peer_blockchain;
        }
    } else {
        println!("Failed to connect to peer: {}", peer);
    }
}

pub  async fn handle_connection(mut stream: TcpStream, blockchain: Arc<Mutex<Blockchain>>, peers: Arc<Mutex<Vec<String>>>) {
    let mut buffer = vec![0; 1024];
    
    // Read the incoming data
    let n = stream.read(&mut buffer).await.unwrap();
    let request = String::from_utf8_lossy(&buffer[..n]);

    if request == "GET_BLOCKCHAIN" {
        // Handle blockchain sync request
        let blockchain = blockchain.lock().await;
        let serialized_blockchain = serde_json::to_string(&*blockchain).unwrap();
        stream.write_all(serialized_blockchain.as_bytes()).await.unwrap();
    } else if request.starts_with("NEW_BLOCK") {
        // Handle new block
        let block_json = &request[9..];  // Strip "NEW_BLOCK" prefix
        if let Ok(new_block) = serde_json::from_str::<Block>(block_json) {
            let mut blockchain = blockchain.lock().await;
            let last_block = blockchain.chain.last().unwrap();

            // Check if the new block is valid
            if new_block.previous_hash == last_block.hash && new_block.is_valid_proof_of_work(blockchain.difficulty) {
                blockchain.chain.push(new_block.clone());
                println!("New block added to the chain: {:?}", new_block.index);

            } else {
                println!("Received an invalid block: {:?}", new_block.index);
            }
        } else {
            println!("Failed to deserialize the new block.");
        }
    } else if request.starts_with("NEW_PEER") {
             let new_peer = request["NEW_PEER".len()..].to_string();
             let mut peers_lock = peers.lock().await;
             if !peers_lock.contains(&new_peer) {
                 peers_lock.push(new_peer.clone());
                 let blockchain = Arc::clone(&blockchain);
                 let peers = Arc::clone(&peers);
                 // start_listener(new_peer.clone(), blockchain, peers).await.unwrap()
             }
        }
    else {
        // Other types of requests (e.g., transactions) can be handled here
        println!("Unknown request: {}", request);
    }
}

pub async fn miner_thread(blockchain: Arc<Mutex<Blockchain>>, tx: Sender<Block>, difficulty: usize) {
    loop {
        let blockchain = blockchain.lock().await;
        let last_block = blockchain.chain.last().unwrap();

        // Create a new block based on the last block
        let mut new_block = Block::new(
            last_block.index + 1,
            last_block.hash.clone(),
            blockchain.collect_pending_transactions(),
        );

        // Start mining (i.e., finding the correct hash)
        new_block.mine(difficulty);

        println!("Block mined : {:?}", new_block.index);

        // Broadcast the new block to other peers
        let _ = tx.send(new_block).await;

        // After broadcasting, let the miner rest a bit
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
}

pub async fn send_message_to_peer(peer_address: String, message: String) -> Result<(), Box<dyn std::error::Error>> {
    // Establish a connection to the peer
    let mut stream = TcpStream::connect(peer_address).await?;

    // Send the message to the peer
    stream.write_all(message.as_bytes()).await?;
    
    // Ensure the data is flushed out
    stream.flush().await?;

    Ok(())
}

pub async fn start_listener(address:String, blockchain: Arc<Mutex<Blockchain>>, peers: Arc<Mutex<Vec<String>>>) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(address.clone()).await?;
    println!("Listening on {}", address);

    loop {
        // Wait for an incoming connection
        let (stream, _) = listener.accept().await?;
        
        // Clone the blockchain Arc and transaction sender for the new task
        let blockchain = Arc::clone(&blockchain);
        let peers = Arc::clone(&peers);

        // Spawn a new task to handle the connection concurrently
        tokio::spawn(async move {
            handle_connection(stream, blockchain, peers).await;

        });
    }
    
}
