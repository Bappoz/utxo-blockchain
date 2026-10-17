use crate::crypto::hashing::Hash;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub prev_tx_hash: Hash, // Hash da transação anterior
    pub output_index: usize,
    pub signature: Option<Vec<u8>>, // Output daquela transação
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub value: u64,
    pub pubkey: String, // Endereço do destinatário
}

impl Transaction {
    pub fn calculate_hash(&self) -> Hash {
        let bytes = bincode::serialize(self).expect("falha no servidor");
        Hash::hash_data(&bytes)
    }

    pub fn coinbase(to: &str, amount: u64) -> Self {
        Self::coinbase_for_height(to, amount, 0)
    }

    /// Creates a coinbase transaction with a height tag so identical rewards
    /// in different blocks have distinct transaction identifiers.
    pub fn coinbase_for_height(to: &str, amount: u64, height: usize) -> Self {
        Transaction {
            inputs: vec![Input {
                prev_tx_hash: Hash::new_empty(),
                output_index: height,
                signature: None, // Inicialmente sem assinatura
            }],
            outputs: vec![Output {
                value: amount,
                pubkey: to.to_string(),
            }],
        }
    }

    /// Previne Replay Attacks garantindo que o conjunto de dados serializado
    /// Gera a mensagem que será assinada
    pub fn get_data_to_sign(&self) -> Vec<u8> {
        // Cria uma cópia da transação sem a assinatura
        let mut temp_tx = self.clone();
        for input in temp_tx.inputs.iter_mut() {
            input.signature = None;
        }
        bincode::serialize(&temp_tx).expect("Falha ao serializar com bincode")
    }

    pub fn sign(&mut self, secret_key: &SigningKey) {
        if self.is_coinbase() {
            return;
        }

        let data = self.get_data_to_sign();
        let signature = secret_key.sign(&data);

        // Convenience for transactions whose inputs all belong to one wallet.
        for input in self.inputs.iter_mut() {
            input.signature = Some(signature.to_vec());
        }
    }

    /// Signs one input. Call this once per input when a transaction spends
    /// outputs controlled by different wallets.
    pub fn sign_input(&mut self, input_index: usize, secret_key: &SigningKey) -> bool {
        if self.is_coinbase() || input_index >= self.inputs.len() {
            return false;
        }

        let data = self.get_data_to_sign();
        let signature = secret_key.sign(&data);
        self.inputs[input_index].signature = Some(signature.to_vec());
        true
    }

    pub fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1
            && self.inputs[0].prev_tx_hash.is_empty()
            && self.inputs[0].signature.is_none()
    }

    pub fn verify(&self, public_key: &VerifyingKey) -> bool {
        !self.inputs.is_empty()
            && !self.is_coinbase()
            && self
                .inputs
                .iter()
                .enumerate()
                .all(|(index, _)| self.verify_input(index, public_key))
    }

    /// Verifies the signature attached to one input against that input's owner.
    pub fn verify_input(&self, input_index: usize, public_key: &VerifyingKey) -> bool {
        if self.is_coinbase() || input_index >= self.inputs.len() {
            return false;
        }

        let Some(signature_bytes) = &self.inputs[input_index].signature else {
            return false;
        };
        let Ok(signature) = Signature::from_slice(signature_bytes) else {
            return false;
        };

        public_key
            .verify(&self.get_data_to_sign(), &signature)
            .is_ok()
    }
}
