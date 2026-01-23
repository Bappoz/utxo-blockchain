use serde::{Serialize, Deserialize};
use chrono::Utc;
use crate::models::transaction::Transaction;
use crate::crypto::hashing::Hash;

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction> 
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockHeader {
    pub timestamp: i64,
    pub prev_block_hash: Hash,
    pub merkle_root: Hash,            // O resumo de todas as transações do bloco
    pub nonce: u64,                     // Usado na fase de mineração
}

impl BlockHeader {
    pub fn calculate_hash(&self) -> Hash {
        Hash::hash_data(self)
    }
}

impl Block {
    pub fn new(prev_block_hash: Hash, transactions: Vec<Transaction>) -> Self {
        // Calcula a Merkle Root 
        let merkle_root = Hash::calculate_merkle_root(&transactions);

        let header = BlockHeader {
            timestamp: Utc::now().timestamp(),
            prev_block_hash,
            merkle_root,
            nonce: 0,
        };

        Block { 
            header, 
            transactions
        }
    }

    pub fn genesis(coinbase_tx: Transaction) -> Self {
        Self::new(Hash::new_empty(), vec![coinbase_tx])
    }
}