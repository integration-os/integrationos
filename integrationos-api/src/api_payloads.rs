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
