use std::fmt;
use hmac::{Hmac, Mac};
use sha2::{Sha256, Digest};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::models::transaction::Transaction;
use serde::{Serialize, Deserialize, Serializer, Deserializer};

const HASH_SIZE: usize = 32;

type HmacSha256 = Hmac<Sha256>;

lazy_static::lazy_static! {
    // Cache global de hashes para otimização de performance
    static ref HASH_CACHE: Arc<Mutex<HashMap<Vec<u8>, Hash>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

/// Representa um hash SHA-256 de 32 bytes
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Hash([u8; HASH_SIZE]);


///Prova criptográfica de Merkle para verificar leve de transações
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof{
    pub index: usize,               // Indice de transação no bloco
    pub siblings: Vec<Hash>,        // Hashes irmaos necessarios para reconstruir a raiz
}

impl Hash {
    
    pub fn new_empty() -> Self {
        Hash([0u8; HASH_SIZE])
    }

    pub fn is_empty(&self) -> bool {
        self.0 == [0u8; HASH_SIZE]
    }
    
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
    
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    
    /// Gera o dado de qualquer hash serializável
    pub fn hash_data<T: Serialize>(data: &T) -> Self {
        let bytes = bincode::serialize(data).expect("Falha na serializacao bincode");
        // Usamos bincode ou serde_json para transformar o dado em bytes antes de hashear
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let first_hash = hasher.finalize();

        let mut hasher2 = Sha256::new();
        hasher2.update(&first_hash);
        let final_hash = hasher2.finalize();

        let mut hash = [0u8; HASH_SIZE];
        hash.copy_from_slice(&final_hash);
        Hash(hash)
    }


    /// Hash com cache (otimização para dados repetidos)
    pub fn hash_data_cached<T: Serialize>(data: &T) -> Self {
        let bytes = bincode::serialize(data).unwrap_or_default();

        {
            let cache = HASH_CACHE.lock().unwrap();
            if let Some(&cache_hash) = cache.get(&bytes) {
                return cache_hash;
            }
        }

        // Se não está no cache, calcula
        let hash = Self::hash_data(data);

        // Armazena no cache
        {
            let mut cache = HASH_CACHE.lock().unwrap();
            // Limita tamanho do cache para evitar consumo excessivo de memoria
            if cache.len() > 10000 {
                cache.clear();
            }
            cache.insert(bytes, hash);
        }

        hash
    }


    /// limpa o cache de hashes
    pub fn clean_cache() {
        HASH_CACHE.lock().unwrap().clear();
    }




    // ========== HMAC (Autenticação de Mensagens) ==========
    
    /// Gera HMAC-SHA256 para autenticação de mensagens
    /// 
    /// Usado para verificar integridade + autenticidade de dados
    /// com uma chave secreta compartilhada
    pub fn hmac<T: Serialize>(data: &T, key: &[u8]) -> Self {
        let bytes = bincode::serialize(data)
            .expect("Falha na serialização bincode");
        
        let mut mac = HmacSha256::new_from_slice(key)
            .expect("HMAC aceita chaves de qualquer tamanho");
        mac.update(&bytes);
        
        let result = mac.finalize();
        let code_bytes = result.into_bytes();
        
        let mut hash = [0u8; HASH_SIZE];
        hash.copy_from_slice(&code_bytes);
        Hash(hash)
    }

    /// Verifica HMAC usando comparação de tempo constante
    /// 
    /// Previne timing attacks ao comparar hashes
    pub fn verify_hmac<T: Serialize>(
        expected_hmac: &Hash,
        data: &T,
        key: &[u8],
    ) -> bool {
        let calculated = Self::hmac(data, key);
        
        // Comparação de tempo constante
        use subtle::ConstantTimeEq;
        calculated.0.ct_eq(&expected_hmac.0).into()
    }

    /// Conta quantos bits zero existem no ínicio do hash
    pub fn count_leading_zeros(&self) -> usize {
        let mut count = 0;
        for &byte in &self.0 {
            if byte == 0 {
                count += 8;
            } else {
                count += byte.leading_zeros() as usize;
                break;
            }
        } count
    }


    // Verifica se o hash atende à dificuldade exigida
    pub fn has_sufficient_difficulty(&self, difficulty: usize) -> bool {
        if difficulty > HASH_SIZE * 8 {
            return false;
        } self.count_leading_zeros() >= difficulty
    }

    // Verifica se o hash comeca com N zeros hexadecimais
    pub fn starts_with_n_zeros(&self, n: usize) -> bool {
        let hex = self.to_hex();
        hex.chars().take(n).all(|c| c == '0')
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
        
        // Concatena os bytes do hash da esquerda com o da direita
        let mut combined = Vec::with_capacity(HASH_SIZE * 2);
        combined.extend_from_slice(left.as_bytes());
        combined.extend_from_slice(right.as_bytes());

        let mut hasher = Sha256::new();
        hasher.update(&combined);
        let first = hasher.finalize();

        // Segundo hash
        let mut hasher2 = Sha256::new();
        hasher2.update(&first);
        let result = hasher2.finalize();

        let mut hash_arr = [0u8; HASH_SIZE];
        hash_arr.copy_from_slice(&result);
        Hash(hash_arr)
    }



    // ========== MERKLE PROOF (Verificação Leve) ==========
    
    /// Gera uma prova de Merkle para uma transação específica
    /// 
    /// Permite provar que uma transação está no bloco sem
    /// precisar baixar todas as transações (SPV - Simplified Payment Verification)
    /// 
    /// Complexidade: O(log n) em vez de O(n)
    pub fn generate_merkle_proof(
        transactions: &[Transaction],
        tx_index: usize,
    ) -> Option<MerkleProof> {
        if tx_index >= transactions.len() {
            return None;
        }

        let mut siblings = Vec::new();
        let mut current_level: Vec<Hash> = transactions
            .iter()
            .map(|tx| Hash::hash_data(tx))
            .collect();
        
        let mut current_index = tx_index;

        // Sobe a árvore salvando os irmãos necessários
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            for (i, chunk) in current_level.chunks(2).enumerate() {
                let left = &chunk[0];
                let right = chunk.get(1).unwrap_or(left);

                // Se o índice atual está neste chunk, salva o irmão
                if i == current_index / 2 {
                    if current_index % 2 == 0 {
                        // TX está à esquerda, salva o da direita
                        siblings.push(*right);
                    } else {
                        // TX está à direita, salva o da esquerda
                        siblings.push(*left);
                    }
                }

                next_level.push(Hash::calculate_hash_tree_branch(left, right));
            }
            
            current_level = next_level;
            current_index /= 2;
        }

        Some(MerkleProof {
            index: tx_index,
            siblings,
        })
    }



    /// Verifica se uma transação pertence a uma Merkle Root usando a prova
    /// 
    /// Reconstrói o caminho até a raiz usando apenas os irmãos fornecidos
    /// 
    /// Retorna true se a transação realmente está no bloco
    pub fn verify_merkle_proof(
        tx: &Transaction,
        merkle_root: &Hash,
        proof: &MerkleProof,
    ) -> bool {
        let mut current_hash = Hash::hash_data(tx);
        let mut index = proof.index;

        // Reconstrói o caminho até a raiz
        for sibling in &proof.siblings {
            current_hash = if index % 2 == 0 {
                // TX está à esquerda
                Hash::calculate_hash_tree_branch(&current_hash, sibling)
            } else {
                // TX está à direita
                Hash::calculate_hash_tree_branch(sibling, &current_hash)
            };
            index /= 2;
        }

        // Comparação de tempo constante (previne timing attacks)
        use subtle::ConstantTimeEq;
        current_hash.0.ct_eq(&merkle_root.0).into()
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
        let hex = self.to_hex();
        f.debug_tuple("hash")
            .field(&format!("{}...{}", &hex[..8], &hex[56..]))
            .finish()
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
            D: Deserializer<'de> 
        {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(s).map_err(|e| {
            serde::de::Error::custom(format!("Erro ao decodificar hex: {}", e))
        })?;
        
        if bytes.len() != HASH_SIZE {
            return Err(serde::de::Error::custom(format!(
                "Tamanho de hash inválido: esperado {}, recebido {}",
                HASH_SIZE,
                bytes.len()
            )));
        }
        
        let mut arr = [0u8; HASH_SIZE];
        arr.copy_from_slice(&bytes);
        Ok(Hash(arr))
    }
}

