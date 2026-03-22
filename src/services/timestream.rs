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
pub struct TimestreamDatabase {
    pub database_name: String,
    pub arn: String,
    pub table_count: u64,
    pub creation_time: f64,
}

#[derive(Debug, Clone)]
pub struct TimestreamTable {
    pub database_name: String,
    pub table_name: String,
    pub arn: String,
    pub retention_properties: RetentionProperties,
    pub creation_time: f64,
}

#[derive(Debug, Clone)]
pub struct RetentionProperties {
    pub memory_store_retention_period_in_hours: u64,
    pub magnetic_store_retention_period_in_days: u64,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct TimestreamState {
    pub databases: DashMap<String, TimestreamDatabase>,
    pub tables: DashMap<String, TimestreamTable>,
}

impl Default for TimestreamState {
    fn default() -> Self {
        Self {
            databases: DashMap::new(),
            tables: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &TimestreamState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("Timestream_20181101.")
        .unwrap_or(target);

    let result = match action {
        "CreateDatabase" => create_database(state, payload),
        "DeleteDatabase" => delete_database(state, payload),
        "DescribeDatabase" => describe_database(state, payload),
        "ListDatabases" => list_databases(state),
        "CreateTable" => create_table(state, payload),
        "DeleteTable" => delete_table(state, payload),
        "DescribeTable" => describe_table(state, payload),
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

fn now_epoch() -> f64 {
    chrono::Utc::now().timestamp() as f64
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_database(state: &TimestreamState, payload: &Value) -> Result<Response, LawsError> {
    let database_name = payload["DatabaseName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DatabaseName is required".to_string()))?
        .to_string();

    if state.databases.contains_key(&database_name) {
        return Err(LawsError::AlreadyExists(format!(
            "Database '{}' already exists",
            database_name
        )));
    }

    let arn = format!("arn:aws:timestream:{REGION}:{ACCOUNT_ID}:database/{database_name}");
    let creation_time = now_epoch();

    let db = TimestreamDatabase {
        database_name: database_name.clone(),
        arn: arn.clone(),
        table_count: 0,
        creation_time,
    };

    state.databases.insert(database_name.clone(), db);

    Ok(json_response(json!({
        "Database": {
            "DatabaseName": database_name,
            "Arn": arn,
            "TableCount": 0,
            "CreationTime": creation_time,
        }
    })))
}

fn delete_database(state: &TimestreamState, payload: &Value) -> Result<Response, LawsError> {
    let database_name = payload["DatabaseName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DatabaseName is required".to_string()))?;

    state
        .databases
        .remove(database_name)
        .ok_or_else(|| LawsError::NotFound(format!("Database '{}' not found", database_name)))?;

    // Remove associated tables
    state.tables.retain(|_, t| t.database_name != database_name);

    Ok(json_response(json!({})))
}

fn describe_database(state: &TimestreamState, payload: &Value) -> Result<Response, LawsError> {
    let database_name = payload["DatabaseName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DatabaseName is required".to_string()))?;

    let db = state
        .databases
        .get(database_name)
        .ok_or_else(|| LawsError::NotFound(format!("Database '{}' not found", database_name)))?;

    let table_count = state
        .tables
        .iter()
        .filter(|t| t.database_name == database_name)
        .count() as u64;

    Ok(json_response(json!({
        "Database": {
            "DatabaseName": db.database_name,
            "Arn": db.arn,
            "TableCount": table_count,
            "CreationTime": db.creation_time,
        }
    })))
}

fn list_databases(state: &TimestreamState) -> Result<Response, LawsError> {
    let databases: Vec<Value> = state
        .databases
        .iter()
        .map(|entry| {
            let db = entry.value();
            json!({
                "DatabaseName": db.database_name,
                "Arn": db.arn,
                "TableCount": db.table_count,
                "CreationTime": db.creation_time,
            })
        })
        .collect();

    Ok(json_response(json!({ "Databases": databases })))
}

fn create_table(state: &TimestreamState, payload: &Value) -> Result<Response, LawsError> {
    let database_name = payload["DatabaseName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DatabaseName is required".to_string()))?
        .to_string();

    let table_name = payload["TableName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("TableName is required".to_string()))?
        .to_string();

    if !state.databases.contains_key(&database_name) {
        return Err(LawsError::NotFound(format!(
            "Database '{}' not found",
            database_name
        )));
    }

    let table_key = format!("{}:{}", database_name, table_name);
    if state.tables.contains_key(&table_key) {
        return Err(LawsError::AlreadyExists(format!(
            "Table '{}' already exists",
            table_name
        )));
    }

    let arn = format!(
        "arn:aws:timestream:{REGION}:{ACCOUNT_ID}:database/{database_name}/table/{table_name}"
    );

    let retention = match payload.get("RetentionProperties") {
        Some(rp) => RetentionProperties {
            memory_store_retention_period_in_hours: rp["MemoryStoreRetentionPeriodInHours"]
                .as_u64()
                .unwrap_or(6),
            magnetic_store_retention_period_in_days: rp["MagneticStoreRetentionPeriodInDays"]
                .as_u64()
                .unwrap_or(73000),
        },
        None => RetentionProperties {
            memory_store_retention_period_in_hours: 6,
            magnetic_store_retention_period_in_days: 73000,
        },
    };

    let creation_time = now_epoch();

    let table = TimestreamTable {
        database_name: database_name.clone(),
        table_name: table_name.clone(),
        arn: arn.clone(),
        retention_properties: retention.clone(),
        creation_time,
    };

    state.tables.insert(table_key, table);

    Ok(json_response(json!({
        "Table": {
            "DatabaseName": database_name,
            "TableName": table_name,
            "Arn": arn,
            "RetentionProperties": {
                "MemoryStoreRetentionPeriodInHours": retention.memory_store_retention_period_in_hours,
                "MagneticStoreRetentionPeriodInDays": retention.magnetic_store_retention_period_in_days,
            },
            "CreationTime": creation_time,
        }
    })))
}

fn delete_table(state: &TimestreamState, payload: &Value) -> Result<Response, LawsError> {
    let database_name = payload["DatabaseName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DatabaseName is required".to_string()))?;

    let table_name = payload["TableName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("TableName is required".to_string()))?;

    let table_key = format!("{}:{}", database_name, table_name);
    state
        .tables
        .remove(&table_key)
        .ok_or_else(|| LawsError::NotFound(format!("Table '{}' not found", table_name)))?;

    Ok(json_response(json!({})))
}

fn describe_table(state: &TimestreamState, payload: &Value) -> Result<Response, LawsError> {
    let database_name = payload["DatabaseName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DatabaseName is required".to_string()))?;

    let table_name = payload["TableName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("TableName is required".to_string()))?;

    let table_key = format!("{}:{}", database_name, table_name);
    let table = state
        .tables
        .get(&table_key)
        .ok_or_else(|| LawsError::NotFound(format!("Table '{}' not found", table_name)))?;

    Ok(json_response(json!({
        "Table": {
            "DatabaseName": table.database_name,
            "TableName": table.table_name,
            "Arn": table.arn,
            "RetentionProperties": {
                "MemoryStoreRetentionPeriodInHours": table.retention_properties.memory_store_retention_period_in_hours,
                "MagneticStoreRetentionPeriodInDays": table.retention_properties.magnetic_store_retention_period_in_days,
            },
            "CreationTime": table.creation_time,
        }
    })))
}

fn list_tables(state: &TimestreamState, payload: &Value) -> Result<Response, LawsError> {
    let database_name = payload["DatabaseName"].as_str();

    let tables: Vec<Value> = state
        .tables
        .iter()
        .filter(|entry| match database_name {
            Some(db) => entry.database_name == db,
            None => true,
        })
        .map(|entry| {
            let t = entry.value();
            json!({
                "DatabaseName": t.database_name,
                "TableName": t.table_name,
                "Arn": t.arn,
                "CreationTime": t.creation_time,
            })
        })
        .collect();

    Ok(json_response(json!({ "Tables": tables })))
}
