use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post};
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
pub struct GraphqlApi {
    pub api_id: String,
    pub name: String,
    pub arn: String,
    pub uris: HashMap<String, String>,
    pub authentication_type: String,
    pub created: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct AppSyncState {
    pub apis: DashMap<String, GraphqlApi>,
}

impl Default for AppSyncState {
    fn default() -> Self {
        Self {
            apis: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<AppSyncState>) -> axum::Router {
    axum::Router::new()
        .route("/v1/apis", post(create_graphql_api).get(list_graphql_apis))
        .route(
            "/v1/apis/{api_id}",
            get(get_graphql_api).delete(delete_graphql_api),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn random_id(len: usize) -> String {
    use rand::Rng;
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(len)
        .map(char::from)
        .collect::<String>()
        .to_lowercase()
}

fn api_to_json(api: &GraphqlApi) -> Value {
    json!({
        "apiId": api.api_id,
        "name": api.name,
        "arn": api.arn,
        "uris": api.uris,
        "authenticationType": api.authentication_type,
        "created": api.created,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_graphql_api(
    State(state): State<Arc<AppSyncState>>,
    Json(payload): Json<Value>,
) -> Response {
    let name = match payload["name"].as_str() {
        Some(n) => n.to_string(),
        None => {
            return rest_json::error_response(&LawsError::InvalidRequest(
                "name is required".to_string(),
            ));
        }
    };

    let api_id = random_id(26);
    let arn = format!(
        "arn:aws:appsync:{}:{}:apis/{}",
        REGION, ACCOUNT_ID, api_id
    );
    let graphql_url = format!(
        "https://{}.appsync-api.{}.amazonaws.com/graphql",
        api_id, REGION
    );

    let mut uris = HashMap::new();
    uris.insert("GRAPHQL".to_string(), graphql_url);

    let authentication_type = payload["authenticationType"]
        .as_str()
        .unwrap_or("API_KEY")
        .to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let api = GraphqlApi {
        api_id: api_id.clone(),
        name,
        arn,
        uris,
        authentication_type,
        created: now,
    };

    let resp = api_to_json(&api);
    state.apis.insert(api_id, api);

    rest_json::created(resp)
}

async fn list_graphql_apis(State(state): State<Arc<AppSyncState>>) -> Response {
    let apis: Vec<Value> = state
        .apis
        .iter()
        .map(|entry| api_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({
        "graphqlApis": apis,
    }))
}

async fn get_graphql_api(
    State(state): State<Arc<AppSyncState>>,
    Path(api_id): Path<String>,
) -> Response {
    match state.apis.get(&api_id) {
        Some(api) => rest_json::ok(json!({
            "graphqlApi": api_to_json(api.value()),
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "GraphQL API not found: {}",
            api_id
        ))),
    }
}

async fn delete_graphql_api(
    State(state): State<Arc<AppSyncState>>,
    Path(api_id): Path<String>,
) -> Response {
    match state.apis.remove(&api_id) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "GraphQL API not found: {}",
            api_id
        ))),
    }
}
