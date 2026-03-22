use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::Json;
use dashmap::DashMap;
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
pub struct DataSet {
    pub data_set_id: String,
    pub arn: String,
    pub name: String,
    pub import_mode: String,
    pub created_time: String,
}

#[derive(Debug, Clone)]
pub struct Dashboard {
    pub dashboard_id: String,
    pub arn: String,
    pub name: String,
    pub version_number: u64,
    pub created_time: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct QuickSightState {
    pub datasets: DashMap<String, DataSet>,
    pub dashboards: DashMap<String, Dashboard>,
}

impl Default for QuickSightState {
    fn default() -> Self {
        Self {
            datasets: DashMap::new(),
            dashboards: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<QuickSightState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/accounts/{accountId}/data-sets",
            axum::routing::post(create_data_set).get(list_data_sets),
        )
        .route(
            "/accounts/{accountId}/data-sets/{id}",
            axum::routing::get(describe_data_set).delete(delete_data_set),
        )
        .route(
            "/accounts/{accountId}/dashboards/{id}",
            axum::routing::post(create_dashboard),
        )
        .route(
            "/accounts/{accountId}/dashboards",
            axum::routing::get(list_dashboards),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn random_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_data_set(
    State(state): State<Arc<QuickSightState>>,
    Path(_account_id): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    let name = payload
        .get("Name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();
    let data_set_id = payload
        .get("DataSetId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned())
        .unwrap_or_else(random_id);
    let import_mode = payload
        .get("ImportMode")
        .and_then(|v| v.as_str())
        .unwrap_or("SPICE")
        .to_owned();

    let arn = format!("arn:aws:quicksight:{REGION}:{ACCOUNT_ID}:dataset/{data_set_id}");
    let created_time = chrono::Utc::now().to_rfc3339();

    let ds = DataSet {
        data_set_id: data_set_id.clone(),
        arn: arn.clone(),
        name: name.clone(),
        import_mode,
        created_time,
    };

    state.datasets.insert(data_set_id.clone(), ds);

    rest_json::created(json!({
        "Arn": arn,
        "DataSetId": data_set_id,
        "RequestId": random_id(),
        "Status": 201
    }))
}

async fn list_data_sets(
    State(state): State<Arc<QuickSightState>>,
    Path(_account_id): Path<String>,
) -> Response {
    let datasets: Vec<Value> = state
        .datasets
        .iter()
        .map(|entry| {
            let ds = entry.value();
            json!({
                "DataSetId": ds.data_set_id,
                "Arn": ds.arn,
                "Name": ds.name,
                "ImportMode": ds.import_mode,
                "CreatedTime": ds.created_time
            })
        })
        .collect();

    rest_json::ok(json!({
        "DataSetSummaries": datasets,
        "RequestId": random_id(),
        "Status": 200
    }))
}

async fn describe_data_set(
    State(state): State<Arc<QuickSightState>>,
    Path((_account_id, id)): Path<(String, String)>,
) -> Response {
    match state.datasets.get(&id) {
        Some(ds) => rest_json::ok(json!({
            "DataSet": {
                "DataSetId": ds.data_set_id,
                "Arn": ds.arn,
                "Name": ds.name,
                "ImportMode": ds.import_mode,
                "CreatedTime": ds.created_time
            },
            "RequestId": random_id(),
            "Status": 200
        })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("data set not found: {id}")))
        }
    }
}

async fn delete_data_set(
    State(state): State<Arc<QuickSightState>>,
    Path((_account_id, id)): Path<(String, String)>,
) -> Response {
    match state.datasets.remove(&id) {
        Some((_, ds)) => rest_json::ok(json!({
            "Arn": ds.arn,
            "DataSetId": ds.data_set_id,
            "RequestId": random_id(),
            "Status": 200
        })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("data set not found: {id}")))
        }
    }
}

async fn create_dashboard(
    State(state): State<Arc<QuickSightState>>,
    Path((_account_id, id)): Path<(String, String)>,
    Json(payload): Json<Value>,
) -> Response {
    let name = payload
        .get("Name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();

    let arn = format!("arn:aws:quicksight:{REGION}:{ACCOUNT_ID}:dashboard/{id}");
    let created_time = chrono::Utc::now().to_rfc3339();

    let dashboard = Dashboard {
        dashboard_id: id.clone(),
        arn: arn.clone(),
        name,
        version_number: 1,
        created_time,
    };

    state.dashboards.insert(id.clone(), dashboard);

    rest_json::created(json!({
        "Arn": arn,
        "DashboardId": id,
        "CreationStatus": "CREATION_SUCCESSFUL",
        "RequestId": random_id(),
        "Status": 201
    }))
}

async fn list_dashboards(
    State(state): State<Arc<QuickSightState>>,
    Path(_account_id): Path<String>,
) -> Response {
    let dashboards: Vec<Value> = state
        .dashboards
        .iter()
        .map(|entry| {
            let d = entry.value();
            json!({
                "DashboardId": d.dashboard_id,
                "Arn": d.arn,
                "Name": d.name,
                "CreatedTime": d.created_time
            })
        })
        .collect();

    rest_json::ok(json!({
        "DashboardSummaryList": dashboards,
        "RequestId": random_id(),
        "Status": 200
    }))
}
