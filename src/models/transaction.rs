use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
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
    pub output_index: usize,
    pub signature: Option<Vec<u8>>,            // Output daquela transação
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub value: u64,
    pub pubkey: String,                 // Endereço do destinatário
}



impl Transaction {
    pub fn calculate_hash(&self) -> Hash {
        Hash::hash_data(self)
    }

    pub fn coinbase(to: &str, amount: u64) -> Self {
        Transaction { 
            inputs: vec![Input {
                prev_tx_hash: Hash::new_empty(),
                output_index: 0,
                signature: None,        // Inicialmente sem assinatura
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
        let data = self.get_data_to_sign();
        let signature = secret_key.sign(&data);

        // Coloca a mesma assinatura em todos os inputs
        for input in self.inputs.iter_mut() {
            input.signature = Some(signature.to_vec());
        }
    }

    pub fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1 && self.inputs[0].prev_tx_hash.is_empty()
    }

    pub fn verify(&self, public_key: &VerifyingKey) -> bool {
        if self.inputs.is_empty() { return false; }

        if self.is_coinbase(){
            return true;
        }
        let data = self.get_data_to_sign();

        for input in &self.inputs {
            if let Some(sig_bytes) = &input.signature {
                // Tenta reconstruir a assinatura a partir dos bytes
                let signature = match Signature::from_slice(sig_bytes) {
                    Ok(s) => s,
                    Err(_) => return false,    
                };
                // Verifica se a assinatura bate com a chave pública e os dados
                if public_key.verify(&data, &signature).is_err() {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }
}