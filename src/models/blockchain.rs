use std::fs::File;
use std::io::{Write, Read};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use crate::models::{block::Block, transaction::{Output, Transaction}};
use crate::crypto::hashing::Hash;
use std::collections::HashMap;

///Representa o identificador único de um Output na rede
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UTXOKey{
    pub tx_hash: Hash,
    pub output_index: usize,
}

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub utxos: HashMap<UTXOKey, Output>,
    pub mempool: Vec<Transaction>,          // Sala de espera
}

// Estrutura para salvar o estado completo
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockchainSnapshot {
    pub chain: Vec<Block>,
    pub utxos: Vec<(UTXOKey, Output)>,
    pub mempool: Vec<Transaction>,
}


pub const MINING_REWARD: u64 = 50;

impl Blockchain {


    /// Inicializa o blockchain com o block genesis
    pub fn new(genesis_block: Block) -> Self {
        let mut bc = Blockchain {
            chain: Vec::new(),
            utxos: HashMap::new(),
            mempool: Vec::new(),
        };
        // Ao iniciar, processa o bloco gênesis para popular os primeiros UTXOs
        bc.add_block(genesis_block);
        bc
    }

    
    pub fn submit_transaction(&mut self, tx: Transaction) -> bool {
        if self.validate_transaction(&tx).is_err() {
            return false;
        }

        // Verifica se a transação já não está na mempool evitando spam
        let tx_hash = tx.calculate_hash();
        if self.mempool.iter().any(|m_tx| m_tx.calculate_hash() == tx_hash) {
            println!("Transacao ja esta na mempool");
            return false;
        }
        self.mempool.push(tx);
        true
    }

    /// O Minerador "limpa" a mempool e cria um novo bloco
    pub fn create_next_block(&mut self, miner_addr: &str, difficulty: usize) -> Block {
        
        let mut total_fees = 0;
        let mut valid_txs = Vec::new();

        // Dreana a mempool e calcula a taxa
        let mempool_txs: Vec<_> = self.mempool.drain(..).collect();
        for tx in mempool_txs {
            if let Ok(fee) = self.validate_transaction(&tx) {
                total_fees += fee;
                valid_txs.push(tx);
            }
        }

        // Recompensa do minerador (50 + taxa)
        let coinbase_reward = MINING_REWARD + total_fees;
        let mut transactions = vec![Transaction::coinbase(miner_addr, coinbase_reward)];
        transactions.extend(valid_txs);

        let prev_hash = self.chain.last()
                .map(|b| b.header.calculate_hash()).unwrap_or(Hash::new_empty());
        
        Block::new(prev_hash, transactions, difficulty)
    }

    

    //Adiciona um bloco à corrente e atualiza o UTXO set
    pub fn add_block(&mut self, block: Block) -> bool {
        
        if !self.validate_block(&block) {
            return false;
        }
        
        for tx in &block.transactions {
            let tx_hash = tx.calculate_hash();

            // Remover utxo usados como entrada (forma gastos)
            // Nota: Transações Coinbase não têm inputs reais para remover
            if!tx.is_coinbase() {
                for input in &tx.inputs {
                    let key = UTXOKey{
                        tx_hash: input.prev_tx_hash,
                        output_index: input.output_index,
                    };
                    self.utxos.remove(&key);
                }
            }
            // Adicionar novos Outputs gerados por esta transação como UTXOs
            for (index, output) in tx.outputs.iter().enumerate() {
                let key = UTXOKey {
                    tx_hash,
                    output_index: index, 
                };
                self.utxos.insert(key, output.clone());
            }
        }
        // Inseri o Bloco na corrente
        self.chain.push(block);
        true
    } 

    /// Calcula o saldo de um endereço (chave pública em hex) 
    pub fn get_balance(&self, address: &str) -> u64 {
        let mut balance = 0;
        // Percorre somente os UTXOs existentes (moedas não gastas)
        for output in self.utxos.values() {
            if output.pubkey == address {
                balance += output.value;
            }
        } balance
    }


}