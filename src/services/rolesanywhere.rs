use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{delete, get, post};
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
pub struct TrustAnchor {
    pub trust_anchor_id: String,
    pub trust_anchor_arn: String,
    pub name: String,
    pub source_type: String,
    pub source_data: Value,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub profile_id: String,
    pub profile_arn: String,
    pub name: String,
    pub role_arns: Vec<String>,
    pub enabled: bool,
    pub duration_seconds: i64,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct RolesAnywhereState {
    pub trust_anchors: DashMap<String, TrustAnchor>,
    pub profiles: DashMap<String, Profile>,
}

impl Default for RolesAnywhereState {
    fn default() -> Self {
        Self {
            trust_anchors: DashMap::new(),
            profiles: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<RolesAnywhereState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/trustanchors",
            post(create_trust_anchor).get(list_trust_anchors),
        )
        .route(
            "/trustanchors/{trust_anchor_id}",
            get(get_trust_anchor).delete(delete_trust_anchor),
        )
        .route("/profiles", post(create_profile).get(list_profiles))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateTrustAnchorRequest {
    name: String,
    #[serde(default)]
    source: Option<TrustAnchorSourceInput>,
}

#[derive(Deserialize)]
struct TrustAnchorSourceInput {
    #[serde(default, rename = "sourceType")]
    source_type: Option<String>,
    #[serde(default, rename = "sourceData")]
    source_data: Option<Value>,
}

async fn create_trust_anchor(
    State(state): State<Arc<RolesAnywhereState>>,
    Json(req): Json<CreateTrustAnchorRequest>,
) -> Response {
    let trust_anchor_id = uuid::Uuid::new_v4().to_string();
    let trust_anchor_arn = format!(
        "arn:aws:rolesanywhere:{REGION}:{ACCOUNT_ID}:trust-anchor/{trust_anchor_id}"
    );
    let now = Utc::now().to_rfc3339();

    let (source_type, source_data) = match req.source {
        Some(s) => (
            s.source_type
                .unwrap_or_else(|| "AWS_ACM_PCA".into()),
            s.source_data.unwrap_or(Value::Null),
        ),
        None => ("AWS_ACM_PCA".to_string(), Value::Null),
    };

    let anchor = TrustAnchor {
        trust_anchor_id: trust_anchor_id.clone(),
        trust_anchor_arn: trust_anchor_arn.clone(),
        name: req.name.clone(),
        source_type: source_type.clone(),
        source_data: source_data.clone(),
        enabled: true,
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    state.trust_anchors.insert(trust_anchor_id.clone(), anchor);

    rest_json::created(json!({
        "trustAnchor": {
            "trustAnchorId": trust_anchor_id,
            "trustAnchorArn": trust_anchor_arn,
            "name": req.name,
            "source": {
                "sourceType": source_type,
                "sourceData": source_data,
            },
            "enabled": true,
            "createdAt": now,
        }
    }))
}

async fn list_trust_anchors(State(state): State<Arc<RolesAnywhereState>>) -> Response {
    let anchors: Vec<Value> = state
        .trust_anchors
        .iter()
        .map(|entry| {
            let a = entry.value();
            json!({
                "trustAnchorId": a.trust_anchor_id,
                "trustAnchorArn": a.trust_anchor_arn,
                "name": a.name,
                "enabled": a.enabled,
                "createdAt": a.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "trustAnchors": anchors }))
}

async fn get_trust_anchor(
    State(state): State<Arc<RolesAnywhereState>>,
    Path(trust_anchor_id): Path<String>,
) -> Response {
    match state.trust_anchors.get(&trust_anchor_id) {
        Some(a) => rest_json::ok(json!({
            "trustAnchor": {
                "trustAnchorId": a.trust_anchor_id,
                "trustAnchorArn": a.trust_anchor_arn,
                "name": a.name,
                "source": {
                    "sourceType": a.source_type,
                    "sourceData": a.source_data,
                },
                "enabled": a.enabled,
                "createdAt": a.created_at,
                "updatedAt": a.updated_at,
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "TrustAnchor not found: {trust_anchor_id}"
        ))),
    }
}

async fn delete_trust_anchor(
    State(state): State<Arc<RolesAnywhereState>>,
    Path(trust_anchor_id): Path<String>,
) -> Response {
    match state.trust_anchors.remove(&trust_anchor_id) {
        Some(_) => rest_json::ok(json!({
            "trustAnchor": { "trustAnchorId": trust_anchor_id }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "TrustAnchor not found: {trust_anchor_id}"
        ))),
    }
}

#[derive(Deserialize)]
struct CreateProfileRequest {
    name: String,
    #[serde(default, rename = "roleArns")]
    role_arns: Option<Vec<String>>,
    #[serde(default, rename = "durationSeconds")]
    duration_seconds: Option<i64>,
}

async fn create_profile(
    State(state): State<Arc<RolesAnywhereState>>,
    Json(req): Json<CreateProfileRequest>,
) -> Response {
    let profile_id = uuid::Uuid::new_v4().to_string();
    let profile_arn = format!(
        "arn:aws:rolesanywhere:{REGION}:{ACCOUNT_ID}:profile/{profile_id}"
    );
    let now = Utc::now().to_rfc3339();

    let profile = Profile {
        profile_id: profile_id.clone(),
        profile_arn: profile_arn.clone(),
        name: req.name.clone(),
        role_arns: req.role_arns.clone().unwrap_or_default(),
        enabled: true,
        duration_seconds: req.duration_seconds.unwrap_or(3600),
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    state.profiles.insert(profile_id.clone(), profile);

    rest_json::created(json!({
        "profile": {
            "profileId": profile_id,
            "profileArn": profile_arn,
            "name": req.name,
            "roleArns": req.role_arns.unwrap_or_default(),
            "enabled": true,
            "createdAt": now,
        }
    }))
}

async fn list_profiles(State(state): State<Arc<RolesAnywhereState>>) -> Response {
    let profiles: Vec<Value> = state
        .profiles
        .iter()
        .map(|entry| {
            let p = entry.value();
            json!({
                "profileId": p.profile_id,
                "profileArn": p.profile_arn,
                "name": p.name,
                "enabled": p.enabled,
                "createdAt": p.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "profiles": profiles }))
}
