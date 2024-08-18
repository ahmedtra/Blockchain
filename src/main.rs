mod models;
fn main() {

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
}
