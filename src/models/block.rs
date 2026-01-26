use serde::{Serialize, Deserialize};
use chrono::Utc;
use crate::models::transaction::{self, Transaction};
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
    pub difficulty: usize,                // Quantos zeros iniciais    
}

impl BlockHeader {
    pub fn calculate_hash(&self) -> Hash {
        Hash::hash_data_cached(self)
    }
}

impl Block {
    pub fn new(prev_block_hash: Hash, transactions: Vec<Transaction>, difficulty: usize) -> Self {
        // Calcula a Merkle Root 
        let merkle_root = Hash::calculate_merkle_root(&transactions);

        let header = BlockHeader {
            timestamp: Utc::now().timestamp(),
            prev_block_hash,
            merkle_root,
            nonce: 0,
            difficulty,
        };

        Block { 
            header, 
            transactions
        }
    }

    pub fn mine(&mut self){
        let start = Utc::now();

        println!("  Mineração iniciada às: {}", start.format("%H:%M:%S"));
        println!("  Alvo de Dificuldade: {} bits zero", self.header.difficulty);

        loop {
            let hash = self.header.calculate_hash();
            if hash.count_leading_zeros() >= self.header.difficulty {
                let end_time = Utc::now();
                let duration = end_time.signed_duration_since(start);

                println!("✨ BLOCO MINERADO!");
                println!("   Hash:  {}", hash); // Usa o fmt::Display que você criou
                println!("   Nonce: {}", self.header.nonce);
                println!("   Tempo gasto: {}ms", duration.num_milliseconds());
                println!("   Velocidade aprox: {} hashes/s",
                    self.header.nonce as f32 / (duration.num_milliseconds() as f32 / 1000.0)
                );
                break;
            }

            // Se não encontrou, incrementa o nonce
            // O wrapping_add evita pânico se o número chegar ao limite de u64
            self.header.nonce = self.header.nonce.wrapping_add(1);

            if self.header.nonce == 0 {
                self.header.timestamp = Utc::now().timestamp();
            }
        }
    }

    pub fn genesis(coinbase_tx: Transaction) -> Self {
        let transactions = vec![coinbase_tx];
        let merkle_root = Hash::calculate_merkle_root(&transactions);

        let header = BlockHeader {
            timestamp: Utc::now().timestamp(),
            prev_block_hash: Hash::new_empty(),
            merkle_root,
            nonce: 0,
            difficulty: 0,
        };
        Block { header, transactions }
    }
}