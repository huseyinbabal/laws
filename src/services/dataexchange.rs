use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, delete};
use axum::Json;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSet {
    pub id: String,
    pub name: String,
    pub description: String,
    pub arn: String,
    pub asset_type: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revision {
    pub id: String,
    pub data_set_id: String,
    pub arn: String,
    pub comment: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct DataExchangeState {
    pub datasets: DashMap<String, DataSet>,
    pub revisions: DashMap<String, Revision>,
}

impl Default for DataExchangeState {
    fn default() -> Self {
        Self {
            datasets: DashMap::new(),
            revisions: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<DataExchangeState>) -> axum::Router {
    axum::Router::new()
        .route("/v1/data-sets", post(create_data_set).get(list_data_sets))
        .route(
            "/v1/data-sets/{data_set_id}",
            get(get_data_set).delete(delete_data_set),
        )
        .route(
            "/v1/data-sets/{data_set_id}/revisions",
            post(create_revision).get(list_revisions),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_data_set(
    State(state): State<Arc<DataExchangeState>>,
    Json(body): Json<Value>,
) -> Response {
    let name = match body["Name"].as_str() {
        Some(n) => n.to_string(),
        None => return rest_json::error_response(&LawsError::InvalidRequest("Missing Name".into())),
    };

    let asset_type = body["AssetType"].as_str().unwrap_or("S3_SNAPSHOT").to_string();
    let description = body["Description"].as_str().unwrap_or("").to_string();
    let now = Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:dataexchange:{REGION}:{ACCOUNT_ID}:data-sets/{id}");

    let dataset = DataSet {
        id: id.clone(),
        name,
        description,
        arn: arn.clone(),
        asset_type,
        created_at: now.clone(),
        updated_at: now,
    };

    let resp = json!({
        "Id": dataset.id,
        "Name": dataset.name,
        "Description": dataset.description,
        "Arn": dataset.arn,
        "AssetType": dataset.asset_type,
        "CreatedAt": dataset.created_at,
        "UpdatedAt": dataset.updated_at,
    });

    state.datasets.insert(id, dataset);
    rest_json::created(resp)
}

async fn list_data_sets(State(state): State<Arc<DataExchangeState>>) -> Response {
    let items: Vec<Value> = state
        .datasets
        .iter()
        .map(|entry| {
            let d = entry.value();
            json!({
                "Id": d.id,
                "Name": d.name,
                "Arn": d.arn,
                "AssetType": d.asset_type,
                "CreatedAt": d.created_at,
                "UpdatedAt": d.updated_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "DataSets": items }))
}

async fn get_data_set(
    State(state): State<Arc<DataExchangeState>>,
    Path(data_set_id): Path<String>,
) -> Response {
    match state.datasets.get(&data_set_id) {
        Some(d) => rest_json::ok(json!({
            "Id": d.id,
            "Name": d.name,
            "Description": d.description,
            "Arn": d.arn,
            "AssetType": d.asset_type,
            "CreatedAt": d.created_at,
            "UpdatedAt": d.updated_at,
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "DataSet not found: {data_set_id}"
        ))),
    }
}

async fn delete_data_set(
    State(state): State<Arc<DataExchangeState>>,
    Path(data_set_id): Path<String>,
) -> Response {
    match state.datasets.remove(&data_set_id) {
        Some(_) => {
            state.revisions.retain(|_, r| r.data_set_id != data_set_id);
            rest_json::no_content()
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "DataSet not found: {data_set_id}"
        ))),
    }
}

async fn create_revision(
    State(state): State<Arc<DataExchangeState>>,
    Path(data_set_id): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    if !state.datasets.contains_key(&data_set_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "DataSet not found: {data_set_id}"
        )));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let arn = format!("arn:aws:dataexchange:{REGION}:{ACCOUNT_ID}:data-sets/{data_set_id}/revisions/{id}");
    let comment = body["Comment"].as_str().unwrap_or("").to_string();

    let revision = Revision {
        id: id.clone(),
        data_set_id: data_set_id.clone(),
        arn: arn.clone(),
        comment,
        created_at: now,
    };

    let resp = json!({
        "Id": revision.id,
        "DataSetId": revision.data_set_id,
        "Arn": revision.arn,
        "Comment": revision.comment,
        "CreatedAt": revision.created_at,
    });

    state.revisions.insert(id, revision);
    rest_json::created(resp)
}

async fn list_revisions(
    State(state): State<Arc<DataExchangeState>>,
    Path(data_set_id): Path<String>,
) -> Response {
    if !state.datasets.contains_key(&data_set_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "DataSet not found: {data_set_id}"
        )));
    }

    let items: Vec<Value> = state
        .revisions
        .iter()
        .filter(|entry| entry.value().data_set_id == data_set_id)
        .map(|entry| {
            let r = entry.value();
            json!({
                "Id": r.id,
                "DataSetId": r.data_set_id,
                "Arn": r.arn,
                "Comment": r.comment,
                "CreatedAt": r.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "Revisions": items }))
}
