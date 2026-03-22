use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Repository {
    pub name: String,
    pub arn: String,
    pub repository_id: String,
    pub clone_url_http: String,
    pub clone_url_ssh: String,
    pub description: String,
    pub default_branch: Option<String>,
    pub branches: Vec<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CodeCommitState {
    pub repositories: DashMap<String, Repository>,
}

impl Default for CodeCommitState {
    fn default() -> Self {
        Self {
            repositories: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &CodeCommitState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("CodeCommit_20150413.")
        .unwrap_or(target);

    let result = match action {
        "CreateRepository" => create_repository(state, payload),
        "DeleteRepository" => delete_repository(state, payload),
        "GetRepository" => get_repository(state, payload),
        "ListRepositories" => list_repositories(state),
        "CreateBranch" => create_branch(state, payload),
        "ListBranches" => list_branches(state, payload),
        _ => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            action
        ))),
    };

    match result {
        Ok(resp) => resp,
        Err(e) => e.into_response(),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn json_response(status: StatusCode, body: Value) -> Response {
    (
        status,
        [("Content-Type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn repository_to_json(r: &Repository) -> Value {
    json!({
        "repositoryName": r.name,
        "Arn": r.arn,
        "repositoryId": r.repository_id,
        "cloneUrlHttp": r.clone_url_http,
        "cloneUrlSsh": r.clone_url_ssh,
        "repositoryDescription": r.description,
        "defaultBranch": r.default_branch,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_repository(state: &CodeCommitState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["repositoryName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("repositoryName is required".to_string()))?
        .to_string();

    if state.repositories.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Repository '{}' already exists",
            name
        )));
    }

    let repo_id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:codecommit:{REGION}:{ACCOUNT_ID}:{name}");
    let clone_url_http = format!("https://git-codecommit.{REGION}.amazonaws.com/v1/repos/{name}");
    let clone_url_ssh = format!("ssh://git-codecommit.{REGION}.amazonaws.com/v1/repos/{name}");
    let description = payload["repositoryDescription"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let repo = Repository {
        name: name.clone(),
        arn,
        repository_id: repo_id,
        clone_url_http,
        clone_url_ssh,
        description,
        default_branch: None,
        branches: Vec::new(),
    };

    let resp = repository_to_json(&repo);
    state.repositories.insert(name, repo);

    Ok(json_response(
        StatusCode::OK,
        json!({ "repositoryMetadata": resp }),
    ))
}

fn delete_repository(state: &CodeCommitState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["repositoryName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("repositoryName is required".to_string()))?;

    let (_, repo) = state
        .repositories
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Repository '{}' not found", name)))?;

    Ok(json_response(
        StatusCode::OK,
        json!({ "repositoryId": repo.repository_id }),
    ))
}

fn get_repository(state: &CodeCommitState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["repositoryName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("repositoryName is required".to_string()))?;

    let repo = state
        .repositories
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("Repository '{}' not found", name)))?;

    Ok(json_response(
        StatusCode::OK,
        json!({ "repositoryMetadata": repository_to_json(&repo) }),
    ))
}

fn list_repositories(state: &CodeCommitState) -> Result<Response, LawsError> {
    let repos: Vec<Value> = state
        .repositories
        .iter()
        .map(|entry| {
            json!({
                "repositoryName": entry.value().name,
                "repositoryId": entry.value().repository_id,
            })
        })
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "repositories": repos }),
    ))
}

fn create_branch(state: &CodeCommitState, payload: &Value) -> Result<Response, LawsError> {
    let repo_name = payload["repositoryName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("repositoryName is required".to_string()))?;

    let branch_name = payload["branchName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("branchName is required".to_string()))?
        .to_string();

    let mut repo = state
        .repositories
        .get_mut(repo_name)
        .ok_or_else(|| LawsError::NotFound(format!("Repository '{}' not found", repo_name)))?;

    if repo.branches.contains(&branch_name) {
        return Err(LawsError::AlreadyExists(format!(
            "Branch '{}' already exists",
            branch_name
        )));
    }

    repo.branches.push(branch_name);
    if repo.default_branch.is_none() {
        repo.default_branch = Some(repo.branches[0].clone());
    }

    Ok(json_response(StatusCode::OK, json!({})))
}

fn list_branches(state: &CodeCommitState, payload: &Value) -> Result<Response, LawsError> {
    let repo_name = payload["repositoryName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("repositoryName is required".to_string()))?;

    let repo = state
        .repositories
        .get(repo_name)
        .ok_or_else(|| LawsError::NotFound(format!("Repository '{}' not found", repo_name)))?;

    Ok(json_response(
        StatusCode::OK,
        json!({ "branches": repo.branches }),
    ))
}
