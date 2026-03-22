pub mod json;
pub mod query;
pub mod rest_json;
pub mod rest_xml;

use axum::http::StatusCode;

/// Maps a `LawsError` variant to the appropriate HTTP status code.
pub fn status_for_error(err: &crate::error::LawsError) -> StatusCode {
    use crate::error::LawsError;
    match err {
        LawsError::NotFound(_) => StatusCode::NOT_FOUND,
        LawsError::AlreadyExists(_) => StatusCode::CONFLICT,
        LawsError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
        LawsError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// A unique request ID in the style AWS returns.
pub fn request_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
