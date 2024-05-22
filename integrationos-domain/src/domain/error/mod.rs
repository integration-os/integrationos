#[cfg(feature = "actix-error")]
pub mod actix_error;
#[cfg(feature = "axum-error")]
pub mod axum_error;

use crate::prelude::StringExt;
use http::StatusCode;
use mongodb::error::WriteFailure;
use serde::Serialize;
use std::convert::AsRef;
use std::{
    error::Error as StdError,
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    sync::Arc,
};
use strum::AsRefStr;
use thiserror::Error as ThisError;

pub trait ErrorMeta {
    fn code(&self) -> ErrorCode;
    fn key(&self) -> ErrorKey;
    fn message(&self) -> ErrorMessage;
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct ErrorCode(u16);

impl ErrorCode {
    pub fn as_u16(&self) -> u16 {
        self.0
    }
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct ErrorKey(String);

impl ErrorKey {
    pub fn internal(key: &str, subtype: Option<&str>) -> Self {
        if let Some(subtype) = subtype {
            ErrorKey(format!("err::internal::{}::{}", key, subtype))
        } else {
            ErrorKey(format!("err::internal::{}", key))
        }
    }

    pub fn application(key: &str, subtype: Option<&str>) -> Self {
        if let Some(subtype) = subtype {
            ErrorKey(format!("err::application::{}::{}", key, subtype))
        } else {
            ErrorKey(format!("err::application::{}", key))
        }
    }
}

impl Display for ErrorKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct ErrorMessage(String);

impl AsRef<str> for ErrorMessage {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for ErrorMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

#[derive(ThisError, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, AsRefStr)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "PascalCase")]
pub enum InternalError {
    #[error("An unknown error occurred: {}", .message)]
    UnknownError {
        message: String,
        subtype: Option<String>,
    },
    #[error("A unique field violation occurred: {}", .message)]
    UniqueFieldViolation {
        message: String,
        subtype: Option<String>,
    },
    #[error("A timeout occurred: {}", .message)]
    Timeout {
        message: String,
        subtype: Option<String>,
    },
    #[error("A connection error occurred: {}", .message)]
    ConnectionError {
        message: String,
        subtype: Option<String>,
    },
    #[error("Entity not found: {}", .message)]
    KeyNotFound {
        message: String,
        subtype: Option<String>,
    },
    #[error("Argument provided is invalid: {}", .message)]
    InvalidArgument {
        message: String,
        subtype: Option<String>,
    },
    #[error("An error while performing an IO operation: {}", .message)]
    IOErr {
        message: String,
        subtype: Option<String>,
    },
    #[error("Encription error: {}", .message)]
    EncryptionError {
        message: String,
        subtype: Option<String>,
    },
    #[error("Decryption error: {}", .message)]
    DecryptionError {
        message: String,
        subtype: Option<String>,
    },
    #[error("Configuration error: {}", .message)]
    ConfigurationError {
        message: String,
        subtype: Option<String>,
    },
    #[error("Serialization error: {}", .message)]
    SerializeError {
        message: String,
        subtype: Option<String>,
    },
    #[error("Deserialization error: {}", .message)]
    DeserializeError {
        message: String,
        subtype: Option<String>,
    },
    #[error("An error occurred running the javascript function: {}", .message)]
    ScriptError {
        message: String,
        subtype: Option<String>,
    },
}

impl From<anyhow::Error> for InternalError {
    fn from(error: anyhow::Error) -> Self {
        match error.downcast_ref::<InternalError>() {
            Some(integration_error) => integration_error.clone(),
            None => InternalError::UnknownError {
                message: error.to_string(),
                subtype: None,
            },
        }
    }
}

impl InternalError {
    pub fn unknown(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::internal(InternalError::UnknownError {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn unique_field_violation(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::internal(InternalError::UniqueFieldViolation {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn timeout(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::internal(InternalError::Timeout {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn script_error(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::internal(InternalError::ScriptError {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn serialize_error(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::internal(InternalError::SerializeError {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn deserialize_error(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::internal(InternalError::DeserializeError {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn configuration_error(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::internal(InternalError::ConfigurationError {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn encryption_error(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::internal(InternalError::EncryptionError {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn decryption_error(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::internal(InternalError::DecryptionError {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn connection_error(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::internal(InternalError::ConnectionError {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn io_err(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::internal(InternalError::IOErr {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn key_not_found(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::internal(InternalError::KeyNotFound {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn invalid_argument(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::internal(InternalError::InvalidArgument {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }
}

impl ErrorMeta for InternalError {
    fn code(&self) -> ErrorCode {
        match self {
            InternalError::UnknownError { .. } => ErrorCode(1000),
            InternalError::UniqueFieldViolation { .. } => ErrorCode(1001),
            InternalError::Timeout { .. } => ErrorCode(1002),
            InternalError::ConnectionError { .. } => ErrorCode(1003),
            InternalError::KeyNotFound { .. } => ErrorCode(1004),
            InternalError::InvalidArgument { .. } => ErrorCode(1005),
            InternalError::IOErr { .. } => ErrorCode(1006),
            InternalError::EncryptionError { .. } => ErrorCode(1007),
            InternalError::DecryptionError { .. } => ErrorCode(1008),
            InternalError::ConfigurationError { .. } => ErrorCode(1009),
            InternalError::ScriptError { .. } => ErrorCode(1010),
            InternalError::SerializeError { .. } => ErrorCode(1011),
            InternalError::DeserializeError { .. } => ErrorCode(1012),
        }
    }

    fn key(&self) -> ErrorKey {
        match self {
            InternalError::UnknownError { subtype, .. } => {
                ErrorKey::internal("unknown", subtype.as_deref())
            }
            InternalError::UniqueFieldViolation { subtype, .. } => {
                ErrorKey::internal("unique_violation", subtype.as_deref())
            }
            InternalError::Timeout { subtype, .. } => {
                ErrorKey::internal("timeout", subtype.as_deref())
            }
            InternalError::ConnectionError { subtype, .. } => {
                ErrorKey::internal("connection_error", subtype.as_deref())
            }
            InternalError::KeyNotFound { subtype, .. } => {
                ErrorKey::internal("key_not_found", subtype.as_deref())
            }
            InternalError::InvalidArgument { subtype, .. } => {
                ErrorKey::internal("invalid_argument", subtype.as_deref())
            }
            InternalError::IOErr { subtype, .. } => {
                ErrorKey::internal("io_err", subtype.as_deref())
            }
            InternalError::EncryptionError { subtype, .. } => {
                ErrorKey::internal("encryption_error", subtype.as_deref())
            }
            InternalError::DecryptionError { subtype, .. } => {
                ErrorKey::internal("decryption_error", subtype.as_deref())
            }
            InternalError::ConfigurationError { subtype, .. } => {
                ErrorKey::internal("configuration_error", subtype.as_deref())
            }
            InternalError::ScriptError { subtype, .. } => {
                ErrorKey::internal("script_error", subtype.as_deref())
            }
            InternalError::SerializeError { subtype, .. } => {
                ErrorKey::internal("serialize_error", subtype.as_deref())
            }
            InternalError::DeserializeError { subtype, .. } => {
                ErrorKey::internal("deserialize_error", subtype.as_deref())
            }
        }
    }

    fn message(&self) -> ErrorMessage {
        match self {
            InternalError::UnknownError { message, .. } => ErrorMessage(message.to_string()),
            InternalError::UniqueFieldViolation { message, .. } => {
                ErrorMessage(message.to_string())
            }
            InternalError::Timeout { message, .. } => ErrorMessage(message.to_string()),
            InternalError::ConnectionError { message, .. } => ErrorMessage(message.to_string()),
            InternalError::KeyNotFound { message, .. } => ErrorMessage(message.to_string()),
            InternalError::InvalidArgument { message, .. } => ErrorMessage(message.to_string()),
            InternalError::IOErr { message, .. } => ErrorMessage(message.to_string()),
            InternalError::EncryptionError { message, .. } => ErrorMessage(message.to_string()),
            InternalError::DecryptionError { message, .. } => ErrorMessage(message.to_string()),
            InternalError::ConfigurationError { message, .. } => ErrorMessage(message.to_string()),
            InternalError::ScriptError { message, .. } => ErrorMessage(message.to_string()),
            InternalError::SerializeError { message, .. } => ErrorMessage(message.to_string()),
            InternalError::DeserializeError { message, .. } => ErrorMessage(message.to_string()),
        }
    }
}

impl Debug for InternalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "{}\n", &self)?;
        let mut current = self.source();

        while let Some(cause) = current {
            writeln!(f, "Caused by:\n\t{}", cause)?;
            current = cause.source();
        }

        Ok(())
    }
}

#[derive(ThisError, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, AsRefStr)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "PascalCase")]
pub enum ApplicationError {
    #[error("Bad Request: {}", .message)]
    BadRequest {
        message: String,
        subtype: Option<String>,
    },
    #[error("Conflict: {}", .message)]
    Conflict {
        message: String,
        subtype: Option<String>,
    },
    #[error("Forbidden: {}", .message)]
    Forbidden {
        message: String,
        subtype: Option<String>,
    },
    #[error("Internal Server Error: {}", .message)]
    InternalServerError {
        message: String,
        subtype: Option<String>,
    },
    #[error("Method Not Allowed: {}", .message)]
    MethodNotAllowed {
        message: String,
        subtype: Option<String>,
    },
    #[error("Not Found: {}", .message)]
    NotFound {
        message: String,
        subtype: Option<String>,
    },
    #[error("Not Implemented: {}", .message)]
    NotImplemented {
        message: String,
        subtype: Option<String>,
    },
    #[error("Precondition Failed: {}", .message)]
    FailedDependency {
        message: String,
        subtype: Option<String>,
    },
    #[error("Service Unavailable: {}", .message)]
    ServiceUnavailable {
        message: String,
        subtype: Option<String>,
    },
    #[error("Too Many Requests: {}", .message)]
    TooManyRequests {
        message: String,
        subtype: Option<String>,
    },
    #[error("Unauthorized: {}", .message)]
    Unauthorized {
        message: String,
        subtype: Option<String>,
    },
    #[error("Unprocessable Entity: {}", .message)]
    UnprocessableEntity {
        message: String,
        subtype: Option<String>,
    },
}

impl From<anyhow::Error> for ApplicationError {
    fn from(error: anyhow::Error) -> Self {
        match error.downcast_ref::<ApplicationError>() {
            Some(integration_error) => integration_error.clone(),
            None => ApplicationError::InternalServerError {
                message: error.to_string(),
                subtype: None,
            },
        }
    }
}

impl ApplicationError {
    pub fn bad_request(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::application(ApplicationError::BadRequest {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn conflict(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::application(ApplicationError::Conflict {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn forbidden(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::application(ApplicationError::Forbidden {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn internal_server_error(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::application(ApplicationError::InternalServerError {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn method_not_allowed(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::application(ApplicationError::MethodNotAllowed {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn not_found(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::application(ApplicationError::NotFound {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn not_implemented(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::application(ApplicationError::NotImplemented {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn failed_dependency(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::application(ApplicationError::FailedDependency {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn service_unavailable(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::application(ApplicationError::ServiceUnavailable {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn too_many_requests(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::application(ApplicationError::TooManyRequests {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn unauthorized(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::application(ApplicationError::Unauthorized {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }

    pub fn unprocessable_entity(message: &str, subtype: Option<&str>) -> IntegrationOSError {
        IntegrationOSError::application(ApplicationError::UnprocessableEntity {
            message: message.to_string(),
            subtype: subtype.map(|s| s.to_string().snake_case()),
        })
    }
}

impl ErrorMeta for ApplicationError {
    fn code(&self) -> ErrorCode {
        match self {
            ApplicationError::BadRequest { .. } => ErrorCode(2000),
            ApplicationError::Conflict { .. } => ErrorCode(2001),
            ApplicationError::Forbidden { .. } => ErrorCode(2002),
            ApplicationError::InternalServerError { .. } => ErrorCode(2003),
            ApplicationError::MethodNotAllowed { .. } => ErrorCode(2004),
            ApplicationError::NotFound { .. } => ErrorCode(2005),
            ApplicationError::NotImplemented { .. } => ErrorCode(2006),
            ApplicationError::FailedDependency { .. } => ErrorCode(2007),
            ApplicationError::ServiceUnavailable { .. } => ErrorCode(2008),
            ApplicationError::TooManyRequests { .. } => ErrorCode(2009),
            ApplicationError::Unauthorized { .. } => ErrorCode(2010),
            ApplicationError::UnprocessableEntity { .. } => ErrorCode(2011),
        }
    }

    fn key(&self) -> ErrorKey {
        match self {
            ApplicationError::BadRequest { subtype, .. } => {
                ErrorKey::application("bad_request", subtype.as_deref())
            }
            ApplicationError::Conflict { subtype, .. } => {
                ErrorKey::application("conflict", subtype.as_deref())
            }
            ApplicationError::Forbidden { subtype, .. } => {
                ErrorKey::application("forbidden", subtype.as_deref())
            }
            ApplicationError::InternalServerError { subtype, .. } => {
                ErrorKey::application("internal_server_error", subtype.as_deref())
            }
            ApplicationError::MethodNotAllowed { subtype, .. } => {
                ErrorKey::application("method_not_allowed", subtype.as_deref())
            }
            ApplicationError::NotFound { subtype, .. } => {
                ErrorKey::application("not_found", subtype.as_deref())
            }
            ApplicationError::NotImplemented { subtype, .. } => {
                ErrorKey::application("not_implemented", subtype.as_deref())
            }
            ApplicationError::FailedDependency { subtype, .. } => {
                ErrorKey::application("failed_dependency", subtype.as_deref())
            }
            ApplicationError::ServiceUnavailable { subtype, .. } => {
                ErrorKey::application("service_unavailable", subtype.as_deref())
            }
            ApplicationError::TooManyRequests { subtype, .. } => {
                ErrorKey::application("too_many_requests", subtype.as_deref())
            }
            ApplicationError::Unauthorized { subtype, .. } => {
                ErrorKey::application("unauthorized", subtype.as_deref())
            }
            ApplicationError::UnprocessableEntity { subtype, .. } => {
                ErrorKey::application("unprocessable_entity", subtype.as_deref())
            }
        }
    }

    fn message(&self) -> ErrorMessage {
        match self {
            ApplicationError::BadRequest { message, .. } => ErrorMessage(message.to_string()),
            ApplicationError::Conflict { message, .. } => ErrorMessage(message.to_string()),
            ApplicationError::Forbidden { message, .. } => ErrorMessage(message.to_string()),
            ApplicationError::InternalServerError { message, .. } => {
                ErrorMessage(message.to_string())
            }
            ApplicationError::MethodNotAllowed { message, .. } => ErrorMessage(message.to_string()),
            ApplicationError::NotFound { message, .. } => ErrorMessage(message.to_string()),
            ApplicationError::NotImplemented { message, .. } => ErrorMessage(message.to_string()),
            ApplicationError::FailedDependency { message, .. } => ErrorMessage(message.to_string()),
            ApplicationError::ServiceUnavailable { message, .. } => {
                ErrorMessage(message.to_string())
            }
            ApplicationError::TooManyRequests { message, .. } => ErrorMessage(message.to_string()),
            ApplicationError::Unauthorized { message, .. } => ErrorMessage(message.to_string()),
            ApplicationError::UnprocessableEntity { message, .. } => {
                ErrorMessage(message.to_string())
            }
        }
    }
}

impl Debug for ApplicationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "{}\n", &self)?;
        let mut current = self.source();

        while let Some(cause) = current {
            writeln!(f, "Caused by:\n\t{}", cause)?;
            current = cause.source();
        }

        Ok(())
    }
}

impl From<InternalError> for ApplicationError {
    fn from(error: InternalError) -> Self {
        match error {
            InternalError::Timeout { .. }
            | InternalError::ConnectionError { .. }
            | InternalError::IOErr { .. }
            | InternalError::EncryptionError { .. }
            | InternalError::DecryptionError { .. }
            | InternalError::ScriptError { .. }
            | InternalError::ConfigurationError { .. }
            | InternalError::UnknownError { .. } => ApplicationError::InternalServerError {
                message: "An unknown error occurred".into(),
                subtype: None,
            },
            InternalError::UniqueFieldViolation { message, subtype } => {
                ApplicationError::Conflict { message, subtype }
            }
            InternalError::KeyNotFound { message, subtype } => {
                ApplicationError::NotFound { message, subtype }
            }
            InternalError::InvalidArgument { message, subtype }
            | InternalError::SerializeError { message, subtype }
            | InternalError::DeserializeError { message, subtype } => {
                ApplicationError::BadRequest { message, subtype }
            }
        }
    }
}

#[derive(ThisError, Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(untagged)]
pub enum IntegrationOSError {
    Internal(InternalError),
    Application(ApplicationError),
}

impl AsRef<str> for IntegrationOSError {
    fn as_ref(&self) -> &str {
        match self {
            IntegrationOSError::Internal(e) => e.as_ref(),
            IntegrationOSError::Application(e) => e.as_ref(),
        }
    }
}

impl From<anyhow::Error> for IntegrationOSError {
    fn from(error: anyhow::Error) -> Self {
        match error.downcast_ref::<IntegrationOSError>() {
            Some(integration_error) => match integration_error {
                internal @ IntegrationOSError::Internal(_) => internal.clone(),
                application @ IntegrationOSError::Application(_) => application.clone(),
            },
            None => IntegrationOSError::Internal(InternalError::UnknownError {
                message: error.to_string(),
                subtype: None,
            }),
        }
    }
}

impl From<Arc<IntegrationOSError>> for IntegrationOSError {
    fn from(error: Arc<IntegrationOSError>) -> Self {
        Arc::unwrap_or_clone(error)
    }
}

impl<'a> From<&'a IntegrationOSError> for StatusCode {
    fn from(value: &'a IntegrationOSError) -> Self {
        match value {
            IntegrationOSError::Internal(e) => match e {
                InternalError::UniqueFieldViolation { .. } => StatusCode::CONFLICT,
                InternalError::Timeout { .. } => StatusCode::GATEWAY_TIMEOUT,
                InternalError::ConnectionError { .. } => StatusCode::BAD_GATEWAY,
                InternalError::KeyNotFound { .. } => StatusCode::NOT_FOUND,
                InternalError::InvalidArgument { .. }
                | InternalError::SerializeError { .. }
                | InternalError::DeserializeError { .. } => StatusCode::BAD_REQUEST,
                InternalError::UnknownError { .. }
                | InternalError::IOErr { .. }
                | InternalError::EncryptionError { .. }
                | InternalError::ConfigurationError { .. }
                | InternalError::ScriptError { .. }
                | InternalError::DecryptionError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            },
            IntegrationOSError::Application(e) => match e {
                ApplicationError::BadRequest { .. } => StatusCode::BAD_REQUEST,
                ApplicationError::Conflict { .. } => StatusCode::CONFLICT,
                ApplicationError::Forbidden { .. } => StatusCode::FORBIDDEN,
                ApplicationError::InternalServerError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
                ApplicationError::MethodNotAllowed { .. } => StatusCode::METHOD_NOT_ALLOWED,
                ApplicationError::NotFound { .. } => StatusCode::NOT_FOUND,
                ApplicationError::NotImplemented { .. } => StatusCode::NOT_IMPLEMENTED,
                ApplicationError::FailedDependency { .. } => StatusCode::FAILED_DEPENDENCY,
                ApplicationError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
                ApplicationError::TooManyRequests { .. } => StatusCode::TOO_MANY_REQUESTS,
                ApplicationError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
                ApplicationError::UnprocessableEntity { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            },
        }
    }
}

impl From<mongodb::error::Error> for IntegrationOSError {
    fn from(err: mongodb::error::Error) -> Self {
        match *err.kind {
            mongodb::error::ErrorKind::InvalidArgument { message, .. } => {
                InternalError::invalid_argument(&message, None)
            }
            mongodb::error::ErrorKind::Authentication { message, .. } => {
                InternalError::connection_error(&message, Some("Authentication failed"))
            }
            mongodb::error::ErrorKind::BsonDeserialization(error) => {
                InternalError::invalid_argument(
                    &error.to_string(),
                    Some("BSON deserialization error"),
                )
            }
            mongodb::error::ErrorKind::BsonSerialization(error) => InternalError::invalid_argument(
                &error.to_string(),
                Some("BSON serialization error"),
            ),
            mongodb::error::ErrorKind::BulkWrite(error) => InternalError::unknown(
                &error
                    .write_errors
                    .into_iter()
                    .map(|e| e.into_iter().map(|e| e.message).collect())
                    .collect::<Vec<String>>()
                    .join(", "),
                Some("Bulk write error"),
            ),
            mongodb::error::ErrorKind::Command(error) => {
                let code = error.code;
                if code == 11000 {
                    InternalError::unique_field_violation(
                        &error.message,
                        Some("A document with the same unique key already exists"),
                    )
                } else {
                    InternalError::unknown(&error.message, Some("Command error"))
                }
            }
            mongodb::error::ErrorKind::DnsResolve { message, .. } => {
                InternalError::connection_error(&message, Some("DNS resolution failed"))
            }
            mongodb::error::ErrorKind::GridFs { .. } => InternalError::unknown(
                "GridFS error",
                Some("An error occurred while interacting with GridFS"),
            ),
            mongodb::error::ErrorKind::Internal { message, .. } => {
                InternalError::unknown(&message, Some("Internal error"))
            }
            mongodb::error::ErrorKind::Io(error) => InternalError::io_err(&error.to_string(), None),
            mongodb::error::ErrorKind::ConnectionPoolCleared { message, .. } => {
                InternalError::connection_error(&message, Some("Connection pool cleared"))
            }
            mongodb::error::ErrorKind::InvalidResponse { message, .. } => {
                InternalError::invalid_argument(&message, Some("Invalid response"))
            }
            mongodb::error::ErrorKind::ServerSelection { message, .. } => {
                InternalError::connection_error(&message, Some("Server selection failed"))
            }
            mongodb::error::ErrorKind::SessionsNotSupported => {
                InternalError::invalid_argument("Sessions not supported", None)
            }
            mongodb::error::ErrorKind::InvalidTlsConfig { message, .. } => {
                InternalError::connection_error(&message, Some("Invalid TLS configuration"))
            }
            mongodb::error::ErrorKind::Write(error) => match error {
                WriteFailure::WriteConcernError(err) => {
                    let code = err.code;

                    if code == 11000 {
                        InternalError::unique_field_violation(
                            &err.message,
                            Some("A document with the same unique key already exists"),
                        )
                    } else {
                        InternalError::unknown(&err.message, Some("Write concern error"))
                    }
                }
                WriteFailure::WriteError(err) => {
                    let code = err.code;
                    if code == 11000 {
                        InternalError::unique_field_violation(
                            &err.message,
                            Some("A document with the same unique key already exists"),
                        )
                    } else {
                        InternalError::unknown(&err.message, Some("Write error"))
                    }
                }
                _ => InternalError::unknown("Write error", Some("An error occurred while writing")),
            },
            mongodb::error::ErrorKind::Transaction { message, .. } => {
                InternalError::unknown(&message, Some("Transaction error"))
            }
            mongodb::error::ErrorKind::IncompatibleServer { message, .. } => {
                InternalError::connection_error(&message, Some("Incompatible server"))
            }
            mongodb::error::ErrorKind::MissingResumeToken => {
                InternalError::invalid_argument("Missing resume token", None)
            }
            mongodb::error::ErrorKind::Custom(_) => InternalError::unknown(
                "Unknown error",
                Some("An error occurred with the MongoDB driver"),
            ),
            mongodb::error::ErrorKind::Shutdown => {
                InternalError::unknown("Shutdown error", Some("The MongoDB driver has shut down"))
            }
            _ => InternalError::unknown("Unknown error", Some("An unknown error occurred")),
        }
    }
}

impl From<IntegrationOSError> for StatusCode {
    fn from(value: IntegrationOSError) -> Self {
        (&value).into()
    }
}

impl IntegrationOSError {
    #[cfg(feature = "actix-error")]
    pub fn status(&self) -> u16 {
        StatusCode::from(self).as_u16()
    }

    fn internal(internal: InternalError) -> Self {
        IntegrationOSError::Internal(internal)
    }

    fn application(application: ApplicationError) -> Self {
        IntegrationOSError::Application(application)
    }

    pub fn from_err_code(status: StatusCode, message: &str, subtype: Option<&str>) -> Self {
        let message = message.to_string();
        let subtype = subtype.map(|s| s.to_string());
        match status {
            StatusCode::BAD_REQUEST => {
                IntegrationOSError::application(ApplicationError::BadRequest { message, subtype })
            }

            StatusCode::CONFLICT => {
                IntegrationOSError::application(ApplicationError::Conflict { message, subtype })
            }

            StatusCode::FORBIDDEN => {
                IntegrationOSError::application(ApplicationError::Forbidden { message, subtype })
            }

            StatusCode::INTERNAL_SERVER_ERROR => {
                IntegrationOSError::application(ApplicationError::InternalServerError {
                    message,
                    subtype,
                })
            }

            StatusCode::METHOD_NOT_ALLOWED => {
                IntegrationOSError::application(ApplicationError::MethodNotAllowed {
                    message,
                    subtype,
                })
            }

            StatusCode::NOT_FOUND => {
                IntegrationOSError::application(ApplicationError::NotFound { message, subtype })
            }

            StatusCode::NOT_IMPLEMENTED => {
                IntegrationOSError::application(ApplicationError::NotImplemented {
                    message,
                    subtype,
                })
            }

            StatusCode::FAILED_DEPENDENCY => {
                IntegrationOSError::application(ApplicationError::FailedDependency {
                    message,
                    subtype,
                })
            }

            StatusCode::SERVICE_UNAVAILABLE => {
                IntegrationOSError::application(ApplicationError::ServiceUnavailable {
                    message,
                    subtype,
                })
            }

            StatusCode::TOO_MANY_REQUESTS => {
                IntegrationOSError::application(ApplicationError::TooManyRequests {
                    message,
                    subtype,
                })
            }

            StatusCode::UNAUTHORIZED => {
                IntegrationOSError::application(ApplicationError::Unauthorized { message, subtype })
            }

            StatusCode::UNPROCESSABLE_ENTITY => {
                IntegrationOSError::application(ApplicationError::UnprocessableEntity {
                    message,
                    subtype,
                })
            }

            _ => {
                if status.is_client_error() {
                    IntegrationOSError::application(ApplicationError::BadRequest {
                        message,
                        subtype,
                    })
                } else {
                    IntegrationOSError::internal(InternalError::IOErr {
                        message: format!(
                            "Unknown error with status code: {}, message: {}",
                            status, message
                        ),
                        subtype,
                    })
                }
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn as_application(&self) -> IntegrationOSError {
        match self {
            IntegrationOSError::Application(e) => IntegrationOSError::Application(e.clone()),
            IntegrationOSError::Internal(e) => IntegrationOSError::Application(e.clone().into()),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn as_json(&self) -> serde_json::Value {
        serde_json::json!({
            "passthrough": {
                "type": self.as_ref(),
                "code": self.code().as_u16(),
                "status": StatusCode::from(self).as_u16(),
                "key": self.key().to_string(),
                "message": self.message().to_string()
            }
        })
    }

    pub fn is_internal(&self) -> bool {
        matches!(self, IntegrationOSError::Internal(_))
    }

    pub fn is_application(&self) -> bool {
        matches!(self, IntegrationOSError::Application(_))
    }
}

impl ErrorMeta for IntegrationOSError {
    fn code(&self) -> ErrorCode {
        match self {
            IntegrationOSError::Internal(e) => e.code(),
            IntegrationOSError::Application(e) => e.code(),
        }
    }

    fn key(&self) -> ErrorKey {
        match self {
            IntegrationOSError::Internal(e) => e.key(),
            IntegrationOSError::Application(e) => e.key(),
        }
    }

    fn message(&self) -> ErrorMessage {
        match self {
            IntegrationOSError::Internal(e) => e.message(),
            IntegrationOSError::Application(e) => e.message(),
        }
    }
}

impl Display for IntegrationOSError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            IntegrationOSError::Internal(e) => write!(f, "{}", e),
            IntegrationOSError::Application(e) => write!(f, "{}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_function() {
        let internal_error: IntegrationOSError = InternalError::unknown("test", None);

        assert_eq!(internal_error.code(), ErrorCode(1000),);
        assert_eq!(internal_error.key(), ErrorKey::internal("unknown", None),);
        assert_eq!(internal_error.message(), ErrorMessage("test".to_string()),);
    }

    #[test]
    fn test_error_code() {
        let code = ErrorCode(400);
        assert_eq!(code.to_string(), "400");
    }

    #[test]
    fn test_error_key() {
        let key = ErrorKey::internal("test", None);
        assert_eq!(key.to_string(), "err::internal::test");
    }

    #[test]
    fn test_error_message() {
        let message = ErrorMessage("test".to_string());
        assert_eq!(message.to_string(), "test");
    }

    #[test]
    fn test_interoperability_between_anyhow_error_and_domain_error() {
        let err = InternalError::unknown("test", None);
        let any_err: anyhow::Error = err.clone().into();
        let code: ErrorCode = any_err.downcast_ref::<IntegrationOSError>().unwrap().code();
        let message: ErrorMessage = any_err
            .downcast_ref::<IntegrationOSError>()
            .unwrap()
            .message();
        let key: ErrorKey = any_err.downcast_ref::<IntegrationOSError>().unwrap().key();

        assert_eq!(code, ErrorCode(1000));
        assert_eq!(message, ErrorMessage("test".to_string()));
        assert_eq!(key, ErrorKey::internal("unknown", None));
    }

    #[test]
    fn test_round_trip_between_domain_error_and_anyhow_error() {
        let err = InternalError::unknown("test", None);
        let any_err: anyhow::Error = err.clone().into();
        let round_trip_err = any_err.into();

        let app_err = ApplicationError::bad_request("test", None);
        let any_app_err: anyhow::Error = app_err.clone().into();
        let round_trip_app_err = any_app_err.into();

        assert_eq!(err, round_trip_err);
        assert_eq!(app_err, round_trip_app_err);
    }

    #[test]
    fn from_internal_error_to_application_error() {
        let internal_error = InternalError::UniqueFieldViolation {
            message: "test".to_string(),
            subtype: None,
        };
        let app_error: ApplicationError = internal_error.into();

        assert_eq!(
            app_error,
            ApplicationError::Conflict {
                message: "test".to_string(),
                subtype: None
            }
        );
    }
}
