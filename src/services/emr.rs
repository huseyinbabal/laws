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
pub struct EmrCluster {
    pub cluster_id: String,
    pub name: String,
    pub arn: String,
    pub status: String,
    pub release_label: String,
    pub instance_count: u32,
    pub master_instance_type: String,
    pub steps: Vec<EmrStep>,
}

#[derive(Debug, Clone)]
pub struct EmrStep {
    pub step_id: String,
    pub name: String,
    pub action_on_failure: String,
    pub status: String,
    pub jar: String,
    pub args: Vec<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct EmrState {
    pub clusters: DashMap<String, EmrCluster>,
}

impl Default for EmrState {
    fn default() -> Self {
        Self {
            clusters: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &EmrState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("ElasticMapReduce.")
        .unwrap_or(target);

    let result = match action {
        "RunJobFlow" => run_job_flow(state, payload),
        "TerminateJobFlows" => terminate_job_flows(state, payload),
        "ListClusters" => list_clusters(state),
        "DescribeCluster" => describe_cluster(state, payload),
        "AddJobFlowSteps" => add_job_flow_steps(state, payload),
        "ListSteps" => list_steps(state, payload),
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

fn cluster_summary_to_json(c: &EmrCluster) -> Value {
    json!({
        "Id": c.cluster_id,
        "Name": c.name,
        "Status": {
            "State": c.status,
        },
        "ClusterArn": c.arn,
    })
}

fn cluster_detail_to_json(c: &EmrCluster) -> Value {
    json!({
        "Id": c.cluster_id,
        "Name": c.name,
        "ClusterArn": c.arn,
        "Status": {
            "State": c.status,
        },
        "ReleaseLabel": c.release_label,
        "InstanceCollectionType": "INSTANCE_GROUP",
        "MasterPublicDnsName": format!("ec2-0-0-0-0.{REGION}.compute.amazonaws.com"),
    })
}

fn step_to_json(s: &EmrStep) -> Value {
    json!({
        "Id": s.step_id,
        "Name": s.name,
        "ActionOnFailure": s.action_on_failure,
        "Status": {
            "State": s.status,
        },
        "Config": {
            "Jar": s.jar,
            "Args": s.args,
        },
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn run_job_flow(state: &EmrState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    let cluster_id = format!("j-{}", &uuid::Uuid::new_v4().to_string()[..13].to_uppercase());
    let arn = format!(
        "arn:aws:elasticmapreduce:{REGION}:{ACCOUNT_ID}:cluster/{cluster_id}"
    );

    let release_label = payload["ReleaseLabel"]
        .as_str()
        .unwrap_or("emr-6.10.0")
        .to_string();

    let instance_count = payload["Instances"]["InstanceCount"]
        .as_u64()
        .unwrap_or(3) as u32;

    let master_instance_type = payload["Instances"]["MasterInstanceType"]
        .as_str()
        .unwrap_or("m5.xlarge")
        .to_string();

    let cluster = EmrCluster {
        cluster_id: cluster_id.clone(),
        name,
        arn,
        status: "STARTING".to_string(),
        release_label,
        instance_count,
        master_instance_type,
        steps: Vec::new(),
    };

    state.clusters.insert(cluster_id.clone(), cluster);

    Ok(json_response(StatusCode::OK, json!({ "JobFlowId": cluster_id })))
}

fn terminate_job_flows(state: &EmrState, payload: &Value) -> Result<Response, LawsError> {
    let job_flow_ids = payload["JobFlowIds"]
        .as_array()
        .ok_or_else(|| LawsError::InvalidRequest("JobFlowIds is required".to_string()))?;

    for id_val in job_flow_ids {
        let id = id_val.as_str().unwrap_or_default();
        if let Some(mut cluster) = state.clusters.get_mut(id) {
            cluster.status = "TERMINATED".to_string();
        }
    }

    Ok(json_response(StatusCode::OK, json!({})))
}

fn list_clusters(state: &EmrState) -> Result<Response, LawsError> {
    let clusters: Vec<Value> = state
        .clusters
        .iter()
        .map(|entry| cluster_summary_to_json(entry.value()))
        .collect();

    Ok(json_response(StatusCode::OK, json!({ "Clusters": clusters })))
}

fn describe_cluster(state: &EmrState, payload: &Value) -> Result<Response, LawsError> {
    let cluster_id = payload["ClusterId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterId is required".to_string()))?;

    let cluster = state
        .clusters
        .get(cluster_id)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Cluster '{}' not found", cluster_id))
        })?;

    Ok(json_response(
        StatusCode::OK,
        json!({ "Cluster": cluster_detail_to_json(&cluster) }),
    ))
}

fn add_job_flow_steps(state: &EmrState, payload: &Value) -> Result<Response, LawsError> {
    let cluster_id = payload["JobFlowId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("JobFlowId is required".to_string()))?;

    let mut cluster = state
        .clusters
        .get_mut(cluster_id)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Cluster '{}' not found", cluster_id))
        })?;

    let steps_input = payload["Steps"]
        .as_array()
        .ok_or_else(|| LawsError::InvalidRequest("Steps is required".to_string()))?;

    let mut step_ids = Vec::new();
    for step_val in steps_input {
        let step_id = format!("s-{}", &uuid::Uuid::new_v4().to_string()[..13].to_uppercase());
        let name = step_val["Name"].as_str().unwrap_or("Step").to_string();
        let action_on_failure = step_val["ActionOnFailure"]
            .as_str()
            .unwrap_or("CONTINUE")
            .to_string();
        let jar = step_val["HadoopJarStep"]["Jar"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let args: Vec<String> = step_val["HadoopJarStep"]["Args"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let step = EmrStep {
            step_id: step_id.clone(),
            name,
            action_on_failure,
            status: "PENDING".to_string(),
            jar,
            args,
        };

        cluster.steps.push(step);
        step_ids.push(json!(step_id));
    }

    Ok(json_response(StatusCode::OK, json!({ "StepIds": step_ids })))
}

fn list_steps(state: &EmrState, payload: &Value) -> Result<Response, LawsError> {
    let cluster_id = payload["ClusterId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterId is required".to_string()))?;

    let cluster = state
        .clusters
        .get(cluster_id)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Cluster '{}' not found", cluster_id))
        })?;

    let steps: Vec<Value> = cluster.steps.iter().map(step_to_json).collect();

    Ok(json_response(StatusCode::OK, json!({ "Steps": steps })))
}
