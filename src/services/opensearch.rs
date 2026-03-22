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
pub struct OpenSearchDomain {
    pub domain_name: String,
    pub arn: String,
    pub domain_id: String,
    pub engine_version: String,
    pub endpoint: String,
    pub cluster_config: Value,
    pub created: bool,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct OpenSearchState {
    pub domains: DashMap<String, OpenSearchDomain>,
}

impl Default for OpenSearchState {
    fn default() -> Self {
        Self {
            domains: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<OpenSearchState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/2021-01-01/opensearch/domain",
            post(create_domain).get(list_domain_names),
        )
        .route(
            "/2021-01-01/opensearch/domain/{name}",
            get(describe_domain).delete(delete_domain),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_domain(
    State(state): State<Arc<OpenSearchState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let domain_name = payload["DomainName"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing DomainName".into()))?
            .to_string();

        let engine_version = payload["EngineVersion"]
            .as_str()
            .unwrap_or("OpenSearch_2.11")
            .to_string();

        let cluster_config = payload.get("ClusterConfig").cloned().unwrap_or(json!({
            "InstanceType": "r6g.large.search",
            "InstanceCount": 1,
        }));

        let domain_id = format!("{ACCOUNT_ID}/{domain_name}");
        let arn = format!("arn:aws:es:{REGION}:{ACCOUNT_ID}:domain/{domain_name}");
        let endpoint = format!("search-{domain_name}-abc123.{REGION}.es.amazonaws.com");

        let domain = OpenSearchDomain {
            domain_name: domain_name.clone(),
            arn: arn.clone(),
            domain_id: domain_id.clone(),
            engine_version: engine_version.clone(),
            endpoint: endpoint.clone(),
            cluster_config: cluster_config.clone(),
            created: true,
        };

        state.domains.insert(domain_name.clone(), domain);

        Ok(rest_json::created(json!({
            "DomainStatus": {
                "DomainName": domain_name,
                "ARN": arn,
                "DomainId": domain_id,
                "EngineVersion": engine_version,
                "Endpoint": endpoint,
                "ClusterConfig": cluster_config,
                "Created": true,
                "Processing": false,
            }
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_domain_names(State(state): State<Arc<OpenSearchState>>) -> Response {
    let domain_names: Vec<Value> = state
        .domains
        .iter()
        .map(|entry| {
            let d = entry.value();
            json!({
                "DomainName": d.domain_name,
                "EngineType": "OpenSearch",
            })
        })
        .collect();

    rest_json::ok(json!({ "DomainNames": domain_names }))
}

async fn describe_domain(
    State(state): State<Arc<OpenSearchState>>,
    Path(name): Path<String>,
) -> Response {
    match state.domains.get(&name) {
        Some(domain) => rest_json::ok(json!({
            "DomainStatus": {
                "DomainName": domain.domain_name,
                "ARN": domain.arn,
                "DomainId": domain.domain_id,
                "EngineVersion": domain.engine_version,
                "Endpoint": domain.endpoint,
                "ClusterConfig": domain.cluster_config,
                "Created": domain.created,
                "Processing": false,
            }
        })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Domain '{}' not found", name)))
        }
    }
}

async fn delete_domain(
    State(state): State<Arc<OpenSearchState>>,
    Path(name): Path<String>,
) -> Response {
    match state.domains.remove(&name) {
        Some((_, domain)) => rest_json::ok(json!({
            "DomainStatus": {
                "DomainName": domain.domain_name,
                "ARN": domain.arn,
                "DomainId": domain.domain_id,
                "Created": true,
                "Deleted": true,
            }
        })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Domain '{}' not found", name)))
        }
    }
}
