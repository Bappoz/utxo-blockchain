use serde::{Serialize, Deserialize};
use chrono::Utc;
use crate::models::transaction::Transaction;
use crate::crypto::hashing::Hash;

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction> 
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockHeader {
    pub timestamp: i64,
    pub prev_block_hash: Hash,
    pub merkle_root: Hash,              // O resumo de todas as transações do bloco
    pub nonce: u64,                     // Usado na fase de mineração
    pub difficulty: u32,                // Quantos zeros iniciais    
}

impl BlockHeader {
    pub fn calculate_hash(&self) -> Hash {
        let bytes = bincode::serialize(self).expect("Falha no servidor");
        Hash::hash_data(&bytes)
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
            difficulty: 0,
        };

        Block { 
            header, 
            transactions
        }
    }

    pub fn mine(&mut self){
        let target_prefix = "0".repeat(self.header.difficulty as usize);

        loop {
            let hash = self.header.calculate_hash();
            let hash_hex = hash.to_hex();

            if hash_hex.starts_with(&target_prefix) {
                println!("   Bloco minerado com sucesso!");
                println!("   Nonce: {}", self.header.nonce);
                println!("   Hash:  {}", hash_hex);
                break;
            } 

            self.header.nonce = self.header.nonce.wrapping_add(1)
        }
    }

    pub fn genesis(coinbase_tx: Transaction) -> Self {
        Self::new(Hash::new_empty(), vec![coinbase_tx])
    }
}