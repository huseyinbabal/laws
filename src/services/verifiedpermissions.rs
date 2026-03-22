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
pub struct PolicyStore {
    pub policy_store_id: String,
    pub arn: String,
    pub created_date: String,
    pub validation_settings: String,
}

#[derive(Debug, Clone)]
pub struct Policy {
    pub policy_id: String,
    pub policy_store_id: String,
    pub policy_type: String,
    pub definition: Value,
    pub created_date: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct VerifiedPermissionsState {
    pub policy_stores: DashMap<String, PolicyStore>,
    pub policies: DashMap<String, Policy>,
}

impl Default for VerifiedPermissionsState {
    fn default() -> Self {
        Self {
            policy_stores: DashMap::new(),
            policies: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &VerifiedPermissionsState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("VerifiedPermissions.")
        .unwrap_or(target);

    let result = match action {
        "CreatePolicyStore" => create_policy_store(state, payload),
        "GetPolicyStore" => get_policy_store(state, payload),
        "ListPolicyStores" => list_policy_stores(state),
        "DeletePolicyStore" => delete_policy_store(state, payload),
        "CreatePolicy" => create_policy(state, payload),
        "ListPolicies" => list_policies(state, payload),
        "IsAuthorized" => is_authorized(state, payload),
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

fn json_response(status: StatusCode, body: Value) -> Response {
    (status, axum::Json(body)).into_response()
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_policy_store(
    state: &VerifiedPermissionsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let policy_store_id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:verifiedpermissions:{REGION}:{ACCOUNT_ID}:policy-store/{policy_store_id}"
    );
    let now = Utc::now().to_rfc3339();
    let validation_settings = payload["validationSettings"]["mode"]
        .as_str()
        .unwrap_or("OFF")
        .to_string();

    let store = PolicyStore {
        policy_store_id: policy_store_id.clone(),
        arn: arn.clone(),
        created_date: now.clone(),
        validation_settings,
    };

    state.policy_stores.insert(policy_store_id.clone(), store);

    Ok(json_response(
        StatusCode::OK,
        json!({
            "policyStoreId": policy_store_id,
            "arn": arn,
            "createdDate": now,
        }),
    ))
}

fn get_policy_store(
    state: &VerifiedPermissionsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let policy_store_id = payload["policyStoreId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing policyStoreId".into()))?;

    let store = state
        .policy_stores
        .get(policy_store_id)
        .ok_or_else(|| {
            LawsError::NotFound(format!("PolicyStore not found: {policy_store_id}"))
        })?;

    Ok(json_response(
        StatusCode::OK,
        json!({
            "policyStoreId": store.policy_store_id,
            "arn": store.arn,
            "createdDate": store.created_date,
            "validationSettings": { "mode": store.validation_settings },
        }),
    ))
}

fn list_policy_stores(state: &VerifiedPermissionsState) -> Result<Response, LawsError> {
    let stores: Vec<Value> = state
        .policy_stores
        .iter()
        .map(|entry| {
            let s = entry.value();
            json!({
                "policyStoreId": s.policy_store_id,
                "arn": s.arn,
                "createdDate": s.created_date,
            })
        })
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "policyStores": stores }),
    ))
}

fn delete_policy_store(
    state: &VerifiedPermissionsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let policy_store_id = payload["policyStoreId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing policyStoreId".into()))?;

    state
        .policy_stores
        .remove(policy_store_id)
        .ok_or_else(|| {
            LawsError::NotFound(format!("PolicyStore not found: {policy_store_id}"))
        })?;

    Ok(json_response(StatusCode::OK, json!({})))
}

fn create_policy(
    state: &VerifiedPermissionsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let policy_store_id = payload["policyStoreId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing policyStoreId".into()))?
        .to_string();

    if !state.policy_stores.contains_key(&policy_store_id) {
        return Err(LawsError::NotFound(format!(
            "PolicyStore not found: {policy_store_id}"
        )));
    }

    let policy_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let policy_type = payload["definition"]["static"]
        .as_object()
        .map(|_| "STATIC")
        .unwrap_or("TEMPLATE_LINKED")
        .to_string();
    let definition = payload["definition"].clone();

    let policy = Policy {
        policy_id: policy_id.clone(),
        policy_store_id: policy_store_id.clone(),
        policy_type: policy_type.clone(),
        definition,
        created_date: now.clone(),
    };

    state.policies.insert(policy_id.clone(), policy);

    Ok(json_response(
        StatusCode::OK,
        json!({
            "policyStoreId": policy_store_id,
            "policyId": policy_id,
            "policyType": policy_type,
            "createdDate": now,
        }),
    ))
}

fn list_policies(
    state: &VerifiedPermissionsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let policy_store_id = payload["policyStoreId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing policyStoreId".into()))?;

    let policies: Vec<Value> = state
        .policies
        .iter()
        .filter(|entry| entry.value().policy_store_id == policy_store_id)
        .map(|entry| {
            let p = entry.value();
            json!({
                "policyStoreId": p.policy_store_id,
                "policyId": p.policy_id,
                "policyType": p.policy_type,
                "createdDate": p.created_date,
            })
        })
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "policies": policies }),
    ))
}

fn is_authorized(
    state: &VerifiedPermissionsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let policy_store_id = payload["policyStoreId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing policyStoreId".into()))?;

    if !state.policy_stores.contains_key(policy_store_id) {
        return Err(LawsError::NotFound(format!(
            "PolicyStore not found: {policy_store_id}"
        )));
    }

    // Simple mock: always allow
    Ok(json_response(
        StatusCode::OK,
        json!({
            "decision": "ALLOW",
            "determiningPolicies": [],
            "errors": [],
        }),
    ))
}
