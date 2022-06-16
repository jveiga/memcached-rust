use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

pub type DynKV = Arc<RwLock<dyn KV + Send + Sync>>;

#[async_trait]
pub trait KV {
    async fn store(&mut self, key: &str, val: &str) -> bool;

    async fn get(&self, key: &str) -> Option<&String>;
}

#[derive(Debug, Default)]
pub struct MemKV {
    m: HashMap<String, String>,
}

#[async_trait]
impl KV for MemKV {
    async fn store(&mut self, key: &str, val: &str) -> bool {
        self.m.insert(key.to_string(), val.to_string()).is_some()
    }
    async fn get(&self, key: &str) -> Option<&String> {
        self.m.get(&key.to_string())
    }
}
