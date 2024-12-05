use std::{fmt::Display, time::Duration};

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
pub struct WrappedRecord {
    pub record: Record,
    pub detatched_task_ch: Option<Sender<RacingResult>>,
}

#[derive(Debug)]
pub enum RacingResult {
    Timout,
    Closed,
    Cancelled,
}

impl Display for RacingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl WrappedRecord {
    pub fn new(db: Storage, shard_index: usize, key: &str, record: Record) -> WrappedRecord {
        match &record.ttl_policy {
            Some(ttl_policy) => {
                let key: String = key.to_string();
                let ttl = ttl_policy.ttl.clone();
                let tc_s = create_ttl_channel(db, shard_index, key, ttl);

                WrappedRecord {
                    record,
                    detatched_task_ch: Some(tc_s),
                }
            }
            None => WrappedRecord {
                record,
                detatched_task_ch: None,
            },
        }
    }

    pub fn update_ttl_policy(&mut self, maybe_new_ttl: Option<Duration>, db: Storage, shard_index: usize, key: String) {
        match maybe_new_ttl {
            Some(new_ttl) => {
                if let Some(detatched_task_ch) = &self.detatched_task_ch {
                    let _ = detatched_task_ch.send(RacingResult::Cancelled);
                }
                //updating ttl
                self.record.update_ttl_policy(new_ttl);

                //creating new ttl channel
                self.detatched_task_ch = Some(create_ttl_channel(db, shard_index, key, new_ttl));
            }
            None => {
                //cancelling previous ttl
                if let Some(detatched_task_ch) = &self.detatched_task_ch {
                    let _ = detatched_task_ch.send(RacingResult::Cancelled);
                }

                self.record.remove_ttl_policy();
            }
        };
    }
}

fn create_ttl_channel(db: Storage, shard_index: usize, key: String, ttl: Duration) -> Sender<RacingResult> {
    let (tc_s, tc_r) = smol::channel::bounded::<RacingResult>(1);

    let ttl_check = ttl_check_fn(db, shard_index, key, tc_r, ttl);
    smol::spawn(ttl_check).detach();
    tc_s
}

async fn ttl_check_fn(
    storage: Storage,
    shard_index: usize,
    key: String,
    detatched_task_ch: Receiver<RacingResult>,
    ttl: Duration,
) {
    // waiting for 3 futures, the first that completes win:
    // 1) if timer is cancelled or closed
    // 2) if timer has timed out
    let racing_result = race(
        async {
            match detatched_task_ch.recv().await {
                Ok(cancelled) => cancelled,
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
                if let Some(wrecord) = locked_table.get(&key) {
                    if let Some(_) = &wrecord.record.ttl_policy {
                        debug!("timout occured, ttl is expired, removing key {}", key);
                        let _prev = locked_table.remove(&key);
                    }
                }
            }
            // channel is cancelled
            RacingResult::Cancelled => debug!("channel cancelled for key {}", key),
            //channel is closed due to record drop
            RacingResult::Closed => debug!("channel closed for key {}", key),
        }
    }
}
