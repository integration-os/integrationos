use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct RootResponse {
    pub success: bool,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct ErrorResponse {
    pub error: String,
}

impl ErrorResponse {
    pub fn new<T: ToString>(error: T) -> Self {
        Self {
            error: error.to_string(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct CreateResponse {
    pub id: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CreatePayload<T> {
    pub payload: T,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UpdateResponse {
    pub id: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DeleteResponse {
    pub id: String,
}
