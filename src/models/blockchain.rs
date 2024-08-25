
use super::block::Block;
use super::transaction::{Transaction, UTXO, Transfer, Transactions};
use std::collections::HashMap;
use super::transaction_util::from_transfer_to_utxo;
use serde::{Deserialize, Serialize};

// `Blockchain` A struct that represents the blockchain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: usize,
    pub utxo_set: HashMap<String, Vec<UTXO>>,  // Keyed by address
    pub pending_transactions: Transaction
}

impl Blockchain {
    pub fn new(difficulty: usize, initial_inputs : Vec<UTXO> ) -> Self {
        let pending_transactions = Transaction {inputs : vec![], outputs : vec![]};
        let genesis_block = Block::new(0, String::from("0"), pending_transactions.clone());
        let chain = vec![genesis_block];
        let mut utxo_set = HashMap::new();
        

        // Populate the UTXO set with initial inputs
        for input in initial_inputs {
            // Check if the address already has UTXOs in the set
            let entry = utxo_set.entry(input.receiver.clone()).or_insert_with(Vec::new);
            // Add the new UTXO to the address's UTXO vector
            entry.push(input);
        }

        Blockchain {
            chain,
            difficulty,
            utxo_set,
            pending_transactions
        }
    }

    pub fn add_transaction(&mut self, transaction : Transaction) -> bool {

        let inputs = transaction.inputs;
        let outputs = transaction.outputs;
        let mut test_utxo_set = self.utxo_set.clone();
        // Check if inputs are valid (i.e., exist in the UTXO set)

        let mut all_inputs = self.pending_transactions.inputs.clone();
        all_inputs.extend(inputs);
        let mut all_outputs = self.pending_transactions.inputs.clone();
        all_outputs.extend(outputs);
        for input in &all_inputs {
            if let Some(utxos) = test_utxo_set.get_mut(&input.receiver) {
                if !utxos.contains(input) {
                    return false;  // Invalid input
                }
                if let Some(index) = utxos.iter().position(|x| &x == &input) {
                    // Remove the element at the found index
                    utxos.remove(index);
                }
            } else {
                return false;  // No UTXOs for this address
            }
        }

        // Create new transaction and update UTXO set
        let new_transaction = Transaction { inputs: all_inputs, outputs: all_outputs};

        self.pending_transactions = new_transaction;
        
        /*
        self.update_utxo_set(&inputs, &outputs);
        
        // Add transaction to the current block (or create a new block)
        let mut new_block = Block::new(self.chain.len() as u64, self.chain.last().unwrap().hash.clone(), vec![new_transaction]);
        new_block.mine(self.difficulty);
        self.chain.push(new_block);
        */
        true
    }

    pub fn add_block(&mut self, block: Block) {
        self.chain.push(block);
    }

    pub fn add_transfer(&mut self, transfer : Transfer){
        let transaction : Transaction = from_transfer_to_utxo(transfer, &self.utxo_set);
        self.add_transaction(transaction);
    }

    pub fn update_utxo_set(&mut self, inputs: &Vec<UTXO>, outputs: &Vec<UTXO>) {
        // Remove spent UTXOs
        for input in inputs {
            if let Some(utxos) = self.utxo_set.get_mut(&input.receiver) {
                utxos.retain(|utxo| utxo != input);
            }
        }
        // Add new UTXOs
        for output in outputs {
            self.utxo_set.entry(output.receiver.clone()).or_insert(vec![]).push(output.clone());
        }
    }

    pub fn get_balance(&self, address: &str) -> u64 {
        self.utxo_set.get(address).map_or(0, |utxos| {
            utxos.iter().map(|utxo| utxo.value).sum()
        })
    }

    pub fn collect_pending_transactions(&self) -> Transaction {
        self.pending_transactions.clone()
    }

    pub fn update_with_new_block(&mut self, new_block : Block){
        self.chain.push(new_block.clone());
        self.update_utxo_set(&new_block.transactions.inputs, &new_block.transactions.outputs);
        self.pending_transactions = Transaction {inputs : vec![], outputs : vec![]};
    }
}
