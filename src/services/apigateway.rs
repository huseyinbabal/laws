use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, put};
use axum::Json;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::LawsError;
use crate::protocol::rest_json;

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize, Deserialize)]
pub struct RestApi {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_date: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ApiResource {
    pub id: String,
    pub api_id: String,
    pub parent_id: Option<String>,
    pub path_part: String,
    pub path: String,
    pub methods: HashMap<String, ApiMethod>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ApiMethod {
    pub http_method: String,
    pub authorization_type: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ApiDeployment {
    pub id: String,
    pub api_id: String,
    pub created_date: String,
    pub description: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ApiStage {
    pub stage_name: String,
    pub api_id: String,
    pub deployment_id: String,
    pub created_date: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ApiGatewayState {
    pub apis: DashMap<String, RestApi>,
    pub resources: DashMap<String, ApiResource>,
    pub deployments: DashMap<String, ApiDeployment>,
    pub stages: DashMap<String, ApiStage>,
}

impl Default for ApiGatewayState {
    fn default() -> Self {
        Self {
            apis: DashMap::new(),
            resources: DashMap::new(),
            deployments: DashMap::new(),
            stages: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<ApiGatewayState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/restapis",
            axum::routing::post(create_rest_api).get(get_rest_apis),
        )
        .route(
            "/restapis/{api_id}",
            get(get_rest_api).delete(delete_rest_api),
        )
        .route(
            "/restapis/{api_id}/resources",
            axum::routing::post(create_resource).get(get_resources),
        )
        .route(
            "/restapis/{api_id}/resources/{resource_id}/methods/{method}",
            put(put_method),
        )
        .route(
            "/restapis/{api_id}/deployments",
            axum::routing::post(create_deployment).get(get_deployments),
        )
        .route(
            "/restapis/{api_id}/stages",
            axum::routing::post(create_stage).get(get_stages),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn random_id(len: usize) -> String {
    use rand::RngExt;
    rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(len)
        .map(char::from)
        .collect::<String>()
}

fn rest_api_to_json(api: &RestApi) -> Value {
    json!({
        "id": api.id,
        "name": api.name,
        "description": api.description,
        "createdDate": api.created_date,
    })
}

fn resource_to_json(res: &ApiResource) -> Value {
    let mut methods_json = serde_json::Map::new();
    for (k, m) in &res.methods {
        methods_json.insert(
            k.clone(),
            json!({
                "httpMethod": m.http_method,
                "authorizationType": m.authorization_type,
            }),
        );
    }
    json!({
        "id": res.id,
        "parentId": res.parent_id,
        "pathPart": res.path_part,
        "path": res.path,
        "resourceMethods": methods_json,
    })
}

fn deployment_to_json(dep: &ApiDeployment) -> Value {
    json!({
        "id": dep.id,
        "description": dep.description,
        "createdDate": dep.created_date,
    })
}

fn stage_to_json(stage: &ApiStage) -> Value {
    json!({
        "stageName": stage.stage_name,
        "deploymentId": stage.deployment_id,
        "createdDate": stage.created_date,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateRestApiRequest {
    name: String,
    #[serde(default)]
    description: Option<String>,
}

async fn create_rest_api(
    State(state): State<Arc<ApiGatewayState>>,
    Json(req): Json<CreateRestApiRequest>,
) -> Response {
    let api_id = random_id(10);
    let now = Utc::now().to_rfc3339();

    let api = RestApi {
        id: api_id.clone(),
        name: req.name,
        description: req.description.unwrap_or_default(),
        created_date: now.clone(),
    };

    // Create root resource automatically.
    let root_id = random_id(10);
    let root_resource = ApiResource {
        id: root_id.clone(),
        api_id: api_id.clone(),
        parent_id: None,
        path_part: String::new(),
        path: "/".to_string(),
        methods: HashMap::new(),
    };

    let resp = rest_api_to_json(&api);
    state.apis.insert(api_id.clone(), api);
    state.resources.insert(root_id, root_resource);

    rest_json::created(resp)
}

async fn get_rest_apis(State(state): State<Arc<ApiGatewayState>>) -> Response {
    let items: Vec<Value> = state
        .apis
        .iter()
        .map(|entry| rest_api_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "item": items }))
}

async fn get_rest_api(
    State(state): State<Arc<ApiGatewayState>>,
    Path(api_id): Path<String>,
) -> Response {
    match state.apis.get(&api_id) {
        Some(api) => rest_json::ok(rest_api_to_json(&api)),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "REST API not found: {api_id}"
        ))),
    }
}

async fn delete_rest_api(
    State(state): State<Arc<ApiGatewayState>>,
    Path(api_id): Path<String>,
) -> Response {
    match state.apis.remove(&api_id) {
        Some(_) => {
            // Clean up associated resources, deployments, and stages.
            state.resources.retain(|_, r| r.api_id != api_id);
            state.deployments.retain(|_, d| d.api_id != api_id);
            state.stages.retain(|_, s| s.api_id != api_id);
            rest_json::no_content()
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "REST API not found: {api_id}"
        ))),
    }
}

#[derive(Deserialize)]
struct CreateResourceRequest {
    #[serde(rename = "pathPart")]
    path_part: String,
    #[serde(rename = "parentId", default)]
    parent_id: Option<String>,
}

async fn create_resource(
    State(state): State<Arc<ApiGatewayState>>,
    Path(api_id): Path<String>,
    Json(req): Json<CreateResourceRequest>,
) -> Response {
    if !state.apis.contains_key(&api_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "REST API not found: {api_id}"
        )));
    }

    // Resolve parent path.
    let parent_path = if let Some(ref pid) = req.parent_id {
        match state.resources.get(pid) {
            Some(parent) => parent.path.clone(),
            None => {
                return rest_json::error_response(&LawsError::NotFound(format!(
                    "Parent resource not found: {pid}"
                )));
            }
        }
    } else {
        // Find root resource for this API.
        let root = state
            .resources
            .iter()
            .find(|r| r.api_id == api_id && r.path == "/");
        match root {
            Some(r) => r.path.clone(),
            None => "/".to_string(),
        }
    };

    let path = if parent_path == "/" {
        format!("/{}", req.path_part)
    } else {
        format!("{}/{}", parent_path, req.path_part)
    };

    let resource_id = random_id(10);
    let resource = ApiResource {
        id: resource_id.clone(),
        api_id,
        parent_id: req.parent_id,
        path_part: req.path_part,
        path,
        methods: HashMap::new(),
    };

    let resp = resource_to_json(&resource);
    state.resources.insert(resource_id, resource);

    rest_json::created(resp)
}

async fn get_resources(
    State(state): State<Arc<ApiGatewayState>>,
    Path(api_id): Path<String>,
) -> Response {
    if !state.apis.contains_key(&api_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "REST API not found: {api_id}"
        )));
    }

    let items: Vec<Value> = state
        .resources
        .iter()
        .filter(|r| r.api_id == api_id)
        .map(|r| resource_to_json(r.value()))
        .collect();

    rest_json::ok(json!({ "item": items }))
}

#[derive(Deserialize)]
struct PutMethodRequest {
    #[serde(rename = "authorizationType", default)]
    authorization_type: Option<String>,
}

async fn put_method(
    State(state): State<Arc<ApiGatewayState>>,
    Path((api_id, resource_id, method)): Path<(String, String, String)>,
    Json(req): Json<PutMethodRequest>,
) -> Response {
    if !state.apis.contains_key(&api_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "REST API not found: {api_id}"
        )));
    }

    let mut resource = match state.resources.get_mut(&resource_id) {
        Some(r) => r,
        None => {
            return rest_json::error_response(&LawsError::NotFound(format!(
                "Resource not found: {resource_id}"
            )));
        }
    };

    let http_method = method.to_uppercase();
    let auth_type = req.authorization_type.unwrap_or_else(|| "NONE".to_string());

    let api_method = ApiMethod {
        http_method: http_method.clone(),
        authorization_type: auth_type.clone(),
    };

    resource.methods.insert(http_method.clone(), api_method);

    rest_json::ok(json!({
        "httpMethod": http_method,
        "authorizationType": auth_type,
    }))
}

#[derive(Deserialize)]
struct CreateDeploymentRequest {
    #[serde(default)]
    description: Option<String>,
}

async fn create_deployment(
    State(state): State<Arc<ApiGatewayState>>,
    Path(api_id): Path<String>,
    Json(req): Json<CreateDeploymentRequest>,
) -> Response {
    if !state.apis.contains_key(&api_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "REST API not found: {api_id}"
        )));
    }

    let deployment_id = random_id(10);
    let deployment = ApiDeployment {
        id: deployment_id.clone(),
        api_id,
        created_date: Utc::now().to_rfc3339(),
        description: req.description.unwrap_or_default(),
    };

    let resp = deployment_to_json(&deployment);
    state.deployments.insert(deployment_id, deployment);

    rest_json::created(resp)
}

async fn get_deployments(
    State(state): State<Arc<ApiGatewayState>>,
    Path(api_id): Path<String>,
) -> Response {
    if !state.apis.contains_key(&api_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "REST API not found: {api_id}"
        )));
    }

    let items: Vec<Value> = state
        .deployments
        .iter()
        .filter(|d| d.api_id == api_id)
        .map(|d| deployment_to_json(d.value()))
        .collect();

    rest_json::ok(json!({ "item": items }))
}

#[derive(Deserialize)]
struct CreateStageRequest {
    #[serde(rename = "stageName")]
    stage_name: String,
    #[serde(rename = "deploymentId")]
    deployment_id: String,
}

async fn create_stage(
    State(state): State<Arc<ApiGatewayState>>,
    Path(api_id): Path<String>,
    Json(req): Json<CreateStageRequest>,
) -> Response {
    if !state.apis.contains_key(&api_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "REST API not found: {api_id}"
        )));
    }

    if !state.deployments.contains_key(&req.deployment_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "Deployment not found: {}",
            req.deployment_id
        )));
    }

    let stage_key = format!("{}:{}", api_id, req.stage_name);
    if state.stages.contains_key(&stage_key) {
        return rest_json::error_response(&LawsError::AlreadyExists(format!(
            "Stage already exists: {}",
            req.stage_name
        )));
    }

    let stage = ApiStage {
        stage_name: req.stage_name,
        api_id,
        deployment_id: req.deployment_id,
        created_date: Utc::now().to_rfc3339(),
    };

    let resp = stage_to_json(&stage);
    state.stages.insert(stage_key, stage);

    rest_json::created(resp)
}

async fn get_stages(
    State(state): State<Arc<ApiGatewayState>>,
    Path(api_id): Path<String>,
) -> Response {
    if !state.apis.contains_key(&api_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "REST API not found: {api_id}"
        )));
    }

    let items: Vec<Value> = state
        .stages
        .iter()
        .filter(|s| s.api_id == api_id)
        .map(|s| stage_to_json(s.value()))
        .collect();

    rest_json::ok(json!({ "item": items }))
}
