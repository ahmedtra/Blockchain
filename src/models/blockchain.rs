
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
}

impl Blockchain {
    pub fn new(difficulty: usize, initial_inputs : Vec<UTXO> ) -> Self {
        let genesis_block = Block::new(0, String::from("0"), vec![]);
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
        }
    }

    pub fn add_transaction(&mut self, inputs: Vec<UTXO>, outputs: Vec<UTXO>) -> bool {
        // Check if inputs are valid (i.e., exist in the UTXO set)
        for input in &inputs {
            if let Some(utxos) = self.utxo_set.get(&input.receiver) {
                if !utxos.contains(input) {
                    return false;  // Invalid input
                }
            } else {
                return false;  // No UTXOs for this address
            }
        }

        // Create new transaction and update UTXO set
        let new_transaction = Transaction { inputs: inputs.clone(), outputs: outputs.clone() };
        self.update_utxo_set(&inputs, &outputs);
        
        // Add transaction to the current block (or create a new block)
        let mut new_block = Block::new(self.chain.len() as u64, self.chain.last().unwrap().hash.clone(), vec![new_transaction]);
        new_block.mine(self.difficulty);
        self.chain.push(new_block);
        true
    }

    pub fn add_block(&mut self, block: Block) {
        self.chain.push(block);
    }

    pub fn add_transfer(&mut self, transfer : Transfer){
        let transaction : Transaction = from_transfer_to_utxo(transfer, &self.utxo_set);
        self.add_transaction(transaction.inputs, transaction.outputs);
    }

    fn update_utxo_set(&mut self, inputs: &Vec<UTXO>, outputs: &Vec<UTXO>) {
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

    pub fn collect_pending_transactions(&self) -> Transactions {
        vec![]
    }
}
