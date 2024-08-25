use super::blockchain::Blockchain;
use super::block::Block;
use super::transaction::Transaction;

use tokio::io::{AsyncWriteExt, AsyncReadExt};
use serde_json;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::Sender;

use std::sync::Arc;
use tokio::sync::Mutex;
use std::cmp::min;


pub async fn connect_and_sync(peer: String, blockchain: Arc<Mutex<Blockchain>>) {
    if let Ok(mut stream) = TcpStream::connect(peer.clone()).await {
        println!("Connected to peer: {}", peer);
        
        // Request the blockchain
        let request = "GET_BLOCKCHAIN";
        stream.write_all(request.as_bytes()).await.unwrap();

        // Read the blockchain from the peer
        let mut buffer = vec![0; 4096];
        let mut data = Vec::new();

        loop {
            if let Ok(bytes_read) = stream.read(&mut buffer).await {
                if bytes_read == 0 {
                    break; // Connection closed or all data received
                }
                data.extend_from_slice(&buffer[..bytes_read]);
            }
        }


        let peer_blockchain: Blockchain = serde_json::from_slice(&data).unwrap();

        // Replace the local blockchain if the peer's blockchain is longer
        let mut local_blockchain = blockchain.lock().await;
        if peer_blockchain.chain.len() > local_blockchain.chain.len() {
            println!("Replacing local blockchain with peer's blockchain");
            *local_blockchain = peer_blockchain;
        }
    } else {
        println!("block chain sync : Failed to connect to peer: {}", peer);
    }
}

pub async fn sync_blockchain(stream: &mut TcpStream, blockchain: Arc<Mutex<Blockchain>>) {
   
        let request = "GET_BLOCKCHAIN";
        stream.write_all(request.as_bytes()).await.unwrap();

        // Read the blockchain from the peer
        let mut buffer = vec![0; 4096];
        let mut data = Vec::new();

        loop {
            if let Ok(bytes_read) = stream.read(&mut buffer).await {
                if bytes_read == 0 {
                    break; // Connection closed or all data received
                }
                data.extend_from_slice(&buffer[..bytes_read]);
            }
        }


        let peer_blockchain: Blockchain = serde_json::from_slice(&data).unwrap();

        // Replace the local blockchain if the peer's blockchain is longer
        let mut local_blockchain = blockchain.lock().await;
        if peer_blockchain.chain.len() > local_blockchain.chain.len() {
            println!("Replacing local blockchain with peer's blockchain");
            *local_blockchain = peer_blockchain;
        }
    
}
pub async fn sync_peer(peer: String, own_address:String){
    if let Ok(mut stream) = TcpStream::connect(peer.clone()).await {
        // Request the blockchain
        let request = "NEW_PEER".to_owned()+&own_address;
        println!("sending peer request {}", request);
        stream.write_all(request.as_bytes()).await;


    } else {
        println!("peer sync : Failed to connect to peer: {}", peer);
    }
}

pub async fn handle_connection(mut stream: TcpStream, blockchain: Arc<Mutex<Blockchain>>, peers: Arc<Mutex<Vec<String>>>, peer: String) {
    let mut buffer = vec![0; 4096];

    loop {
        tokio::select! {
            // Read from the stream
            n = stream.read(&mut buffer) => {
                match n {
                    Ok(0) => break, // Connection closed
                    Ok(n) => {
                        let request = String::from_utf8_lossy(&buffer[..n]);

                        if request == "GET_BLOCKCHAIN" {
                            handle_get_blockchain(&mut stream, &blockchain).await;
                        } else if request.starts_with("NEW_BLOCK") {
                            handle_new_block(&mut stream, &request, &blockchain).await;
                        } else if request.starts_with("NEW_PEER") {
                            handle_new_peer(&request, &peers).await;
                        } else if request.starts_with("TRANSACTION") {
                            handle_transaction(&request, &blockchain).await;
                        } else {
                            println!("Unknown request: {}", request);
                        }
                    },
                    Err(e) => {
                        println!("Failed to read from stream: {:?}", e);
                        break;
                    }
                }
            }
        }
    }
}

async fn handle_get_blockchain(stream: &mut TcpStream, blockchain: &Arc<Mutex<Blockchain>>) {
    println!("Handling GET_BLOCKCHAIN request");
    let blockchain = blockchain.lock().await;
    let serialized_blockchain = serde_json::to_string(&*blockchain).unwrap();
    
    let chunks = serialized_blockchain.as_bytes().chunks(4096); // Chunk size, adjust as needed
    for chunk in chunks {
        if let Err(e) = stream.write_all(chunk).await {
            println!("Failed to send blockchain data: {:?}", e);
            break;
        }
    }
}

async fn handle_new_block(stream: &mut TcpStream, request: &str, blockchain: &Arc<Mutex<Blockchain>>) {
    let block_json = &request[9..];  // Strip "NEW_BLOCK" prefix
    if let Ok(new_block) = serde_json::from_str::<Block>(block_json) {
        let blockchain_arc = Arc::clone(blockchain);
        let mut blockchain = blockchain.lock().await;
        let last_block = blockchain.chain.last().unwrap();

        if new_block.previous_hash == last_block.hash && new_block.is_valid_proof_of_work(blockchain.difficulty) {
            blockchain.update_with_new_block(new_block.clone());
            println!("New block added to the chain: {:?}", new_block.index);
        } else if new_block.index > last_block.index + 2 {
            println!("Blockchain outdated, resyncing, new block: {:?}", new_block.index);

            drop(blockchain); // Release lock before resync
            sync_blockchain(stream, blockchain_arc).await;
        }
    } else {
        println!("Failed to deserialize the new block.");
    }
}

async fn handle_new_peer(request: &str, peers: &Arc<Mutex<Vec<String>>>) {
    let new_peer = request["NEW_PEER".len()..].to_string();
    println!("Adding peer {}", new_peer);
    let mut peers_lock = peers.lock().await;
    if !peers_lock.contains(&new_peer) {
        peers_lock.push(new_peer.clone());
    }
}

async fn handle_transaction(request: &str, blockchain: &Arc<Mutex<Blockchain>>) {
    let transaction_json = &request["TRANSACTION".len()..];
    if let Ok(transaction) = serde_json::from_str::<Transaction>(transaction_json) {
        println!("New transaction added");
        let mut blockchain = blockchain.lock().await;
        blockchain.add_transaction(transaction);
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

pub async fn start_listener(address:String, blockchain: Arc<Mutex<Blockchain>>, peers: Arc<Mutex<Vec<String>>>) {
    if let Ok(listener) = TcpListener::bind(address.clone()).await {
            println!{"addres binded to {}", address};
            while let Ok((stream, _)) = listener.accept().await {
                let blockchain = Arc::clone(&blockchain);
                let peers = Arc::clone(&peers);
        
                // Spawn a new task to handle the connection
                
                tokio::spawn(handle_connection(stream, blockchain, peers, address.clone()));
            }
          }
        else 
            {
                println!{"addres cant bind to {}", address}; 
                return
            };
        

    
}
