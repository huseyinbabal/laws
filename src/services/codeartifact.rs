use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Json;
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
pub struct CodeArtifactDomain {
    pub name: String,
    pub arn: String,
    pub owner: String,
    pub status: String,
    pub created: String,
}

#[derive(Debug, Clone)]
pub struct CodeArtifactRepository {
    pub name: String,
    pub arn: String,
    pub domain_name: String,
    pub description: String,
    pub created: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CodeArtifactState {
    pub domains: DashMap<String, CodeArtifactDomain>,
    pub repositories: DashMap<String, CodeArtifactRepository>,
}

impl Default for CodeArtifactState {
    fn default() -> Self {
        Self {
            domains: DashMap::new(),
            repositories: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Query params
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct DomainQuery {
    domain: Option<String>,
}

#[derive(Deserialize)]
struct RepoQuery {
    domain: Option<String>,
    repository: Option<String>,
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<CodeArtifactState>) -> axum::Router {
    axum::Router::new()
        .route("/v1/domain", post(create_domain).delete(delete_domain))
        .route("/v1/domains", get(list_domains))
        .route(
            "/v1/repository",
            post(create_repository).delete(delete_repository),
        )
        .route("/v1/repositories", get(list_repositories))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_domain(
    State(state): State<Arc<CodeArtifactState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let name = payload["domain"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("domain is required".into()))?
            .to_string();

        if state.domains.contains_key(&name) {
            return Err(LawsError::AlreadyExists(format!(
                "Domain '{}' already exists",
                name
            )));
        }

        let arn = format!("arn:aws:codeartifact:{REGION}:{ACCOUNT_ID}:domain/{name}");
        let now = chrono::Utc::now().to_rfc3339();

        let domain = CodeArtifactDomain {
            name: name.clone(),
            arn: arn.clone(),
            owner: ACCOUNT_ID.to_string(),
            status: "Active".to_string(),
            created: now,
        };

        let resp = domain_to_json(&domain);
        state.domains.insert(name, domain);

        Ok(rest_json::created(json!({ "domain": resp })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_domains(State(state): State<Arc<CodeArtifactState>>) -> Response {
    let domains: Vec<Value> = state
        .domains
        .iter()
        .map(|entry| {
            let d = entry.value();
            json!({
                "name": d.name,
                "owner": d.owner,
                "arn": d.arn,
                "status": d.status,
            })
        })
        .collect();

    rest_json::ok(json!({ "domains": domains }))
}

async fn delete_domain(
    State(state): State<Arc<CodeArtifactState>>,
    Query(params): Query<DomainQuery>,
) -> Response {
    let name = match params.domain {
        Some(n) => n,
        None => {
            return rest_json::error_response(&LawsError::InvalidRequest(
                "domain query parameter is required".into(),
            ))
        }
    };

    match state.domains.remove(&name) {
        Some((_, domain)) => rest_json::ok(json!({ "domain": domain_to_json(&domain) })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Domain '{}' not found", name)))
        }
    }
}

async fn create_repository(
    State(state): State<Arc<CodeArtifactState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let repository = payload["repository"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("repository is required".into()))?
            .to_string();

        let domain_name = payload["domain"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("domain is required".into()))?
            .to_string();

        let description = payload["description"].as_str().unwrap_or("").to_string();

        if !state.domains.contains_key(&domain_name) {
            return Err(LawsError::NotFound(format!(
                "Domain '{}' not found",
                domain_name
            )));
        }

        let arn = format!(
            "arn:aws:codeartifact:{REGION}:{ACCOUNT_ID}:repository/{domain_name}/{repository}"
        );
        let now = chrono::Utc::now().to_rfc3339();

        let repo = CodeArtifactRepository {
            name: repository.clone(),
            arn,
            domain_name,
            description,
            created: now,
        };

        let resp = repo_to_json(&repo);
        state.repositories.insert(repository, repo);

        Ok(rest_json::created(json!({ "repository": resp })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_repositories(State(state): State<Arc<CodeArtifactState>>) -> Response {
    let repos: Vec<Value> = state
        .repositories
        .iter()
        .map(|entry| {
            let r = entry.value();
            json!({
                "name": r.name,
                "arn": r.arn,
                "domainName": r.domain_name,
                "description": r.description,
            })
        })
        .collect();

    rest_json::ok(json!({ "repositories": repos }))
}

async fn delete_repository(
    State(state): State<Arc<CodeArtifactState>>,
    Query(params): Query<RepoQuery>,
) -> Response {
    let name = match params.repository {
        Some(n) => n,
        None => {
            return rest_json::error_response(&LawsError::InvalidRequest(
                "repository query parameter is required".into(),
            ))
        }
    };

    match state.repositories.remove(&name) {
        Some((_, repo)) => rest_json::ok(json!({ "repository": repo_to_json(&repo) })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Repository '{}' not found",
            name
        ))),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn domain_to_json(d: &CodeArtifactDomain) -> Value {
    json!({
        "name": d.name,
        "arn": d.arn,
        "owner": d.owner,
        "status": d.status,
        "createdTime": d.created,
    })
}

fn repo_to_json(r: &CodeArtifactRepository) -> Value {
    json!({
        "name": r.name,
        "arn": r.arn,
        "domainName": r.domain_name,
        "description": r.description,
        "createdTime": r.created,
    })
}
