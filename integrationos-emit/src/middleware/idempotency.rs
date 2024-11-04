use crate::domain::idempotency::IdempotencyKey;
use axum::{body::Body, middleware::Next, response::Response};
use http::Request;
use integrationos_domain::{ApplicationError, IntegrationOSError};

pub const IDEMPOTENCY_HEADER_STR: &str = "x-integrationos-idempotency-key";
const MAX_LENGTH: usize = 50;

pub async fn header_idempotency(
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, IntegrationOSError> {
    let Some(idempotency_key) = req.headers().get(IDEMPOTENCY_HEADER_STR) else {
        return Err(ApplicationError::bad_request(
            "Please provide an idempotency key",
            None,
        ));
    };

    let idempotency_key = idempotency_key
        .to_str()
        .map_err(|_| ApplicationError::bad_request("Invalid idempotency key", None))?;

    if idempotency_key.is_empty() {
        return Err(ApplicationError::bad_request(
            "Invalid idempotency key, cannot be empty",
            None,
        ));
    }

    if idempotency_key.len() > MAX_LENGTH {
        return Err(ApplicationError::bad_request(
            "Idempotency key is too long, max length is 50",
            None,
        ));
    }

    let data = IdempotencyKey::new(idempotency_key.to_owned());

    req.extensions_mut().insert(data);
    Ok(next.run(req).await)
}
