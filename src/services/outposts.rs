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
pub struct Outpost {
    pub outpost_id: String,
    pub outpost_arn: String,
    pub name: String,
    pub description: String,
    pub site_id: String,
    pub availability_zone: String,
    pub life_cycle_status: String,
    pub owner_id: String,
}

#[derive(Debug, Clone)]
pub struct OutpostSite {
    pub site_id: String,
    pub site_arn: String,
    pub name: String,
    pub description: String,
    pub account_id: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct OutpostsState {
    pub outposts: DashMap<String, Outpost>,
    pub sites: DashMap<String, OutpostSite>,
}

impl Default for OutpostsState {
    fn default() -> Self {
        Self {
            outposts: DashMap::new(),
            sites: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<OutpostsState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/outposts",
            axum::routing::post(create_outpost).get(list_outposts),
        )
        .route(
            "/outposts/{id}",
            axum::routing::get(get_outpost).delete(delete_outpost),
        )
        .route(
            "/sites",
            axum::routing::post(create_site).get(list_sites),
        )
        .route(
            "/sites/{id}",
            axum::routing::get(get_site),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn random_id(prefix: &str) -> String {
    format!("{}-{}", prefix, &uuid::Uuid::new_v4().to_string()[..12])
}

fn outpost_to_json(o: &Outpost) -> Value {
    json!({
        "OutpostId": o.outpost_id,
        "OutpostArn": o.outpost_arn,
        "Name": o.name,
        "Description": o.description,
        "SiteId": o.site_id,
        "AvailabilityZone": o.availability_zone,
        "LifeCycleStatus": o.life_cycle_status,
        "OwnerId": o.owner_id,
    })
}

fn site_to_json(s: &OutpostSite) -> Value {
    json!({
        "SiteId": s.site_id,
        "SiteArn": s.site_arn,
        "Name": s.name,
        "Description": s.description,
        "AccountId": s.account_id,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_outpost(
    State(state): State<Arc<OutpostsState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let name = payload["Name"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
            .to_string();

        let description = payload["Description"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let site_id = payload["SiteId"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let availability_zone = payload["AvailabilityZone"]
            .as_str()
            .unwrap_or(&format!("{REGION}a"))
            .to_string();

        let outpost_id = random_id("op");
        let outpost_arn = format!(
            "arn:aws:outposts:{REGION}:{ACCOUNT_ID}:outpost/{outpost_id}"
        );

        let outpost = Outpost {
            outpost_id: outpost_id.clone(),
            outpost_arn,
            name,
            description,
            site_id,
            availability_zone,
            life_cycle_status: "ACTIVE".to_string(),
            owner_id: ACCOUNT_ID.to_string(),
        };

        let resp = outpost_to_json(&outpost);
        state.outposts.insert(outpost_id, outpost);

        Ok(rest_json::created(json!({ "Outpost": resp })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_outposts(
    State(state): State<Arc<OutpostsState>>,
) -> Response {
    let outposts: Vec<Value> = state
        .outposts
        .iter()
        .map(|entry| outpost_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "Outposts": outposts }))
}

async fn get_outpost(
    State(state): State<Arc<OutpostsState>>,
    Path(id): Path<String>,
) -> Response {
    match state.outposts.get(&id) {
        Some(o) => rest_json::ok(json!({ "Outpost": outpost_to_json(o.value()) })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Outpost '{}' not found",
            id
        ))),
    }
}

async fn delete_outpost(
    State(state): State<Arc<OutpostsState>>,
    Path(id): Path<String>,
) -> Response {
    match state.outposts.remove(&id) {
        Some(_) => rest_json::ok(json!({})),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Outpost '{}' not found",
            id
        ))),
    }
}

async fn create_site(
    State(state): State<Arc<OutpostsState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let name = payload["Name"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
            .to_string();

        let description = payload["Description"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let site_id = random_id("os");
        let site_arn = format!(
            "arn:aws:outposts:{REGION}:{ACCOUNT_ID}:site/{site_id}"
        );

        let site = OutpostSite {
            site_id: site_id.clone(),
            site_arn,
            name,
            description,
            account_id: ACCOUNT_ID.to_string(),
        };

        let resp = site_to_json(&site);
        state.sites.insert(site_id, site);

        Ok(rest_json::created(json!({ "Site": resp })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_sites(
    State(state): State<Arc<OutpostsState>>,
) -> Response {
    let sites: Vec<Value> = state
        .sites
        .iter()
        .map(|entry| site_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "Sites": sites }))
}

async fn get_site(
    State(state): State<Arc<OutpostsState>>,
    Path(id): Path<String>,
) -> Response {
    match state.sites.get(&id) {
        Some(s) => rest_json::ok(json!({ "Site": site_to_json(s.value()) })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Site '{}' not found",
            id
        ))),
    }
}
