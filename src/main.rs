mod core;
mod logger;
mod record;
mod storage;
mod wrapped_record;
mod http_handler;
mod query_parser;
mod errors;
mod query_handler;
mod backup_handler;

use core::{Mapper, MapperBuilder};

fn main() {
    Mapper::new(MapperBuilder {
        password: None,
        address: Some("127.0.0.1:6379"),
        async_loggin: Some(false),
        logging_level: Some("debug"),
        backup_interval: None,
        backup_path: None,
        cluster_nodes: None,
    })
    .unwrap().start().unwrap();
}