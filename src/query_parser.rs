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
        let path = req.url().path();
        let method = req.method();

        match method {
            http_types::Method::Get => get_api(path),
            http_types::Method::Put => match req.body_bytes().await {
                Ok(body) => put_api(req.clone().url().path(), body),
                Err(e) => {
                    error!("put without body: {}", e);
                    Err(DeserializationError::UnparsableQuery)
                },
            },
            _ => Err(DeserializationError::QueryNotFound),
        }
    }
}

fn put_api(path: &str, body: Vec<u8>) -> Result<Query, DeserializationError> {

    if let Some(set) = extract_wildcards(path, "/SET/*") {
        if let Some(key) = set.get(0) {
            return Ok(Query::Set {
                key: key.clone(),
                data: body,
            });
        } else {
            return Err(DeserializationError::UnparsableQuery);
        }
    }

    if let Some(setex) = extract_wildcards(path, "/SETEX/*/*") {
        if let (Some(key), Some(dur)) = (setex.get(0), setex.get(1)) {
            return match parse_duration(dur.as_str()) {
                Ok(dur) => Ok(Query::SetEx {
                    key: key.clone(),
                    data: body,
                    ttl: Duration::from(dur),
                }),
                Err(_) => Err(DeserializationError::UnparsableDuration),
            };
        } else {
            return Err(DeserializationError::UnparsableQuery);
        }
    }

    return Err(DeserializationError::QueryNotFound);
}

fn get_api(path: &str) -> Result<Query, DeserializationError> {

    if let Some(get) = extract_wildcards(path, "/GET/*") {
        return match get.get(0) {
            Some(el) => Ok(Query::Get { key: el.clone() }),
            None => Err(DeserializationError::UnparsableQuery),
        };
    }

    if let Some(set) = extract_wildcards(path, "/SET/*/*") {
        if let (Some(key), Some(val)) = (set.get(0), set.get(1)) {
            return Ok(Query::Set {
                key: key.clone(),
                data: val.as_bytes().to_vec(),
            });
        } else {
            return Err(DeserializationError::UnparsableQuery);
        }
    }

    if let Some(setex) = extract_wildcards(path, "/SETEX/*/*/*") {
        if let (Some(key), Some(val), Some(dur)) = (setex.get(0), setex.get(1), setex.get(2)) {
            return match parse_duration(dur.as_str()) {
                Ok(dur) => Ok(Query::SetEx {
                    key: key.clone(),
                    data: val.as_bytes().to_vec(),
                    ttl: Duration::from(dur),
                }),
                Err(_) => Err(DeserializationError::UnparsableDuration),
            };
        } else {
            return Err(DeserializationError::UnparsableQuery);
        }
    }

    if let Some(del) = extract_wildcards(path, "/DEL/*") {
        return match del.get(0) {
            Some(el) => Ok(Query::Del { key: el.clone() }),
            None => Err(DeserializationError::UnparsableQuery),
        };
    }

    if let Some(exists) = extract_wildcards(path, "/EXISTS/*") {
        return match exists.get(0) {
            Some(el) => Ok(Query::Exists { key: el.clone() }),
            None => Err(DeserializationError::UnparsableQuery),
        };
    }

    if let Some(expire) = extract_wildcards(path, "/EXPIRE/*/*") {
        if let (Some(key), Some(dur)) = (expire.get(0), expire.get(1)) {
            return match parse_duration(dur.as_str()) {
                Ok(dur) => Ok(Query::Expire {
                    key: key.clone(),
                    ttl: Duration::from(dur),
                }),
                Err(_) => Err(DeserializationError::UnparsableDuration),
            };
        } else {
            return Err(DeserializationError::UnparsableQuery);
        }
    }

    if let Some(ttl) = extract_wildcards(path, "/TTL/*") {
        return match ttl.get(0) {
            Some(el) => Ok(Query::Ttl { key: el.clone() }),
            None => Err(DeserializationError::UnparsableQuery),
        };
    }

    if let Some(ttl) = extract_wildcards(path, "/PERSIST/*") {
        return match ttl.get(0) {
            Some(el) => Ok(Query::Persist { key: el.clone() }),
            None => Err(DeserializationError::UnparsableQuery),
        };
    }

    if let Some(_) = extract_wildcards(path, "/INFO") {
        return Ok(Query::Info);
    }

    if let Some(_) = extract_wildcards(path, "/FLUSHALL") {
        return Ok(Query::FlushAll);
    }

    if let Some(_) = extract_wildcards(path, "/DBSIZE") {
        return Ok(Query::DbSize);
    }

    if let Some(_) = extract_wildcards(path, "/PING") {
        return Ok(Query::Ping);
    }

    return Err(DeserializationError::QueryNotFound);
}

fn extract_wildcards(url: &str, pattern: &str) -> Option<Vec<String>> {
    // Create a regex pattern, replacing `*` with a capture group for wildcards
    let mut regex_pattern = pattern.replace("*", r"([^/]+)");

    // Add start (^) and end ($) anchors to match the whole URL
    regex_pattern = format!("^{}$", regex_pattern);

    // Compile the regex
    let maybe_re = Regex::new(&regex_pattern);

    if let Ok(re) = maybe_re {
        // Attempt to match the URL
        if let Some(captures) = re.captures(url) {
            // Collect all captured groups (wildcards) into a vector
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
