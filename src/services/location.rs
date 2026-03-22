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
pub struct LocationMap {
    pub map_name: String,
    pub arn: String,
    pub description: String,
    pub configuration: Value,
    pub created: String,
}

#[derive(Debug, Clone)]
pub struct GeofenceCollection {
    pub collection_name: String,
    pub arn: String,
    pub description: String,
    pub created: String,
}

#[derive(Debug, Clone)]
pub struct Tracker {
    pub tracker_name: String,
    pub arn: String,
    pub description: String,
    pub created: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct LocationState {
    pub maps: DashMap<String, LocationMap>,
    pub geofence_collections: DashMap<String, GeofenceCollection>,
    pub trackers: DashMap<String, Tracker>,
}

impl Default for LocationState {
    fn default() -> Self {
        Self {
            maps: DashMap::new(),
            geofence_collections: DashMap::new(),
            trackers: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<LocationState>) -> axum::Router {
    axum::Router::new()
        .route("/maps/v0/maps", post(create_map))
        .route("/maps/v0/list-maps", post(list_maps))
        .route(
            "/maps/v0/maps/{name}",
            get(get_map).delete(delete_map),
        )
        .route(
            "/geofencing/v0/collections",
            post(create_geofence_collection),
        )
        .route(
            "/geofencing/v0/list-collections",
            post(list_geofence_collections),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_map(
    State(state): State<Arc<LocationState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let map_name = payload["MapName"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("MapName is required".into()))?
            .to_string();

        let description = payload["Description"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let configuration = payload["Configuration"]
            .clone();

        let arn = format!(
            "arn:aws:geo:{REGION}:{ACCOUNT_ID}:map/{map_name}"
        );
        let now = chrono::Utc::now().to_rfc3339();

        let map = LocationMap {
            map_name: map_name.clone(),
            arn: arn.clone(),
            description,
            configuration,
            created: now.clone(),
        };

        state.maps.insert(map_name.clone(), map);

        Ok(rest_json::created(json!({
            "MapName": map_name,
            "MapArn": arn,
            "CreateTime": now,
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_maps(State(state): State<Arc<LocationState>>) -> Response {
    let entries: Vec<Value> = state
        .maps
        .iter()
        .map(|entry| {
            let m = entry.value();
            json!({
                "MapName": m.map_name,
                "MapArn": m.arn,
                "Description": m.description,
                "CreateTime": m.created,
            })
        })
        .collect();

    rest_json::ok(json!({ "Entries": entries }))
}

async fn get_map(
    State(state): State<Arc<LocationState>>,
    Path(name): Path<String>,
) -> Response {
    match state.maps.get(&name) {
        Some(m) => rest_json::ok(json!({
            "MapName": m.map_name,
            "MapArn": m.arn,
            "Description": m.description,
            "Configuration": m.configuration,
            "CreateTime": m.created,
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Map '{}' not found",
            name
        ))),
    }
}

async fn delete_map(
    State(state): State<Arc<LocationState>>,
    Path(name): Path<String>,
) -> Response {
    match state.maps.remove(&name) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Map '{}' not found",
            name
        ))),
    }
}

async fn create_geofence_collection(
    State(state): State<Arc<LocationState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let collection_name = payload["CollectionName"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("CollectionName is required".into()))?
            .to_string();

        let description = payload["Description"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let arn = format!(
            "arn:aws:geo:{REGION}:{ACCOUNT_ID}:geofence-collection/{collection_name}"
        );
        let now = chrono::Utc::now().to_rfc3339();

        let collection = GeofenceCollection {
            collection_name: collection_name.clone(),
            arn: arn.clone(),
            description,
            created: now.clone(),
        };

        state
            .geofence_collections
            .insert(collection_name.clone(), collection);

        Ok(rest_json::created(json!({
            "CollectionName": collection_name,
            "CollectionArn": arn,
            "CreateTime": now,
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_geofence_collections(
    State(state): State<Arc<LocationState>>,
) -> Response {
    let entries: Vec<Value> = state
        .geofence_collections
        .iter()
        .map(|entry| {
            let c = entry.value();
            json!({
                "CollectionName": c.collection_name,
                "CollectionArn": c.arn,
                "Description": c.description,
                "CreateTime": c.created,
            })
        })
        .collect();

    rest_json::ok(json!({ "Entries": entries }))
}
