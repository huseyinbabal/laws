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
pub struct GlobalNetwork {
    pub global_network_id: String,
    pub global_network_arn: String,
    pub description: String,
    pub state: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Site {
    pub site_id: String,
    pub site_arn: String,
    pub global_network_id: String,
    pub description: String,
    pub state: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Device {
    pub device_id: String,
    pub device_arn: String,
    pub global_network_id: String,
    pub site_id: String,
    pub description: String,
    pub state: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct NetworkManagerState {
    pub global_networks: DashMap<String, GlobalNetwork>,
    pub sites: DashMap<String, Site>,
    pub devices: DashMap<String, Device>,
}

impl Default for NetworkManagerState {
    fn default() -> Self {
        Self {
            global_networks: DashMap::new(),
            sites: DashMap::new(),
            devices: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<NetworkManagerState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/global-networks",
            axum::routing::post(create_global_network).get(get_global_networks),
        )
        .route(
            "/global-networks/{id}",
            axum::routing::delete(delete_global_network),
        )
        .route(
            "/global-networks/{id}/sites",
            axum::routing::post(create_site).get(get_sites),
        )
        .route(
            "/global-networks/{id}/devices",
            axum::routing::post(create_device).get(get_devices),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn random_id(prefix: &str) -> String {
    format!("{}-{}", prefix, &uuid::Uuid::new_v4().to_string()[..8])
}

fn global_network_to_json(gn: &GlobalNetwork) -> Value {
    json!({
        "GlobalNetworkId": gn.global_network_id,
        "GlobalNetworkArn": gn.global_network_arn,
        "Description": gn.description,
        "State": gn.state,
        "CreatedAt": gn.created_at,
    })
}

fn site_to_json(s: &Site) -> Value {
    json!({
        "SiteId": s.site_id,
        "SiteArn": s.site_arn,
        "GlobalNetworkId": s.global_network_id,
        "Description": s.description,
        "State": s.state,
        "CreatedAt": s.created_at,
    })
}

fn device_to_json(d: &Device) -> Value {
    json!({
        "DeviceId": d.device_id,
        "DeviceArn": d.device_arn,
        "GlobalNetworkId": d.global_network_id,
        "SiteId": d.site_id,
        "Description": d.description,
        "State": d.state,
        "CreatedAt": d.created_at,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_global_network(
    State(state): State<Arc<NetworkManagerState>>,
    Json(payload): Json<Value>,
) -> Response {
    let description = payload["Description"].as_str().unwrap_or("").to_string();

    let id = random_id("global-network");
    let arn = format!("arn:aws:networkmanager:{REGION}:{ACCOUNT_ID}:global-network/{id}");
    let created_at = chrono::Utc::now().to_rfc3339();

    let gn = GlobalNetwork {
        global_network_id: id.clone(),
        global_network_arn: arn,
        description,
        state: "AVAILABLE".to_string(),
        created_at,
    };

    let resp = global_network_to_json(&gn);
    state.global_networks.insert(id, gn);

    rest_json::created(json!({ "GlobalNetwork": resp }))
}

async fn get_global_networks(State(state): State<Arc<NetworkManagerState>>) -> Response {
    let networks: Vec<Value> = state
        .global_networks
        .iter()
        .map(|entry| global_network_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "GlobalNetworks": networks }))
}

async fn delete_global_network(
    State(state): State<Arc<NetworkManagerState>>,
    Path(id): Path<String>,
) -> Response {
    match state.global_networks.remove(&id) {
        Some((_, gn)) => {
            state.sites.retain(|_, s| s.global_network_id != id);
            state.devices.retain(|_, d| d.global_network_id != id);
            rest_json::ok(json!({ "GlobalNetwork": global_network_to_json(&gn) }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "GlobalNetwork '{}' not found",
            id
        ))),
    }
}

async fn create_site(
    State(state): State<Arc<NetworkManagerState>>,
    Path(global_network_id): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    if !state.global_networks.contains_key(&global_network_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "GlobalNetwork '{}' not found",
            global_network_id
        )));
    }

    let description = payload["Description"].as_str().unwrap_or("").to_string();

    let site_id = random_id("site");
    let site_arn =
        format!("arn:aws:networkmanager:{REGION}:{ACCOUNT_ID}:site/{global_network_id}/{site_id}");
    let created_at = chrono::Utc::now().to_rfc3339();

    let site = Site {
        site_id: site_id.clone(),
        site_arn,
        global_network_id,
        description,
        state: "AVAILABLE".to_string(),
        created_at,
    };

    let resp = site_to_json(&site);
    state.sites.insert(site_id, site);

    rest_json::created(json!({ "Site": resp }))
}

async fn get_sites(
    State(state): State<Arc<NetworkManagerState>>,
    Path(global_network_id): Path<String>,
) -> Response {
    let sites: Vec<Value> = state
        .sites
        .iter()
        .filter(|entry| entry.value().global_network_id == global_network_id)
        .map(|entry| site_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "Sites": sites }))
}

async fn create_device(
    State(state): State<Arc<NetworkManagerState>>,
    Path(global_network_id): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    if !state.global_networks.contains_key(&global_network_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "GlobalNetwork '{}' not found",
            global_network_id
        )));
    }

    let description = payload["Description"].as_str().unwrap_or("").to_string();

    let site_id = payload["SiteId"].as_str().unwrap_or("").to_string();

    let device_id = random_id("device");
    let device_arn = format!(
        "arn:aws:networkmanager:{REGION}:{ACCOUNT_ID}:device/{global_network_id}/{device_id}"
    );
    let created_at = chrono::Utc::now().to_rfc3339();

    let device = Device {
        device_id: device_id.clone(),
        device_arn,
        global_network_id,
        site_id,
        description,
        state: "AVAILABLE".to_string(),
        created_at,
    };

    let resp = device_to_json(&device);
    state.devices.insert(device_id, device);

    rest_json::created(json!({ "Device": resp }))
}

async fn get_devices(
    State(state): State<Arc<NetworkManagerState>>,
    Path(global_network_id): Path<String>,
) -> Response {
    let devices: Vec<Value> = state
        .devices
        .iter()
        .filter(|entry| entry.value().global_network_id == global_network_id)
        .map(|entry| device_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "Devices": devices }))
}
