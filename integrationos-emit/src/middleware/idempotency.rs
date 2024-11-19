use crate::domain::idempotency::IdempotencyKey;
use axum::{body::Body, middleware::Next, response::Response};
use http::Request;
use integrationos_domain::{prefix::IdPrefix, ApplicationError, Id, IntegrationOSError};

pub const IDEMPOTENCY_HEADER_STR: &str = "x-integrationos-idempotency-key";

pub async fn header_idempotency(
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, IntegrationOSError> {
    if let Some(idempotency_key) = req.headers().get(IDEMPOTENCY_HEADER_STR) {
        let idempotency_key = idempotency_key
            .to_str()
            .map_err(|_| ApplicationError::bad_request("Invalid idempotency key", None))?;

        if idempotency_key.is_empty() {
            return Err(ApplicationError::bad_request(
                "Invalid idempotency key, cannot be empty",
                None,
            ));
        }

        let id = Id::try_from(idempotency_key.to_owned())
            .map_err(|_| ApplicationError::bad_request("Invalid idempotency key", None))?;

        let data = IdempotencyKey::new(id);
        req.extensions_mut().insert(data);
    } else {
        let data = IdempotencyKey::new(Id::now(IdPrefix::Idempotency));
        req.extensions_mut().insert(data);
    }
    Ok(next.run(req).await)
}
