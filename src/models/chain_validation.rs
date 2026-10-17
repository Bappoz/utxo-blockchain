use std::collections::{HashMap, HashSet};

use ed25519_dalek::VerifyingKey;

use crate::crypto::hashing::Hash;
use crate::models::block::Block;
use crate::models::blockchain::{Blockchain, MINING_REWARD};
use crate::models::transaction::Transaction;

const MINIMUM_BLOCK_DIFFICULTY_BITS: usize = 16;
const MAXIMUM_BLOCK_DIFFICULTY_BITS: usize = 256;

impl Blockchain {
    /// Validates a regular transaction against the current UTXO set and
    /// returns its fee.
    pub fn validate_transaction(&self, tx: &Transaction) -> Result<u64, String> {
        if tx.is_coinbase() {
            return Err(
                "Coinbase transactions are only valid as the first transaction in a block".into(),
            );
        }
        if tx.inputs.is_empty() || tx.outputs.is_empty() {
            return Err("Transaction must have at least one input and output".into());
        }

        let mut seen_inputs = HashSet::new();
        let mut input_value = 0u64;

        for (input_index, input) in tx.inputs.iter().enumerate() {
            let key = super::blockchain::UTXOKey {
                tx_hash: input.prev_tx_hash,
                output_index: input.output_index,
            };
            if !seen_inputs.insert(key.clone()) {
                return Err("Transaction contains the same input more than once".into());
            }

            let utxo = self
                .utxos
                .get(&key)
                .ok_or_else(|| "Input does not exist or has already been spent".to_string())?;
            let public_key = decode_public_key(&utxo.pubkey)?;
            if !tx.verify_input(input_index, &public_key) {
                return Err(format!("Invalid signature for input {input_index}"));
            }

            input_value = input_value
                .checked_add(utxo.value)
                .ok_or_else(|| "Input value sum exceeds u64::MAX".to_string())?;
        }

        let mut output_value = 0u64;
        for output in &tx.outputs {
            decode_public_key(&output.pubkey)?;
            output_value = output_value
                .checked_add(output.value)
                .ok_or_else(|| "Output value sum exceeds u64::MAX".to_string())?;
        }

        if input_value < output_value {
            return Err("Transaction outputs exceed its inputs".into());
        }

        Ok(input_value - output_value)
    }

    /// Validates a candidate block without changing the chain or UTXO state.
    pub fn validate_block(&self, block: &Block) -> bool {
        if block.transactions.is_empty() || block.header.difficulty > MAXIMUM_BLOCK_DIFFICULTY_BITS
        {
            return false;
        }

        if let Some(last_block) = self.chain.last() {
            if block.header.prev_block_hash != last_block.header.calculate_hash() {
                return false;
            }
            if block.header.difficulty < MINIMUM_BLOCK_DIFFICULTY_BITS {
                return false;
            }
            if self.chain.len() > 1 && block.header.difficulty != last_block.header.difficulty {
                return false;
            }
            if !block
                .header
                .calculate_hash()
                .has_sufficient_difficulty(block.header.difficulty)
            {
                return false;
            }
        } else if !block.header.prev_block_hash.is_empty() || block.header.difficulty != 0 {
            return false;
        }

        if Hash::calculate_merkle_root(&block.transactions) != block.header.merkle_root {
            return false;
        }

        self.validate_mining_reward(block)
    }

    /// Checks that a block has exactly one first-position coinbase, spends
    /// valid UTXOs in sequence, and does not claim more than the subsidy plus
    /// collected fees. The genesis subsidy must equal the configured reward.
    pub fn validate_mining_reward(&self, block: &Block) -> bool {
        let Some(coinbase) = block.transactions.first() else {
            return false;
        };
        if !coinbase.is_coinbase()
            || block
                .transactions
                .iter()
                .filter(|tx| tx.is_coinbase())
                .count()
                != 1
            || coinbase.inputs[0].output_index != self.chain.len()
        {
            return false;
        }

        let Ok(coinbase_value) = sum_outputs(coinbase) else {
            return false;
        };
        let Ok(total_fees) = self.validate_transactions_in_block(block) else {
            return false;
        };
        let Some(maximum_reward) = MINING_REWARD.checked_add(total_fees) else {
            return false;
        };

        if self.chain.is_empty() {
            coinbase_value == MINING_REWARD
        } else {
            coinbase_value <= maximum_reward
        }
    }

    /// Replays blocks from genesis through the same stateful block validator
    /// used when accepting new blocks.
    pub fn validate_full_chain(blocks: &[Block]) -> bool {
        if blocks.is_empty() {
            return false;
        }

        let mut replayed = Blockchain {
            chain: Vec::new(),
            utxos: HashMap::new(),
            mempool: Vec::new(),
        };

        for block in blocks {
            if !replayed.add_block(block.clone()) {
                return false;
            }
        }
        true
    }

    fn validate_transactions_in_block(&self, block: &Block) -> Result<u64, String> {
        let mut staged = Blockchain {
            chain: self.chain.clone(),
            utxos: self.utxos.clone(),
            mempool: Vec::new(),
        };
        let mut total_fees = 0u64;

        for tx in block.transactions.iter().skip(1) {
            let fee = staged.validate_transaction(tx)?;
            total_fees = total_fees
                .checked_add(fee)
                .ok_or_else(|| "Block fee sum exceeds u64::MAX".to_string())?;
            MINING_REWARD
                .checked_add(total_fees)
                .ok_or_else(|| "Block reward exceeds u64::MAX".to_string())?;
            Blockchain::apply_transaction_to_utxos(&mut staged.utxos, tx);
        }

        Ok(total_fees)
    }
}

fn sum_outputs(tx: &Transaction) -> Result<u64, String> {
    if tx.outputs.is_empty() {
        return Err("Coinbase transaction must have at least one output".into());
    }

    tx.outputs.iter().try_fold(0u64, |total, output| {
        decode_public_key(&output.pubkey)?;
        total
            .checked_add(output.value)
            .ok_or_else(|| "Output value sum exceeds u64::MAX".to_string())
    })
}

fn decode_public_key(address: &str) -> Result<VerifyingKey, String> {
    let bytes = hex::decode(address).map_err(|_| "Public key is not valid hex".to_string())?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "Public key must be 32 bytes".to_string())?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| "Public key is not a valid Ed25519 key".into())
}
