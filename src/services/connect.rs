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
pub struct ConnectInstance {
    pub id: String,
    pub arn: String,
    pub identity_management_type: String,
    pub instance_alias: String,
    pub status: String,
    pub created_time: String,
}

#[derive(Debug, Clone)]
pub struct ContactFlow {
    pub id: String,
    pub arn: String,
    pub instance_id: String,
    pub name: String,
    pub flow_type: String,
    pub state: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ConnectState {
    pub instances: DashMap<String, ConnectInstance>,
    pub contact_flows: DashMap<String, ContactFlow>,
}

impl Default for ConnectState {
    fn default() -> Self {
        Self {
            instances: DashMap::new(),
            contact_flows: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<ConnectState>) -> axum::Router {
    axum::Router::new()
        .route("/instance", axum::routing::put(create_instance))
        .route(
            "/instance-summary-list",
            axum::routing::post(list_instances),
        )
        .route(
            "/instance/{id}",
            axum::routing::get(describe_instance).delete(delete_instance),
        )
        .route(
            "/contact-flow/{instanceId}",
            axum::routing::put(create_contact_flow),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn random_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_instance(
    State(state): State<Arc<ConnectState>>,
    Json(payload): Json<Value>,
) -> Response {
    let identity_management_type = payload
        .get("IdentityManagementType")
        .and_then(|v| v.as_str())
        .unwrap_or("CONNECT_MANAGED")
        .to_owned();
    let instance_alias = payload
        .get("InstanceAlias")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    let id = random_id();
    let arn = format!("arn:aws:connect:{REGION}:{ACCOUNT_ID}:instance/{id}");
    let created_time = chrono::Utc::now().to_rfc3339();

    let instance = ConnectInstance {
        id: id.clone(),
        arn: arn.clone(),
        identity_management_type,
        instance_alias,
        status: "ACTIVE".into(),
        created_time,
    };

    state.instances.insert(id.clone(), instance);

    rest_json::created(json!({
        "Id": id,
        "Arn": arn
    }))
}

async fn list_instances(State(state): State<Arc<ConnectState>>) -> Response {
    let instances: Vec<Value> = state
        .instances
        .iter()
        .map(|entry| {
            let i = entry.value();
            json!({
                "Id": i.id,
                "Arn": i.arn,
                "IdentityManagementType": i.identity_management_type,
                "InstanceAlias": i.instance_alias,
                "InstanceStatus": i.status,
                "CreatedTime": i.created_time
            })
        })
        .collect();

    rest_json::ok(json!({
        "InstanceSummaryList": instances
    }))
}

async fn describe_instance(
    State(state): State<Arc<ConnectState>>,
    Path(id): Path<String>,
) -> Response {
    match state.instances.get(&id) {
        Some(i) => rest_json::ok(json!({
            "Instance": {
                "Id": i.id,
                "Arn": i.arn,
                "IdentityManagementType": i.identity_management_type,
                "InstanceAlias": i.instance_alias,
                "InstanceStatus": i.status,
                "CreatedTime": i.created_time
            }
        })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("instance not found: {id}")))
        }
    }
}

async fn delete_instance(
    State(state): State<Arc<ConnectState>>,
    Path(id): Path<String>,
) -> Response {
    match state.instances.remove(&id) {
        Some(_) => {
            state.contact_flows.retain(|_, cf| cf.instance_id != id);
            rest_json::no_content()
        }
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("instance not found: {id}")))
        }
    }
}

async fn create_contact_flow(
    State(state): State<Arc<ConnectState>>,
    Path(instance_id): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    if !state.instances.contains_key(&instance_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "instance not found: {instance_id}"
        )));
    }

    let name = payload
        .get("Name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();
    let flow_type = payload
        .get("Type")
        .and_then(|v| v.as_str())
        .unwrap_or("CONTACT_FLOW")
        .to_owned();

    let id = random_id();
    let arn =
        format!("arn:aws:connect:{REGION}:{ACCOUNT_ID}:instance/{instance_id}/contact-flow/{id}");

    let cf = ContactFlow {
        id: id.clone(),
        arn: arn.clone(),
        instance_id,
        name,
        flow_type,
        state: "ACTIVE".into(),
    };

    state.contact_flows.insert(id.clone(), cf);

    rest_json::created(json!({
        "ContactFlowId": id,
        "ContactFlowArn": arn
    }))
}
