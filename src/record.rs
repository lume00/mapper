use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub data: Vec<u8>,
    pub ttl_policy: Option<TTLPolicy>,
}

#[derive(Debug, Clone)]
pub struct TTLPolicy {
    pub ttl: Duration,
    pub last_policy_update: Instant,
}

impl Record {
    pub fn new(data: Vec<u8>, ttl: Option<Duration>) -> Self {
        Self {
            data,
            ttl_policy: {
                match ttl {
                    Some(ttl) => Some(TTLPolicy::new(ttl)),
                    None => None,
                }
            },
        }
    }

    /// Updates the Time-To-Live (TTL) policy of the current instance.
    /// - If a TTL policy already exists (`self.ttl_policy` is `Some`), this function updates its `ttl` value.
    /// - If no TTL policy exists (`self.ttl_policy` is `None`), this function initializes a new `TTLPolicy`
    ///   with the provided TTL value and assigns it to `self.ttl_policy`.
    ///
    /// This function ensures that the TTL policy is always set to a valid state, 
    /// either by updating an existing policy or creating a new one.
    pub fn update_ttl_policy(&mut self, ttl: Duration) {
        self.ttl_policy = Some(TTLPolicy::new(ttl))
    }

    pub fn remove_ttl_policy(&mut self) {
        self.ttl_policy = None;
    }
}

impl TTLPolicy {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            last_policy_update: Instant::now(),
        }
    }

    fn is_expired(&self) -> bool {
        self.last_policy_update.elapsed() > self.ttl
    }

    pub fn expire_in(&self) -> Duration {
        if !self.is_expired() {
            self.ttl - self.last_policy_update.elapsed()
        } else {
            Duration::ZERO
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct SerializableTTLPolicy {
    ttl_secs: u64,
    last_policy_update: u64,
}

impl From<TTLPolicy> for SerializableTTLPolicy {
    fn from(policy: TTLPolicy) -> Self {
        SerializableTTLPolicy {
            ttl_secs: policy.ttl.as_secs(),
            last_policy_update: policy.last_policy_update.elapsed().as_secs(),
        }
    }
}

impl From<SerializableTTLPolicy> for TTLPolicy {
    fn from(serializable: SerializableTTLPolicy) -> Self {
        TTLPolicy {
            ttl: Duration::from_secs(serializable.ttl_secs),
            last_policy_update: Instant::now() - Duration::from_secs(serializable.last_policy_update),
        }
    }
}

impl Serialize for TTLPolicy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let serializable = SerializableTTLPolicy::from(self.clone());
        serializable.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TTLPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let serializable = SerializableTTLPolicy::deserialize(deserializer)?;
        Ok(TTLPolicy::from(serializable))
    }
}
