use crate::server::AppState;
use axum::{body::Body, extract::State, middleware::Next, response::Response};
use http::Request;
use integrationos_domain::{
    ApplicationError, Claims, IntegrationOSError, DEFAULT_AUDIENCE, DEFAULT_ISSUER,
    FALLBACK_AUDIENCE, FALLBACK_ISSUER,
};
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
    pub fn from_state(state: &Arc<AppState>) -> Self {
        let mut validation = Validation::default();
        validation.set_audience(&[DEFAULT_AUDIENCE, FALLBACK_AUDIENCE]);
        validation.set_issuer(&[DEFAULT_ISSUER, FALLBACK_ISSUER]);
        Self {
            validation,
            decoding_key: DecodingKey::from_secret(state.config.jwt_secret.as_ref()),
        }
    }
}

pub async fn jwt_auth_middleware(
    State(state): State<Arc<JwtState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, IntegrationOSError> {
    let Some(auth_header) = req.headers().get(http::header::AUTHORIZATION) else {
        info!("missing authorization header");
        return Err(ApplicationError::unauthorized(
            "You are not authorized to access this resource",
            None,
        ));
    };

    let Ok(auth_header) = auth_header.to_str() else {
        info!("invalid authorization header");
        return Err(ApplicationError::unauthorized(
            "You are not authorized to access this resource",
            None,
        ));
    };

    if !auth_header.starts_with(BEARER_PREFIX) {
        info!("invalid authorization header");
        return Err(ApplicationError::unauthorized(
            "You are not authorized to access this resource",
            None,
        ));
    }

    let token = &auth_header[BEARER_PREFIX.len()..];

    match jsonwebtoken::decode::<Claims>(token, &state.decoding_key, &state.validation) {
        Ok(decoded_token) => {
            req.extensions_mut().insert(Arc::new(decoded_token.claims));
            Ok(next.run(req).await)
        }
        Err(e) => {
            info!("invalid JWT token : {:?}", e);
            Err(ApplicationError::forbidden(
                "You are not authorized to access this resource",
                None,
            ))
        }
    }
}
