use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum LawsError {
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl LawsError {
    pub fn error_code(&self) -> &str {
        match self {
            LawsError::NotFound(_) => "ResourceNotFoundException",
            LawsError::AlreadyExists(_) => "ResourceAlreadyExistsException",
            LawsError::InvalidRequest(_) => "InvalidParameterValueException",
            LawsError::Internal(_) => "InternalServiceError",
        }
    }
}

impl IntoResponse for LawsError {
    fn into_response(self) -> Response {
        let status = match &self {
            LawsError::NotFound(_) => StatusCode::NOT_FOUND,
            LawsError::AlreadyExists(_) => StatusCode::CONFLICT,
            LawsError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            LawsError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = serde_json::json!({
            "__type": self.error_code(),
            "message": self.to_string(),
        });
        (status, axum::Json(body)).into_response()
    }
}
