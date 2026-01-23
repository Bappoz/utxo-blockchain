use serde::{Serialize, Deserialize};
use crate::crypto::hashing::Hash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,  
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub prev_tx_hash: Hash,           // Hash da transação anterior
    pub output_index: usize,            // Output daquela transação
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub value: u64,
    pub pubkey: String,                 // Endereço do destinatário
}


impl Transaction {
    pub fn coinbase(to: &str, amount: u64) -> Self {
        Transaction { 
            inputs: vec![Input {
                prev_tx_hash: Hash::new_empty(),
                output_index: 0,
            }], 
            outputs: vec![Output {
                value: amount,
                pubkey: to.to_string(),
            }], 
        }
    }

    pub fn calculate_hash(&self) -> Hash {
        Hash::hash_data(self)
    }
}