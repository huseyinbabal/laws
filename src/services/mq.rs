use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{delete, get, post};
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
pub struct MqBroker {
    pub broker_id: String,
    pub broker_name: String,
    pub arn: String,
    pub engine_type: String,
    pub engine_version: String,
    pub host_instance_type: String,
    pub deployment_mode: String,
    pub broker_state: String,
    pub created: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct MqState {
    pub brokers: DashMap<String, MqBroker>,
}

impl Default for MqState {
    fn default() -> Self {
        Self {
            brokers: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<MqState>) -> axum::Router {
    axum::Router::new()
        .route("/v1/brokers", post(create_broker).get(list_brokers))
        .route(
            "/v1/brokers/{id}",
            get(describe_broker).delete(delete_broker),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn broker_to_json(b: &MqBroker) -> Value {
    json!({
        "BrokerId": b.broker_id,
        "BrokerName": b.broker_name,
        "BrokerArn": b.arn,
        "EngineType": b.engine_type,
        "EngineVersion": b.engine_version,
        "HostInstanceType": b.host_instance_type,
        "DeploymentMode": b.deployment_mode,
        "BrokerState": b.broker_state,
        "Created": b.created,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_broker(State(state): State<Arc<MqState>>, Json(payload): Json<Value>) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let broker_name = payload["BrokerName"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing BrokerName".into()))?
            .to_string();

        let engine_type = payload["EngineType"]
            .as_str()
            .unwrap_or("ACTIVEMQ")
            .to_string();

        let engine_version = payload["EngineVersion"]
            .as_str()
            .unwrap_or("5.17.6")
            .to_string();

        let host_instance_type = payload["HostInstanceType"]
            .as_str()
            .unwrap_or("mq.m5.large")
            .to_string();

        let deployment_mode = payload["DeploymentMode"]
            .as_str()
            .unwrap_or("SINGLE_INSTANCE")
            .to_string();

        let broker_id = uuid::Uuid::new_v4().to_string();
        let arn = format!("arn:aws:mq:{REGION}:{ACCOUNT_ID}:broker:{broker_name}:{broker_id}");
        let now = chrono::Utc::now().to_rfc3339();

        let broker = MqBroker {
            broker_id: broker_id.clone(),
            broker_name: broker_name.clone(),
            arn: arn.clone(),
            engine_type,
            engine_version,
            host_instance_type,
            deployment_mode,
            broker_state: "RUNNING".to_string(),
            created: now,
        };

        state.brokers.insert(broker_id.clone(), broker);

        Ok(rest_json::created(json!({
            "BrokerId": broker_id,
            "BrokerArn": arn,
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_brokers(State(state): State<Arc<MqState>>) -> Response {
    let summaries: Vec<Value> = state
        .brokers
        .iter()
        .map(|entry| {
            let b = entry.value();
            json!({
                "BrokerId": b.broker_id,
                "BrokerName": b.broker_name,
                "BrokerArn": b.arn,
                "BrokerState": b.broker_state,
                "EngineType": b.engine_type,
                "DeploymentMode": b.deployment_mode,
                "Created": b.created,
            })
        })
        .collect();

    rest_json::ok(json!({ "BrokerSummaries": summaries }))
}

async fn describe_broker(State(state): State<Arc<MqState>>, Path(id): Path<String>) -> Response {
    match state.brokers.get(&id) {
        Some(broker) => rest_json::ok(broker_to_json(broker.value())),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Broker '{}' not found", id)))
        }
    }
}

async fn delete_broker(State(state): State<Arc<MqState>>, Path(id): Path<String>) -> Response {
    match state.brokers.remove(&id) {
        Some((_, broker)) => rest_json::ok(json!({
            "BrokerId": broker.broker_id,
        })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Broker '{}' not found", id)))
        }
    }
}
