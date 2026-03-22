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
pub struct Registry {
    pub registry_name: String,
    pub registry_arn: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Schema {
    pub schema_name: String,
    pub schema_arn: String,
    pub registry_name: String,
    pub description: String,
    pub schema_version: String,
    pub schema_type: String,
    pub content: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct SchemasState {
    pub registries: DashMap<String, Registry>,
    pub schemas: DashMap<String, Schema>,
}

impl Default for SchemasState {
    fn default() -> Self {
        Self {
            registries: DashMap::new(),
            schemas: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<SchemasState>) -> axum::Router {
    axum::Router::new()
        .route("/v1/registries", axum::routing::get(list_registries))
        .route(
            "/v1/registries/{registry_name}",
            axum::routing::post(create_registry)
                .get(describe_registry)
                .delete(delete_registry),
        )
        .route(
            "/v1/registries/{registry_name}/schemas",
            axum::routing::get(list_schemas),
        )
        .route(
            "/v1/registries/{registry_name}/schemas/{schema_name}",
            axum::routing::post(create_schema).get(describe_schema),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn registry_to_json(r: &Registry) -> Value {
    json!({
        "RegistryName": r.registry_name,
        "RegistryArn": r.registry_arn,
        "Description": r.description,
        "Tags": {},
    })
}

fn schema_to_json(s: &Schema) -> Value {
    json!({
        "SchemaName": s.schema_name,
        "SchemaArn": s.schema_arn,
        "RegistryName": s.registry_name,
        "Description": s.description,
        "SchemaVersion": s.schema_version,
        "Type": s.schema_type,
        "Content": s.content,
        "LastModified": s.created_at,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_registry(
    State(state): State<Arc<SchemasState>>,
    Path(registry_name): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        if state.registries.contains_key(&registry_name) {
            return Err(LawsError::AlreadyExists(format!(
                "Registry '{}' already exists",
                registry_name
            )));
        }

        let description = payload["Description"].as_str().unwrap_or("").to_string();

        let registry_arn =
            format!("arn:aws:schemas:{REGION}:{ACCOUNT_ID}:registry/{registry_name}");
        let created_at = chrono::Utc::now().to_rfc3339();

        let registry = Registry {
            registry_name: registry_name.clone(),
            registry_arn,
            description,
            created_at,
        };

        let resp = registry_to_json(&registry);
        state.registries.insert(registry_name, registry);

        Ok(rest_json::created(resp))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_registries(State(state): State<Arc<SchemasState>>) -> Response {
    let registries: Vec<Value> = state
        .registries
        .iter()
        .map(|entry| registry_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "Registries": registries }))
}

async fn describe_registry(
    State(state): State<Arc<SchemasState>>,
    Path(registry_name): Path<String>,
) -> Response {
    match state.registries.get(&registry_name) {
        Some(r) => rest_json::ok(registry_to_json(r.value())),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Registry '{}' not found",
            registry_name
        ))),
    }
}

async fn delete_registry(
    State(state): State<Arc<SchemasState>>,
    Path(registry_name): Path<String>,
) -> Response {
    match state.registries.remove(&registry_name) {
        Some(_) => {
            state
                .schemas
                .retain(|_, s| s.registry_name != registry_name);
            rest_json::no_content()
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Registry '{}' not found",
            registry_name
        ))),
    }
}

async fn create_schema(
    State(state): State<Arc<SchemasState>>,
    Path((registry_name, schema_name)): Path<(String, String)>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        if !state.registries.contains_key(&registry_name) {
            return Err(LawsError::NotFound(format!(
                "Registry '{}' not found",
                registry_name
            )));
        }

        let key = format!("{}:{}", registry_name, schema_name);

        let description = payload["Description"].as_str().unwrap_or("").to_string();

        let schema_type = payload["Type"].as_str().unwrap_or("OpenApi3").to_string();

        let content = payload["Content"].as_str().unwrap_or("{}").to_string();

        let schema_arn =
            format!("arn:aws:schemas:{REGION}:{ACCOUNT_ID}:schema/{registry_name}/{schema_name}");
        let created_at = chrono::Utc::now().to_rfc3339();

        let schema = Schema {
            schema_name: schema_name.clone(),
            schema_arn,
            registry_name,
            description,
            schema_version: "1".to_string(),
            schema_type,
            content,
            created_at,
        };

        let resp = schema_to_json(&schema);
        state.schemas.insert(key, schema);

        Ok(rest_json::created(resp))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_schemas(
    State(state): State<Arc<SchemasState>>,
    Path(registry_name): Path<String>,
) -> Response {
    let schemas: Vec<Value> = state
        .schemas
        .iter()
        .filter(|entry| entry.value().registry_name == registry_name)
        .map(|entry| {
            let s = entry.value();
            json!({
                "SchemaName": s.schema_name,
                "SchemaArn": s.schema_arn,
                "SchemaVersion": s.schema_version,
            })
        })
        .collect();

    rest_json::ok(json!({ "Schemas": schemas }))
}

async fn describe_schema(
    State(state): State<Arc<SchemasState>>,
    Path((registry_name, schema_name)): Path<(String, String)>,
) -> Response {
    let key = format!("{}:{}", registry_name, schema_name);
    match state.schemas.get(&key) {
        Some(s) => rest_json::ok(schema_to_json(s.value())),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Schema '{}' not found in registry '{}'",
            schema_name, registry_name
        ))),
    }
}
