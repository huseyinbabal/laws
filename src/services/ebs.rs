use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::Json;
use chrono::Utc;
use dashmap::DashMap;
use serde_json::{json, Value};

use crate::error::LawsError;
use crate::protocol::rest_json;

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub snapshot_id: String,
    pub volume_size: i64,
    pub status: String,
    pub block_size: i64,
    pub blocks: HashMap<i64, Vec<u8>>,
    pub created_at: String,
}

pub struct EbsState {
    pub snapshots: DashMap<String, Snapshot>,
}

impl Default for EbsState {
    fn default() -> Self {
        Self {
            snapshots: DashMap::new(),
        }
    }
}

pub fn router(state: Arc<EbsState>) -> axum::Router {
    axum::Router::new()
        .route("/snapshots", post(start_snapshot))
        .route("/snapshots/{snapshot_id}/complete", post(complete_snapshot))
        .route(
            "/snapshots/{snapshot_id}/blocks/{block_index}",
            put(put_snapshot_block).get(get_snapshot_block),
        )
        .route("/snapshots/{snapshot_id}/blocks", get(list_snapshot_blocks))
        .route(
            "/snapshots/{snapshot_id}/changedblocks",
            get(list_changed_blocks),
        )
        .with_state(state)
}

async fn start_snapshot(
    State(state): State<Arc<EbsState>>,
    Json(body): Json<Value>,
) -> Response {
    let volume_size = body["VolumeSize"].as_i64().unwrap_or(1);
    let block_size = 524288;
    let snapshot_id = format!("snap-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let now = Utc::now().to_rfc3339();

    let snapshot = Snapshot {
        snapshot_id: snapshot_id.clone(),
        volume_size,
        status: "pending".to_string(),
        block_size,
        blocks: HashMap::new(),
        created_at: now,
    };

    let resp = json!({
        "SnapshotId": snapshot.snapshot_id,
        "VolumeSize": snapshot.volume_size,
        "Status": snapshot.status,
        "BlockSize": snapshot.block_size,
    });

    state.snapshots.insert(snapshot_id, snapshot);
    rest_json::created(resp)
}

async fn complete_snapshot(
    State(state): State<Arc<EbsState>>,
    Path(snapshot_id): Path<String>,
) -> Response {
    match state.snapshots.get_mut(&snapshot_id) {
        Some(mut s) => {
            s.status = "completed".to_string();
            rest_json::ok(json!({ "Status": "completed" }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Snapshot not found: {snapshot_id}"
        ))),
    }
}

async fn put_snapshot_block(
    State(state): State<Arc<EbsState>>,
    Path((snapshot_id, block_index)): Path<(String, i64)>,
) -> Response {
    match state.snapshots.get_mut(&snapshot_id) {
        Some(mut s) => {
            s.blocks.insert(block_index, vec![0u8; 512]);
            rest_json::ok(json!({
                "Checksum": "mock-checksum",
                "ChecksumAlgorithm": "SHA256",
            }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Snapshot not found: {snapshot_id}"
        ))),
    }
}

async fn get_snapshot_block(
    State(state): State<Arc<EbsState>>,
    Path((snapshot_id, block_index)): Path<(String, i64)>,
) -> Response {
    match state.snapshots.get(&snapshot_id) {
        Some(s) => {
            if s.blocks.contains_key(&block_index) {
                rest_json::ok(json!({
                    "DataLength": 512,
                    "Checksum": "mock-checksum",
                    "ChecksumAlgorithm": "SHA256",
                }))
            } else {
                rest_json::error_response(&LawsError::NotFound(format!(
                    "Block not found: {block_index}"
                )))
            }
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Snapshot not found: {snapshot_id}"
        ))),
    }
}

async fn list_snapshot_blocks(
    State(state): State<Arc<EbsState>>,
    Path(snapshot_id): Path<String>,
) -> Response {
    match state.snapshots.get(&snapshot_id) {
        Some(s) => {
            let blocks: Vec<Value> = s
                .blocks
                .iter()
                .map(|(idx, _)| {
                    json!({
                        "BlockIndex": idx,
                        "BlockToken": format!("token-{}", idx),
                    })
                })
                .collect();

            rest_json::ok(json!({
                "Blocks": blocks,
                "VolumeSize": s.volume_size,
                "BlockSize": s.block_size,
            }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Snapshot not found: {snapshot_id}"
        ))),
    }
}

async fn list_changed_blocks(
    State(state): State<Arc<EbsState>>,
    Path(snapshot_id): Path<String>,
) -> Response {
    match state.snapshots.get(&snapshot_id) {
        Some(s) => {
            let blocks: Vec<Value> = s
                .blocks
                .iter()
                .map(|(idx, _)| {
                    json!({
                        "BlockIndex": idx,
                        "FirstBlockToken": format!("token-{}", idx),
                    })
                })
                .collect();

            rest_json::ok(json!({
                "ChangedBlocks": blocks,
                "VolumeSize": s.volume_size,
                "BlockSize": s.block_size,
            }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Snapshot not found: {snapshot_id}"
        ))),
    }
}
