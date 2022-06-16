use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;

use crate::{command_parse, ParseCommandError};
use crate::{Command, DynKV};

use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::task::yield_now;
use tower::Service;
use tower::ServiceBuilder;

const STORED: &[u8] = b"STORED\r\n";
const UPDATED: &[u8] = b"UPDATED\r\n";
const NEWLINE: &[u8] = b"\r\n";
const END: &[u8] = b"END\r\n";

#[derive(Debug)]
enum GetResult {
    Found { key: String, value: String },
    NotFound(String),
}

#[derive(Debug)]
enum CommandResponse {
    GetResponse(Vec<GetResult>),
    SetStored,
    SetUpdated,
}

unsafe impl Send for CommandResponse {}

#[derive(Debug, Error)]
enum CommandError {
    #[error("bla {0}")]
    Something(#[from] ParseCommandError),
}

unsafe impl Send for CommandError {}

#[derive(Clone)]
pub struct Server {
    pub repo: DynKV,
}

impl Service<Request> for Server {
    type Response = CommandResponse;
    type Error = CommandError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let repo = self.repo.clone();
        let fut = async move {
            let cmd = command_parse(&req.body)?;
            match cmd {
                Command::Get(keys) => {
                    let repo = repo.read().await;
                    let mut results = vec![];
                    for key in keys {
                        results.push(match repo.get(&key).await {
                            Some(val) => GetResult::Found {
                                key: key.to_string(),
                                value: val.to_string(),
                            },
                            None => GetResult::NotFound(key.to_string()),
                        });
                    }
                    Ok(CommandResponse::GetResponse(results))
                }
                Command::Set(key, value) => match repo.write().await.store(&key, &value).await {
                    true => Ok(CommandResponse::SetStored),
                    _ => Ok(CommandResponse::SetUpdated),
                },
            }
        };

        Box::pin(fut)
    }
}

#[derive(Debug)]
struct Request {
    body: String,
}

const MAX_MESSAGE_SIZE: usize = 1024;
const TCP_TIMEOUT: u128 = 1000;

pub async fn tcp_server(addr: SocketAddr, repo: DynKV) -> io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let server = Server { repo };
    let service = ServiceBuilder::new().service(server);
    loop {
        let mut service = service.clone();
        match listener.accept().await {
            Ok((socket, _addr)) => {
                tokio::spawn(async move {
                    let now = std::time::Instant::now();
                    let mut socket = socket;
                    let mut buf = vec![0u8; MAX_MESSAGE_SIZE];
                    while std::time::Instant::now().duration_since(now).as_millis() < TCP_TIMEOUT {
                        let read = socket.try_read(&mut buf);
                        let n = match read {
                            Ok(n) => n,
                            Err(e) => {
                                yield_now().await;
                                if e.kind() == io::ErrorKind::WouldBlock {
                                    continue;
                                }
                                return Ok::<_, io::Error>(());
                            }
                        };
                        let data = String::from_utf8_lossy(&buf[..n]);
                        match service
                            .call(Request {
                                body: data.to_string(),
                            })
                            .await
                        {
                            Ok(CommandResponse::GetResponse(v)) => {
                                for value in v {
                                    if let GetResult::Found { key: _key, value } = value {
                                        let _ = socket.write(value.as_bytes()).await?;
                                        let _ = socket.write(NEWLINE).await?;
                                    }
                                }
                                let _ = socket.write(END).await?;
                            }
                            Ok(CommandResponse::SetStored) => {
                                let _ = socket.write(STORED).await?;
                            }
                            Ok(CommandResponse::SetUpdated) => {
                                let _ = socket.write(UPDATED).await?;
                            }
                            Err(e) => eprintln!("{e}"),
                        };
                        yield_now().await;
                    }
                    Ok(())
                });
            }
            Err(e) => println!("couldn't get client: {:?}", e),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Command, DynKV, MemKV};
    use std::sync::Arc;

    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_get_call() {
        let kv = MemKV::default();
        let mut server = Server {
            repo: Arc::new(RwLock::new(kv)) as DynKV,
        };
        assert!(server
            .call(Request {
                body: Command::Get(vec!["abcd".to_string()]).to_string_command(),
            })
            .await
            .is_ok());
    }
}
