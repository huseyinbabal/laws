use std::sync::atomic::{AtomicU64, Ordering};

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
pub struct EmailIdentity {
    pub identity: String,
    pub identity_type: String,
    pub verified: bool,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct SesState {
    pub identities: DashMap<String, EmailIdentity>,
    pub sent_count: AtomicU64,
}

impl Default for SesState {
    fn default() -> Self {
        Self {
            identities: DashMap::new(),
            sent_count: AtomicU64::new(0),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &SesState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("SimpleEmailServiceV2.")
        .unwrap_or(target);

    let result = match action {
        "SendEmail" => send_email(state, payload),
        "CreateEmailIdentity" => create_email_identity(state, payload),
        "DeleteEmailIdentity" => delete_email_identity(state, payload),
        "ListEmailIdentities" => list_email_identities(state),
        "GetEmailIdentity" => get_email_identity(state, payload),
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

fn determine_identity_type(identity: &str) -> &'static str {
    if identity.contains('@') {
        "EMAIL_ADDRESS"
    } else {
        "DOMAIN"
    }
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn send_email(state: &SesState, _payload: &Value) -> Result<Response, LawsError> {
    state.sent_count.fetch_add(1, Ordering::SeqCst);

    let message_id = uuid::Uuid::new_v4().to_string();

    Ok(json_response(json!({
        "MessageId": message_id
    })))
}

fn create_email_identity(state: &SesState, payload: &Value) -> Result<Response, LawsError> {
    let identity = payload
        .get("EmailIdentity")
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest("missing required field: EmailIdentity".into()))?
        .to_owned();

    if state.identities.contains_key(&identity) {
        return Err(LawsError::AlreadyExists(format!(
            "email identity already exists: {identity}"
        )));
    }

    let identity_type = determine_identity_type(&identity);

    let email_identity = EmailIdentity {
        identity: identity.clone(),
        identity_type: identity_type.to_owned(),
        verified: true,
    };

    state.identities.insert(identity, email_identity);

    Ok(json_response(json!({
        "IdentityType": identity_type,
        "VerifiedForSendingStatus": true,
        "DkimAttributes": {
            "SigningEnabled": true,
            "Status": "SUCCESS"
        }
    })))
}

fn delete_email_identity(state: &SesState, payload: &Value) -> Result<Response, LawsError> {
    let identity = payload
        .get("EmailIdentity")
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest("missing required field: EmailIdentity".into()))?;

    state
        .identities
        .remove(identity)
        .ok_or_else(|| LawsError::NotFound(format!("email identity not found: {identity}")))?;

    Ok(json_response(json!({})))
}

fn list_email_identities(state: &SesState) -> Result<Response, LawsError> {
    let identities: Vec<Value> = state
        .identities
        .iter()
        .map(|entry| {
            let id = entry.value();
            json!({
                "IdentityType": id.identity_type,
                "IdentityName": id.identity,
                "SendingEnabled": id.verified
            })
        })
        .collect();

    Ok(json_response(json!({
        "EmailIdentities": identities
    })))
}

fn get_email_identity(state: &SesState, payload: &Value) -> Result<Response, LawsError> {
    let identity = payload
        .get("EmailIdentity")
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest("missing required field: EmailIdentity".into()))?;

    let id = state
        .identities
        .get(identity)
        .ok_or_else(|| LawsError::NotFound(format!("email identity not found: {identity}")))?;

    Ok(json_response(json!({
        "IdentityType": id.identity_type,
        "VerifiedForSendingStatus": id.verified,
        "DkimAttributes": {
            "SigningEnabled": true,
            "Status": "SUCCESS"
        },
        "FeedbackForwardingStatus": true,
        "Policies": {}
    })))
}
