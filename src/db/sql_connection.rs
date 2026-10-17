use std::io;

use rusqlite::types::Type;
use rusqlite::{Connection, Result, Transaction as SqlTransaction, params};

use crate::crypto::hashing::Hash;
use crate::models::block::Block;
use crate::models::blockchain::UTXOKey;
use crate::models::transaction::{Output, Transaction};

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
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS blocks (
                height      INTEGER PRIMARY KEY,
                hash        TEXT NOT NULL UNIQUE,
                prev_hash   TEXT NOT NULL,
                merkle_root TEXT NOT NULL,
                timestamp   INTEGER NOT NULL,
                nonce       INTEGER NOT NULL,
                difficulty  INTEGER NOT NULL,
                raw_data    BLOB NOT NULL
            );

            CREATE TABLE IF NOT EXISTS transactions (
                tx_hash      TEXT PRIMARY KEY,
                block_hash   TEXT NOT NULL,
                block_height INTEGER NOT NULL,
                is_coinbase  INTEGER NOT NULL,
                raw_data     BLOB NOT NULL,
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
                tx_hash     TEXT PRIMARY KEY,
                raw_data    BLOB NOT NULL,
                received_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_utxos_pubkey ON utxos(pubkey);
            CREATE INDEX IF NOT EXISTS idx_tx_block ON transactions(block_hash);
            ",
        )
    }

    /// Saves a block and all of its transaction rows atomically.
    pub fn save_block(&mut self, block: &Block, height: usize) -> Result<()> {
        let height = to_sql_i64(height)?;
        let nonce = to_sql_i64(block.header.nonce)?;
        let difficulty = to_sql_i64(block.header.difficulty)?;
        let hash = block.header.calculate_hash().to_hex();
        let prev_hash = block.header.prev_block_hash.to_hex();
        let merkle_root = block.header.merkle_root.to_hex();
        let raw_data = serialize_for_sql(block)?;

        let transaction = self.conn.transaction()?;
        transaction.execute(
            "INSERT INTO blocks
            (height, hash, prev_hash, merkle_root, timestamp, nonce, difficulty, raw_data)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                height,
                hash,
                prev_hash,
                merkle_root,
                block.header.timestamp,
                nonce,
                difficulty,
                raw_data,
            ],
        )?;

        for tx in &block.transactions {
            Self::save_transaction(&transaction, tx, &hash, height)?;
        }

        transaction.commit()
    }

    pub fn load_all_blocks(&self) -> Result<Vec<Block>> {
        let mut statement = self
            .conn
            .prepare("SELECT raw_data FROM blocks ORDER BY height ASC")?;
        let rows = statement.query_map([], |row| {
            let raw_data: Vec<u8> = row.get(0)?;
            bincode::deserialize(&raw_data).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Blob, Box::new(error))
            })
        })?;
        rows.collect()
    }

    pub fn get_chain_height(&self) -> Result<usize> {
        let height: Option<i64> =
            self.conn
                .query_row("SELECT MAX(height) FROM blocks", [], |row| row.get(0))?;
        match height {
            Some(height) => from_sql_usize(height, 0),
            None => Ok(0),
        }
    }

    fn save_transaction(
        transaction: &SqlTransaction<'_>,
        tx: &Transaction,
        block_hash: &str,
        height: i64,
    ) -> Result<()> {
        let tx_hash = tx.calculate_hash().to_hex();
        let raw_data = serialize_for_sql(tx)?;
        transaction.execute(
            "INSERT INTO transactions
            (tx_hash, block_hash, block_height, is_coinbase, raw_data)
            VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                tx_hash,
                block_hash,
                height,
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
            |row| {
                let raw_data: Vec<u8> = row.get(0)?;
                bincode::deserialize(&raw_data).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(0, Type::Blob, Box::new(error))
                })
            },
        );
        match result {
            Ok(transaction) => Ok(Some(transaction)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn save_utxo(&self, key: &UTXOKey, output: &Output) -> Result<()> {
        let output_index = to_sql_i64(key.output_index)?;
        let value = to_sql_i64(output.value)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO utxos (tx_hash, output_index, value, pubkey, spent)
             VALUES (?1, ?2, ?3, ?4, 0)",
            params![key.tx_hash.to_hex(), output_index, value, output.pubkey],
        )?;
        Ok(())
    }

    pub fn spend_utxo(&self, key: &UTXOKey) -> Result<()> {
        let output_index = to_sql_i64(key.output_index)?;
        self.conn.execute(
            "UPDATE utxos SET spent = 1 WHERE tx_hash = ?1 AND output_index = ?2",
            params![key.tx_hash.to_hex(), output_index],
        )?;
        Ok(())
    }

    pub fn get_balance(&self, pubkey: &str) -> Result<u64> {
        let balance: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(value), 0) FROM utxos WHERE pubkey = ?1 AND spent = 0",
            params![pubkey],
            |row| row.get(0),
        )?;
        from_sql_u64(balance, 0)
    }

    pub fn get_utxos_for_address(&self, pubkey: &str) -> Result<Vec<(UTXOKey, Output)>> {
        let mut statement = self.conn.prepare(
            "SELECT tx_hash, output_index, value FROM utxos
             WHERE pubkey = ?1 AND spent = 0",
        )?;
        let rows = statement.query_map(params![pubkey], |row| {
            let tx_hash_hex: String = row.get(0)?;
            let bytes = hex::decode(&tx_hash_hex)
                .map_err(|error| conversion_error(0, Type::Text, error.to_string()))?;
            if bytes.len() != 32 {
                return Err(conversion_error(
                    0,
                    Type::Text,
                    "Transaction hash must be 32 bytes",
                ));
            }
            let mut hash_bytes = [0u8; 32];
            hash_bytes.copy_from_slice(&bytes);

            let output_index = from_sql_usize(row.get(1)?, 1)?;
            let value = from_sql_u64(row.get(2)?, 2)?;
            Ok((Hash::new(hash_bytes), output_index, value))
        })?;

        rows.map(|row| {
            let (tx_hash, output_index, value) = row?;
            let key = UTXOKey {
                tx_hash,
                output_index,
            };
            let output = Output {
                value,
                pubkey: pubkey.to_string(),
            };
            Ok((key, output))
        })
        .collect()
    }

    pub fn add_to_mempool(&self, tx: &Transaction) -> Result<()> {
        let tx_hash = tx.calculate_hash().to_hex();
        let raw_data = serialize_for_sql(tx)?;
        let received_at = chrono::Utc::now().timestamp();
        self.conn.execute(
            "INSERT OR IGNORE INTO mempool (tx_hash, raw_data, received_at)
             VALUES (?1, ?2, ?3)",
            params![tx_hash, raw_data, received_at],
        )?;
        Ok(())
    }

    /// Returns pending transactions and deletes them in one transaction. A
    /// malformed row aborts the operation and leaves the mempool intact.
    pub fn drain_mempool(&mut self) -> Result<Vec<Transaction>> {
        let transaction = self.conn.transaction()?;
        let pending = {
            let mut statement =
                transaction.prepare("SELECT raw_data FROM mempool ORDER BY received_at")?;
            let rows = statement.query_map([], |row| {
                let raw_data: Vec<u8> = row.get(0)?;
                bincode::deserialize(&raw_data).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(0, Type::Blob, Box::new(error))
                })
            })?;
            rows.collect::<Result<Vec<Transaction>>>()?
        };
        transaction.execute("DELETE FROM mempool", [])?;
        transaction.commit()?;
        Ok(pending)
    }
}

fn serialize_for_sql<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
    bincode::serialize(value)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

fn to_sql_i64<T>(value: T) -> Result<i64>
where
    T: TryInto<i64>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    value
        .try_into()
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

fn from_sql_usize(value: i64, column: usize) -> Result<usize> {
    usize::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(column, Type::Integer, Box::new(error))
    })
}

fn from_sql_u64(value: i64, column: usize) -> Result<u64> {
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(column, Type::Integer, Box::new(error))
    })
}

fn conversion_error(column: usize, column_type: Type, message: impl ToString) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column,
        column_type,
        Box::new(io::Error::new(
            io::ErrorKind::InvalidData,
            message.to_string(),
        )),
    )
}
