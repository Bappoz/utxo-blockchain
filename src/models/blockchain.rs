use crate::crypto::hashing::Hash;
use crate::models::{
    block::Block,
    transaction::{Output, Transaction},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

///Representa o identificador único de um Output na rede
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UTXOKey {
    pub tx_hash: Hash,
    pub output_index: usize,
}

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub utxos: HashMap<UTXOKey, Output>,
    pub mempool: Vec<Transaction>, // Sala de espera
}

// Estrutura para salvar o estado completo
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockchainSnapshot {
    #[serde(default)]
    pub version: u32,
    pub chain: Vec<Block>,
    pub utxos: Vec<(UTXOKey, Output)>,
    pub mempool: Vec<Transaction>,
}

pub const MINING_REWARD: u64 = 50;
pub const BLOCKCHAIN_SNAPSHOT_VERSION: u32 = 1;

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
        if self
            .mempool
            .iter()
            .any(|m_tx| m_tx.calculate_hash() == tx_hash)
        {
            println!("Transacao ja esta na mempool");
            return false;
        }

        let spends_reserved_input = tx.inputs.iter().any(|input| {
            self.mempool.iter().any(|pending| {
                pending.inputs.iter().any(|pending_input| {
                    pending_input.prev_tx_hash == input.prev_tx_hash
                        && pending_input.output_index == input.output_index
                })
            })
        });
        if spends_reserved_input {
            return false;
        }

        self.mempool.push(tx);
        true
    }

    /// O Minerador "limpa" a mempool e cria um novo bloco
    pub fn create_next_block(&mut self, miner_addr: &str, difficulty: usize) -> Block {
        let mut total_fees = 0u64;
        let mut valid_txs = Vec::new();
        let mempool_txs: Vec<_> = self.mempool.drain(..).collect();
        let mut staged = Blockchain {
            chain: self.chain.clone(),
            utxos: self.utxos.clone(),
            mempool: Vec::new(),
        };

        for tx in mempool_txs {
            if let Ok(fee) = staged.validate_transaction(&tx) {
                let Some(new_total_fees) = total_fees.checked_add(fee) else {
                    continue;
                };
                if MINING_REWARD.checked_add(new_total_fees).is_none() {
                    continue;
                }

                total_fees = new_total_fees;
                Self::apply_transaction_to_utxos(&mut staged.utxos, &tx);
                valid_txs.push(tx);
            }
        }

        let coinbase_reward = MINING_REWARD + total_fees;
        let mut transactions = vec![Transaction::coinbase_for_height(
            miner_addr,
            coinbase_reward,
            self.chain.len(),
        )];
        transactions.extend(valid_txs);

        let prev_hash = self
            .chain
            .last()
            .map(|b| b.header.calculate_hash())
            .unwrap_or(Hash::new_empty());

        Block::new(prev_hash, transactions, difficulty)
    }

    //Adiciona um bloco à corrente e atualiza o UTXO set
    pub fn add_block(&mut self, block: Block) -> bool {
        if !self.validate_block(&block) {
            return false;
        }

        for tx in &block.transactions {
            Self::apply_transaction_to_utxos(&mut self.utxos, tx);
        }

        self.chain.push(block);
        true
    }

    pub(crate) fn apply_transaction_to_utxos(
        utxos: &mut HashMap<UTXOKey, Output>,
        tx: &Transaction,
    ) {
        if !tx.is_coinbase() {
            for input in &tx.inputs {
                let key = UTXOKey {
                    tx_hash: input.prev_tx_hash,
                    output_index: input.output_index,
                };
                utxos.remove(&key);
            }
        }

        let tx_hash = tx.calculate_hash();
        for (index, output) in tx.outputs.iter().enumerate() {
            let key = UTXOKey {
                tx_hash,
                output_index: index,
            };
            utxos.insert(key, output.clone());
        }
    }

    /// Calculates an address balance from the current unspent outputs.
    pub fn get_balance(&self, address: &str) -> Result<u64, String> {
        self.utxos
            .values()
            .filter(|output| output.pubkey == address)
            .try_fold(0u64, |balance, output| {
                balance
                    .checked_add(output.value)
                    .ok_or_else(|| "Balance exceeds u64::MAX".to_string())
            })
    }
}
