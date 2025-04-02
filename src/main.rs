mod core;
mod logger;
mod record;
mod storage;
mod wrapped_record;
mod http_handler;
mod http_query_parser;
mod errors;
mod query_handler;
mod backup_handler;

use core::{Mapper, MapperBuilder};

use clap::Parser;

fn main() {
    Mapper::new(MapperBuilder::parse())
        .unwrap()
        .start()
        .unwrap();
}