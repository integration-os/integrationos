use crate::IntegrationOSError;
use axum::{
    response::{IntoResponse, Response},
    Json,
};
use http::StatusCode;

impl IntoResponse for IntegrationOSError {
    fn into_response(self) -> Response {
        (&self).into_response()
    }
}

impl IntoResponse for &IntegrationOSError {
    fn into_response(self) -> Response {
        let body = self.to_owned().as_application().as_json();

        let status: StatusCode = self.into();

        (status, Json(body)).into_response()
    }
}
