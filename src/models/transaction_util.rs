
use std::collections::HashMap;

use super::transaction::{Transaction, UTXO, Transfer};


pub fn from_transfer_to_utxo(transfer : Transfer, utxo_set : &HashMap<String, Vec<UTXO>>) -> Transaction
{
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    let mut total_input_value = 0;

    while total_input_value < transfer.value {
        if let Some(utxos) = utxo_set.get(&transfer.sender) {
   
            for utxo in utxos {
                inputs.push(utxo.clone());
                total_input_value += inputs.last().unwrap().value;
            }
        }
    }

    // Create outputs
    outputs.push(UTXO { receiver: transfer.receiver, value: transfer.value });

    // Handle change if necessary
    if total_input_value > transfer.value {
        let change = total_input_value - transfer.value;
        outputs.push(UTXO { receiver: transfer.sender, value: change });
    }


    Transaction {inputs, outputs}
}
