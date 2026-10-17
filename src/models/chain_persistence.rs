use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::models::blockchain::{BLOCKCHAIN_SNAPSHOT_VERSION, Blockchain, BlockchainSnapshot};

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

impl Blockchain {
    /// Writes a snapshot to a temporary file in the destination directory and
    /// replaces the previous file only after the new snapshot is fully synced.
    pub fn save_to_file(&self, path: &str) -> io::Result<()> {
        let snapshot = BlockchainSnapshot {
            version: BLOCKCHAIN_SNAPSHOT_VERSION,
            chain: self.chain.clone(),
            utxos: self
                .utxos
                .iter()
                .map(|(key, output)| (key.clone(), output.clone()))
                .collect(),
            mempool: self.mempool.clone(),
        };
        let destination = Path::new(path);
        let (mut file, temporary_path) = create_temporary_file(destination)?;

        if let Err(error) = serde_json::to_writer_pretty(&mut file, &snapshot) {
            let _ = fs::remove_file(&temporary_path);
            return Err(io::Error::new(io::ErrorKind::InvalidData, error));
        }
        if let Err(error) = file.write_all(b"\n").and_then(|()| file.sync_all()) {
            let _ = fs::remove_file(&temporary_path);
            return Err(error);
        }
        drop(file);

        if let Err(error) = fs::rename(&temporary_path, destination) {
            let _ = fs::remove_file(&temporary_path);
            return Err(error);
        }

        #[cfg(unix)]
        {
            let parent = parent_directory(destination);
            File::open(parent)?.sync_all()?;
        }

        println!("Blockchain saved to: {}", path);
        Ok(())
    }

    /// Loads a snapshot, replays its blocks to rebuild the UTXO set, and keeps
    /// only mempool transactions that remain valid against the rebuilt state.
    pub fn load_from_file(path: &str) -> io::Result<Self> {
        let file = File::open(path)?;
        let snapshot: BlockchainSnapshot = serde_json::from_reader(file)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        if snapshot.version > BLOCKCHAIN_SNAPSHOT_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Unsupported blockchain snapshot version {}",
                    snapshot.version
                ),
            ));
        }

        let mut blockchain = Blockchain::rebuild_from_chain(&snapshot.chain)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        for transaction in snapshot.mempool {
            blockchain.submit_transaction(transaction);
        }

        Ok(blockchain)
    }
}

fn create_temporary_file(destination: &Path) -> io::Result<(File, PathBuf)> {
    let parent = parent_directory(destination);
    let file_name = destination.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Snapshot destination must include a file name",
        )
    })?;

    for _ in 0..32 {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = OsString::from(".");
        temporary_name.push(file_name);
        temporary_name.push(format!(".tmp.{}.{}", std::process::id(), sequence));
        let temporary_path = parent.join(temporary_name);

        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => return Ok((file, temporary_path)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Could not allocate a unique temporary snapshot file",
    ))
}

fn parent_directory(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}
