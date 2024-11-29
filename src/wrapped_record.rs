use std::{
    fmt::Display,
    time::Duration,
};

use log::debug;
use smol::{
    channel::{Receiver, Sender},
    future::race,
    Timer,
};

use crate::{
    record::Record,
    storage::Storage,
};

#[derive(Debug, Clone)]
pub struct WrapperRecord {
    pub record: Record,
    pub detatched_task_ch: Option<Sender<Interruption>>,
}

#[derive(Debug)]
pub enum Interruption {
    Cancelled,
    TTLChanged,
}

#[derive(Debug)]
pub enum RacingResult {
    Timout,
    Closed,
    Custom(Interruption),
}

impl Display for RacingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl WrapperRecord {
    pub fn new(db: Storage, shard_index: usize, key: &str, record: Record) -> WrapperRecord {
        match &record.ttl_policy {
            Some(ttl_policy) => {
                let key: String = key.to_string();
                let ttl = ttl_policy.ttl.clone();

                let (tc_s, tc_r) = smol::channel::bounded::<Interruption>(1);

                let ttl_check = ttl_check_fn(db, shard_index, key, tc_r, ttl);
                smol::spawn(ttl_check).detach();

                WrapperRecord {
                    record,
                    detatched_task_ch: Some(tc_s),
                }
            }
            None => WrapperRecord {
                record,
                detatched_task_ch: None,
            },
        }
    }
}

async fn ttl_check_fn(
    storage: Storage,
    shard_index: usize,
    key: String,
    detatched_task_ch: Receiver<Interruption>,
    ttl: Duration,
) {
    // waiting for 3 futures, the first that completes win:
    // 1) if timer is cancelled or closed
    // 2) if ttl has changed
    // 3) if timer has timed out
    let racing_result = race(
        async {
            match detatched_task_ch.recv().await {
                Ok(Interruption::Cancelled) => RacingResult::Custom(Interruption::Cancelled),
                Ok(Interruption::TTLChanged) => RacingResult::Custom(Interruption::TTLChanged),
                Err(_) => RacingResult::Closed,
            }
        },
        async {
            Timer::after(ttl).await;
            RacingResult::Timout
        },
    )
    .await;

    if let Some(shard) = storage.0.get(shard_index) {
        match racing_result {
            // timer has timed out
            RacingResult::Timout => {
                let mut locked_table = shard.write().await;
                if let Some(record) = locked_table.get(&key) {
                    if let Some(_) = &record.record.ttl_policy {                        
                        debug!("timout occured, ttl is expired, removing key {}", key);
                        let _prev = locked_table.remove(&key);
                    }
                }
                //dropping table write lock
            }
            // ttl has changed need to spawn new task and cancel current running
            RacingResult::Custom(Interruption::TTLChanged) => {
                if let Some(record) = shard.read().await.get(&key) {
                    if let Some(_) = &record.record.ttl_policy {
                        Box::pin(ttl_check_fn(
                            storage.clone(),
                            shard_index,
                            key,
                            detatched_task_ch,
                            ttl,
                        ))
                        .await;
                    }
                }
            }
            // channel is cancelled
            RacingResult::Custom(Interruption::Cancelled) => {
                debug!("channel cancelled for key {}", key)
            }
            RacingResult::Closed => debug!("channel closed for key {}", key),
        }
    }
}
