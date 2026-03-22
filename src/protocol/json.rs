//! AWS JSON protocol handler.
//!
//! Services like DynamoDB and CloudWatch Logs use this protocol. The operation
//! name is conveyed via the `X-Amz-Target` header in the format
//! `ServiceName.ActionName` (e.g. `DynamoDB_20120810.PutItem`).

use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::error::LawsError;

/// Parsed target from the `X-Amz-Target` header.
#[derive(Debug)]
pub struct JsonTarget {
    /// The full header value (e.g. `DynamoDB_20120810.PutItem`).
    pub full: String,
    /// The service portion before the dot.
    pub service: String,
    /// The action portion after the dot.
    pub action: String,
}

/// Extract and parse the `X-Amz-Target` header.
///
/// Expected format: `ServiceName.ActionName`
pub fn parse_target(headers: &HeaderMap) -> Result<JsonTarget, LawsError> {
    let value = headers
        .get("x-amz-target")
        .ok_or_else(|| LawsError::InvalidRequest("missing X-Amz-Target header".into()))?
        .to_str()
        .map_err(|e| LawsError::InvalidRequest(format!("invalid X-Amz-Target header: {e}")))?;

    let (service, action) = value
        .split_once('.')
        .ok_or_else(|| {
            LawsError::InvalidRequest(format!(
                "X-Amz-Target must be in ServiceName.ActionName format, got: {value}"
            ))
        })?;

    Ok(JsonTarget {
        full: value.to_owned(),
        service: service.to_owned(),
        action: action.to_owned(),
    })
}

/// Build a success JSON response with the standard `application/x-amz-json-1.0`
/// content type.
pub fn json_response(body: serde_json::Value) -> Response {
    (
        StatusCode::OK,
        [("content-type", "application/x-amz-json-1.0")],
        serde_json::to_string(&body).unwrap(),
    )
        .into_response()
}

/// Build a JSON error response in the style AWS JSON-protocol services return.
///
/// ```json
/// { "__type": "ErrorCode", "message": "..." }
/// ```
pub fn json_error_response(err: &LawsError) -> Response {
    let status = super::status_for_error(err);
    let body = serde_json::json!({
        "__type": err.error_code(),
        "message": err.to_string(),
    });

    (
        status,
        [("content-type", "application/x-amz-json-1.0")],
        serde_json::to_string(&body).unwrap(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_target() {
        let mut headers = HeaderMap::new();
        headers.insert("x-amz-target", "DynamoDB_20120810.PutItem".parse().unwrap());

        let target = parse_target(&headers).unwrap();
        assert_eq!(target.service, "DynamoDB_20120810");
        assert_eq!(target.action, "PutItem");
    }

    #[test]
    fn missing_target_header() {
        let headers = HeaderMap::new();
        assert!(parse_target(&headers).is_err());
    }

    #[test]
    fn invalid_target_format() {
        let mut headers = HeaderMap::new();
        headers.insert("x-amz-target", "NoDotsHere".parse().unwrap());
        assert!(parse_target(&headers).is_err());
    }
}
