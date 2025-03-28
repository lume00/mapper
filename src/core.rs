use std::{
    error, io,
    net::{SocketAddr, TcpListener, TcpStream},
    time::Duration,
};

use ctrlc::Error;
use log::{error, info, Level};
use smol::{future::race, Async};

use crate::{
    backup_handler::BackupHandler, http_handler::hadle_client, logger::setup_logger, storage::Storage,
};


#[derive(Debug)]
pub struct MapperBuilder<'a> {
    pub(crate) password: Option<&'a str>,
    pub(crate) address: Option<&'a str>,
    pub(crate) async_loggin: Option<bool>,
    pub(crate) logging_level: Option<&'a str>,
    pub(crate) backup_interval: Option<Duration>,
    pub(crate) backup_path: Option<&'a str>,
    pub(crate) cluster_nodes: Option<Vec<(&'a str, Option<&'a str>)>>,
}

enum Signal {
    Quit,
    Listen(io::Result<(Async<TcpStream>, SocketAddr)>),
}

pub struct Mapper {
    ctrlc_channel: (smol::channel::Sender<()>, smol::channel::Receiver<()>),
    password: Option<String>,
    socket_address: SocketAddr,
    backup_interval: Duration,
    backup_path: String,
}

impl Mapper {
    pub fn new(mapper_params: MapperBuilder) -> Result<Self, Box<dyn error::Error>> {
        setup_logger(
            mapper_params.async_loggin.unwrap_or(false),
            grab_logger_level(&mapper_params),
        );

        let address = mapper_params.address.unwrap_or("127.0.0.1:6379");

        let socket_address = address
            .parse::<SocketAddr>()
            .expect("unable to parse socket address");

        let (ctrlc_tx, ctrlc_rx) = smol::channel::bounded::<()>(1);

        Ok(Mapper {
            password: mapper_params.password.map(|s| s.to_string()),
            ctrlc_channel: (ctrlc_tx, ctrlc_rx),
            socket_address,
            backup_interval: mapper_params.backup_interval.unwrap_or( Duration::from_secs(30)),
            backup_path: mapper_params.backup_path.unwrap_or( ".").to_string(),
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
            BackupHandler::new(self.backup_interval, self.backup_path.clone(), storage.clone())
                .recover_and_backup()
                .await;

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

fn grab_logger_level(mapper_params: &MapperBuilder<'_>) -> Level {
    let logging_level = match mapper_params.logging_level {
        Some(logging_level) => Level::iter()
            .find(|e| -> bool { e.as_str().to_lowercase() == logging_level.to_lowercase() })
            .unwrap_or(Level::Info),
        None => Level::Info,
    };
    logging_level
}