use axum::response::{IntoResponse, Response};
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
pub struct CloudMapNamespace {
    pub id: String,
    pub name: String,
    pub arn: String,
    pub ns_type: String,
    pub created: f64,
}

#[derive(Debug, Clone)]
pub struct CloudMapService {
    pub id: String,
    pub name: String,
    pub arn: String,
    pub namespace_id: String,
    pub created: f64,
    pub instances: DashMap<String, Value>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CloudMapState {
    pub namespaces: DashMap<String, CloudMapNamespace>,
    pub services: DashMap<String, CloudMapService>,
}

impl Default for CloudMapState {
    fn default() -> Self {
        Self {
            namespaces: DashMap::new(),
            services: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &CloudMapState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("Route53AutoNaming_v20170314.")
        .unwrap_or(target);

    let result = match action {
        "CreatePrivateDnsNamespace" => create_private_dns_namespace(state, payload),
        "DeleteNamespace" => delete_namespace(state, payload),
        "ListNamespaces" => list_namespaces(state),
        "CreateService" => create_service(state, payload),
        "DeleteService" => delete_service(state, payload),
        "ListServices" => list_services(state),
        "RegisterInstance" => register_instance(state, payload),
        "DeregisterInstance" => deregister_instance(state, payload),
        other => Err(LawsError::InvalidRequest(format!(
            "unknown action: {other}"
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

fn now_epoch() -> f64 {
    chrono::Utc::now().timestamp() as f64
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_private_dns_namespace(
    state: &CloudMapState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:servicediscovery:{REGION}:{ACCOUNT_ID}:namespace/{id}"
    );

    let ns = CloudMapNamespace {
        id: id.clone(),
        name,
        arn,
        ns_type: "DNS_PRIVATE".to_string(),
        created: now_epoch(),
    };

    let operation_id = uuid::Uuid::new_v4().to_string();
    state.namespaces.insert(id, ns);

    Ok(json_response(json!({
        "OperationId": operation_id,
    })))
}

fn delete_namespace(state: &CloudMapState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["Id"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Id is required".to_string()))?;

    state
        .namespaces
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("Namespace '{}' not found", id)))?;

    let operation_id = uuid::Uuid::new_v4().to_string();

    Ok(json_response(json!({
        "OperationId": operation_id,
    })))
}

fn list_namespaces(state: &CloudMapState) -> Result<Response, LawsError> {
    let items: Vec<Value> = state
        .namespaces
        .iter()
        .map(|entry| {
            let ns = entry.value();
            json!({
                "Id": ns.id,
                "Name": ns.name,
                "Arn": ns.arn,
                "Type": ns.ns_type,
                "CreateDate": ns.created,
            })
        })
        .collect();

    Ok(json_response(json!({ "Namespaces": items })))
}

fn create_service(state: &CloudMapState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    let namespace_id = payload["NamespaceId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("NamespaceId is required".to_string()))?
        .to_string();

    if !state.namespaces.contains_key(&namespace_id) {
        return Err(LawsError::NotFound(format!(
            "Namespace '{}' not found",
            namespace_id
        )));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:servicediscovery:{REGION}:{ACCOUNT_ID}:service/{id}"
    );

    let svc = CloudMapService {
        id: id.clone(),
        name: name.clone(),
        arn: arn.clone(),
        namespace_id,
        created: now_epoch(),
        instances: DashMap::new(),
    };

    state.services.insert(id.clone(), svc);

    Ok(json_response(json!({
        "Service": {
            "Id": id,
            "Name": name,
            "Arn": arn,
        }
    })))
}

fn delete_service(state: &CloudMapState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["Id"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Id is required".to_string()))?;

    state
        .services
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("Service '{}' not found", id)))?;

    Ok(json_response(json!({})))
}

fn list_services(state: &CloudMapState) -> Result<Response, LawsError> {
    let items: Vec<Value> = state
        .services
        .iter()
        .map(|entry| {
            let svc = entry.value();
            json!({
                "Id": svc.id,
                "Name": svc.name,
                "Arn": svc.arn,
                "NamespaceId": svc.namespace_id,
                "CreateDate": svc.created,
            })
        })
        .collect();

    Ok(json_response(json!({ "Services": items })))
}

fn register_instance(state: &CloudMapState, payload: &Value) -> Result<Response, LawsError> {
    let service_id = payload["ServiceId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ServiceId is required".to_string()))?;

    let instance_id = payload["InstanceId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("InstanceId is required".to_string()))?
        .to_string();

    let attributes = payload["Attributes"].clone();

    let svc = state
        .services
        .get(service_id)
        .ok_or_else(|| LawsError::NotFound(format!("Service '{}' not found", service_id)))?;

    svc.instances.insert(instance_id, attributes);

    let operation_id = uuid::Uuid::new_v4().to_string();

    Ok(json_response(json!({
        "OperationId": operation_id,
    })))
}

fn deregister_instance(state: &CloudMapState, payload: &Value) -> Result<Response, LawsError> {
    let service_id = payload["ServiceId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ServiceId is required".to_string()))?;

    let instance_id = payload["InstanceId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("InstanceId is required".to_string()))?;

    let svc = state
        .services
        .get(service_id)
        .ok_or_else(|| LawsError::NotFound(format!("Service '{}' not found", service_id)))?;

    svc.instances
        .remove(instance_id)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Instance '{}' not found", instance_id))
        })?;

    let operation_id = uuid::Uuid::new_v4().to_string();

    Ok(json_response(json!({
        "OperationId": operation_id,
    })))
}
