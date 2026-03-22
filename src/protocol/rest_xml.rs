//! REST-XML protocol response helpers.
//!
//! S3 is the primary service using this protocol. Routing is handled by axum
//! path extractors; this module provides response formatting for XML bodies.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::error::LawsError;

/// Build a success XML response with an optional body.
pub fn xml_response(status: StatusCode, body: &str) -> Response {
    if body.is_empty() {
        return status.into_response();
    }

    let xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>{body}"#);

    (
        status,
        [("content-type", "application/xml")],
        xml,
    )
        .into_response()
}

/// Shorthand for a 200 OK XML response.
pub fn ok(body: &str) -> Response {
    xml_response(StatusCode::OK, body)
}

/// Shorthand for a 204 No Content response.
pub fn no_content() -> Response {
    StatusCode::NO_CONTENT.into_response()
}

/// Build an S3-style XML error response.
///
/// ```xml
/// <?xml version="1.0" encoding="UTF-8"?>
/// <Error>
///   <Code>NoSuchBucket</Code>
///   <Message>The specified bucket does not exist</Message>
///   <RequestId>...</RequestId>
/// </Error>
/// ```
pub fn error_response(err: &LawsError) -> Response {
    let status = super::status_for_error(err);
    let request_id = super::request_id();
    let code = err.error_code();
    let err_msg = err.to_string();
    let message = quick_xml::escape::escape(&err_msg);

    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
  <Code>{code}</Code>
  <Message>{message}</Message>
  <RequestId>{request_id}</RequestId>
</Error>"#
    );

    (status, [("content-type", "application/xml")], body).into_response()
}
