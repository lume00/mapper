use std::{
    error, io,
    net::{SocketAddr, TcpListener, TcpStream},
    time::Duration,
};

use ctrlc::Error;
use log::{error, info, Level};
use smol::{future::race, Async};
use clap::Parser;

use crate::{
    backup_handler::BackupHandler, http_handler::hadle_client, logger::setup_logger, storage::Storage,
};


#[derive(Parser, Debug)]
#[command(name = "Mapper")]
#[command(about = "A simple and concurrent in memory database", long_about = None)]
pub struct MapperBuilder {
    #[arg(long, help = "Api key for authentication")]
    pub(crate) api_key: Option<String>,

    #[arg(long, help = "Socket address to bind", default_value = "127.0.0.1:6379")]
    pub(crate) address: String,

    #[arg(long, help = "Enable asynchronous logging", default_value_t = false, hide = true)]
    pub(crate) async_logging: bool,

    #[arg(long, help = "Logging level (e.g., info, debug, error)", default_value = "info")]
    pub(crate) logging_level: String,

    #[arg(long, help = "Enable backup functionality", default_value_t = true)]
    pub(crate) backup: bool,

    #[arg(long, help = "Backup interval in seconds", default_value_t = 240u64)]
    pub(crate) backup_interval: u64,

    #[arg(long, help = "Path for backup files", default_value = ".")]
    pub(crate) backup_path: String,
}

enum Signal {
    Quit,
    Listen(io::Result<(Async<TcpStream>, SocketAddr)>),
}

pub struct Backup {
    backup_interval: Duration,
    backup_path: String,
}

pub struct Mapper {
    ctrlc_channel: (smol::channel::Sender<()>, smol::channel::Receiver<()>),
    password: Option<String>,
    socket_address: SocketAddr,
    backup: Option<Backup>,
}

impl Mapper {
    pub fn new(mapper_params: MapperBuilder) -> Result<Self, Box<dyn error::Error>> {
        setup_logger(
            mapper_params.async_logging,
            grab_logger_level(&mapper_params),
        );

        let socket_address = mapper_params.address
            .parse::<SocketAddr>()
            .expect("unable to parse socket address");

        let (ctrlc_tx, ctrlc_rx) = smol::channel::bounded::<()>(1);

        Ok(Mapper {
            password: mapper_params.api_key.map(|s| s.to_string()),
            ctrlc_channel: (ctrlc_tx, ctrlc_rx),
            socket_address,
            backup: mapper_params
                .backup
                .then(|| Backup {
                    backup_interval: Duration::from_secs(mapper_params.backup_interval),
                    backup_path: mapper_params.backup_path
                }),
        })
    }

    pub fn start(&self) -> Result<(), Error> {
        ctrlc::set_handler({
            let s = self.ctrlc_channel.0.clone();
            move || {
                info!("received termination signal, sending termination signal to event loop");
                let _ = s.send_blocking(());
            }
        })?;

        let storage = Storage::default();

        smol::block_on(async {
            if let Some(backup_params) = &self.backup {
                BackupHandler::new(backup_params.backup_interval, backup_params.backup_path.clone(), storage.clone())
                .recover_and_backup()
                .await;
            }

            let listener = Async::<TcpListener>::bind(self.socket_address)
                .expect("unable to start tcplistener");

            info!("listening on {}", self.socket_address);

            loop {
                let signal = race(async { Signal::Listen(listener.accept().await) }, async {
                    match self.ctrlc_channel.1.recv().await {
                        Ok(_) | Err(_) => Signal::Quit
                    }
                })
                .await;

                match signal {
                    Signal::Quit => break,
                    Signal::Listen(maybe_stream) => match maybe_stream {
                        Ok(stream) => smol::spawn(hadle_client(
                            stream.0,
                            stream.1,
                            storage.clone(),
                            self.password.clone()
                        ))
                        .detach(),
                        Err(e) => error!("async tcpstream error: {}", e),
                    },
                }
            }
        });

        Ok(())
    }
}

fn grab_logger_level(mapper_params: &MapperBuilder) -> Level {
    let logging_level = mapper_params.logging_level.clone();
    Level::iter()
            .find(|e| e.as_str().to_lowercase() == logging_level.to_lowercase())
            .unwrap_or(Level::Info)
}