use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post};
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
pub struct HttpApi {
    pub api_id: String,
    pub name: String,
    pub protocol_type: String,
    pub description: String,
    pub api_endpoint: String,
    pub created_date: String,
}

#[derive(Debug, Clone)]
pub struct Route {
    pub route_id: String,
    pub api_id: String,
    pub route_key: String,
    pub target: String,
    pub authorization_type: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ApiGatewayV2State {
    pub apis: DashMap<String, HttpApi>,
    pub routes: DashMap<String, Route>,
}

impl Default for ApiGatewayV2State {
    fn default() -> Self {
        Self {
            apis: DashMap::new(),
            routes: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<ApiGatewayV2State>) -> axum::Router {
    axum::Router::new()
        .route("/v2/apis", post(create_api).get(get_apis))
        .route(
            "/v2/apis/{api_id}",
            get(get_api).delete(delete_api),
        )
        .route(
            "/v2/apis/{api_id}/routes",
            post(create_route).get(get_routes),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn api_to_json(api: &HttpApi) -> Value {
    json!({
        "apiId": api.api_id,
        "name": api.name,
        "protocolType": api.protocol_type,
        "description": api.description,
        "apiEndpoint": api.api_endpoint,
        "createdDate": api.created_date,
    })
}

fn route_to_json(route: &Route) -> Value {
    json!({
        "routeId": route.route_id,
        "apiId": route.api_id,
        "routeKey": route.route_key,
        "target": route.target,
        "authorizationType": route.authorization_type,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateApiRequest {
    name: String,
    #[serde(rename = "protocolType", default)]
    protocol_type: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

async fn create_api(
    State(state): State<Arc<ApiGatewayV2State>>,
    Json(req): Json<CreateApiRequest>,
) -> Response {
    let api_id = uuid::Uuid::new_v4().to_string()[..10].to_string();
    let now = Utc::now().to_rfc3339();
    let protocol_type = req.protocol_type.unwrap_or_else(|| "HTTP".to_string());

    let api = HttpApi {
        api_id: api_id.clone(),
        name: req.name,
        protocol_type,
        description: req.description.unwrap_or_default(),
        api_endpoint: format!("https://{api_id}.execute-api.{REGION}.amazonaws.com"),
        created_date: now,
    };

    let resp = api_to_json(&api);
    state.apis.insert(api_id, api);

    rest_json::created(resp)
}

async fn get_apis(State(state): State<Arc<ApiGatewayV2State>>) -> Response {
    let items: Vec<Value> = state
        .apis
        .iter()
        .map(|entry| api_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "items": items }))
}

async fn get_api(
    State(state): State<Arc<ApiGatewayV2State>>,
    Path(api_id): Path<String>,
) -> Response {
    match state.apis.get(&api_id) {
        Some(api) => rest_json::ok(api_to_json(api.value())),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "API not found: {api_id}"
        ))),
    }
}

async fn delete_api(
    State(state): State<Arc<ApiGatewayV2State>>,
    Path(api_id): Path<String>,
) -> Response {
    match state.apis.remove(&api_id) {
        Some(_) => {
            state.routes.retain(|_, r| r.api_id != api_id);
            rest_json::no_content()
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "API not found: {api_id}"
        ))),
    }
}

#[derive(Deserialize)]
struct CreateRouteRequest {
    #[serde(rename = "routeKey")]
    route_key: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(rename = "authorizationType", default)]
    authorization_type: Option<String>,
}

async fn create_route(
    State(state): State<Arc<ApiGatewayV2State>>,
    Path(api_id): Path<String>,
    Json(req): Json<CreateRouteRequest>,
) -> Response {
    if !state.apis.contains_key(&api_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "API not found: {api_id}"
        )));
    }

    let route_id = uuid::Uuid::new_v4().to_string()[..10].to_string();

    let route = Route {
        route_id: route_id.clone(),
        api_id: api_id.clone(),
        route_key: req.route_key,
        target: req.target.unwrap_or_default(),
        authorization_type: req.authorization_type.unwrap_or_else(|| "NONE".to_string()),
    };

    let resp = route_to_json(&route);
    state.routes.insert(route_id, route);

    rest_json::created(resp)
}

async fn get_routes(
    State(state): State<Arc<ApiGatewayV2State>>,
    Path(api_id): Path<String>,
) -> Response {
    if !state.apis.contains_key(&api_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "API not found: {api_id}"
        )));
    }

    let items: Vec<Value> = state
        .routes
        .iter()
        .filter(|entry| entry.value().api_id == api_id)
        .map(|entry| route_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "items": items }))
}
