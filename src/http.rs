use crate::DynKV;
use std::net::SocketAddr;

use axum::{
    extract::{Extension, Query},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
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
    Query(_params): Query<Option<GetParams>>,
    Extension(_user_repo): Extension<DynKV>,
) -> Result<String, KVGetError> {
    todo!()
}

async fn kv_set(Extension(_user_repo): Extension<DynKV>) -> Result<String, KVGetError> {
    todo!()
}

#[derive(Error, Debug)]
enum KVGetError {}

impl IntoResponse for KVGetError {
    fn into_response(self) -> Response {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct GetParams(String);

#[must_use]
fn _parse_get_query_params(params: &str) -> Option<GetParams> {
    let split: Vec<&str> = params.split('=').take(2).collect();
    if split.len() != 2 {
        return None;
    }
    if split[0] != "key" {
        return None;
    }
    Some(GetParams(split[1].to_string()))
}

#[derive(Debug)]
struct SetParams {
    key: String,
    value: String,
}

fn parse_set_query_params(params: &str) -> Option<SetParams> {
    let split: Vec<&str> = params.split('=').take(2).collect();
    if split.len() != 2 {
        return None;
    }
    Some(SetParams {
        key: split[0].to_string(),
        value: split[1].to_string(),
    })
}
