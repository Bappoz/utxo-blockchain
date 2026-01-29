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
    chain: Vec<Block>,
    utxos: Vec<(UTXOKey, Output)>,
    mempool: Vec<Transaction>,
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
        if !self.validate_transaction(&tx) {
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
        // 1. Cria a recompensa (Coinbase)
        let mut transactions = vec![Transaction::coinbase(miner_addr, MINING_REWARD)];

        // 2. Coleta TODAS as transações da mempool e limpa a fila
        // O método drain(..) remove os itens e os retorna
        transactions.extend(self.mempool.drain(..));

        // 3. Pega o hash do bloco anterior
        let prev_hash = self.chain.last()
            .map(|b| b.header.calculate_hash())
            .unwrap_or(Hash::new_empty());

        // 4. Cria o bloco pronto para minerar
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

    /// Valida se uma transação pode ser aceita antes de minerar o bloco.
    /// Esta é a defesa principal contra Double Spending e falta de fundos.
    pub fn validate_transaction(&self, tx: &Transaction) -> bool {
        if tx.is_coinbase() { return true }

        let mut input_value = 0;

        for input in &tx.inputs {
            let key = UTXOKey {
                tx_hash: input.prev_tx_hash,
                output_index: input.output_index,
            };

            // Double spending check. O Utxo existe no nosso set?
            if let Some(utxo) = self.utxos.get(&key) {
                input_value += utxo.value;  

                // RECUPERAR A CHAVE PÚBLICA DO DONO
                let pubkey_bytes = hex::decode(&utxo.pubkey).expect("Falha ao decodificar hex");
                let public_key = VerifyingKey::from_bytes(
                    &pubkey_bytes.try_into().expect("Tamanho de chave inválido")
                ).expect("Falha ao reconstruir VerifyingKey");

                // A assinatura do Input prova que o dono autorizou?
                if tx.verify(&public_key) == false{
                    println!(" Erro: Assinatura inválida para o UTXO fornecido!");
                    return false;
                }
            } else {
                println!(" Erro: Input aponta para UTXO inexistente ou já gasto!");
                return false;
            }          
        }

        // Verificar Saldo
        let output_value: u64 = tx.outputs.iter().map(|o| o.value).sum();
        if input_value < output_value {
            println!(" Erro: Saldo insuficiente!");
            return false;
        } true
    }




    // Valida um bloco completo antes de adcionar à corrente
    pub fn validate_block(&self, block: &Block) -> bool {
        // Verifica se o bloco aponta para o hash
        if let Some(last_block) = self.chain.last() {
            if block.header.prev_block_hash != last_block.header.calculate_hash(){
                println!("Bloco aponta para o hash anterior incorreto");
                return false;
            }
        }

        let block_hash = block.header.calculate_hash();
        if block_hash.count_leading_zeros() < block.header.difficulty {
            println!(" Erro: Bloco não atingiu o objetivo de bits ({} bits necessários)", block.header.difficulty);
            return false;
        }

        // Regra de Recompensa
        if !self.validate_mining_reward(block) {
            return false;
        }

        for tx in &block.transactions {
            if !self.validate_transaction(tx){
                return false;
            }
        }
        true
    }



    pub fn validate_mining_reward(&self, block: &Block) -> bool {
        let coinbase: Vec<&Transaction> = block.transactions.iter()
            .filter(|tx| tx.is_coinbase())
            .collect();

        if coinbase.len() != 1 {
            println!("Bloco deve ter exatamente uma transação coinbase");
            return false;
        }

        if coinbase[0].outputs[0].value != MINING_REWARD {
            println!("Recompensa de mineracao incorreta");
            return false;
        }
        true
    }




    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let snapshot = BlockchainSnapshot {
            chain: self.chain.clone(),
            utxos: self.utxos.iter().map(|(k, y)| (k.clone(), y.clone())).collect(),
            mempool: self.mempool.clone(),
        };

        let json = serde_json::to_string_pretty(&snapshot)
            .expect("Erro ao serializar blockchain");
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;

        println!("Blockchain salvo com sucesso em: {}", path);
        Ok(())
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let mut json = String::new();
        file.read_to_string(&mut json)?;

        let snapshots: BlockchainSnapshot = serde_json::from_str(&json)
            .expect("Error ao ler json");
        let utxos_map: HashMap<UTXOKey, Output> = snapshots.utxos.into_iter().collect();

        Ok(Blockchain{
            chain: snapshots.chain,
            utxos: utxos_map,
            mempool: snapshots.mempool,
        })
    }
}