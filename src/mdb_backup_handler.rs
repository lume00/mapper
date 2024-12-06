use std::{fmt::format, time::Duration};

use log::{debug, error};
use serde::Serialize;
use smol::{fs::{self, File, OpenOptions}, io::AsyncWriteExt, Timer};

use crate::storage::Storage;

pub(crate) struct BackupHandler {
    interval: Duration,
    path: String,
    storage: Storage,
}

impl BackupHandler {
    pub(crate) fn new(interval: Duration, path: String, storage: Storage) -> Self {
        Self {
            interval,
            path,
            storage,
        }
    }

    pub(crate) async fn start_backup(&self) {
        let insterval = self.interval.clone();
        let path = self.path.clone();
        let storage = self.storage.clone();
        smol::spawn(async move {
            loop {

                let storage_shard_len = storage.0.len();
                for i in 0..storage_shard_len {
                    let curr_shard = storage.0.get(i).unwrap();
                    match bincode::serialize(&curr_shard.read().await.0) {
                        Ok(bin_encoded) => if let Err(e) =  append_to_file(&path, bin_encoded).await {
                            error!("{}", e);
                            break;
                        } else {
                            if i == storage_shard_len - 1 {
                                match fs::rename("mdb.tmp", "mdb").await {
                                    Ok(_) => debug!("mdb file successfully created"),
                                    Err(e) => error!("error renaming file: {}", e),
                                }
                            }
                        },
                        Err(e) => {
                            error!("{}", e);
                            break;
                        },
                    }
                }

                Timer::after(insterval).await;
            }
        })
        .detach();
    }
}

async fn append_to_file(file_path: &str, content: Vec<u8>) -> std::io::Result<()> {
    // Open the file in append mode
    let mut file = OpenOptions::new()
        .append(true)
        .create(true) // Create the file if it does not exist
        .open(format!("∂mdb.tmp"))
        .await?;
    
    // Write the content to the file
    file.write_all(&content).await?;
    Ok(())
}
