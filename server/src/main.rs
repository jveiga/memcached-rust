use std::net::SocketAddr;
use std::sync::Arc;

use memcached::{http_server, tcp_server, DynKV, MemKV};

use tokio::sync::RwLock;

// #[tokio::main]
#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let kv = MemKV::default();
    let repo = Arc::new(RwLock::new(kv)) as DynKV;
    let http_addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let http_repo = repo.clone();
    tokio::spawn(async move { http_server(http_addr, http_repo.clone()).await });
    let tcp_addr = SocketAddr::from(([127, 0, 0, 1], 4000));
    let _ = tcp_server(tcp_addr, repo.clone()).await;
    // let tcp_kv = Server{kv: repo.clone()};
}
