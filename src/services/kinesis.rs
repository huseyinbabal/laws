use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::post;
use chrono::Utc;
use dashmap::DashMap;
use serde_json::{json, Value};

use crate::error::LawsError;
use crate::protocol::json::{json_error_response, json_response, parse_target};

// ---------------------------------------------------------------------------
// State & data model
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct KinesisRecord {
    pub data: String,
    pub partition_key: String,
    pub sequence_number: String,
    pub timestamp: f64,
}

#[derive(Clone, Debug)]
pub struct KinesisStream {
    pub stream_name: String,
    pub arn: String,
    pub shard_count: u64,
    pub status: String,
    pub records: Vec<KinesisRecord>,
}

pub struct KinesisState {
    pub streams: DashMap<String, KinesisStream>,
    sequence_counter: AtomicU64,
}

impl Default for KinesisState {
    fn default() -> Self {
        Self {
            streams: DashMap::new(),
            sequence_counter: AtomicU64::new(1),
        }
    }
}

impl KinesisState {
    fn next_sequence_number(&self) -> String {
        let n = self.sequence_counter.fetch_add(1, Ordering::SeqCst);
        format!(
            "4960000000000000000000000000000000000000000000000000{:04}",
            n
        )
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<KinesisState>) -> axum::Router {
    axum::Router::new()
        .route("/", post(handle))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Main dispatch handler
// ---------------------------------------------------------------------------

pub fn handle_request(state: &KinesisState, target: &str, body: &[u8]) -> Response {
    let action = target.split('.').next_back().unwrap_or("");

    let body: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(e) => {
            return json_error_response(&LawsError::InvalidRequest(format!(
                "invalid JSON body: {e}"
            )))
        }
    };

    let result = match action {
        "CreateStream" => create_stream(state, &body),
        "DeleteStream" => delete_stream(state, &body),
        "ListStreams" => list_streams(state),
        "DescribeStream" => describe_stream(state, &body),
        "PutRecord" => put_record(state, &body),
        "PutRecords" => put_records(state, &body),
        "GetShardIterator" => get_shard_iterator(state, &body),
        "GetRecords" => get_records(state, &body),
        other => Err(LawsError::InvalidRequest(format!(
            "unknown action: {other}"
        ))),
    };

    match result {
        Ok(v) => json_response(v),
        Err(e) => json_error_response(&e),
    }
}

async fn handle(
    State(state): State<Arc<KinesisState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target = match parse_target(&headers) {
        Ok(t) => t,
        Err(e) => return json_error_response(&e),
    };

    handle_request(&state, &target.action, &body)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_str<'a>(body: &'a Value, field: &str) -> Result<&'a str, LawsError> {
    body.get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest(format!("missing required field: {field}")))
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_stream(state: &KinesisState, body: &Value) -> Result<Value, LawsError> {
    let stream_name = require_str(body, "StreamName")?.to_owned();
    let shard_count = body.get("ShardCount").and_then(|v| v.as_u64()).unwrap_or(1);

    if state.streams.contains_key(&stream_name) {
        return Err(LawsError::AlreadyExists(format!(
            "stream already exists: {stream_name}"
        )));
    }

    let arn = format!("arn:aws:kinesis:us-east-1:000000000000:stream/{stream_name}");

    let stream = KinesisStream {
        stream_name: stream_name.clone(),
        arn,
        shard_count,
        status: "ACTIVE".into(),
        records: Vec::new(),
    };

    state.streams.insert(stream_name, stream);
    Ok(json!({}))
}

fn delete_stream(state: &KinesisState, body: &Value) -> Result<Value, LawsError> {
    let stream_name = require_str(body, "StreamName")?;
    state
        .streams
        .remove(stream_name)
        .ok_or_else(|| LawsError::NotFound(format!("stream not found: {stream_name}")))?;
    Ok(json!({}))
}

fn list_streams(state: &KinesisState) -> Result<Value, LawsError> {
    let names: Vec<String> = state
        .streams
        .iter()
        .map(|entry| entry.key().clone())
        .collect();
    Ok(json!({
        "StreamNames": names,
        "HasMoreStreams": false,
    }))
}

fn describe_stream(state: &KinesisState, body: &Value) -> Result<Value, LawsError> {
    let stream_name = require_str(body, "StreamName")?;
    let stream = state
        .streams
        .get(stream_name)
        .ok_or_else(|| LawsError::NotFound(format!("stream not found: {stream_name}")))?;

    let mut shards = Vec::new();
    for i in 0..stream.shard_count {
        shards.push(json!({
            "ShardId": format!("shardId-{:012}", i),
            "HashKeyRange": {
                "StartingHashKey": "0",
                "EndingHashKey": "340282366920938463463374607431768211455"
            },
            "SequenceNumberRange": {
                "StartingSequenceNumber": "49600000000000000000000000000000000000000000000000000001"
            }
        }));
    }

    Ok(json!({
        "StreamDescription": {
            "StreamName": stream.stream_name,
            "StreamARN": stream.arn,
            "StreamStatus": stream.status,
            "Shards": shards,
            "HasMoreShards": false,
            "RetentionPeriodHours": 24,
            "StreamCreationTimestamp": Utc::now().timestamp(),
            "EncryptionType": "NONE",
        }
    }))
}

fn put_record(state: &KinesisState, body: &Value) -> Result<Value, LawsError> {
    let stream_name = require_str(body, "StreamName")?;
    let data = require_str(body, "Data")?.to_owned();
    let partition_key = require_str(body, "PartitionKey")?.to_owned();

    let mut stream = state
        .streams
        .get_mut(stream_name)
        .ok_or_else(|| LawsError::NotFound(format!("stream not found: {stream_name}")))?;

    let seq = state.next_sequence_number();
    let record = KinesisRecord {
        data,
        partition_key,
        sequence_number: seq.clone(),
        timestamp: Utc::now().timestamp_millis() as f64 / 1000.0,
    };
    stream.records.push(record);

    Ok(json!({
        "ShardId": "shardId-000000000000",
        "SequenceNumber": seq,
        "EncryptionType": "NONE",
    }))
}

fn put_records(state: &KinesisState, body: &Value) -> Result<Value, LawsError> {
    let stream_name = require_str(body, "StreamName")?;
    let records_arr = body
        .get("Records")
        .and_then(|v| v.as_array())
        .ok_or_else(|| LawsError::InvalidRequest("missing Records array".into()))?;

    let mut stream = state
        .streams
        .get_mut(stream_name)
        .ok_or_else(|| LawsError::NotFound(format!("stream not found: {stream_name}")))?;

    let mut result_records = Vec::new();
    let now = Utc::now().timestamp_millis() as f64 / 1000.0;

    for rec in records_arr {
        let data = rec
            .get("Data")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        let partition_key = rec
            .get("PartitionKey")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();

        let seq = state.next_sequence_number();
        stream.records.push(KinesisRecord {
            data,
            partition_key,
            sequence_number: seq.clone(),
            timestamp: now,
        });

        result_records.push(json!({
            "ShardId": "shardId-000000000000",
            "SequenceNumber": seq,
        }));
    }

    Ok(json!({
        "FailedRecordCount": 0,
        "Records": result_records,
    }))
}

fn get_shard_iterator(state: &KinesisState, body: &Value) -> Result<Value, LawsError> {
    let stream_name = require_str(body, "StreamName")?;
    let _shard_id = require_str(body, "ShardId")?;
    let _iterator_type = require_str(body, "ShardIteratorType")?;

    if !state.streams.contains_key(stream_name) {
        return Err(LawsError::NotFound(format!(
            "stream not found: {stream_name}"
        )));
    }

    // Encode stream name into the iterator token so GetRecords can find records.
    let iterator = format!(
        "AAAAAAAAAAEaaaa-iterator-{stream_name}-{}",
        uuid::Uuid::new_v4()
    );

    Ok(json!({
        "ShardIterator": iterator,
    }))
}

fn get_records(state: &KinesisState, body: &Value) -> Result<Value, LawsError> {
    let iterator = require_str(body, "ShardIterator")?;

    // Extract stream name from our iterator token format:
    // "AAAAAAAAAAEaaaa-iterator-{stream_name}-{uuid}"
    let stream_name = iterator
        .strip_prefix("AAAAAAAAAAEaaaa-iterator-")
        .and_then(|rest| {
            rest.rsplit_once('-').map(|(_name, _uuid_last)| {
                // The UUID contains hyphens, so we need to find the stream name portion.
                // Our format: {stream_name}-{uuid} where uuid is 36 chars (8-4-4-4-12).
                // Take everything before the last 36 chars.
                if rest.len() > 36 {
                    &rest[..rest.len() - 37] // -37 to also remove the hyphen before UUID
                } else {
                    rest
                }
            })
        })
        .ok_or_else(|| LawsError::InvalidRequest("invalid shard iterator".into()))?;

    let stream = state
        .streams
        .get(stream_name)
        .ok_or_else(|| LawsError::NotFound(format!("stream not found: {stream_name}")))?;

    let records: Vec<Value> = stream
        .records
        .iter()
        .map(|r| {
            json!({
                "Data": r.data,
                "PartitionKey": r.partition_key,
                "SequenceNumber": r.sequence_number,
                "ApproximateArrivalTimestamp": r.timestamp,
            })
        })
        .collect();

    let next_iterator = format!(
        "AAAAAAAAAAEaaaa-iterator-{stream_name}-{}",
        uuid::Uuid::new_v4()
    );

    Ok(json!({
        "Records": records,
        "MillisBehindLatest": 0,
        "NextShardIterator": next_iterator,
    }))
}
