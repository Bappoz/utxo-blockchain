use ed25519_dalek::VerifyingKey;
use crate::models::blockchain::{Blockchain, UTXOKey, MINING_REWARD};
use crate::models::block::Block;
use crate::models::transaction::Transaction;

impl Blockchain {
    pub fn validate_transaction(&self, tx: &Transaction) -> Result<u64, String> {
        if tx.is_coinbase() { return Ok(0); }

        let mut input_value = 0;
        for input in &tx.inputs {
            let key = UTXOKey { tx_hash: input.prev_tx_hash, output_index: input.output_index };

            if let Some(utxo) = self.utxos.get(&key) {
                input_value += utxo.value;
                let pubkey_bytes = hex::decode(&utxo.pubkey).map_err(|_| "Hex inválido")?;
                let public_key = VerifyingKey::from_bytes(&pubkey_bytes.try_into().map_err(|_| "Tamanho chave inválido")?).map_err(|_| "Erro chave")?;

                if !tx.verify(&public_key) {
                    return Err("Assinatura inválida!".to_string());
                }
            } else {
                return Err("Input inexistente ou gasto!".to_string());
            }
        }

        let output_value: u64 = tx.outputs.iter().map(|o| o.value).sum();
        if input_value < output_value {
            return Err("Saldo insuficiente!".to_string());
        }
        Ok(input_value - output_value)
    }

    pub fn validate_block(&self, block: &Block) -> bool {
        if let Some(last_block) = self.chain.last() {
            if block.header.prev_block_hash != last_block.header.calculate_hash() { return false; }
        }

        if block.header.calculate_hash().count_leading_zeros() < block.header.difficulty { return false; }
        if !self.validate_mining_reward(block) { return false; }

        block.transactions.iter().all(|tx| self.validate_transaction(tx).is_ok())
    }

    pub fn validate_mining_reward(&self, block: &Block) -> bool {
        if let Some(cb_tx) = block.transactions.iter().find(|tx| tx.is_coinbase()) {
            let total_fees: u64 = block.transactions.iter()
                .filter(|tx| !tx.is_coinbase())
                .map(|tx| self.validate_transaction(tx).unwrap_or(0))
                .sum();

            cb_tx.outputs[0].value == MINING_REWARD + total_fees
        } else { false }
    }
}