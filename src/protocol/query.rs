//! AWS Query protocol handler.
//!
//! Services like EC2, IAM, STS, SQS, and SNS use this protocol.
//! Requests arrive as `application/x-www-form-urlencoded` bodies (POST) or
//! query-string parameters (GET) with an `Action` field that names the
//! operation and additional key-value parameters.

use std::collections::HashMap;

use axum::body::Bytes;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};

use crate::error::LawsError;

/// Parsed AWS Query request.
#[derive(Debug)]
pub struct QueryRequest {
    /// The `Action` value (e.g. `CreateQueue`, `SendMessage`).
    pub action: String,
    /// All parameters from the query string / form body (including `Action`).
    pub params: HashMap<String, String>,
}

/// Parse an AWS Query-protocol request from either the URI query string or a
/// `application/x-www-form-urlencoded` POST body. The body takes precedence
/// when present and non-empty.
///
/// Also supports the newer JSON protocol variant where the action is specified
/// via the `X-Amz-Target` header (e.g. `AmazonSQS.ListQueues`) and the body
/// is JSON. In this case, the action is extracted from the header and JSON
/// body fields are flattened into the params map.
pub fn parse_query_request(
    uri: &Uri,
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<QueryRequest, LawsError> {
    // Check for JSON protocol via X-Amz-Target header
    if let Some(target) = headers.get("x-amz-target").and_then(|v| v.to_str().ok()) {
        if let Some(action) = target.split('.').next_back() {
            let mut params = HashMap::new();
            params.insert("Action".to_string(), action.to_string());

            // Parse JSON body if present and merge into params
            if !body.is_empty() {
                if let Ok(body_str) = std::str::from_utf8(body) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body_str) {
                        if let Some(obj) = json.as_object() {
                            for (k, v) in obj {
                                let val = match v {
                                    serde_json::Value::String(s) => s.clone(),
                                    other => other.to_string(),
                                };
                                params.insert(k.clone(), val);
                            }
                        }
                    }
                }
            }

            return Ok(QueryRequest {
                action: action.to_string(),
                params,
            });
        }
    }

    let raw = if !body.is_empty() && is_form_encoded(headers) {
        std::str::from_utf8(body)
            .map_err(|e| LawsError::InvalidRequest(format!("invalid utf-8 body: {e}")))?
            .to_owned()
    } else {
        uri.query().unwrap_or_default().to_owned()
    };

    let params: HashMap<String, String> = form_urlencoded::parse(raw.as_bytes())
        .into_owned()
        .collect();

    let action = params
        .get("Action")
        .cloned()
        .ok_or_else(|| LawsError::InvalidRequest("missing required parameter: Action".into()))?;

    Ok(QueryRequest { action, params })
}

/// Build a success XML response in the standard AWS Query envelope.
///
/// ```xml
/// <{action}Response xmlns="...">
///   <{action}Result>
///     {inner_xml}
///   </{action}Result>
///   <ResponseMetadata>
///     <RequestId>...</RequestId>
///   </ResponseMetadata>
/// </{action}Response>
/// ```
pub fn xml_response(action: &str, inner_xml: &str) -> Response {
    let request_id = super::request_id();
    let body = format!(
        r#"<{action}Response xmlns="https://iam.amazonaws.com/doc/2010-05-08/">
  <{action}Result>
    {inner_xml}
  </{action}Result>
  <ResponseMetadata>
    <RequestId>{request_id}</RequestId>
  </ResponseMetadata>
</{action}Response>"#
    );

    (
        StatusCode::OK,
        [("content-type", "text/xml; charset=utf-8")],
        body,
    )
        .into_response()
}

/// Build an AWS Query error XML response.
///
/// ```xml
/// <ErrorResponse xmlns="...">
///   <Error>
///     <Code>...</Code>
///     <Message>...</Message>
///   </Error>
///   <RequestId>...</RequestId>
/// </ErrorResponse>
/// ```
pub fn xml_error_response(err: &LawsError) -> Response {
    let request_id = super::request_id();
    let status = super::status_for_error(err);
    let code = err.error_code();
    let err_msg = err.to_string();
    let message = quick_xml::escape::escape(&err_msg);

    let body = format!(
        r#"<ErrorResponse>
  <Error>
    <Code>{code}</Code>
    <Message>{message}</Message>
  </Error>
  <RequestId>{request_id}</RequestId>
</ErrorResponse>"#
    );

    (status, [("content-type", "text/xml; charset=utf-8")], body).into_response()
}

fn is_form_encoded(headers: &HeaderMap) -> bool {
    headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("application/x-www-form-urlencoded"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_from_query_string() {
        let uri: Uri = "/?Action=DescribeInstances&InstanceId.1=i-abc123"
            .parse()
            .unwrap();
        let headers = HeaderMap::new();
        let body = Bytes::new();

        let req = parse_query_request(&uri, &headers, &body).unwrap();
        assert_eq!(req.action, "DescribeInstances");
        assert_eq!(req.params.get("InstanceId.1").unwrap(), "i-abc123");
    }

    #[test]
    fn parse_from_form_body() {
        let uri: Uri = "/".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            "content-type",
            "application/x-www-form-urlencoded".parse().unwrap(),
        );
        let body = Bytes::from("Action=CreateQueue&QueueName=my-queue");

        let req = parse_query_request(&uri, &headers, &body).unwrap();
        assert_eq!(req.action, "CreateQueue");
        assert_eq!(req.params.get("QueueName").unwrap(), "my-queue");
    }

    #[test]
    fn missing_action_is_error() {
        let uri: Uri = "/?Foo=bar".parse().unwrap();
        let headers = HeaderMap::new();
        let body = Bytes::new();

        let result = parse_query_request(&uri, &headers, &body);
        assert!(result.is_err());
    }
}
