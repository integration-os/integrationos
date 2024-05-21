#[macro_export]
macro_rules! unauthorized {
    () => {
        (
            http::StatusCode::UNAUTHORIZED,
            axum::Json($crate::api_payloads::ErrorResponse {
                error: "Unauthorized".to_string(),
            }),
        )
    };
}

#[macro_export]
macro_rules! not_found {
    ($e:expr) => {
        (
            http::StatusCode::NOT_FOUND,
            axum::Json($crate::api_payloads::ErrorResponse {
                error: format!("{} not found", $e),
            }),
        )
    };
}

#[macro_export]
macro_rules! not_found_with_custom_message {
    ($e:expr) => {
        (
            http::StatusCode::NOT_FOUND,
            axum::Json($crate::api_payloads::ErrorResponse {
                error: format!("{}", $e),
            }),
        )
    };
}

#[macro_export]
macro_rules! internal_server_error {
    () => {
        (
            http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json($crate::api_payloads::ErrorResponse {
                error: "Internal error".to_string(),
            }),
        )
    };
}

#[macro_export]
macro_rules! too_many_requests {
    () => {
        (
            http::StatusCode::TOO_MANY_REQUESTS,
            axum::Json($crate::api_payloads::ErrorResponse {
                error: "Too Many Requests".to_string(),
            }),
        )
    };
}

#[macro_export]
macro_rules! bad_request {
    ($e:expr) => {
        (
            http::StatusCode::BAD_REQUEST,
            axum::Json($crate::api_payloads::ErrorResponse {
                error: $e.to_string(),
            }),
        )
    };
}

#[macro_export]
macro_rules! service_unavailable {
    () => {
        (
            http::StatusCode::SERVICE_UNAVAILABLE,
            axum::Json($crate::api_payloads::ErrorResponse {
                error: "Service unavailable".to_string(),
            }),
        )
    };
}

#[macro_export]
macro_rules! debug_error {
    ($e:expr) => {
        (
            http::StatusCode::UNPROCESSABLE_ENTITY,
            axum::Json($crate::api_payloads::ErrorResponse { error: $e }),
        )
    };
}
