use log::error;

use crate::{errors::{self}, query_parser::Query, record::Record, storage::Storage};

pub(crate) async fn handle_query(query: Query, storage: Storage) -> Result<String, errors::Errors> {
    match query {
        Query::Get { key } => match storage.get_record(&key).await {
            Ok(rec) => match String::from_utf8(rec.data) {
                Ok(rec) => Ok(rec),
                Err(err) => {
                    error!("{}", err);
                    Err(errors::Errors::DeserializationError(errors::DeserializationError::UnparsableBytes))
                },
            },
            Err(err) => Err(errors::Errors::TransactionError(err)),
        },
        Query::Set { key, data } => match storage.set_record(&key, Record::new(data, None)).await {
            Ok(_) => Ok("".to_string()),
            Err(err) => Err(errors::Errors::TransactionError(err)),
        },
        Query::SetEx { key, data, ttl } => {
            match storage.set_record(&key, Record::new(data, Some(ttl))).await {
                Ok(_) => Ok("".to_string()),
                Err(err) => Err(errors::Errors::TransactionError(err)),
            }
        }
        Query::Del { key } => match storage.remove_record(&key).await {
            Ok(_) => Ok("".to_string()),
            Err(err) => Err(errors::Errors::TransactionError(err)),
        },
        Query::Exists { key } => match storage.get_record(&key).await {
            Ok(_) => Ok("".to_string()),
            Err(err) => Err(errors::Errors::TransactionError(err)),
        },
        Query::Expire { key, ttl } => {
            match storage
                .update_ttl(
                    &key,
                    Some(ttl),
                )
                .await
            {
                Ok(_) => Ok("".to_string()),
                Err(err) => Err(errors::Errors::TransactionError(err)),
            }
        }
        Query::Ttl { key } => match storage.get_record(&key).await {
            Ok(rec) => match rec.ttl_policy {
                Some(ttl_policy) => Ok(format!("{}s", ttl_policy.expire_in().as_secs())),
                None => Err(errors::Errors::TransactionError(errors::TransactionError::TTLNotFound)),
            },
            Err(err) => Err(errors::Errors::TransactionError(err)),
        },
        Query::Info => Ok("mapper".to_string()),
        Query::FlushAll => {
            storage.flush_all().await;
            Ok("".to_string())
        }
        Query::DbSize => Ok(storage.db_size().await.to_string()),
        Query::Ping => Ok("pong".to_string()),
        Query::Persist { key } => {
            match storage.update_ttl(&key, None).await {
                Ok(_) => Ok("".to_string()),
                Err(err) => Err(errors::Errors::TransactionError(err)),
            }
        },
    }
}
