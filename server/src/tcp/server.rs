use std::future::Future;
use std::io;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use super::command_parse;
use super::parser::ParseCommandError;
use crate::{Command, DynKV};

use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::task::yield_now;
use tower::filter::Predicate;
use tower::Service;
use tower::ServiceBuilder;

const PARSE_ERROR: &[u8] = b"PARSE ERROR\r\n";
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
            let cmd = command_parse(&req.body).map_err(|err| CommandError::Something(err))?;
            match dbg!(cmd) {
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

struct RateLimitEntry {}

#[derive(Clone, Debug, Default)]
struct RateLimi {
    store: Arc<std::collections::BTreeMap<IpAddr, std::time::SystemTime>>,
}

impl Predicate<Request> for RateLimi {
    type Request = Request;
    fn check(&mut self, req: Self::Request) -> Result<Request, tower::BoxError> {
        // self.store.entry(req.ip).o
        Ok(req)
    }
}

#[derive(Debug)]
struct Request {
    body: String,
    ip: IpAddr,
}

unsafe impl Send for Request {}

const MAX_MESSAGE_SIZE: usize = 1024;

pub async fn tcp_server(addr: SocketAddr, repo: DynKV) -> io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let server = Server { repo };
    let service = ServiceBuilder::new()
        .filter(RateLimi::default())
        .service(server);
    loop {
        let mut service = service.clone();
        match listener.accept().await {
            Ok((socket, addr)) => {
                tokio::spawn(async move {
                    let now = std::time::Instant::now();
                    let mut socket = socket;
                    let mut buf = vec![0u8; MAX_MESSAGE_SIZE];
                    while std::time::Instant::now().duration_since(now).as_millis() < 1000 {
                        let n = socket.try_read(&mut buf);
                        let n = match n {
                            Ok(n) => n,
                            Err(e) => {
                                yield_now().await;
                                if e.kind() == io::ErrorKind::WouldBlock {
                                    continue;
                                }
                                return Ok(());
                            }
                        };
                        let data = if let Ok(data) = std::str::from_utf8(&buf[..n]) {
                            data
                        } else {
                            let _ = socket.write(PARSE_ERROR).await?;
                            return Ok::<_, io::Error>(());
                        };
                        match service
                            .call(Request {
                                body: data.to_string(),
                                ip: addr.ip(),
                            })
                            .await
                        {
                            Ok(CommandResponse::GetResponse(v)) => {
                                for value in v {
                                    match value {
                                        GetResult::Found { key: _key, value } => {
                                            socket.write(value.as_bytes()).await?;
                                            socket.write(NEWLINE).await?;
                                        }
                                        _ => {}
                                    }
                                }
                                socket.write(END).await?;
                            }
                            Ok(CommandResponse::SetStored) => {
                                socket.write(STORED).await?;
                            }
                            Ok(CommandResponse::SetUpdated) => {
                                socket.write(UPDATED).await?;
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
                ip: "127.0.0.1".parse().unwrap(),
            })
            .await
            .is_ok());
    }
}
