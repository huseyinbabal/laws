use axum::response::{IntoResponse, Response};
use chrono::Utc;
use dashmap::DashMap;
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DeliveryStream {
    pub name: String,
    pub arn: String,
    pub status: String,
    pub destination_type: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct FirehoseState {
    pub streams: DashMap<String, DeliveryStream>,
}

impl Default for FirehoseState {
    fn default() -> Self {
        Self {
            streams: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &FirehoseState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("Firehose_20150804.")
        .unwrap_or(target);

    let result = match action {
        "CreateDeliveryStream" => create_delivery_stream(state, payload),
        "DeleteDeliveryStream" => delete_delivery_stream(state, payload),
        "DescribeDeliveryStream" => describe_delivery_stream(state, payload),
        "ListDeliveryStreams" => list_delivery_streams(state),
        "PutRecord" => put_record(state, payload),
        "PutRecordBatch" => put_record_batch(state, payload),
        _ => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            action
        ))),
    };

    match result {
        Ok(resp) => resp,
        Err(e) => e.into_response(),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn json_response(body: Value) -> Response {
    (
        StatusCode::OK,
        [("Content-Type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_delivery_stream(
    state: &FirehoseState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload
        .get("DeliveryStreamName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            LawsError::InvalidRequest("missing required field: DeliveryStreamName".into())
        })?
        .to_owned();

    if state.streams.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "delivery stream already exists: {name}"
        )));
    }

    let arn = format!(
        "arn:aws:firehose:{REGION}:{ACCOUNT_ID}:deliverystream/{name}"
    );

    let stream = DeliveryStream {
        name: name.clone(),
        arn: arn.clone(),
        status: "ACTIVE".into(),
        destination_type: "S3".into(),
        created_at: Utc::now().to_rfc3339(),
    };

    state.streams.insert(name, stream);

    Ok(json_response(json!({
        "DeliveryStreamARN": arn
    })))
}

fn delete_delivery_stream(
    state: &FirehoseState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload
        .get("DeliveryStreamName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            LawsError::InvalidRequest("missing required field: DeliveryStreamName".into())
        })?;

    state
        .streams
        .remove(name)
        .ok_or_else(|| {
            LawsError::NotFound(format!("delivery stream not found: {name}"))
        })?;

    Ok(json_response(json!({})))
}

fn describe_delivery_stream(
    state: &FirehoseState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload
        .get("DeliveryStreamName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            LawsError::InvalidRequest("missing required field: DeliveryStreamName".into())
        })?;

    let stream = state
        .streams
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("delivery stream not found: {name}")))?;

    Ok(json_response(json!({
        "DeliveryStreamDescription": {
            "DeliveryStreamName": stream.name,
            "DeliveryStreamARN": stream.arn,
            "DeliveryStreamStatus": stream.status,
            "DeliveryStreamType": "DirectPut",
            "CreateTimestamp": stream.created_at,
            "Destinations": [{
                "DestinationId": "destinationId-000000000001",
                "S3DestinationDescription": {
                    "BucketARN": format!("arn:aws:s3:::{}", stream.name),
                    "CompressionFormat": "UNCOMPRESSED"
                }
            }],
            "HasMoreDestinations": false
        }
    })))
}

fn list_delivery_streams(state: &FirehoseState) -> Result<Response, LawsError> {
    let names: Vec<String> = state
        .streams
        .iter()
        .map(|entry| entry.key().clone())
        .collect();

    Ok(json_response(json!({
        "DeliveryStreamNames": names,
        "HasMoreDeliveryStreams": false
    })))
}

fn put_record(state: &FirehoseState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload
        .get("DeliveryStreamName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            LawsError::InvalidRequest("missing required field: DeliveryStreamName".into())
        })?;

    if !state.streams.contains_key(name) {
        return Err(LawsError::NotFound(format!(
            "delivery stream not found: {name}"
        )));
    }

    let record_id = uuid::Uuid::new_v4().to_string();

    Ok(json_response(json!({
        "RecordId": record_id,
        "Encrypted": false
    })))
}

fn put_record_batch(state: &FirehoseState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload
        .get("DeliveryStreamName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            LawsError::InvalidRequest("missing required field: DeliveryStreamName".into())
        })?;

    if !state.streams.contains_key(name) {
        return Err(LawsError::NotFound(format!(
            "delivery stream not found: {name}"
        )));
    }

    let records = payload
        .get("Records")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);

    let request_responses: Vec<Value> = (0..records)
        .map(|_| {
            json!({
                "RecordId": uuid::Uuid::new_v4().to_string()
            })
        })
        .collect();

    Ok(json_response(json!({
        "FailedPutCount": 0,
        "Encrypted": false,
        "RequestResponses": request_responses
    })))
}
