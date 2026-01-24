use ed25519_dalek::VerifyingKey;
use crate::models::{block::Block, transaction::{Output, Transaction}};
use crate::crypto::hashing::Hash;
use std::collections::HashMap;

///Representa o identificador único de um Output na rede
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UTXOKey{
    pub tx_hash: Hash,
    pub output_index: usize,
}

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub utxos: HashMap<UTXOKey, Output>,
}

impl Blockchain {
    /// Inicializa o blockchain com o block genesis
    pub fn new(genesis_block: Block) -> Self {
        let mut bc = Blockchain {
            chain: Vec::new(),
            utxos: HashMap::new(),
        };
        // Ao iniciar, processa o bloco gênesis para popular os primeiros UTXOs
        bc.add_block(genesis_block);
        bc
    }

    //Adiciona um bloco à corrente e atualiza o UTXO set
    pub fn add_block(&mut self, block: Block) {
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
    pub fn validate_transaction(&self, tx: Transaction) -> bool {
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
}