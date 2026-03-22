//! REST-JSON protocol response helpers.
//!
//! Services like Lambda, Secrets Manager, and SSM use this protocol. Routing is
//! handled by axum path extractors; this module provides response formatting.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::error::LawsError;

/// Build a success JSON response with the standard `application/json` content type.
pub fn json_response(status: StatusCode, body: serde_json::Value) -> Response {
    (
        status,
        [("content-type", "application/json")],
        serde_json::to_string(&body).unwrap(),
    )
        .into_response()
}

/// Shorthand for a 200 OK JSON response.
pub fn ok(body: serde_json::Value) -> Response {
    json_response(StatusCode::OK, body)
}

/// Shorthand for a 201 Created JSON response.
pub fn created(body: serde_json::Value) -> Response {
    json_response(StatusCode::CREATED, body)
}

/// Shorthand for a 204 No Content response with an empty body.
pub fn no_content() -> Response {
    StatusCode::NO_CONTENT.into_response()
}

/// Build a REST-JSON error response. AWS REST-JSON services return errors as:
///
/// ```json
/// { "message": "...", "__type": "ErrorCode" }
/// ```
///
/// with an `x-amzn-errortype` header.
pub fn error_response(err: &LawsError) -> Response {
    let status = super::status_for_error(err);
    let code = err.error_code();
    let body = serde_json::json!({
        "__type": code,
        "message": err.to_string(),
    });

    (
        status,
        [
            ("content-type", "application/json"),
            ("x-amzn-errortype", code),
        ],
        serde_json::to_string(&body).unwrap(),
    )
        .into_response()
}
