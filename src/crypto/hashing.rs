use std::fmt;
use sha2::{Sha256, Digest};
use crate::models::transaction::Transaction;
use serde::{Serialize, Deserialize, Serializer, Deserializer};

const HASH_SIZE: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Hash([u8; HASH_SIZE]);

impl Hash {
    /// Gera o dado de qualquer hash serializável
    pub fn hash_data<T: Serialize>(data: &T) -> Self {
        let bytes = serde_json::to_vec(data).unwrap_or_default();
        // Usamos bincode ou serde_json para transformar o dado em bytes antes de hashear
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let sha3_256_hash = hasher.finalize();

        
        let mut hash = [0u8; HASH_SIZE];
        hash.copy_from_slice(&sha3_256_hash);
        Hash(hash)
    }

    pub fn new_empty() -> Self {
        Hash([0u8; HASH_SIZE])
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Calcula a Merkle Root a partir de uma lista de transações
    pub fn calculate_merkle_root(transactions: &[Transaction]) -> Hash {
        if transactions.is_empty() {
            return Hash::new_empty();
        }

        // Gerar os hashes iniciais (folhas da árvore)
        let mut current_level: Vec<Hash> = transactions
            .iter()
            .map(|tx| Hash::hash_data(tx))
            .collect();

        // Subir a árvore até sobrar apenas o hash
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in current_level.chunks(2) {
                let left = &chunk[0];
                // Se não houver um par (ímpar), duplica o da esquerda (padrão Bitcoin)
                let right = chunk.get(1).unwrap_or(left);

                let parent_hash= Hash::calculate_hash_tree_branch(left, right);
                next_level.push(parent_hash);
            }
            current_level = next_level;
        }
        current_level[0]
    }

    // Função de auxilio que calcula o hash de duas child nodes
    pub fn calculate_hash_tree_branch(left: &Hash, right: &Hash) -> Hash {
        let mut hasher = sha2::Sha256::new();
        // Concatena os bytes do hash da esquerda com o da direita
        hasher.update(left.as_bytes());
        hasher.update(right.as_bytes());

        let result = hasher.finalize();
        let mut hash_arr = [0u8; HASH_SIZE];
        hash_arr.copy_from_slice(&result);
        Hash(hash_arr)
    }

}

// Implementação do Display que permite usar println!("{}", hash)
impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

// Implementar Debug para facilitar visualização técnica
impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("hash").field(&self.to_hex()).finish()
    }
}

// --- Serialização Personalizada para o Serde ---
// Isso permite que o Serde salve o hash como uma string legível em JSON
impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(s)
            .map_err(|e| serde::de::Error::custom(format!("Error ao tentar converter em bytes: {}", e)))?;
        if bytes.len() != HASH_SIZE {
            return Err(serde::de::Error::custom("Tamanho de hash inválido"));
        }
        let mut arr = [0u8; HASH_SIZE];
        arr.copy_from_slice(&bytes);
        Ok(Hash(arr))
    }
}

