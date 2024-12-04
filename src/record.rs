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

    /// Updates the Time-To-Live (TTL) policy of the current instance.
    /// - If a TTL policy already exists (`self.ttl_policy` is `Some`), this function updates its `ttl` value.
    /// - If no TTL policy exists (`self.ttl_policy` is `None`), this function initializes a new `TTLPolicy`
    ///   with the provided TTL value and assigns it to `self.ttl_policy`.
    ///
    /// This function ensures that the TTL policy is always set to a valid state, 
    /// either by updating an existing policy or creating a new one.
    pub fn update_ttl_policy(&mut self, ttl: Duration) {
        match &mut self.ttl_policy {
            Some(ttl_policy) => {
                ttl_policy.ttl = ttl
            },
            None => *&mut self.ttl_policy = Some(TTLPolicy::new(ttl)),
        }
    }

    pub fn remove_ttl_policy(&mut self) {
        self.ttl_policy = None;
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
