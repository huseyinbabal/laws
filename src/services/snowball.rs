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
pub struct SnowballJob {
    pub job_id: String,
    pub job_type: String,
    pub job_state: String,
    pub snowball_type: String,
    pub description: String,
    pub address_id: String,
    pub role_arn: String,
    pub cluster_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct SnowballCluster {
    pub cluster_id: String,
    pub cluster_state: String,
    pub description: String,
    pub address_id: String,
    pub role_arn: String,
    pub snowball_type: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct SnowballState {
    pub jobs: DashMap<String, SnowballJob>,
    pub clusters: DashMap<String, SnowballCluster>,
}

impl Default for SnowballState {
    fn default() -> Self {
        Self {
            jobs: DashMap::new(),
            clusters: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &SnowballState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("AWSIESnowballJobManagementService.")
        .unwrap_or(target);

    let result = match action {
        "CreateJob" => create_job(state, payload),
        "DescribeJob" => describe_job(state, payload),
        "ListJobs" => list_jobs(state),
        "CancelJob" => cancel_job(state, payload),
        "UpdateJob" => update_job(state, payload),
        "CreateCluster" => create_cluster(state, payload),
        "ListClusters" => list_clusters(state),
        other => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            other
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

fn random_id(prefix: &str) -> String {
    format!("{}-{}", prefix, &uuid::Uuid::new_v4().to_string()[..8])
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_job(state: &SnowballState, payload: &Value) -> Result<Response, LawsError> {
    let job_type = payload["JobType"].as_str().unwrap_or("IMPORT").to_string();

    let snowball_type = payload["SnowballType"]
        .as_str()
        .unwrap_or("STANDARD")
        .to_string();

    let description = payload["Description"].as_str().unwrap_or("").to_string();

    let address_id = payload["AddressId"].as_str().unwrap_or("").to_string();

    let role_arn = payload["RoleARN"]
        .as_str()
        .unwrap_or(&format!("arn:aws:iam::{ACCOUNT_ID}:role/snowball-role"))
        .to_string();

    let cluster_id = payload["ClusterId"].as_str().unwrap_or("").to_string();

    let job_id = random_id("JID");
    let created_at = chrono::Utc::now().to_rfc3339();

    let job = SnowballJob {
        job_id: job_id.clone(),
        job_type,
        job_state: "New".to_string(),
        snowball_type,
        description,
        address_id,
        role_arn,
        cluster_id,
        created_at,
    };

    state.jobs.insert(job_id.clone(), job);

    Ok(json_response(json!({
        "JobId": job_id,
    })))
}

fn describe_job(state: &SnowballState, payload: &Value) -> Result<Response, LawsError> {
    let job_id = payload["JobId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing JobId".into()))?;

    let job = state
        .jobs
        .get(job_id)
        .ok_or_else(|| LawsError::NotFound(format!("Job '{}' not found", job_id)))?;

    Ok(json_response(json!({
        "JobMetadata": {
            "JobId": job.job_id,
            "JobType": job.job_type,
            "JobState": job.job_state,
            "SnowballType": job.snowball_type,
            "Description": job.description,
            "AddressId": job.address_id,
            "RoleARN": job.role_arn,
            "ClusterId": job.cluster_id,
            "CreationDate": job.created_at,
            "Resources": {
                "S3Resources": [],
            },
        }
    })))
}

fn list_jobs(state: &SnowballState) -> Result<Response, LawsError> {
    let jobs: Vec<Value> = state
        .jobs
        .iter()
        .map(|entry| {
            let j = entry.value();
            json!({
                "JobId": j.job_id,
                "JobState": j.job_state,
                "IsMaster": false,
                "JobType": j.job_type,
                "SnowballType": j.snowball_type,
                "CreationDate": j.created_at,
                "Description": j.description,
            })
        })
        .collect();

    Ok(json_response(json!({
        "JobListEntries": jobs,
    })))
}

fn cancel_job(state: &SnowballState, payload: &Value) -> Result<Response, LawsError> {
    let job_id = payload["JobId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing JobId".into()))?;

    let mut job = state
        .jobs
        .get_mut(job_id)
        .ok_or_else(|| LawsError::NotFound(format!("Job '{}' not found", job_id)))?;

    job.job_state = "Cancelled".to_string();

    Ok(json_response(json!({})))
}

fn update_job(state: &SnowballState, payload: &Value) -> Result<Response, LawsError> {
    let job_id = payload["JobId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing JobId".into()))?;

    let mut job = state
        .jobs
        .get_mut(job_id)
        .ok_or_else(|| LawsError::NotFound(format!("Job '{}' not found", job_id)))?;

    if let Some(desc) = payload["Description"].as_str() {
        job.description = desc.to_string();
    }
    if let Some(role) = payload["RoleARN"].as_str() {
        job.role_arn = role.to_string();
    }
    if let Some(addr) = payload["AddressId"].as_str() {
        job.address_id = addr.to_string();
    }

    Ok(json_response(json!({})))
}

fn create_cluster(state: &SnowballState, payload: &Value) -> Result<Response, LawsError> {
    let description = payload["Description"].as_str().unwrap_or("").to_string();

    let address_id = payload["AddressId"].as_str().unwrap_or("").to_string();

    let role_arn = payload["RoleARN"]
        .as_str()
        .unwrap_or(&format!("arn:aws:iam::{ACCOUNT_ID}:role/snowball-role"))
        .to_string();

    let snowball_type = payload["SnowballType"]
        .as_str()
        .unwrap_or("EDGE")
        .to_string();

    let cluster_id = random_id("CID");
    let created_at = chrono::Utc::now().to_rfc3339();

    let cluster = SnowballCluster {
        cluster_id: cluster_id.clone(),
        cluster_state: "AwaitingQuorum".to_string(),
        description,
        address_id,
        role_arn,
        snowball_type,
        created_at,
    };

    state.clusters.insert(cluster_id.clone(), cluster);

    Ok(json_response(json!({
        "ClusterId": cluster_id,
    })))
}

fn list_clusters(state: &SnowballState) -> Result<Response, LawsError> {
    let clusters: Vec<Value> = state
        .clusters
        .iter()
        .map(|entry| {
            let c = entry.value();
            json!({
                "ClusterId": c.cluster_id,
                "ClusterState": c.cluster_state,
                "CreationDate": c.created_at,
                "Description": c.description,
            })
        })
        .collect();

    Ok(json_response(json!({
        "ClusterListEntries": clusters,
    })))
}
