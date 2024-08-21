mod models;
use std::env;
use tokio::sync::{Mutex, mpsc};

use std::sync::Arc;

#[tokio::main]
async fn  main() {
    /* 
    // Create some UTXOs
    let utxo1 = models::transaction::UTXO { value: 50, receiver: String::from("Alice") };
    let utxo2 = models::transaction::UTXO { value: 30, receiver: String::from("Alice") };
    

    let mut blockchain = models::blockchain::Blockchain::new(4, vec![utxo1.clone(), utxo2.clone()]);


    // Create a transaction
    blockchain.add_transaction(vec![utxo1.clone()], vec![models::transaction::UTXO { value: 50, receiver: String::from("Bob") }]);
    
    // Check balances
    println!("Alice's balance: {}", blockchain.get_balance("Alice"));
    println!("Bob's balance: {}", blockchain.get_balance("Bob"));

    let transfer = models::transaction::Transfer {value : 10, sender : String::from("Bob"), receiver: String::from("Alice")};

    blockchain.add_transfer(transfer);
    // Check balances
    println!("Alice's balance: {}", blockchain.get_balance("Alice"));
    println!("Bob's balance: {}", blockchain.get_balance("Bob")); 
    */

    let args: Vec<String> = env::args().collect();
    let address = if args.len() > 1 { &args[1] } else { "127.0.0.1:8080" };
    let initial_peers: Vec<String> = args.iter().skip(1).map(|s| s.to_string()).collect();

    let initial_utxos = vec![];
    let blockchain = Arc::new(Mutex::new(models::blockchain::Blockchain::new(4, initial_utxos)));
    let (tx, mut rx) = mpsc::channel(32);

    tokio::spawn(models::connection::miner_thread(Arc::clone(&blockchain), tx, 4));
    let peers = Arc::new(Mutex::new(initial_peers));
    
    let peers_send = Arc::clone(&peers);

    tokio::spawn(async move {
        while let Some(new_block) = rx.recv().await {
            println!("Received new block: {:?}", new_block.index);
            let peers = peers_send.lock().await;
            // Here we would broadcast the new block to all connected peers
            for peer in peers.iter().cloned() {
                let block_json = serde_json::to_string(&new_block).unwrap();
                let message = format!("NEW_BLOCK{}", block_json);
                let _ = models::connection::send_message_to_peer(peer, message).await;
            }
        }
    });

    let peers_send = Arc::clone(&peers);

    let mut peers_to_process = {
        let peers_lock = peers_send.lock().await;
        peers_lock.clone()  // Clone the list of peers
    };
 

    for peer in &peers_to_process {
        let blockchain = Arc::clone(&blockchain);
        let peers = Arc::clone(&peers);
        if peer != address{

            println!("connect to {}", peer);
            tokio::spawn(models::connection::connect_and_sync(peer.clone(), address.to_string().clone(), blockchain.clone())); 
        }
        tokio::spawn(models::connection::start_listener(peer.clone(), blockchain, peers));
    }
    
    loop {
        let peers_send = Arc::clone(&peers);

        let peers_to_process_new = {
            let peers_lock = peers_send.lock().await;
            peers_lock.clone()  // Clone the list of peers
        };

        for peer in &peers_to_process_new {
            if !peers_to_process.contains(&peer)
            {
                let blockchain = Arc::clone(&blockchain);
                let peers = Arc::clone(&peers);
                println!("connect to {}", peer);
                models::connection::start_listener(peer.clone(), blockchain, peers).await;
            }
        }

        peers_to_process = peers_to_process_new.clone();
        println!("Initial peers: {:?}", *peers);
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    }
}
