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
pub struct KendraIndex {
    pub id: String,
    pub name: String,
    pub arn: String,
    pub role_arn: String,
    pub status: String,
    pub created_at: f64,
}

#[derive(Debug, Clone)]
pub struct KendraDataSource {
    pub id: String,
    pub index_id: String,
    pub name: String,
    pub ds_type: String,
    pub status: String,
    pub created_at: f64,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct KendraState {
    pub indexes: DashMap<String, KendraIndex>,
    pub data_sources: DashMap<String, KendraDataSource>,
}

impl Default for KendraState {
    fn default() -> Self {
        Self {
            indexes: DashMap::new(),
            data_sources: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &KendraState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("AWSKendraFrontendService.")
        .unwrap_or(target);

    let result = match action {
        "CreateIndex" => create_index(state, payload),
        "DeleteIndex" => delete_index(state, payload),
        "DescribeIndex" => describe_index(state, payload),
        "ListIndices" => list_indices(state),
        "CreateDataSource" => create_data_source(state, payload),
        "DeleteDataSource" => delete_data_source(state, payload),
        "ListDataSources" => list_data_sources(state, payload),
        "Query" => query(state, payload),
        other => Err(LawsError::InvalidRequest(format!(
            "unknown action: {other}"
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

fn create_index(state: &KendraState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    let role_arn = payload["RoleArn"]
        .as_str()
        .unwrap_or("arn:aws:iam::000000000000:role/kendra-role")
        .to_string();

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:kendra:{REGION}:{ACCOUNT_ID}:index/{id}");

    let index = KendraIndex {
        id: id.clone(),
        name,
        arn,
        role_arn,
        status: "ACTIVE".to_string(),
        created_at: now_epoch(),
    };

    state.indexes.insert(id.clone(), index);

    Ok(json_response(json!({ "Id": id })))
}

fn delete_index(state: &KendraState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["Id"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Id is required".to_string()))?;

    state
        .indexes
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("Index '{}' not found", id)))?;

    Ok(json_response(json!({})))
}

fn describe_index(state: &KendraState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["Id"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Id is required".to_string()))?;

    let index = state
        .indexes
        .get(id)
        .ok_or_else(|| LawsError::NotFound(format!("Index '{}' not found", id)))?;

    Ok(json_response(json!({
        "Id": index.id,
        "Name": index.name,
        "IndexArn": index.arn,
        "RoleArn": index.role_arn,
        "Status": index.status,
        "CreatedAt": index.created_at,
    })))
}

fn list_indices(state: &KendraState) -> Result<Response, LawsError> {
    let items: Vec<Value> = state
        .indexes
        .iter()
        .map(|entry| {
            let idx = entry.value();
            json!({
                "Id": idx.id,
                "Name": idx.name,
                "Status": idx.status,
                "CreatedAt": idx.created_at,
            })
        })
        .collect();

    Ok(json_response(json!({
        "IndexConfigurationSummaryItems": items,
    })))
}

fn create_data_source(state: &KendraState, payload: &Value) -> Result<Response, LawsError> {
    let index_id = payload["IndexId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("IndexId is required".to_string()))?
        .to_string();

    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    let ds_type = payload["Type"].as_str().unwrap_or("S3").to_string();

    if !state.indexes.contains_key(&index_id) {
        return Err(LawsError::NotFound(format!(
            "Index '{}' not found",
            index_id
        )));
    }

    let id = uuid::Uuid::new_v4().to_string();

    let ds = KendraDataSource {
        id: id.clone(),
        index_id,
        name,
        ds_type,
        status: "ACTIVE".to_string(),
        created_at: now_epoch(),
    };

    state.data_sources.insert(id.clone(), ds);

    Ok(json_response(json!({ "Id": id })))
}

fn delete_data_source(state: &KendraState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["Id"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Id is required".to_string()))?;

    state
        .data_sources
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("DataSource '{}' not found", id)))?;

    Ok(json_response(json!({})))
}

fn list_data_sources(state: &KendraState, payload: &Value) -> Result<Response, LawsError> {
    let index_id = payload["IndexId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("IndexId is required".to_string()))?;

    let items: Vec<Value> = state
        .data_sources
        .iter()
        .filter(|entry| entry.value().index_id == index_id)
        .map(|entry| {
            let ds = entry.value();
            json!({
                "Id": ds.id,
                "Name": ds.name,
                "Type": ds.ds_type,
                "Status": ds.status,
                "CreatedAt": ds.created_at,
            })
        })
        .collect();

    Ok(json_response(json!({
        "SummaryItems": items,
    })))
}

fn query(state: &KendraState, payload: &Value) -> Result<Response, LawsError> {
    let index_id = payload["IndexId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("IndexId is required".to_string()))?;

    if !state.indexes.contains_key(index_id) {
        return Err(LawsError::NotFound(format!(
            "Index '{}' not found",
            index_id
        )));
    }

    Ok(json_response(json!({
        "QueryId": uuid::Uuid::new_v4().to_string(),
        "ResultItems": [],
        "TotalNumberOfResults": 0,
    })))
}
