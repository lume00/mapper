use std::time::Duration;

#[derive(Debug)]
pub struct RecordHandle {
    pub(crate) new_ttl: Option<Duration>
}