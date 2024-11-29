mod engine;
mod logger;
mod record;
mod storage;
mod wrapped_record;
mod http_handler;
mod query_parser;
mod errors;
mod query_handler;

use engine::{Mapper, MapperBuilder};

fn main() {
    Mapper::new(MapperBuilder {
        password: None,
        address: None,
        async_loggin: Some(false),
        logging_level: Some("debug")
    })
    .unwrap()
    .start()
    .unwrap();
}