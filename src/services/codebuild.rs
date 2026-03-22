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
pub struct CodeBuildProject {
    pub name: String,
    pub arn: String,
    pub source_type: String,
    pub environment: Value,
    pub service_role: String,
    pub created: String,
}

#[derive(Debug, Clone)]
pub struct CodeBuildBuild {
    pub id: String,
    pub arn: String,
    pub project_name: String,
    pub build_status: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub source_version: Option<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CodeBuildState {
    pub projects: DashMap<String, CodeBuildProject>,
    pub builds: DashMap<String, CodeBuildBuild>,
}

impl Default for CodeBuildState {
    fn default() -> Self {
        Self {
            projects: DashMap::new(),
            builds: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &CodeBuildState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target.strip_prefix("CodeBuild_20161006.").unwrap_or(target);

    let result = match action {
        "CreateProject" => create_project(state, payload).await,
        "DeleteProject" => delete_project(state, payload).await,
        "ListProjects" => list_projects(state).await,
        "BatchGetProjects" => batch_get_projects(state, payload).await,
        "StartBuild" => start_build(state, payload).await,
        "StopBuild" => stop_build(state, payload).await,
        "BatchGetBuilds" => batch_get_builds(state, payload).await,
        "ListBuildsForProject" => list_builds_for_project(state, payload).await,
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

fn json_response(body: Value) -> Response {
    (
        StatusCode::OK,
        [("Content-Type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn project_to_json(p: &CodeBuildProject) -> Value {
    json!({
        "name": p.name,
        "arn": p.arn,
        "source": {
            "type": p.source_type,
        },
        "environment": p.environment,
        "serviceRole": p.service_role,
        "created": p.created,
    })
}

fn build_to_json(b: &CodeBuildBuild) -> Value {
    json!({
        "id": b.id,
        "arn": b.arn,
        "projectName": b.project_name,
        "buildStatus": b.build_status,
        "startTime": b.start_time,
        "endTime": b.end_time,
        "sourceVersion": b.source_version,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

async fn create_project(
    state: &CodeBuildState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("name is required".to_string()))?
        .to_string();

    if state.projects.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Project already exists: {}",
            name
        )));
    }

    let arn = format!(
        "arn:aws:codebuild:{}:{}:project/{}",
        REGION, ACCOUNT_ID, name
    );
    let source_type = payload["source"]["type"]
        .as_str()
        .unwrap_or("CODECOMMIT")
        .to_string();
    let environment = payload["environment"].clone();
    let service_role = payload["serviceRole"].as_str().unwrap_or("").to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let project = CodeBuildProject {
        name: name.clone(),
        arn,
        source_type,
        environment,
        service_role,
        created: now,
    };

    let resp = project_to_json(&project);
    state.projects.insert(name, project);

    Ok(json_response(json!({
        "project": resp,
    })))
}

async fn delete_project(
    state: &CodeBuildState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("name is required".to_string()))?;

    state
        .projects
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Project not found: {}", name)))?;

    Ok(json_response(json!({})))
}

async fn list_projects(state: &CodeBuildState) -> Result<Response, LawsError> {
    let projects: Vec<String> = state
        .projects
        .iter()
        .map(|entry| entry.value().name.clone())
        .collect();

    Ok(json_response(json!({
        "projects": projects,
    })))
}

async fn batch_get_projects(
    state: &CodeBuildState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let names = payload["names"]
        .as_array()
        .ok_or_else(|| LawsError::InvalidRequest("names is required".to_string()))?;

    let mut projects = Vec::new();
    let mut projects_not_found = Vec::new();

    for name_val in names {
        let name = name_val.as_str().unwrap_or_default();
        match state.projects.get(name) {
            Some(p) => projects.push(project_to_json(p.value())),
            None => projects_not_found.push(json!(name)),
        }
    }

    Ok(json_response(json!({
        "projects": projects,
        "projectsNotFound": projects_not_found,
    })))
}

async fn start_build(
    state: &CodeBuildState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let project_name = payload["projectName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("projectName is required".to_string()))?
        .to_string();

    if !state.projects.contains_key(&project_name) {
        return Err(LawsError::NotFound(format!(
            "Project not found: {}",
            project_name
        )));
    }

    let build_uuid = uuid::Uuid::new_v4().to_string();
    let build_id = format!("{}:{}", project_name, build_uuid);
    let arn = format!(
        "arn:aws:codebuild:{}:{}:build/{}",
        REGION, ACCOUNT_ID, build_id
    );
    let now = chrono::Utc::now().to_rfc3339();
    let source_version = payload["sourceVersion"].as_str().map(|s| s.to_string());

    let build = CodeBuildBuild {
        id: build_id.clone(),
        arn,
        project_name,
        build_status: "IN_PROGRESS".to_string(),
        start_time: now,
        end_time: None,
        source_version,
    };

    let resp = build_to_json(&build);
    state.builds.insert(build_id, build);

    Ok(json_response(json!({
        "build": resp,
    })))
}

async fn stop_build(
    state: &CodeBuildState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let id = payload["id"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("id is required".to_string()))?;

    let mut build = state
        .builds
        .get_mut(id)
        .ok_or_else(|| LawsError::NotFound(format!("Build not found: {}", id)))?;

    build.build_status = "STOPPED".to_string();
    build.end_time = Some(chrono::Utc::now().to_rfc3339());

    let resp = build_to_json(&build);

    Ok(json_response(json!({
        "build": resp,
    })))
}

async fn batch_get_builds(
    state: &CodeBuildState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let ids = payload["ids"]
        .as_array()
        .ok_or_else(|| LawsError::InvalidRequest("ids is required".to_string()))?;

    let mut builds = Vec::new();
    let mut builds_not_found = Vec::new();

    for id_val in ids {
        let id = id_val.as_str().unwrap_or_default();
        match state.builds.get(id) {
            Some(b) => builds.push(build_to_json(b.value())),
            None => builds_not_found.push(json!(id)),
        }
    }

    Ok(json_response(json!({
        "builds": builds,
        "buildsNotFound": builds_not_found,
    })))
}

async fn list_builds_for_project(
    state: &CodeBuildState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let project_name = payload["projectName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("projectName is required".to_string()))?;

    if !state.projects.contains_key(project_name) {
        return Err(LawsError::NotFound(format!(
            "Project not found: {}",
            project_name
        )));
    }

    let ids: Vec<String> = state
        .builds
        .iter()
        .filter(|entry| entry.value().project_name == project_name)
        .map(|entry| entry.value().id.clone())
        .collect();

    Ok(json_response(json!({
        "ids": ids,
    })))
}
