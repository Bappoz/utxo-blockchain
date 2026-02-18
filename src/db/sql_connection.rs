use crate::models::block::Block;
use crate::models::blockchain::UTXOKey;
use crate::models::transaction::{Output, Transaction};
use rusqlite::{params, Connection, Result};

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS blocks (
                height      INTEGER PRIMARY KEY,
                hash        TEXT NOT NULL UNIQUE,
                prev_hash   TEXT NOT NULL,
                merkle_root TEXT NOT NULL,
                timestamp   INTEGER NOT NULL,
                nonce       INTEGER NOT NULL,
                difficulty  INTEGER NOT NULL,
                raw_data    BLOB NOT NULL      -- bloco serializado completo
            );

            CREATE TABLE IF NOT EXISTS transactions (
                tx_hash     TEXT PRIMARY KEY,
                block_hash  TEXT NOT NULL,
                block_height INTEGER NOT NULL,
                is_coinbase INTEGER NOT NULL,
                raw_data    BLOB NOT NULL,
                FOREIGN KEY (block_hash) REFERENCES blocks(hash)
            );

            CREATE TABLE IF NOT EXISTS utxos (
                tx_hash      TEXT NOT NULL,
                output_index INTEGER NOT NULL,
                value        INTEGER NOT NULL,
                pubkey       TEXT NOT NULL,
                spent        INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (tx_hash, output_index)
            );

            CREATE TABLE IF NOT EXISTS mempool (
                tx_hash  TEXT PRIMARY KEY,
                raw_data BLOB NOT NULL,
                received_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_utxos_pubkey ON utxos(pubkey);
            CREATE INDEX IF NOT EXISTS idx_tx_block ON transactions(block_hash);
            ",
        )?;
        Ok(())
    }

    pub fn save_block(&self, block: &Block, height: usize) -> Result<()> {
        let hash = block.header.calculate_hash().to_hex();
        let prev_hash = block.header.prev_block_hash.to_hex();
        let merkle_root = block.header.merkle_root.to_hex();
        let raw_data = bincode::serialize(block).unwrap();

        self.conn.execute(
            "INSERT OR REPLACE INTO blocks
            (height, hash, prev_hash, merkle_root, timestamp, nonce, difficulty, raw_data)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                height as i64,
                hash,
                prev_hash,
                merkle_root,
                block.header.timestamp as i64,
                block.header.nonce as i64,
                block.header.difficulty as i64,
                raw_data,
            ],
        )?;

        // Sabe each transaction block
        for tx in &block.transactions {
            self.save_transactions(tx, &hash, height)?;
        }
        Ok(())
    }

    pub fn load_all_blocks(&self) -> Result<Vec<Block>> {
        let mut stmt = self
            .conn
            .prepare("SELECT raw_data FROM blocks ORDER by height ASC")?;
        let blocks = stmt
            .query_map([], |row| {
                let raw: Vec<u8> = row.get(0)?;
                Ok(raw)
            })?
            .filter_map(|r| r.ok())
            .filter_map(|raw| bincode::deserialize::<Block>(&raw).ok())
            .collect();

        Ok(blocks)
    }

    pub fn get_chain_height(&self) -> Result<usize> {
        let height: usize =
            self.conn
                .query_row("SELECT COALESCE(MAX(height), 0) FROM blocks", [], |row| {
                    row.get::<_, i64>(0).map(|h| h as usize)
                })?;
        Ok(height)
    }

    fn save_transactions(&self, tx: &Transaction, block_hash: &str, height: usize) -> Result<()> {
        let tx_hash = tx.calculate_hash().to_hex();
        let raw_data = bincode::serialize(tx).unwrap();

        self.conn.execute(
            "INSERT OR REPLACE INTO transactions
            (tx_hash, block_hash, block_height, is_coinbase, raw_data)
            VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                tx_hash,
                block_hash,
                height as i64,
                tx.is_coinbase() as i32,
                raw_data
            ],
        )?;
        Ok(())
    }

    pub fn get_transaction(&self, tx_hash: &str) -> Result<Option<Transaction>> {
        let result = self.conn.query_row(
            "SELECT raw_data FROM transactions WHERE tx_hash = ?1",
            params![tx_hash],
            |row| row.get::<_, Vec<u8>>(0),
        );
        match result {
            Ok(raw) => Ok(bincode::deserialize(&raw).ok()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // ─── UTXOs ─────────────────────────────────────────────────────────────

    pub fn save_utxo(
        &self,
        key: &UTXOKey,
        output: &crate::models::transaction::Output,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO utxos (tx_hash, output_index, value, pubkey, spent)
             VALUES (?1, ?2, ?3, ?4, 0)",
            params![
                key.tx_hash.to_hex(),
                key.output_index as i64,
                output.value as i64,
                output.pubkey,
            ],
        )?;
        Ok(())
    }

    pub fn spend_utxo(&self, key: &UTXOKey) -> Result<()> {
        self.conn.execute(
            "UPDATE utxos SET spent = 1 WHERE tx_hash = ?1 AND output_index = ?2",
            params![key.tx_hash.to_hex(), key.output_index as i64],
        )?;
        Ok(())
    }

    pub fn get_balance(&self, pubkey: &str) -> Result<u64> {
        let balance: u64 = self.conn.query_row(
            "SELECT COALESCE(SUM(value), 0) FROM utxos WHERE pubkey = ?1 AND spent = 0",
            params![pubkey],
            |row| row.get::<_, i64>(0).map(|v| v as u64),
        )?;
        Ok(balance)
    }

    pub fn get_utxos_for_address(&self, pubkey: &str) -> Result<Vec<(UTXOKey, Output)>> {
        // Útil para construir transações (wallet)
        let mut stmt = self.conn.prepare(
            "SELECT tx_hash, output_index, value FROM utxos
             WHERE pubkey = ?1 AND spent = 0",
        )?;
        let rows = stmt.query_map(params![pubkey], |row| {
            let tx_hash_hex: String = row.get(0)?;
            let output_index: usize = row.get::<_, i64>(1)? as usize;
            let value: u64 = row.get::<_, i64>(2)? as u64;
            Ok((tx_hash_hex, output_index, value))
        })?;

        let mut result = Vec::new();
        for row in rows.flatten() {
            let (tx_hash_hex, output_index, value) = row;
            if let Ok(bytes) = hex::decode(&tx_hash_hex) {
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    use crate::crypto::hashing::Hash;
                    let key = UTXOKey {
                        tx_hash: Hash::new(arr),
                        output_index,
                    };
                    let output = Output {
                        value,
                        pubkey: pubkey.to_string(),
                    };
                    result.push((key, output));
                }
            }
        }
        Ok(result)
    }

    // ─── MEMPOOL ───────────────────────────────────────────────────────────

    pub fn add_to_mempool(&self, tx: &Transaction) -> Result<()> {
        let tx_hash = tx.calculate_hash().to_hex();
        let raw_data = bincode::serialize(tx).unwrap();
        let now = chrono::Utc::now().timestamp();

        self.conn.execute(
            "INSERT OR IGNORE INTO mempool (tx_hash, raw_data, received_at)
             VALUES (?1, ?2, ?3)",
            params![tx_hash, raw_data, now],
        )?;
        Ok(())
    }

    pub fn drain_mempool(&self) -> Result<Vec<Transaction>> {
        let mut stmt = self
            .conn
            .prepare("SELECT raw_data FROM mempool ORDER BY received_at")?;
        let txs: Vec<Transaction> = stmt
            .query_map([], |row| row.get::<_, Vec<u8>>(0))?
            .filter_map(|r| r.ok())
            .filter_map(|raw| bincode::deserialize(&raw).ok())
            .collect();

        self.conn.execute("DELETE FROM mempool", [])?;
        Ok(txs)
    }
}
