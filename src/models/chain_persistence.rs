use std::fs::File;
use std::io::{Write, Read};
use std::collections::HashMap;
use crate::models::blockchain::{Blockchain, BlockchainSnapshot, UTXOKey};
use crate::models::transaction::Output;

impl Blockchain {
    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let snapshot = BlockchainSnapshot {
            chain: self.chain.clone(),
            utxos: self.utxos.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            mempool: self.mempool.clone(),
        };

        let json = serde_json::to_string_pretty(&snapshot).expect("Erro ao serializar");
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
        println!(" Blockchain salva em: {}", path);
        Ok(())
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let mut json = String::new();
        file.read_to_string(&mut json)?;

        let snapshot: BlockchainSnapshot = serde_json::from_str(&json).expect("Erro ao ler JSON");
        let utxos_map: HashMap<UTXOKey, Output> = snapshot.utxos.into_iter().collect();

        Ok(Blockchain {
            chain: snapshot.chain,
            utxos: utxos_map,
            mempool: snapshot.mempool,
        })
    }
}