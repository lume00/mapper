extern crate log;

use std::{
    sync::{
        mpsc::{self, channel, Sender},
        OnceLock,
    },
    thread,
};

use log::{Level, Metadata};

static LAZY_ASYNC_LOGGER: OnceLock<Sender<String>> = OnceLock::new();

struct StdoutLogger;
impl log::Log for StdoutLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            let message = format!(
                "[{}] [{}] {}",
                record.level(),
                record.module_path().unwrap_or("mapper"),
                record.args()
            );

            match LAZY_ASYNC_LOGGER.get() {
                Some(msg_channel) => {
                    let _ = msg_channel.send(message);
                }
                None => println!("{}", message)
            }
        }
    }

    fn flush(&self) {}
}

// Setup function to initialize the logger
pub fn setup_logger(enable_async_logging: bool, max_level: Level) {
    if enable_async_logging {
        LAZY_ASYNC_LOGGER.get_or_init(|| -> mpsc::Sender<String> {
            let (s, r) = channel::<String>();
            thread::spawn(move || loop {
                match r.recv() {
                    Ok(msg) => println!("{}", msg),
                    Err(_) => break,
                }
            });

            s
        });
    }
    
    log::set_max_level(max_level.to_level_filter());
    let already_set_logger = log::set_logger(&StdoutLogger);
    if let Err(already_set) = already_set_logger {
        eprintln!("error during logger initialization: {}", already_set);
    }
}
