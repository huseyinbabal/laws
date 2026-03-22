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
pub struct Keyspace {
    pub keyspace_name: String,
    pub resource_arn: String,
}

#[derive(Debug, Clone)]
pub struct KeyspacesTable {
    pub keyspace_name: String,
    pub table_name: String,
    pub resource_arn: String,
    pub status: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct KeyspacesState {
    pub keyspaces: DashMap<String, Keyspace>,
    pub tables: DashMap<String, KeyspacesTable>,
}

impl Default for KeyspacesState {
    fn default() -> Self {
        Self {
            keyspaces: DashMap::new(),
            tables: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &KeyspacesState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("KeyspacesService.")
        .unwrap_or(target);

    let result = match action {
        "CreateKeyspace" => create_keyspace(state, payload),
        "DeleteKeyspace" => delete_keyspace(state, payload),
        "GetKeyspace" => get_keyspace(state, payload),
        "ListKeyspaces" => list_keyspaces(state),
        "CreateTable" => create_table(state, payload),
        "DeleteTable" => delete_table(state, payload),
        "GetTable" => get_table(state, payload),
        "ListTables" => list_tables(state, payload),
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

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_keyspace(
    state: &KeyspacesState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let keyspace_name = payload["keyspaceName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("keyspaceName is required".to_string()))?
        .to_string();

    if state.keyspaces.contains_key(&keyspace_name) {
        return Err(LawsError::AlreadyExists(format!(
            "Keyspace '{}' already exists",
            keyspace_name
        )));
    }

    let resource_arn = format!(
        "arn:aws:cassandra:{REGION}:{ACCOUNT_ID}:/keyspace/{keyspace_name}"
    );

    let keyspace = Keyspace {
        keyspace_name: keyspace_name.clone(),
        resource_arn: resource_arn.clone(),
    };

    state.keyspaces.insert(keyspace_name.clone(), keyspace);

    Ok(json_response(json!({
        "resourceArn": resource_arn,
    })))
}

fn delete_keyspace(
    state: &KeyspacesState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let keyspace_name = payload["keyspaceName"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("keyspaceName is required".to_string())
        })?;

    state
        .keyspaces
        .remove(keyspace_name)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Keyspace '{}' not found", keyspace_name))
        })?;

    // Remove associated tables
    state.tables.retain(|_, t| t.keyspace_name != keyspace_name);

    Ok(json_response(json!({})))
}

fn get_keyspace(
    state: &KeyspacesState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let keyspace_name = payload["keyspaceName"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("keyspaceName is required".to_string())
        })?;

    let keyspace = state
        .keyspaces
        .get(keyspace_name)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Keyspace '{}' not found", keyspace_name))
        })?;

    Ok(json_response(json!({
        "keyspaceName": keyspace.keyspace_name,
        "resourceArn": keyspace.resource_arn,
    })))
}

fn list_keyspaces(state: &KeyspacesState) -> Result<Response, LawsError> {
    let keyspaces: Vec<Value> = state
        .keyspaces
        .iter()
        .map(|entry| {
            let ks = entry.value();
            json!({
                "keyspaceName": ks.keyspace_name,
                "resourceArn": ks.resource_arn,
            })
        })
        .collect();

    Ok(json_response(json!({ "keyspaces": keyspaces })))
}

fn create_table(
    state: &KeyspacesState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let keyspace_name = payload["keyspaceName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("keyspaceName is required".to_string()))?
        .to_string();

    let table_name = payload["tableName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("tableName is required".to_string()))?
        .to_string();

    if !state.keyspaces.contains_key(&keyspace_name) {
        return Err(LawsError::NotFound(format!(
            "Keyspace '{}' not found",
            keyspace_name
        )));
    }

    let table_key = format!("{}:{}", keyspace_name, table_name);
    if state.tables.contains_key(&table_key) {
        return Err(LawsError::AlreadyExists(format!(
            "Table '{}' already exists",
            table_name
        )));
    }

    let resource_arn = format!(
        "arn:aws:cassandra:{REGION}:{ACCOUNT_ID}:/keyspace/{keyspace_name}/table/{table_name}"
    );

    let table = KeyspacesTable {
        keyspace_name: keyspace_name.clone(),
        table_name: table_name.clone(),
        resource_arn: resource_arn.clone(),
        status: "ACTIVE".to_string(),
    };

    state.tables.insert(table_key, table);

    Ok(json_response(json!({
        "resourceArn": resource_arn,
    })))
}

fn delete_table(
    state: &KeyspacesState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let keyspace_name = payload["keyspaceName"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("keyspaceName is required".to_string())
        })?;

    let table_name = payload["tableName"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("tableName is required".to_string())
        })?;

    let table_key = format!("{}:{}", keyspace_name, table_name);
    state
        .tables
        .remove(&table_key)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Table '{}' not found", table_name))
        })?;

    Ok(json_response(json!({})))
}

fn get_table(
    state: &KeyspacesState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let keyspace_name = payload["keyspaceName"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("keyspaceName is required".to_string())
        })?;

    let table_name = payload["tableName"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("tableName is required".to_string())
        })?;

    let table_key = format!("{}:{}", keyspace_name, table_name);
    let table = state
        .tables
        .get(&table_key)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Table '{}' not found", table_name))
        })?;

    Ok(json_response(json!({
        "keyspaceName": table.keyspace_name,
        "tableName": table.table_name,
        "resourceArn": table.resource_arn,
        "status": table.status,
    })))
}

fn list_tables(
    state: &KeyspacesState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let keyspace_name = payload["keyspaceName"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("keyspaceName is required".to_string())
        })?;

    if !state.keyspaces.contains_key(keyspace_name) {
        return Err(LawsError::NotFound(format!(
            "Keyspace '{}' not found",
            keyspace_name
        )));
    }

    let tables: Vec<Value> = state
        .tables
        .iter()
        .filter(|entry| entry.keyspace_name == keyspace_name)
        .map(|entry| {
            let t = entry.value();
            json!({
                "keyspaceName": t.keyspace_name,
                "tableName": t.table_name,
                "resourceArn": t.resource_arn,
            })
        })
        .collect();

    Ok(json_response(json!({ "tables": tables })))
}
