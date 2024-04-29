use crate::{endpoints::ApiError, server::AppState, unauthorized};
use axum::{body::Body, extract::State, middleware::Next, response::Response};
use http::Request;
use integrationos_domain::Claims;
use jsonwebtoken::{DecodingKey, Validation};
use std::sync::Arc;
use tracing::info;

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

pub async fn jwt_auth(
    State(state): State<Arc<JwtState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
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
            req.extensions_mut().insert(Arc::new(decoded_token.claims));
            Ok(next.run(req).await)
        }
        Err(e) => {
            info!("invalid JWT token : {:?}", e);
            Err(unauthorized!())
        }
    }
}
