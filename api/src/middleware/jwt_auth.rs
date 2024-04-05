use crate::{api_payloads::ErrorResponse, server::AppState, unauthorized};
use axum::{extract::State, middleware::Next, response::Response, Json};
use http::{Request, StatusCode};
use jsonwebtoken::{DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Claims {
    #[serde(rename = "_id")]
    pub id: String,
    pub email: String,
    pub username: String,
    pub user_key: String,
    pub first_name: String,
    pub last_name: String,
    pub buildable_id: String,
    pub container_id: String,
    pub pointers: Vec<String>,
    pub is_buildable_core: bool,
    pub iat: i64,
    pub exp: i64,
    pub aud: String,
    pub iss: String,
}

const BEARER_PREFIX: &str = "Bearer ";

#[derive(Clone)]
pub struct JwtState {
    validation: Validation,
    decoding_key: DecodingKey,
}

impl JwtState {
    pub fn new(state: &Arc<AppState>) -> Self {
        Self {
            validation: Default::default(),
            decoding_key: DecodingKey::from_secret(state.config.jwt_secret.as_ref()),
        }
    }
}

pub async fn jwt_auth<B>(
    State(state): State<Arc<JwtState>>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let Some(auth_header) = req.headers().get(http::header::AUTHORIZATION) else {
        info!("missing authorization header");
        return Err(unauthorized!());
    };

    let Ok(auth_header) = auth_header.to_str() else {
        info!("invalid authorization header");
        return Err(unauthorized!());
    };

    if !auth_header.starts_with(BEARER_PREFIX) {
        info!("invalid authorization header");
        return Err(unauthorized!());
    }

    let token = &auth_header[BEARER_PREFIX.len()..];

    match jsonwebtoken::decode::<Claims>(token, &state.decoding_key, &state.validation) {
        Ok(decoded_token) => {
            req.extensions_mut().insert(decoded_token.claims);
            Ok(next.run(req).await)
        }
        Err(e) => {
            info!("invalid JWT token : {:?}", e);
            Err(unauthorized!())
        }
    }
}
