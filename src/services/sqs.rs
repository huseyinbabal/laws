use std::collections::VecDeque;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, Uri};
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use chrono::Utc;
use dashmap::DashMap;
use md5::{Digest, Md5};

use crate::error::LawsError;
use crate::protocol::query::{parse_query_request, xml_error_response, xml_response};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SqsMessage {
    pub message_id: String,
    pub body: String,
    pub receipt_handle: Option<String>,
    pub md5_of_body: String,
    pub sent_timestamp: String,
}

#[derive(Clone, Debug)]
pub struct SqsQueue {
    pub name: String,
    pub url: String,
    pub messages: VecDeque<SqsMessage>,
    pub attributes: std::collections::HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SqsState {
    pub queues: Arc<DashMap<String, SqsQueue>>,
}

impl SqsState {
    pub fn new() -> Self {
        Self {
            queues: Arc::new(DashMap::new()),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<SqsState>) -> Router {
    Router::new()
        .route("/", post(handle_sqs))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn md5_hex(data: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}

fn queue_url(queue_name: &str) -> String {
    format!("http://localhost:4566/{ACCOUNT_ID}/{queue_name}")
}

fn queue_name_from_url(url: &str) -> Option<String> {
    url.rsplit('/').next().map(|s| s.to_string())
}

// ---------------------------------------------------------------------------
// Dispatch handler
// ---------------------------------------------------------------------------

pub fn handle_request(
    state: &SqsState,
    headers: &HeaderMap,
    body: &Bytes,
    uri: &Uri,
) -> Response {
    let req = match parse_query_request(uri, headers, body) {
        Ok(r) => r,
        Err(e) => return xml_error_response(&e),
    };

    let result = match req.action.as_str() {
        "CreateQueue" => create_queue(state, &req.params),
        "DeleteQueue" => delete_queue(state, &req.params),
        "ListQueues" => list_queues(state),
        "SendMessage" => send_message(state, &req.params),
        "ReceiveMessage" => receive_message(state, &req.params),
        "DeleteMessage" => delete_message(state, &req.params),
        "GetQueueUrl" => get_queue_url(state, &req.params),
        "PurgeQueue" => purge_queue(state, &req.params),
        _ => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            req.action
        ))),
    };

    match result {
        Ok(resp) => resp,
        Err(e) => xml_error_response(&e),
    }
}

async fn handle_sqs(
    State(state): State<Arc<SqsState>>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_request(&state, &headers, &body, &uri)
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_queue(
    state: &SqsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let name = params
        .get("QueueName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing QueueName".into()))?;

    let url = queue_url(name);

    if !state.queues.contains_key(name) {
        let queue = SqsQueue {
            name: name.clone(),
            url: url.clone(),
            messages: VecDeque::new(),
            attributes: std::collections::HashMap::new(),
        };
        state.queues.insert(name.clone(), queue);
    }

    let inner = format!("<QueueUrl>{}</QueueUrl>", quick_xml::escape::escape(&url));
    Ok(xml_response("CreateQueue", &inner))
}

fn delete_queue(
    state: &SqsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let url = params
        .get("QueueUrl")
        .ok_or_else(|| LawsError::InvalidRequest("Missing QueueUrl".into()))?;

    let name = queue_name_from_url(url)
        .ok_or_else(|| LawsError::InvalidRequest("Invalid QueueUrl".into()))?;

    state
        .queues
        .remove(&name)
        .ok_or_else(|| LawsError::NotFound(format!("Queue {name} not found")))?;

    Ok(xml_response("DeleteQueue", ""))
}

fn list_queues(state: &SqsState) -> Result<Response, LawsError> {
    let mut inner = String::new();
    for entry in state.queues.iter() {
        let url = quick_xml::escape::escape(&entry.value().url);
        inner.push_str(&format!("<QueueUrl>{url}</QueueUrl>\n"));
    }
    Ok(xml_response("ListQueues", &inner))
}

fn send_message(
    state: &SqsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let url = params
        .get("QueueUrl")
        .ok_or_else(|| LawsError::InvalidRequest("Missing QueueUrl".into()))?;
    let message_body = params
        .get("MessageBody")
        .ok_or_else(|| LawsError::InvalidRequest("Missing MessageBody".into()))?;

    let name = queue_name_from_url(url)
        .ok_or_else(|| LawsError::InvalidRequest("Invalid QueueUrl".into()))?;

    let message_id = uuid::Uuid::new_v4().to_string();
    let md5 = md5_hex(message_body);

    let msg = SqsMessage {
        message_id: message_id.clone(),
        body: message_body.clone(),
        receipt_handle: None,
        md5_of_body: md5.clone(),
        sent_timestamp: Utc::now().timestamp_millis().to_string(),
    };

    let mut queue = state
        .queues
        .get_mut(&name)
        .ok_or_else(|| LawsError::NotFound(format!("Queue {name} not found")))?;
    queue.messages.push_back(msg);

    let inner = format!(
        "<MessageId>{message_id}</MessageId>\n<MD5OfMessageBody>{md5}</MD5OfMessageBody>"
    );
    Ok(xml_response("SendMessage", &inner))
}

fn receive_message(
    state: &SqsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let url = params
        .get("QueueUrl")
        .ok_or_else(|| LawsError::InvalidRequest("Missing QueueUrl".into()))?;

    let max: usize = params
        .get("MaxNumberOfMessages")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
        .min(10);

    let name = queue_name_from_url(url)
        .ok_or_else(|| LawsError::InvalidRequest("Invalid QueueUrl".into()))?;

    let mut queue = state
        .queues
        .get_mut(&name)
        .ok_or_else(|| LawsError::NotFound(format!("Queue {name} not found")))?;

    let mut messages_xml = String::new();
    let count = max.min(queue.messages.len());

    for _ in 0..count {
        if let Some(mut msg) = queue.messages.pop_front() {
            let receipt_handle = uuid::Uuid::new_v4().to_string();
            msg.receipt_handle = Some(receipt_handle.clone());

            let body_escaped = quick_xml::escape::escape(&msg.body);
            messages_xml.push_str(&format!(
                r#"<Message>
  <MessageId>{}</MessageId>
  <ReceiptHandle>{receipt_handle}</ReceiptHandle>
  <MD5OfBody>{}</MD5OfBody>
  <Body>{body_escaped}</Body>
</Message>
"#,
                msg.message_id, msg.md5_of_body
            ));
        }
    }

    Ok(xml_response("ReceiveMessage", &messages_xml))
}

fn delete_message(
    state: &SqsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let url = params
        .get("QueueUrl")
        .ok_or_else(|| LawsError::InvalidRequest("Missing QueueUrl".into()))?;
    let _receipt_handle = params
        .get("ReceiptHandle")
        .ok_or_else(|| LawsError::InvalidRequest("Missing ReceiptHandle".into()))?;

    let name = queue_name_from_url(url)
        .ok_or_else(|| LawsError::InvalidRequest("Invalid QueueUrl".into()))?;

    if !state.queues.contains_key(&name) {
        return Err(LawsError::NotFound(format!("Queue {name} not found")));
    }

    // Messages were already dequeued on ReceiveMessage; DeleteMessage is
    // an acknowledgement. In this mock we simply return success.
    Ok(xml_response("DeleteMessage", ""))
}

fn get_queue_url(
    state: &SqsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let name = params
        .get("QueueName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing QueueName".into()))?;

    let queue = state
        .queues
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("Queue {name} not found")))?;

    let url = quick_xml::escape::escape(&queue.url);
    let inner = format!("<QueueUrl>{url}</QueueUrl>");
    Ok(xml_response("GetQueueUrl", &inner))
}

fn purge_queue(
    state: &SqsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let queue_url = params
        .get("QueueUrl")
        .ok_or_else(|| LawsError::InvalidRequest("Missing QueueUrl".into()))?;

    let name = queue_name_from_url(queue_url)
        .ok_or_else(|| LawsError::InvalidRequest("Invalid QueueUrl".into()))?;

    let mut queue = state
        .queues
        .get_mut(&name)
        .ok_or_else(|| LawsError::NotFound(format!("Queue {name} not found")))?;

    queue.messages.clear();

    Ok(xml_response("PurgeQueue", ""))
}

// ---------------------------------------------------------------------------
// JSON protocol handler (awsjson1.0 – used by newer AWS CLI / SDK versions)
// ---------------------------------------------------------------------------

fn json_response(body: serde_json::Value) -> Response {
    use axum::response::IntoResponse;
    (
        axum::http::StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/x-amz-json-1.0",
        )],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn json_error(code: &str, message: &str, status: axum::http::StatusCode) -> Response {
    use axum::response::IntoResponse;
    let body = serde_json::json!({
        "__type": code,
        "message": message,
    });
    (
        status,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/x-amz-json-1.0",
        )],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

/// Handle SQS requests that arrive via the JSON protocol
/// (`X-Amz-Target: AmazonSQS.<Action>`, body is JSON).
pub fn handle_json_request(
    state: &SqsState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    // "AmazonSQS.ListQueues" → "ListQueues"
    let action = target.rsplit('.').next().unwrap_or("");

    match action {
        "CreateQueue" => json_create_queue(state, payload),
        "DeleteQueue" => json_delete_queue(state, payload),
        "ListQueues" => json_list_queues(state, payload),
        "SendMessage" => json_send_message(state, payload),
        "ReceiveMessage" => json_receive_message(state, payload),
        "DeleteMessage" => json_delete_message(state, payload),
        "GetQueueUrl" => json_get_queue_url(state, payload),
        "PurgeQueue" => json_purge_queue(state, payload),
        _ => json_error(
            "InvalidAction",
            &format!("Unknown action: {action}"),
            axum::http::StatusCode::BAD_REQUEST,
        ),
    }
}

fn json_str<'a>(v: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(|v| v.as_str())
}

fn json_create_queue(state: &SqsState, payload: &serde_json::Value) -> Response {
    let name = match json_str(payload, "QueueName") {
        Some(n) => n,
        None => {
            return json_error(
                "MissingParameter",
                "Missing QueueName",
                axum::http::StatusCode::BAD_REQUEST,
            )
        }
    };

    let url = queue_url(name);

    if !state.queues.contains_key(name) {
        let queue = SqsQueue {
            name: name.to_string(),
            url: url.clone(),
            messages: VecDeque::new(),
            attributes: std::collections::HashMap::new(),
        };
        state.queues.insert(name.to_string(), queue);
    }

    json_response(serde_json::json!({ "QueueUrl": url }))
}

fn json_delete_queue(state: &SqsState, payload: &serde_json::Value) -> Response {
    let url = match json_str(payload, "QueueUrl") {
        Some(u) => u,
        None => {
            return json_error(
                "MissingParameter",
                "Missing QueueUrl",
                axum::http::StatusCode::BAD_REQUEST,
            )
        }
    };

    let name = match queue_name_from_url(url) {
        Some(n) => n,
        None => {
            return json_error(
                "InvalidParameterValue",
                "Invalid QueueUrl",
                axum::http::StatusCode::BAD_REQUEST,
            )
        }
    };

    match state.queues.remove(&name) {
        Some(_) => json_response(serde_json::json!({})),
        None => json_error(
            "AWS.SimpleQueueService.NonExistentQueue",
            &format!("Queue {name} not found"),
            axum::http::StatusCode::BAD_REQUEST,
        ),
    }
}

fn json_list_queues(state: &SqsState, _payload: &serde_json::Value) -> Response {
    let urls: Vec<String> = state.queues.iter().map(|e| e.value().url.clone()).collect();
    json_response(serde_json::json!({ "QueueUrls": urls }))
}

fn json_send_message(state: &SqsState, payload: &serde_json::Value) -> Response {
    let url = match json_str(payload, "QueueUrl") {
        Some(u) => u,
        None => {
            return json_error(
                "MissingParameter",
                "Missing QueueUrl",
                axum::http::StatusCode::BAD_REQUEST,
            )
        }
    };
    let message_body = match json_str(payload, "MessageBody") {
        Some(b) => b,
        None => {
            return json_error(
                "MissingParameter",
                "Missing MessageBody",
                axum::http::StatusCode::BAD_REQUEST,
            )
        }
    };

    let name = match queue_name_from_url(url) {
        Some(n) => n,
        None => {
            return json_error(
                "InvalidParameterValue",
                "Invalid QueueUrl",
                axum::http::StatusCode::BAD_REQUEST,
            )
        }
    };

    let message_id = uuid::Uuid::new_v4().to_string();
    let md5 = md5_hex(message_body);

    let msg = SqsMessage {
        message_id: message_id.clone(),
        body: message_body.to_string(),
        receipt_handle: None,
        md5_of_body: md5.clone(),
        sent_timestamp: Utc::now().timestamp_millis().to_string(),
    };

    match state.queues.get_mut(&name) {
        Some(mut queue) => {
            queue.messages.push_back(msg);
            json_response(serde_json::json!({
                "MessageId": message_id,
                "MD5OfMessageBody": md5,
            }))
        }
        None => json_error(
            "AWS.SimpleQueueService.NonExistentQueue",
            &format!("Queue {name} not found"),
            axum::http::StatusCode::BAD_REQUEST,
        ),
    }
}

fn json_receive_message(state: &SqsState, payload: &serde_json::Value) -> Response {
    let url = match json_str(payload, "QueueUrl") {
        Some(u) => u,
        None => {
            return json_error(
                "MissingParameter",
                "Missing QueueUrl",
                axum::http::StatusCode::BAD_REQUEST,
            )
        }
    };

    let max: usize = payload
        .get("MaxNumberOfMessages")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .min(10) as usize;

    let name = match queue_name_from_url(url) {
        Some(n) => n,
        None => {
            return json_error(
                "InvalidParameterValue",
                "Invalid QueueUrl",
                axum::http::StatusCode::BAD_REQUEST,
            )
        }
    };

    let mut queue = match state.queues.get_mut(&name) {
        Some(q) => q,
        None => {
            return json_error(
                "AWS.SimpleQueueService.NonExistentQueue",
                &format!("Queue {name} not found"),
                axum::http::StatusCode::BAD_REQUEST,
            )
        }
    };

    let count = max.min(queue.messages.len());
    let mut messages = Vec::new();

    for _ in 0..count {
        if let Some(mut msg) = queue.messages.pop_front() {
            let receipt_handle = uuid::Uuid::new_v4().to_string();
            msg.receipt_handle = Some(receipt_handle.clone());
            messages.push(serde_json::json!({
                "MessageId": msg.message_id,
                "ReceiptHandle": receipt_handle,
                "MD5OfBody": msg.md5_of_body,
                "Body": msg.body,
            }));
        }
    }

    json_response(serde_json::json!({ "Messages": messages }))
}

fn json_delete_message(state: &SqsState, payload: &serde_json::Value) -> Response {
    let url = match json_str(payload, "QueueUrl") {
        Some(u) => u,
        None => {
            return json_error(
                "MissingParameter",
                "Missing QueueUrl",
                axum::http::StatusCode::BAD_REQUEST,
            )
        }
    };
    let _receipt_handle = match json_str(payload, "ReceiptHandle") {
        Some(r) => r,
        None => {
            return json_error(
                "MissingParameter",
                "Missing ReceiptHandle",
                axum::http::StatusCode::BAD_REQUEST,
            )
        }
    };

    let name = match queue_name_from_url(url) {
        Some(n) => n,
        None => {
            return json_error(
                "InvalidParameterValue",
                "Invalid QueueUrl",
                axum::http::StatusCode::BAD_REQUEST,
            )
        }
    };

    if !state.queues.contains_key(&name) {
        return json_error(
            "AWS.SimpleQueueService.NonExistentQueue",
            &format!("Queue {name} not found"),
            axum::http::StatusCode::BAD_REQUEST,
        );
    }

    json_response(serde_json::json!({}))
}

fn json_get_queue_url(state: &SqsState, payload: &serde_json::Value) -> Response {
    let name = match json_str(payload, "QueueName") {
        Some(n) => n,
        None => {
            return json_error(
                "MissingParameter",
                "Missing QueueName",
                axum::http::StatusCode::BAD_REQUEST,
            )
        }
    };

    match state.queues.get(name) {
        Some(queue) => json_response(serde_json::json!({ "QueueUrl": queue.url })),
        None => json_error(
            "AWS.SimpleQueueService.NonExistentQueue",
            &format!("Queue {name} not found"),
            axum::http::StatusCode::BAD_REQUEST,
        ),
    }
}

fn json_purge_queue(state: &SqsState, payload: &serde_json::Value) -> Response {
    let queue_url = match json_str(payload, "QueueUrl") {
        Some(u) => u,
        None => {
            return json_error(
                "MissingParameter",
                "Missing QueueUrl",
                axum::http::StatusCode::BAD_REQUEST,
            )
        }
    };

    let name = match queue_name_from_url(queue_url) {
        Some(n) => n,
        None => {
            return json_error(
                "InvalidParameterValue",
                "Invalid QueueUrl",
                axum::http::StatusCode::BAD_REQUEST,
            )
        }
    };

    match state.queues.get_mut(&name) {
        Some(mut queue) => {
            queue.messages.clear();
            json_response(serde_json::json!({}))
        }
        None => json_error(
            "AWS.SimpleQueueService.NonExistentQueue",
            &format!("Queue {name} not found"),
            axum::http::StatusCode::BAD_REQUEST,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_of_body() {
        // MD5("hello") = 5d41402abc4b2a76b9719d911017c592
        assert_eq!(md5_hex("hello"), "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn queue_name_extraction() {
        let url = "http://localhost:4566/000000000000/my-queue";
        assert_eq!(queue_name_from_url(url), Some("my-queue".to_string()));
    }
}
