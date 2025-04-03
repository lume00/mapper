use std::time::Duration;

use http_types::Request;
use humantime::parse_duration;
use log::error;
use regex::Regex;

use crate::errors::DeserializationError;

#[derive(Debug)]
pub enum Query {
    Get {
        key: String,
    },
    Set {
        key: String,
        data: Vec<u8>,
    },
    SetEx {
        key: String,
        data: Vec<u8>,
        ttl: Duration,
    },
    Del {
        key: String,
    },
    Exists {
        key: String,
    },
    Expire {
        key: String,
        ttl: Duration,
    },
    Ttl {
        key: String,
    },
    Persist {
        key: String,
    },
    Info,
    FlushAll,
    DbSize,
    Ping,
}

impl Query {
    pub async fn try_from(mut req: Request) -> Result<Self, DeserializationError> {
        let path = req.url().path().to_string();
        let method = req.method();
        match method {
            http_types::Method::Get => get_api(&path),
            http_types::Method::Put => match req.body_bytes().await {
                Ok(body) => put_api(&*path, body),
                Err(e) => {
                    error!("put without body: {}", e);
                    Err(DeserializationError::UnparsableQuery)
                }
            },
            _ => Err(DeserializationError::QueryNotFound),
        }
    }
}

macro_rules! match_api {
    ($path:expr, $pattern:expr, $query:expr) => {
        if let Some(captures) = extract_wildcards($path, $pattern) {
            return $query(captures);
        }
    };
}

fn put_api(path: &str, body: Vec<u8>) -> Result<Query, DeserializationError> {
    match_api!(path, "/SET/*", |captures: Vec<String>| {
        println!("body: {:?}", String::from_utf8(body.clone()));
        captures
            .get(0)
            .map_or(Err(DeserializationError::UnparsableQuery), |key| {
                Ok(Query::Set {
                    key: key.clone(),
                    data: body,
                })
            })
    });

    match_api!(path, "/SETEX/*/*", |captures: Vec<String>| {
        if let (Some(key), Some(dur)) = (captures.get(0), captures.get(1)) {
            match parse_duration(dur.as_str()) {
                Ok(dur) => Ok(Query::SetEx {
                    key: key.clone(),
                    data: body,
                    ttl: Duration::from(dur),
                }),
                Err(_) => Err(DeserializationError::UnparsableDuration),
            }
        } else {
            Err(DeserializationError::UnparsableQuery)
        }
    });

    Err(DeserializationError::QueryNotFound)
}

fn get_api(path: &str) -> Result<Query, DeserializationError> {
    match_api!(path, "/GET/*", |captures: Vec<String>| {
        captures
            .get(0)
            .map_or(Err(DeserializationError::UnparsableQuery), |el| {
                Ok(Query::Get { key: el.clone() })
            })
    });

    match_api!(path, "/SET/*/*", |captures: Vec<String>| {
        if let (Some(key), Some(val)) = (captures.get(0), captures.get(1)) {
            Ok(Query::Set {
                key: key.clone(),
                data: val.as_bytes().to_vec(),
            })
        } else {
            Err(DeserializationError::UnparsableQuery)
        }
    });

    match_api!(path, "/SETEX/*/*/*", |captures: Vec<String>| {
        if let (Some(key), Some(val), Some(dur)) =
            (captures.get(0), captures.get(1), captures.get(2))
        {
            match parse_duration(dur.as_str()) {
                Ok(dur) => Ok(Query::SetEx {
                    key: key.clone(),
                    data: val.as_bytes().to_vec(),
                    ttl: Duration::from(dur),
                }),
                Err(_) => Err(DeserializationError::UnparsableDuration),
            }
        } else {
            Err(DeserializationError::UnparsableQuery)
        }
    });

    match_api!(path, "/DEL/*", |captures: Vec<String>| {
        captures
            .get(0)
            .map_or(Err(DeserializationError::UnparsableQuery), |el| {
                Ok(Query::Del { key: el.clone() })
            })
    });

    match_api!(path, "/EXISTS/*", |captures: Vec<String>| {
        captures
            .get(0)
            .map_or(Err(DeserializationError::UnparsableQuery), |el| {
                Ok(Query::Exists { key: el.clone() })
            })
    });

    match_api!(path, "/EXPIRE/*/*", |captures: Vec<String>| {
        if let (Some(key), Some(dur)) = (captures.get(0), captures.get(1)) {
            match parse_duration(dur.as_str()) {
                Ok(dur) => Ok(Query::Expire {
                    key: key.clone(),
                    ttl: Duration::from(dur),
                }),
                Err(_) => Err(DeserializationError::UnparsableDuration),
            }
        } else {
            Err(DeserializationError::UnparsableQuery)
        }
    });

    match_api!(path, "/TTL/*", |captures: Vec<String>| {
        captures
            .get(0)
            .map_or(Err(DeserializationError::UnparsableQuery), |el| {
                Ok(Query::Ttl { key: el.clone() })
            })
    });

    match_api!(path, "/PERSIST/*", |captures: Vec<String>| {
        captures
            .get(0)
            .map_or(Err(DeserializationError::UnparsableQuery), |el| {
                Ok(Query::Persist { key: el.clone() })
            })
    });

    match_api!(path, "/INFO", |_| Ok(Query::Info));

    match_api!(path, "/FLUSHALL", |_| Ok(Query::FlushAll));

    match_api!(path, "/DBSIZE", |_| Ok(Query::DbSize));

    match_api!(path, "/PING", |_| Ok(Query::Ping));

    Err(DeserializationError::QueryNotFound)
}

fn extract_wildcards(url: &str, pattern: &str) -> Option<Vec<String>> {
    // Create a regex pattern, replacing `*` with a capture group for wildcards
    let mut regex_pattern = pattern.replace("*", r"([^/]+)");

    // Add start (^) and end ($) anchors to match the whole URL
    regex_pattern = format!("^{}$", regex_pattern);
    let maybe_re = Regex::new(&regex_pattern);

    if let Ok(re) = maybe_re {
        if let Some(captures) = re.captures(url) {
            let wildcards = captures
                .iter()
                .skip(1) // Skip the full match
                .filter_map(|cap| cap.map(|m| m.as_str().to_string()))
                .collect();

            return Some(wildcards);
        } else {
            return None;
        }
    }
    None
}
