use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post, put};
use axum::Json;
use chrono::Utc;
use dashmap::DashMap;
use serde::Deserialize;
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
pub struct QuantumTask {
    pub quantum_task_arn: String,
    pub device_arn: String,
    pub status: String,
    pub shots: i64,
    pub output_s3_bucket: String,
    pub output_s3_directory: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Device {
    pub device_arn: String,
    pub device_name: String,
    pub device_type: String,
    pub device_status: String,
    pub provider_name: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct BraketState {
    pub quantum_tasks: DashMap<String, QuantumTask>,
    pub devices: DashMap<String, Device>,
}

impl Default for BraketState {
    fn default() -> Self {
        let devices = DashMap::new();
        // Seed some default devices
        let sv1 = Device {
            device_arn: format!("arn:aws:braket:{REGION}::device/quantum-simulator/amazon/sv1"),
            device_name: "SV1".to_string(),
            device_type: "SIMULATOR".to_string(),
            device_status: "ONLINE".to_string(),
            provider_name: "Amazon".to_string(),
        };
        devices.insert(sv1.device_arn.clone(), sv1);

        Self {
            quantum_tasks: DashMap::new(),
            devices,
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<BraketState>) -> axum::Router {
    axum::Router::new()
        .route("/quantum-task", post(create_quantum_task))
        .route(
            "/quantum-task/{task_id}",
            get(get_quantum_task),
        )
        .route("/quantum-tasks", post(search_quantum_tasks))
        .route(
            "/quantum-task/{task_id}/cancel",
            put(cancel_quantum_task),
        )
        .route("/device/{device_arn}", get(get_device))
        .route("/devices", post(search_devices))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateQuantumTaskRequest {
    #[serde(alias = "deviceArn")]
    device_arn: String,
    #[serde(alias = "shots", default)]
    shots: Option<i64>,
    #[serde(alias = "outputS3Bucket", default)]
    output_s3_bucket: Option<String>,
    #[serde(alias = "outputS3KeyPrefix", default)]
    output_s3_directory: Option<String>,
}

async fn create_quantum_task(
    State(state): State<Arc<BraketState>>,
    Json(req): Json<CreateQuantumTaskRequest>,
) -> Response {
    let task_id = uuid::Uuid::new_v4().to_string();
    let quantum_task_arn = format!(
        "arn:aws:braket:{REGION}:{ACCOUNT_ID}:quantum-task/{task_id}"
    );
    let now = Utc::now().to_rfc3339();

    let task = QuantumTask {
        quantum_task_arn: quantum_task_arn.clone(),
        device_arn: req.device_arn,
        status: "CREATED".to_string(),
        shots: req.shots.unwrap_or(1000),
        output_s3_bucket: req
            .output_s3_bucket
            .unwrap_or_else(|| "braket-output".into()),
        output_s3_directory: req
            .output_s3_directory
            .unwrap_or_else(|| "results".into()),
        created_at: now,
    };

    state.quantum_tasks.insert(task_id, task);

    rest_json::created(json!({ "quantumTaskArn": quantum_task_arn }))
}

async fn get_quantum_task(
    State(state): State<Arc<BraketState>>,
    Path(task_id): Path<String>,
) -> Response {
    match state.quantum_tasks.get(&task_id) {
        Some(t) => rest_json::ok(json!({
            "quantumTaskArn": t.quantum_task_arn,
            "deviceArn": t.device_arn,
            "status": t.status,
            "shots": t.shots,
            "outputS3Bucket": t.output_s3_bucket,
            "outputS3Directory": t.output_s3_directory,
            "createdAt": t.created_at,
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "QuantumTask not found: {task_id}"
        ))),
    }
}

async fn search_quantum_tasks(
    State(state): State<Arc<BraketState>>,
    Json(_body): Json<Value>,
) -> Response {
    let tasks: Vec<Value> = state
        .quantum_tasks
        .iter()
        .map(|entry| {
            let t = entry.value();
            json!({
                "quantumTaskArn": t.quantum_task_arn,
                "deviceArn": t.device_arn,
                "status": t.status,
                "shots": t.shots,
                "createdAt": t.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "quantumTasks": tasks }))
}

async fn cancel_quantum_task(
    State(state): State<Arc<BraketState>>,
    Path(task_id): Path<String>,
) -> Response {
    match state.quantum_tasks.get_mut(&task_id) {
        Some(mut t) => {
            t.status = "CANCELLING".to_string();
            rest_json::ok(json!({
                "quantumTaskArn": t.quantum_task_arn,
                "cancellationStatus": "CANCELLING",
            }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "QuantumTask not found: {task_id}"
        ))),
    }
}

async fn get_device(
    State(state): State<Arc<BraketState>>,
    Path(device_arn): Path<String>,
) -> Response {
    match state.devices.get(&device_arn) {
        Some(d) => rest_json::ok(json!({
            "deviceArn": d.device_arn,
            "deviceName": d.device_name,
            "deviceType": d.device_type,
            "deviceStatus": d.device_status,
            "providerName": d.provider_name,
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Device not found: {device_arn}"
        ))),
    }
}

async fn search_devices(
    State(state): State<Arc<BraketState>>,
    Json(_body): Json<Value>,
) -> Response {
    let devices: Vec<Value> = state
        .devices
        .iter()
        .map(|entry| {
            let d = entry.value();
            json!({
                "deviceArn": d.device_arn,
                "deviceName": d.device_name,
                "deviceType": d.device_type,
                "deviceStatus": d.device_status,
                "providerName": d.provider_name,
            })
        })
        .collect();

    rest_json::ok(json!({ "devices": devices }))
}
