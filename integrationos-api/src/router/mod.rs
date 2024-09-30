pub mod public;
pub mod secured_jwt;
pub mod secured_key;

use crate::server::AppState;
use axum::{response::IntoResponse, routing::get, Json, Router};
use http::StatusCode;
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Deserialize, Debug)]
pub struct ServerResponse<T>
where
    T: Serialize,
{
    pub response_type: String,
    pub args: T,
}

impl<T> Serialize for ServerResponse<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let args_value = match serde_json::to_value(&self.args) {
            Ok(value) => value,
            Err(_) => return Err(serde::ser::Error::custom("Serialization of args failed")),
        };

        if let Value::Object(ref args_map) = args_value {
            let mut state = serializer.serialize_map(None)?;

            state.serialize_entry("type", &self.response_type)?;

            for (key, value) in args_map {
                state.serialize_entry(key, value)?;
            }

            state.end()
        } else {
            self.args.serialize(serializer)
        }
    }
}

impl<T> ServerResponse<T>
where
    T: Serialize,
{
    pub fn new<R>(response_type: R, args: T) -> Self
    where
        R: Into<String>,
    {
        Self {
            response_type: response_type.into(),
            args,
        }
    }

    pub fn error(error: T) -> Self {
        Self {
            response_type: "error".to_string(),
            args: error,
        }
    }
}

pub async fn get_router(state: &Arc<AppState>) -> Router<Arc<AppState>> {
    let path = format!("/{}", state.config.api_version);
    let public_path = format!("{path}/public");
    Router::new()
        .nest(&public_path, public::get_router(state))
        .nest(&path, secured_key::get_router(state).await)
        .nest(&path, secured_jwt::get_router(state).await)
        .route("/", get(get_root))
        .fallback(not_found_handler)
        .layer(CorsLayer::permissive())
}

pub async fn get_root() -> impl IntoResponse {
    Json(ServerResponse {
        response_type: "success".to_string(),
        args: json!({
            "success": true,
        }),
    })
}

pub async fn not_found_handler() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(ServerResponse {
            response_type: "error".to_string(),
            args: json!({
                "error": "Not found", // For backward compatibility
            }),
        }),
    )
}
