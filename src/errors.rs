use std::{error, fmt};

#[derive(Debug)]
pub enum Errors {
    // HttpError,
    TransactionError(TransactionError),
    DeserializationError(DeserializationError)
}

impl error::Error for Errors {}
impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Errors::TransactionError(te) => te.fmt(f),
            Errors::DeserializationError(de) => de.fmt(f),
            // Errors::HttpError => write!(f, "http error"),
        }
    }
}

#[derive(Debug)]
pub enum TransactionError {
    ShardNotFound,
    RecordNotFound,
    TTLNotFound,
}

impl error::Error for TransactionError {}
impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
                TransactionError::ShardNotFound => write!(f, "shard_not_found"),
                TransactionError::RecordNotFound => write!(f, "record_not_found"),
                TransactionError::TTLNotFound => write!(f, "ttl_not_found"),
        }
    }
}

#[derive(Debug)]
pub enum DeserializationError {
    QueryNotFound,
    UnparsableQuery,
    UnparsableDuration,
    UnparsableBytes,
}

impl error::Error for DeserializationError {}
impl fmt::Display for DeserializationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DeserializationError::QueryNotFound => write!(f, "query_not_found"),
            DeserializationError::UnparsableQuery => write!(f, "unparsable_query"),
            DeserializationError::UnparsableDuration => write!(f, "unparsable_duration"),
            DeserializationError::UnparsableBytes => write!(f, "unparsable_bytes"),
        }
    }
}