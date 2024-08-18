use std::cmp::PartialEq;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UTXO {
   pub value: u64,
   pub receiver: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transfer {
   pub value: u64,
   pub receiver: String,
   pub sender : String,
}

// Modify the Transaction struct to include inputs and outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
   pub inputs: Vec<UTXO>,     // UTXOs being spent
   pub outputs: Vec<UTXO>,    // New UTXOs being created
}
pub type Transactions = Vec<Transaction>;