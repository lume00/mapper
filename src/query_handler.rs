use log::error;

use crate::{errors::{self}, query_parser::Query, record::Record, storage::Storage};

fn handle_ok_result<T, F>(result: Result<T, errors::TransactionError>, handler: F) -> Result<String, errors::Errors>
where
    F: FnOnce(T) -> Result<String, errors::Errors>,
{
    match result {
        Ok(value) => handler(value),
        Err(err) => {
            error!("{}", err);
            Err(errors::Errors::TransactionError(err))
        }
    }
}

fn record_to_string(record: Record) -> Result<String, errors::Errors> {
    match String::from_utf8(record.data) {
        Ok(rec_string) => Ok(rec_string),
        Err(err) => {
            error!("{}", err);
            Err(errors::Errors::DeserializationError(errors::DeserializationError::UnparsableBytes))
        }
    }
}

pub(crate) async fn handle_query(query: Query, storage: Storage) -> Result<String, errors::Errors> {
    match query {
        Query::Get { key } => handle_ok_result(storage.get_record(&key).await, record_to_string),
        Query::Set { key, data } => handle_ok_result(
            storage.set_record(&key, Record::new(data, None)).await,
            |_| Ok(String::new()),
        ),
        Query::SetEx { key, data, ttl } => handle_ok_result(
            storage.set_record(&key, Record::new(data, Some(ttl))).await,
            |_| Ok(String::new()),
        ),
        Query::Del { key } => handle_ok_result(
            storage.remove_record(&key).await,
            |_| Ok(String::new()),
        ),
        Query::Exists { key } => handle_ok_result(
            storage.get_record(&key).await,
            |_| Ok(String::new()),
        ),
        Query::Expire { key, ttl } => handle_ok_result(
            storage.update_ttl(&key, Some(ttl)).await,
            |_| Ok(String::new()),
        ),
        Query::Ttl { key } => handle_ok_result(storage.get_record(&key).await, |rec: Record| {
            match rec.ttl_policy {
                Some(ttl_policy) => Ok(format!("{}s", ttl_policy.expire_in().as_secs())),
                None => Err(errors::Errors::TransactionError(errors::TransactionError::TTLNotFound)),
            }
        }),
        Query::Info => Ok("mapper".to_string()),
        Query::FlushAll => {
            storage.flush_all().await;
            Ok(String::new())
        }
        Query::DbSize => Ok(storage.db_size().await.to_string()),
        Query::Ping => Ok("pong".to_string()),
        Query::Persist { key } => handle_ok_result(
            storage.update_ttl(&key, None).await,
            |_| Ok(String::new()),
        ),
    }
}
