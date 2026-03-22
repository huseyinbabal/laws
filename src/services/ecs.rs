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
pub struct EcsCluster {
    pub cluster_name: String,
    pub arn: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct EcsTaskDefinition {
    pub family: String,
    pub revision: u32,
    pub arn: String,
    pub container_definitions: Vec<Value>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct EcsTask {
    pub task_arn: String,
    pub cluster_arn: String,
    pub task_definition_arn: String,
    pub last_status: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct EcsState {
    pub clusters: DashMap<String, EcsCluster>,
    pub task_definitions: DashMap<String, EcsTaskDefinition>,
    pub tasks: DashMap<String, EcsTask>,
}

impl Default for EcsState {
    fn default() -> Self {
        let clusters = DashMap::new();

        // Create default cluster
        let default_arn = format!("arn:aws:ecs:{REGION}:{ACCOUNT_ID}:cluster/default");
        clusters.insert(
            "default".to_string(),
            EcsCluster {
                cluster_name: "default".to_string(),
                arn: default_arn,
                status: "ACTIVE".to_string(),
            },
        );

        Self {
            clusters,
            task_definitions: DashMap::new(),
            tasks: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &EcsState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("AmazonEC2ContainerServiceV20141113.")
        .unwrap_or(target);

    let result = match action {
        "CreateCluster" => create_cluster(state, payload),
        "DeleteCluster" => delete_cluster(state, payload),
        "ListClusters" => list_clusters(state),
        "DescribeClusters" => describe_clusters(state, payload),
        "RegisterTaskDefinition" => register_task_definition(state, payload),
        "DeregisterTaskDefinition" => deregister_task_definition(state, payload),
        "ListTaskDefinitions" => list_task_definitions(state),
        "RunTask" => run_task(state, payload),
        "StopTask" => stop_task(state, payload),
        "ListTasks" => list_tasks(state, payload),
        "DescribeTasks" => describe_tasks(state, payload),
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

fn cluster_to_json(c: &EcsCluster) -> Value {
    json!({
        "clusterName": c.cluster_name,
        "clusterArn": c.arn,
        "status": c.status,
        "registeredContainerInstancesCount": 0,
        "runningTasksCount": 0,
        "pendingTasksCount": 0,
    })
}

fn task_def_to_json(td: &EcsTaskDefinition) -> Value {
    json!({
        "taskDefinitionArn": td.arn,
        "family": td.family,
        "revision": td.revision,
        "containerDefinitions": td.container_definitions,
        "status": td.status,
    })
}

fn task_to_json(t: &EcsTask) -> Value {
    json!({
        "taskArn": t.task_arn,
        "clusterArn": t.cluster_arn,
        "taskDefinitionArn": t.task_definition_arn,
        "lastStatus": t.last_status,
        "createdAt": t.created_at,
    })
}

/// Resolve a cluster identifier (name or ARN) to the cluster name.
fn resolve_cluster_name(identifier: &str) -> String {
    if identifier.starts_with("arn:") {
        identifier
            .rsplit('/')
            .next()
            .unwrap_or(identifier)
            .to_string()
    } else {
        identifier.to_string()
    }
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_cluster(state: &EcsState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["clusterName"]
        .as_str()
        .unwrap_or("default")
        .to_string();

    if state.clusters.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Cluster '{}' already exists",
            name
        )));
    }

    let arn = format!("arn:aws:ecs:{REGION}:{ACCOUNT_ID}:cluster/{name}");
    let cluster = EcsCluster {
        cluster_name: name.clone(),
        arn,
        status: "ACTIVE".to_string(),
    };

    let resp = cluster_to_json(&cluster);
    state.clusters.insert(name, cluster);

    Ok(json_response(json!({ "cluster": resp })))
}

fn delete_cluster(state: &EcsState, payload: &Value) -> Result<Response, LawsError> {
    let identifier = payload["cluster"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("cluster is required".to_string()))?;

    let name = resolve_cluster_name(identifier);

    let (_, cluster) = state
        .clusters
        .remove(&name)
        .ok_or_else(|| LawsError::NotFound(format!("Cluster '{}' not found", name)))?;

    Ok(json_response(json!({ "cluster": cluster_to_json(&cluster) })))
}

fn list_clusters(state: &EcsState) -> Result<Response, LawsError> {
    let arns: Vec<String> = state
        .clusters
        .iter()
        .map(|entry| entry.value().arn.clone())
        .collect();

    Ok(json_response(json!({ "clusterArns": arns })))
}

fn describe_clusters(state: &EcsState, payload: &Value) -> Result<Response, LawsError> {
    let identifiers = payload["clusters"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut clusters = Vec::new();
    let mut failures = Vec::new();

    for id_val in &identifiers {
        let id = id_val.as_str().unwrap_or_default();
        let name = resolve_cluster_name(id);
        match state.clusters.get(&name) {
            Some(c) => clusters.push(cluster_to_json(c.value())),
            None => failures.push(json!({
                "arn": id,
                "reason": "MISSING",
            })),
        }
    }

    Ok(json_response(json!({
        "clusters": clusters,
        "failures": failures,
    })))
}

fn register_task_definition(state: &EcsState, payload: &Value) -> Result<Response, LawsError> {
    let family = payload["family"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("family is required".to_string()))?
        .to_string();

    let container_definitions = payload["containerDefinitions"]
        .as_array()
        .cloned()
        .ok_or_else(|| {
            LawsError::InvalidRequest("containerDefinitions is required".to_string())
        })?;

    // Determine next revision number for this family
    let revision = {
        let mut max_rev: u32 = 0;
        for entry in state.task_definitions.iter() {
            let td = entry.value();
            if td.family == family && td.revision > max_rev {
                max_rev = td.revision;
            }
        }
        max_rev + 1
    };

    let key = format!("{family}:{revision}");
    let arn = format!(
        "arn:aws:ecs:{REGION}:{ACCOUNT_ID}:task-definition/{key}"
    );

    let td = EcsTaskDefinition {
        family: family.clone(),
        revision,
        arn,
        container_definitions,
        status: "ACTIVE".to_string(),
    };

    let resp = task_def_to_json(&td);
    state.task_definitions.insert(key, td);

    Ok(json_response(json!({ "taskDefinition": resp })))
}

fn deregister_task_definition(
    state: &EcsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let key = payload["taskDefinition"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("taskDefinition is required".to_string())
        })?
        .to_string();

    let mut td = state
        .task_definitions
        .get_mut(&key)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Task definition '{}' not found", key))
        })?;

    td.status = "INACTIVE".to_string();
    let resp = task_def_to_json(&td);

    Ok(json_response(json!({ "taskDefinition": resp })))
}

fn list_task_definitions(state: &EcsState) -> Result<Response, LawsError> {
    let arns: Vec<String> = state
        .task_definitions
        .iter()
        .filter(|entry| entry.value().status == "ACTIVE")
        .map(|entry| entry.value().arn.clone())
        .collect();

    Ok(json_response(json!({ "taskDefinitionArns": arns })))
}

fn run_task(state: &EcsState, payload: &Value) -> Result<Response, LawsError> {
    let cluster_id = payload["cluster"]
        .as_str()
        .unwrap_or("default");
    let cluster_name = resolve_cluster_name(cluster_id);

    let cluster = state
        .clusters
        .get(&cluster_name)
        .ok_or_else(|| LawsError::NotFound(format!("Cluster '{}' not found", cluster_name)))?;
    let cluster_arn = cluster.arn.clone();

    let task_def_key = payload["taskDefinition"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("taskDefinition is required".to_string())
        })?;

    let td = state
        .task_definitions
        .get(task_def_key)
        .ok_or_else(|| {
            LawsError::NotFound(format!(
                "Task definition '{}' not found",
                task_def_key
            ))
        })?;
    let td_arn = td.arn.clone();

    let count = payload["count"].as_u64().unwrap_or(1) as usize;
    let now = chrono::Utc::now().to_rfc3339();

    let mut tasks = Vec::new();
    for _ in 0..count {
        let task_id = uuid::Uuid::new_v4().to_string();
        let task_arn = format!(
            "arn:aws:ecs:{REGION}:{ACCOUNT_ID}:task/{cluster_name}/{task_id}"
        );

        let task = EcsTask {
            task_arn: task_arn.clone(),
            cluster_arn: cluster_arn.clone(),
            task_definition_arn: td_arn.clone(),
            last_status: "RUNNING".to_string(),
            created_at: now.clone(),
        };

        let resp_json = task_to_json(&task);
        state.tasks.insert(task_arn, task);
        tasks.push(resp_json);
    }

    Ok(json_response(json!({
        "tasks": tasks,
        "failures": [],
    })))
}

fn stop_task(state: &EcsState, payload: &Value) -> Result<Response, LawsError> {
    let _cluster_id = payload["cluster"]
        .as_str()
        .unwrap_or("default");

    let task_arn = payload["task"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("task is required".to_string()))?;

    let mut task = state
        .tasks
        .get_mut(task_arn)
        .ok_or_else(|| LawsError::NotFound(format!("Task '{}' not found", task_arn)))?;

    task.last_status = "STOPPED".to_string();
    let resp = task_to_json(&task);

    Ok(json_response(json!({ "task": resp })))
}

fn list_tasks(state: &EcsState, payload: &Value) -> Result<Response, LawsError> {
    let cluster_id = payload["cluster"]
        .as_str()
        .unwrap_or("default");
    let cluster_name = resolve_cluster_name(cluster_id);

    let cluster = state
        .clusters
        .get(&cluster_name)
        .ok_or_else(|| LawsError::NotFound(format!("Cluster '{}' not found", cluster_name)))?;
    let cluster_arn = cluster.arn.clone();

    let arns: Vec<String> = state
        .tasks
        .iter()
        .filter(|entry| entry.value().cluster_arn == cluster_arn)
        .map(|entry| entry.value().task_arn.clone())
        .collect();

    Ok(json_response(json!({ "taskArns": arns })))
}

fn describe_tasks(state: &EcsState, payload: &Value) -> Result<Response, LawsError> {
    let _cluster_id = payload["cluster"]
        .as_str()
        .unwrap_or("default");

    let task_arns = payload["tasks"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut tasks = Vec::new();
    let mut failures = Vec::new();

    for arn_val in &task_arns {
        let arn = arn_val.as_str().unwrap_or_default();
        match state.tasks.get(arn) {
            Some(t) => tasks.push(task_to_json(t.value())),
            None => failures.push(json!({
                "arn": arn,
                "reason": "MISSING",
            })),
        }
    }

    Ok(json_response(json!({
        "tasks": tasks,
        "failures": failures,
    })))
}
