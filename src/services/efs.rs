use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::Json;
use dashmap::DashMap;
use serde_json::{json, Value};

use crate::error::LawsError;
use crate::protocol::rest_json;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FileSystem {
    pub file_system_id: String,
    pub arn: String,
    pub creation_token: String,
    pub life_cycle_state: String,
    pub performance_mode: String,
    pub throughput_mode: String,
    pub encrypted: bool,
    pub size_in_bytes: u64,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct EfsState {
    pub file_systems: DashMap<String, FileSystem>,
}

impl Default for EfsState {
    fn default() -> Self {
        Self {
            file_systems: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<EfsState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/2015-02-01/file-systems",
            axum::routing::post(create_file_system).get(describe_file_systems),
        )
        .route(
            "/2015-02-01/file-systems/{id}",
            axum::routing::get(describe_file_system).delete(delete_file_system),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn file_system_to_json(fs: &FileSystem) -> Value {
    json!({
        "FileSystemId": fs.file_system_id,
        "FileSystemArn": fs.arn,
        "CreationToken": fs.creation_token,
        "LifeCycleState": fs.life_cycle_state,
        "PerformanceMode": fs.performance_mode,
        "ThroughputMode": fs.throughput_mode,
        "Encrypted": fs.encrypted,
        "SizeInBytes": {
            "Value": fs.size_in_bytes,
        },
        "OwnerId": ACCOUNT_ID,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_file_system(
    State(state): State<Arc<EfsState>>,
    Json(payload): Json<Value>,
) -> Response {
    let creation_token = payload["CreationToken"]
        .as_str()
        .unwrap_or("")
        .to_string();

    // Check for duplicate creation token
    for entry in state.file_systems.iter() {
        if entry.value().creation_token == creation_token && !creation_token.is_empty() {
            return rest_json::error_response(&LawsError::AlreadyExists(format!(
                "File system with CreationToken '{}' already exists",
                creation_token
            )));
        }
    }

    let fs_id = format!("fs-{}", &uuid::Uuid::new_v4().to_string()[..12]);
    let arn = format!(
        "arn:aws:elasticfilesystem:{REGION}:{ACCOUNT_ID}:file-system/{fs_id}"
    );
    let performance_mode = payload["PerformanceMode"]
        .as_str()
        .unwrap_or("generalPurpose")
        .to_string();
    let throughput_mode = payload["ThroughputMode"]
        .as_str()
        .unwrap_or("bursting")
        .to_string();
    let encrypted = payload["Encrypted"].as_bool().unwrap_or(false);

    let fs = FileSystem {
        file_system_id: fs_id.clone(),
        arn,
        creation_token,
        life_cycle_state: "available".to_string(),
        performance_mode,
        throughput_mode,
        encrypted,
        size_in_bytes: 0,
    };

    let resp = file_system_to_json(&fs);
    state.file_systems.insert(fs_id, fs);

    rest_json::created(resp)
}

async fn describe_file_systems(State(state): State<Arc<EfsState>>) -> Response {
    let items: Vec<Value> = state
        .file_systems
        .iter()
        .map(|entry| file_system_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "FileSystems": items }))
}

async fn describe_file_system(
    State(state): State<Arc<EfsState>>,
    Path(id): Path<String>,
) -> Response {
    match state.file_systems.get(&id) {
        Some(fs) => rest_json::ok(file_system_to_json(&fs)),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "File system not found: {id}"
        ))),
    }
}

async fn delete_file_system(
    State(state): State<Arc<EfsState>>,
    Path(id): Path<String>,
) -> Response {
    match state.file_systems.remove(&id) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "File system not found: {id}"
        ))),
    }
}
