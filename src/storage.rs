use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
    time::Duration,
};

use crate::{
    errors::TransactionError,
    record::Record,
    wrapped_record::{TTLResult, WrappedRecord},
};
use crossbeam_utils::CachePadded;
use serde::{Deserialize, Serialize};
use smol::lock::RwLock;

#[derive(Debug, Clone)]
pub struct Storage(pub(crate) Arc<[CachePadded<RwLock<Shard>>; 4]>);

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Shard(pub(crate) HashMap<String, WrappedRecord>);

impl Default for Storage {
    fn default() -> Self {
        Self(Arc::new(
            std::array::from_fn(|_| CachePadded::new(RwLock::new(Shard::default())))
        ))
    }
}

impl Storage {
    fn hash_key(&self, key: &str) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hasher_finished = hasher.finish() as usize;
        let len = self.0.len();

        hasher_finished % len
    }

    pub async fn flush_all(&self) {
        for rwlock in self.0.iter() {
            rwlock.write().await.0.clear();
        }
    }

    pub async fn db_size(&self) -> usize {
        let mut tot_cap: usize = 0;
        for rwlock in self.0.iter() {
            tot_cap += rwlock.read().await.0.capacity();
        }
        return tot_cap;
    }

    pub async fn get_record(&self, key: &str) -> Result<Record, TransactionError> {
        match self.0.get(self.hash_key(key)) {
            Some(shard) => match shard.read().await.0.get(key) {
                Some(data) => Ok(data.record.clone()),
                None => Err(TransactionError::RecordNotFound),
            },
            None => Err(TransactionError::ShardNotFound),
        }
    }

    pub async fn update_ttl(
        &self,
        key: &str,
        new_ttl: Option<Duration>,
    ) -> Result<(), TransactionError> {
        let hash_index = self.hash_key(key);
        match self.0.get(hash_index) {
            Some(shard) => {
                let mut record_lock = shard.write().await;
                match record_lock.0.get_mut(key) {
                    Some(wrecord) => {
                        wrecord.update_ttl_policy(
                            new_ttl,
                            self.clone(),
                            hash_index,
                            key.to_owned(),
                        );

                        Ok(())
                    }
                    None => Err(TransactionError::RecordNotFound),
                }
            }
            None => Err(TransactionError::ShardNotFound),
        }
    }

    pub async fn set_record(
        &self,
        key: &str,
        client_record: Record,
    ) -> Result<(), TransactionError> {
        let shard_index = self.hash_key(key);
        match self.0.get(shard_index) {
            Some(shard) => {
                let mut locked_db = shard.write().await;
                let maybe_prev = locked_db.0.insert(
                    key.to_owned(),
                    WrappedRecord::new(self.clone(), shard_index, key, client_record),
                );

                if let Some(prev) = maybe_prev {
                    if let Some(timer) = prev.detatched_task_ch {
                        let _ = timer.send(TTLResult::Cancelled);
                    }
                }
                return Ok(());
            }
            None => Err(TransactionError::ShardNotFound),
        }
    }

    pub async fn remove_record(&self, key: &String) -> Result<(), TransactionError> {
        match self.0.get(self.hash_key(key)) {
            Some(shard) => {
                let maybe_prev = shard.write().await.0.remove(key);
                if let Some(prev) = maybe_prev {
                    if let Some(timer) = prev.detatched_task_ch {
                        let _ = timer.send(TTLResult::Cancelled);
                    }
                }

                Ok(())
            }
            None => Err(TransactionError::ShardNotFound),
        }
    }
}
