use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;

use crate::DynKV;

use axum::{
    body::Body,
    extract::{Extension, Query},
    http::Request,
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
    Query(params): Query<BTreeMap<String, String>>,
    Extension(user_repo): Extension<DynKV>,
) -> Result<String, KVGetError> {
    let key = if let Some(key) = params.keys().next() {
        key
    } else {
        return Ok("ada".to_string());
    };

    match dbg!(user_repo.read().await.get(key).await){
        Some(val) => Ok(val.to_string()),
        None => {
            Ok("NOT FOUND".to_string())
        },
    }
}

async fn kv_set(
    Query(params): Query<HashMap<String, String>>,
    Extension(_user_repo): Extension<DynKV>,
) -> Result<String, KVGetError> {
    println!("{:?}", params);
    Ok("hi".to_string())
}

#[derive(Error, Debug)]
enum KVGetError {}

impl IntoResponse for KVGetError {
    fn into_response(self) -> Response {
        todo!()
    }
}

#[derive(Debug)]
struct GetParams(String);

// #[must_use]
// fn _parse_get_query_params(params: &str) -> Option<GetParams> {
//     let split: Vec<&str> = params.split('=').take(2).collect();
//     if split.len() != 2 {
//         return None;
//     }
//     if split[0] != "key" {
//         return None;
//     }
//     todo!()
// }

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
