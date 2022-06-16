use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;

use crate::DynKV;

use axum::{
    extract::{Extension, Query},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use thiserror::Error;

pub async fn http_server(addr: SocketAddr, kv_repo: DynKV) {
    // Build our application with some routes
    let app = Router::new()
        .route("/get", get(kv_get))
        .route("/set", get(kv_set))
        // Add our `user_repo` to all request's extensions so handlers can access
        // it.
        .layer(Extension(kv_repo));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn kv_get(
    Query(params): Query<BTreeMap<String, String>>,
    Extension(user_repo): Extension<DynKV>,
) -> Result<String, KVGetError> {
    let key = if let Some(key) = params.keys().next() {
        key
    } else {
        return Ok("ada".to_string());
    };

    match user_repo.read().await.get(key).await {
        Some(val) => Ok(val.to_string()),
        None => Ok("NOT FOUND".to_string()),
    }
}

async fn kv_set(
    Query(params): Query<HashMap<String, String>>,
    Extension(user_repo): Extension<DynKV>,
) -> Result<String, KVGetError> {
    let (key, value) = if let Some(key) = params.iter().next() {
        key
    } else {
        return Ok("ada".to_string());
    };
    match user_repo.write().await.store(key, value).await {
        true => Ok("UPDATED".to_string()),
        false => Ok("STORED".to_string()),
    }
}

#[derive(Error, Debug)]
enum KVGetError {}

impl IntoResponse for KVGetError {
    fn into_response(self) -> Response {
        todo!()
    }
}
