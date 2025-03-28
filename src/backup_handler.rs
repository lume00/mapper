use std::time::Duration;

use log::{debug, error};
use smol::{
    fs::{self, create_dir_all, File, OpenOptions}, io::{AsyncReadExt, AsyncWriteExt}, stream::StreamExt, Timer
};

use crate::storage::Storage;

const MDB_FILE_NAME: &'static str = "mdb";
const MDB_FILE_EXTENSION: &'static str = "txt";

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

    async fn recover(&self, storage_shard_len: usize) {
        let shard_dir_path = format!("{}/backup", self.path);
        let mut entries = match fs::read_dir(&shard_dir_path).await {
            Ok(entries) => entries,
            Err(e) => {
                debug!("error reading shard directory: {}", e);
                return;
            }
        };

        while let Some(entry) = entries.next().await {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    error!("error reading directory entry: {}", e);
                    continue;
                }
            };
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some(MDB_FILE_EXTENSION) {
                let mut file = match File::open(&path).await {
                    Ok(file) => file,
                    Err(e) => {
                        error!("error opening shard file: {}", e);
                        continue;
                    }
                };
                let mut buff = Vec::new();
                if let Err(e) = file.read_to_end(&mut buff).await {
                    error!("error reading shard file: {}", e);
                    continue;
                }
               
                match bincode::deserialize(&buff) {
                    Ok(deserialized_shard) => {
                        let shard_num: usize = path.file_stem().and_then(|s| s.to_str()).and_then(|s| s.split('_').last()).and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
                        if shard_num < storage_shard_len {
                            *self.storage.0[shard_num].write().await = deserialized_shard;
                        }
                    }
                    Err(e) => error!("error deserializing shard file: {}", e),
                }
            }
        }
    }

    pub(crate) async fn recover_and_backup(&self) {
        let storage_shard_len = self.storage.0.len();
        self.recover(storage_shard_len).await;

        let interval = self.interval.clone();
        let path = self.path.clone();
        let storage = self.storage.clone();

        let mut ticker = Timer::interval(interval);

        smol::spawn(async move {
            Timer::after(interval.clone()).await;
            loop {
                if let None = ticker.next().await {
                    break;
                }
                
                for i in 0..storage_shard_len {
                    let curr_shard = storage.0.get(i).unwrap();

                    match bincode::serialize(&curr_shard.read().await.0) {
                        Ok(ser_content) => {
                            if let Err(e) = write_backup(&path, ser_content, i).await {
                                error!("{}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            error!("{}", e);
                            break;
                        }
                    }
                }
            }
        })
        .detach();
    }
}

#[inline]
fn get_mdb_shard(shard_num: usize) -> String {
    format!("{}_{}.{}", MDB_FILE_NAME, shard_num, MDB_FILE_EXTENSION)
}

async fn write_backup(path: &str, content: Vec<u8>, shard_num: usize) -> std::io::Result<()> {
    println!("{}", String::from_utf8_lossy(&content[..]));
    // Create the directory for storing shard files if it doesn't exist
    let shard_dir_path = format!("{}/backup", path);
    create_dir_all(&shard_dir_path).await?;
 
    // Create or overwrite the MDB file for the shard
    let mdb_file_path = format!("{}/{}", shard_dir_path, get_mdb_shard(shard_num));
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&mdb_file_path)
        .await?;

    file.write_all(&content[..]).await?;
    file.flush().await?;
    file.close().await?;
    
    Ok(())
}
