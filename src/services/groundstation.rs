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
pub struct Satellite {
    pub satellite_id: String,
    pub satellite_arn: String,
    pub norad_satellite_id: u32,
    pub ground_stations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub config_id: String,
    pub config_arn: String,
    pub name: String,
    pub config_type: String,
    pub config_data: Value,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct GroundStationState {
    pub satellites: DashMap<String, Satellite>,
    pub configs: DashMap<String, Config>,
}

impl Default for GroundStationState {
    fn default() -> Self {
        Self {
            satellites: DashMap::new(),
            configs: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<GroundStationState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/satellite",
            axum::routing::get(list_satellites),
        )
        .route(
            "/satellite/{id}",
            axum::routing::get(get_satellite),
        )
        .route(
            "/config",
            axum::routing::post(create_config).get(list_configs),
        )
        .route(
            "/config/{config_type}/{id}",
            axum::routing::get(get_config).delete(delete_config),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn list_satellites(
    State(state): State<Arc<GroundStationState>>,
) -> Response {
    let satellites: Vec<Value> = state
        .satellites
        .iter()
        .map(|e| {
            let s = e.value();
            json!({
                "satelliteId": s.satellite_id,
                "satelliteArn": s.satellite_arn,
                "noradSatelliteID": s.norad_satellite_id,
                "groundStations": s.ground_stations
            })
        })
        .collect();

    rest_json::ok(json!({
        "satelliteList": satellites
    }))
}

async fn get_satellite(
    State(state): State<Arc<GroundStationState>>,
    Path(id): Path<String>,
) -> Response {
    match state.satellites.get(&id) {
        Some(s) => rest_json::ok(json!({
            "satelliteId": s.satellite_id,
            "satelliteArn": s.satellite_arn,
            "noradSatelliteID": s.norad_satellite_id,
            "groundStations": s.ground_stations
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Satellite not found: {id}"
        ))),
    }
}

async fn create_config(
    State(state): State<Arc<GroundStationState>>,
    Json(payload): Json<Value>,
) -> Response {
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();

    let config_type = payload
        .get("configType")
        .and_then(|v| v.as_str())
        .unwrap_or("tracking-config")
        .to_owned();

    let config_data = payload
        .get("configData")
        .cloned()
        .unwrap_or(json!({}));

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:groundstation:{REGION}:{ACCOUNT_ID}:config/{config_type}/{id}"
    );

    let config = Config {
        config_id: id.clone(),
        config_arn: arn.clone(),
        name: name.clone(),
        config_type: config_type.clone(),
        config_data,
    };

    state.configs.insert(id.clone(), config);

    rest_json::created(json!({
        "configId": id,
        "configArn": arn,
        "configType": config_type,
        "name": name
    }))
}

async fn list_configs(
    State(state): State<Arc<GroundStationState>>,
) -> Response {
    let configs: Vec<Value> = state
        .configs
        .iter()
        .map(|e| {
            let c = e.value();
            json!({
                "configId": c.config_id,
                "configArn": c.config_arn,
                "configType": c.config_type,
                "name": c.name
            })
        })
        .collect();

    rest_json::ok(json!({
        "configList": configs
    }))
}

async fn get_config(
    State(state): State<Arc<GroundStationState>>,
    Path((_config_type, id)): Path<(String, String)>,
) -> Response {
    match state.configs.get(&id) {
        Some(c) => rest_json::ok(json!({
            "configId": c.config_id,
            "configArn": c.config_arn,
            "configType": c.config_type,
            "name": c.name,
            "configData": c.config_data
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Config not found: {id}"
        ))),
    }
}

async fn delete_config(
    State(state): State<Arc<GroundStationState>>,
    Path((_config_type, id)): Path<(String, String)>,
) -> Response {
    match state.configs.remove(&id) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Config not found: {id}"
        ))),
    }
}
