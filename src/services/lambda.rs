use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::error::LawsError;
use crate::protocol::rest_json;
use crate::storage::mem::MemoryStore;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize, Deserialize)]
pub struct LambdaFunction {
    pub function_name: String,
    pub function_arn: String,
    pub runtime: String,
    pub handler: String,
    pub role: String,
    pub description: String,
    pub timeout: u32,
    pub memory_size: u32,
    pub code_sha256: String,
    pub code_size: u64,
    pub last_modified: String,
    pub state: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct LambdaState {
    pub functions: MemoryStore<LambdaFunction>,
}

impl Default for LambdaState {
    fn default() -> Self {
        Self {
            functions: MemoryStore::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<LambdaState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/2015-03-31/functions",
            post(create_function).get(list_functions),
        )
        .route(
            "/2015-03-31/functions/{name}",
            get(get_function).delete(delete_function),
        )
        .route(
            "/2015-03-31/functions/{name}/invocations",
            post(invoke_function),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateFunctionRequest {
    #[serde(rename = "FunctionName")]
    function_name: String,
    #[serde(rename = "Runtime", default)]
    runtime: Option<String>,
    #[serde(rename = "Handler", default)]
    handler: Option<String>,
    #[serde(rename = "Role", default)]
    role: Option<String>,
    #[serde(rename = "Description", default)]
    description: Option<String>,
    #[serde(rename = "Timeout", default)]
    timeout: Option<u32>,
    #[serde(rename = "MemorySize", default)]
    memory_size: Option<u32>,
    #[serde(rename = "Code", default)]
    code: Option<Value>,
}

async fn create_function(
    State(state): State<Arc<LambdaState>>,
    Json(req): Json<CreateFunctionRequest>,
) -> Response {
    match do_create_function(&state, req) {
        Ok(v) => rest_json::created(v),
        Err(e) => rest_json::error_response(&e),
    }
}

fn do_create_function(state: &LambdaState, req: CreateFunctionRequest) -> Result<Value, LawsError> {
    if state.functions.contains(&req.function_name) {
        return Err(LawsError::AlreadyExists(format!(
            "function already exists: {}",
            req.function_name
        )));
    }

    let arn = format!(
        "arn:aws:lambda:{REGION}:{ACCOUNT_ID}:function:{}",
        req.function_name
    );

    // Generate a fake code SHA-256.
    let mut hasher = Sha256::new();
    hasher.update(req.function_name.as_bytes());
    let code_sha256 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        hasher.finalize(),
    );

    let func = LambdaFunction {
        function_name: req.function_name.clone(),
        function_arn: arn,
        runtime: req.runtime.unwrap_or_else(|| "python3.12".into()),
        handler: req.handler.unwrap_or_else(|| "index.handler".into()),
        role: req
            .role
            .unwrap_or_else(|| format!("arn:aws:iam::{ACCOUNT_ID}:role/lambda-role")),
        description: req.description.unwrap_or_default(),
        timeout: req.timeout.unwrap_or(3),
        memory_size: req.memory_size.unwrap_or(128),
        code_sha256,
        code_size: 0,
        last_modified: Utc::now().to_rfc3339(),
        state: "Active".into(),
    };

    let resp = function_to_json(&func);
    state.functions.insert(req.function_name, func);
    Ok(resp)
}

async fn list_functions(State(state): State<Arc<LambdaState>>) -> Response {
    let funcs: Vec<Value> = state
        .functions
        .list_values()
        .iter()
        .map(function_to_json)
        .collect();

    rest_json::ok(json!({ "Functions": funcs }))
}

async fn get_function(State(state): State<Arc<LambdaState>>, Path(name): Path<String>) -> Response {
    match state.functions.get(&name) {
        Some(func) => rest_json::ok(json!({
            "Configuration": function_to_json(&func),
            "Code": {
                "Location": format!("https://awslambda-{REGION}-tasks.s3.{REGION}.amazonaws.com/snapshots/{ACCOUNT_ID}/{}", name),
            },
        })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("function not found: {name}")))
        }
    }
}

async fn delete_function(
    State(state): State<Arc<LambdaState>>,
    Path(name): Path<String>,
) -> Response {
    match state.functions.remove(&name) {
        Some(_) => rest_json::no_content(),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("function not found: {name}")))
        }
    }
}

async fn invoke_function(
    State(state): State<Arc<LambdaState>>,
    Path(name): Path<String>,
) -> Response {
    if !state.functions.contains(&name) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "function not found: {name}"
        )));
    }

    // Mock invocation response.
    rest_json::ok(json!({
        "statusCode": 200,
        "body": "{}",
    }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn function_to_json(func: &LambdaFunction) -> Value {
    json!({
        "FunctionName": func.function_name,
        "FunctionArn": func.function_arn,
        "Runtime": func.runtime,
        "Handler": func.handler,
        "Role": func.role,
        "Description": func.description,
        "Timeout": func.timeout,
        "MemorySize": func.memory_size,
        "CodeSha256": func.code_sha256,
        "CodeSize": func.code_size,
        "LastModified": func.last_modified,
        "State": func.state,
    })
}
