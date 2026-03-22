use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::Json;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub domain_name: String,
    pub default_expiration_days: i64,
    pub created_at: String,
    pub last_updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub profile_id: String,
    pub domain_name: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email_address: Option<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CustomerProfilesState {
    pub domains: DashMap<String, Domain>,
    pub profiles: DashMap<String, Profile>,
}

impl Default for CustomerProfilesState {
    fn default() -> Self {
        Self {
            domains: DashMap::new(),
            profiles: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<CustomerProfilesState>) -> axum::Router {
    axum::Router::new()
        .route("/domains", get(list_domains))
        .route(
            "/domains/{domain_name}",
            post(create_domain).get(get_domain).delete(delete_domain),
        )
        .route(
            "/domains/{domain_name}/profiles/objects",
            post(put_profile_object),
        )
        .route(
            "/domains/{domain_name}/profiles/search",
            post(search_profiles),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_domain(
    State(state): State<Arc<CustomerProfilesState>>,
    Path(domain_name): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    let now = Utc::now().to_rfc3339();
    let default_expiration_days = body["DefaultExpirationDays"].as_i64().unwrap_or(365);

    let domain = Domain {
        domain_name: domain_name.clone(),
        default_expiration_days,
        created_at: now.clone(),
        last_updated_at: now,
    };

    let resp = json!({
        "DomainName": domain.domain_name,
        "DefaultExpirationDays": domain.default_expiration_days,
        "CreatedAt": domain.created_at,
        "LastUpdatedAt": domain.last_updated_at,
    });

    state.domains.insert(domain_name, domain);
    rest_json::created(resp)
}

async fn list_domains(State(state): State<Arc<CustomerProfilesState>>) -> Response {
    let items: Vec<Value> = state
        .domains
        .iter()
        .map(|entry| {
            let d = entry.value();
            json!({
                "DomainName": d.domain_name,
                "CreatedAt": d.created_at,
                "LastUpdatedAt": d.last_updated_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "Items": items }))
}

async fn get_domain(
    State(state): State<Arc<CustomerProfilesState>>,
    Path(domain_name): Path<String>,
) -> Response {
    match state.domains.get(&domain_name) {
        Some(d) => rest_json::ok(json!({
            "DomainName": d.domain_name,
            "DefaultExpirationDays": d.default_expiration_days,
            "CreatedAt": d.created_at,
            "LastUpdatedAt": d.last_updated_at,
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Domain not found: {domain_name}"
        ))),
    }
}

async fn delete_domain(
    State(state): State<Arc<CustomerProfilesState>>,
    Path(domain_name): Path<String>,
) -> Response {
    match state.domains.remove(&domain_name) {
        Some(_) => {
            state.profiles.retain(|_, p| p.domain_name != domain_name);
            rest_json::ok(json!({ "Message": "Domain deleted" }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Domain not found: {domain_name}"
        ))),
    }
}

async fn put_profile_object(
    State(state): State<Arc<CustomerProfilesState>>,
    Path(domain_name): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    if !state.domains.contains_key(&domain_name) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "Domain not found: {domain_name}"
        )));
    }

    let profile_id = uuid::Uuid::new_v4().to_string();

    let profile = Profile {
        profile_id: profile_id.clone(),
        domain_name: domain_name.clone(),
        first_name: body["Object"].as_str().and_then(|s| {
            serde_json::from_str::<Value>(s)
                .ok()
                .and_then(|v| v["FirstName"].as_str().map(String::from))
        }),
        last_name: body["Object"].as_str().and_then(|s| {
            serde_json::from_str::<Value>(s)
                .ok()
                .and_then(|v| v["LastName"].as_str().map(String::from))
        }),
        email_address: body["Object"].as_str().and_then(|s| {
            serde_json::from_str::<Value>(s)
                .ok()
                .and_then(|v| v["EmailAddress"].as_str().map(String::from))
        }),
    };

    state.profiles.insert(profile_id.clone(), profile);
    rest_json::created(json!({ "ProfileObjectUniqueKey": profile_id }))
}

async fn search_profiles(
    State(state): State<Arc<CustomerProfilesState>>,
    Path(domain_name): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    if !state.domains.contains_key(&domain_name) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "Domain not found: {domain_name}"
        )));
    }

    let items: Vec<Value> = state
        .profiles
        .iter()
        .filter(|entry| entry.value().domain_name == domain_name)
        .map(|entry| {
            let p = entry.value();
            json!({
                "ProfileId": p.profile_id,
                "FirstName": p.first_name,
                "LastName": p.last_name,
                "EmailAddress": p.email_address,
            })
        })
        .collect();

    rest_json::ok(json!({ "Items": items }))
}
