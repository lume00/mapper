use std::net::{SocketAddr, TcpStream};

use http_types::{Method, Request, Response, StatusCode};
use log::error;
use smol::Async;

use crate::{query_handler, query_parser::Query, storage::Storage};

const BEARIER_TOKEN: &'static str = "Authorization";

pub(crate) async fn hadle_client(
    stream: Async<TcpStream>,
    address: SocketAddr,
    storage: Storage,
    maybe_password: Option<String>,
) {
    let stream = async_dup::Arc::new(stream);

    if let Err(e) = async_h1::accept(stream, move |req| {
        handle_http_request(req, storage.clone(), maybe_password.clone())
    })
    .await
    {
        error!("{} from {}", e, address);
    }
}

async fn handle_http_request(
    req: Request,
    storage: Storage,
    maybe_password: Option<String>,
) -> http_types::Result<Response> {
    if let Some(password) = maybe_password {
        match req.header(BEARIER_TOKEN) {
            Some(password_from_header) => {
                if password_from_header != password.as_str() {
                    return Ok(Response::new(StatusCode::Forbidden));
                }
            }
            None => return Ok(Response::new(StatusCode::Forbidden)),
        }
    }

    match req.method() {
        Method::Get | Method::Put => match Query::try_from(req).await {
            Ok(query) => Ok(match query_handler::handle_query(query, storage).await {
                Ok(query_data) => {
                    let mut http_res = Response::new(StatusCode::Ok);
                    http_res.set_body(query_data);
                    http_res
                }
                Err(error) => {
                    let status = match &error {
                        crate::errors::Errors::TransactionError(transaction_error) => {
                            match transaction_error {
                                crate::errors::TransactionError::ShardNotFound
                                | crate::errors::TransactionError::RecordNotFound
                                | crate::errors::TransactionError::TTLNotFound => {
                                    StatusCode::NotFound
                                }
                            }
                        }
                        crate::errors::Errors::DeserializationError(deserialization_error) => {
                            match deserialization_error {
                                crate::errors::DeserializationError::QueryNotFound => {
                                    StatusCode::NotFound
                                }
                                crate::errors::DeserializationError::UnparsableQuery
                                | crate::errors::DeserializationError::UnparsableDuration
                                | crate::errors::DeserializationError::UnparsableBytes => {
                                    StatusCode::InternalServerError
                                }
                            }
                        }
                    };
                    let mut http_res = Response::new(status);
                    http_res.set_body(error.to_string());
                    http_res
                }
            }),
            Err(e) => {
                let mut http_res = Response::new(StatusCode::InternalServerError);
                http_res.set_body(e.to_string());
                Ok(http_res)
            }
        },
        _ => Ok(Response::new(StatusCode::NotFound)),
    }
}
