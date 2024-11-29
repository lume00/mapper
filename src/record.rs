use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Record {
    pub data: Vec<u8>,
    pub ttl_policy: Option<TTLPolicy>,
}

#[derive(Debug, Clone)]
pub struct TTLPolicy {
    pub ttl: Duration,
    pub creation: Instant,
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

    pub fn update_ttl(&mut self, ttl: Duration) {
        if let Some(policy) = &mut self.ttl_policy {
            policy.ttl = ttl;
        }
    }
}

impl TTLPolicy {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            creation: Instant::now(),
        }
    }

    fn is_expired(&self) -> bool {
        self.creation.elapsed() < self.ttl
    }

    pub fn expire_in(&self) -> Duration {
        if self.is_expired() {
            self.ttl - self.creation.elapsed()
        } else {
            Duration::ZERO
        }
    }
}
