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
pub struct DataLake {
    pub data_lake_arn: String,
    pub region: String,
    pub encryption_key: String,
    pub lifecycle_configuration: Value,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Subscriber {
    pub subscriber_id: String,
    pub subscriber_arn: String,
    pub subscriber_name: String,
    pub subscriber_description: String,
    pub access_types: Vec<String>,
    pub sources: Vec<Value>,
    pub status: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct SecurityLakeState {
    pub data_lakes: DashMap<String, DataLake>,
    pub subscribers: DashMap<String, Subscriber>,
}

impl Default for SecurityLakeState {
    fn default() -> Self {
        Self {
            data_lakes: DashMap::new(),
            subscribers: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<SecurityLakeState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/v1/datalake",
            axum::routing::post(create_data_lake)
                .get(get_data_lake)
                .delete(delete_data_lake),
        )
        .route(
            "/v1/datalake/subscribers",
            axum::routing::post(create_subscriber).get(list_subscribers),
        )
        .route(
            "/v1/datalake/subscribers/{id}",
            axum::routing::get(get_subscriber),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn data_lake_to_json(dl: &DataLake) -> Value {
    json!({
        "dataLakeArn": dl.data_lake_arn,
        "region": dl.region,
        "encryptionConfiguration": {
            "kmsKeyId": dl.encryption_key,
        },
        "lifecycleConfiguration": dl.lifecycle_configuration,
        "status": dl.status,
        "createTime": dl.created_at,
    })
}

fn subscriber_to_json(s: &Subscriber) -> Value {
    json!({
        "subscriberId": s.subscriber_id,
        "subscriberArn": s.subscriber_arn,
        "subscriberName": s.subscriber_name,
        "subscriberDescription": s.subscriber_description,
        "accessTypes": s.access_types,
        "sources": s.sources,
        "subscriberStatus": s.status,
        "createdAt": s.created_at,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_data_lake(
    State(state): State<Arc<SecurityLakeState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let configurations = payload["configurations"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut data_lakes = Vec::new();

        for config in &configurations {
            let region = config["region"].as_str().unwrap_or(REGION).to_string();

            let encryption_key = config["encryptionConfiguration"]["kmsKeyId"]
                .as_str()
                .unwrap_or("aws/s3")
                .to_string();

            let lifecycle = config["lifecycleConfiguration"].clone();

            let data_lake_arn =
                format!("arn:aws:securitylake:{region}:{ACCOUNT_ID}:data-lake/default");
            let created_at = chrono::Utc::now().to_rfc3339();

            let dl = DataLake {
                data_lake_arn: data_lake_arn.clone(),
                region: region.clone(),
                encryption_key,
                lifecycle_configuration: if lifecycle.is_null() {
                    json!({})
                } else {
                    lifecycle
                },
                status: "COMPLETED".to_string(),
                created_at,
            };

            data_lakes.push(data_lake_to_json(&dl));
            state.data_lakes.insert(region, dl);
        }

        if data_lakes.is_empty() {
            let data_lake_arn =
                format!("arn:aws:securitylake:{REGION}:{ACCOUNT_ID}:data-lake/default");
            let created_at = chrono::Utc::now().to_rfc3339();

            let dl = DataLake {
                data_lake_arn,
                region: REGION.to_string(),
                encryption_key: "aws/s3".to_string(),
                lifecycle_configuration: json!({}),
                status: "COMPLETED".to_string(),
                created_at,
            };

            data_lakes.push(data_lake_to_json(&dl));
            state.data_lakes.insert(REGION.to_string(), dl);
        }

        Ok(rest_json::ok(json!({
            "dataLakes": data_lakes,
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn get_data_lake(State(state): State<Arc<SecurityLakeState>>) -> Response {
    let data_lakes: Vec<Value> = state
        .data_lakes
        .iter()
        .map(|entry| data_lake_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "dataLakes": data_lakes }))
}

async fn delete_data_lake(
    State(state): State<Arc<SecurityLakeState>>,
    Json(payload): Json<Value>,
) -> Response {
    let regions: Vec<String> = payload["regions"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if regions.is_empty() {
        state.data_lakes.clear();
    } else {
        for region in &regions {
            state.data_lakes.remove(region);
        }
    }

    rest_json::ok(json!({}))
}

async fn create_subscriber(
    State(state): State<Arc<SecurityLakeState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let subscriber_name = payload["subscriberName"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing subscriberName".into()))?
            .to_string();

        let subscriber_description = payload["subscriberDescription"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let access_types: Vec<String> = payload["accessTypes"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(|| vec!["S3".to_string()]);

        let sources: Vec<Value> = payload["sources"].as_array().cloned().unwrap_or_default();

        let subscriber_id = uuid::Uuid::new_v4().to_string();
        let subscriber_arn =
            format!("arn:aws:securitylake:{REGION}:{ACCOUNT_ID}:subscriber/{subscriber_id}");
        let created_at = chrono::Utc::now().to_rfc3339();

        let subscriber = Subscriber {
            subscriber_id: subscriber_id.clone(),
            subscriber_arn,
            subscriber_name,
            subscriber_description,
            access_types,
            sources,
            status: "ACTIVE".to_string(),
            created_at,
        };

        let resp = subscriber_to_json(&subscriber);
        state.subscribers.insert(subscriber_id, subscriber);

        Ok(rest_json::created(json!({ "subscriber": resp })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_subscribers(State(state): State<Arc<SecurityLakeState>>) -> Response {
    let subscribers: Vec<Value> = state
        .subscribers
        .iter()
        .map(|entry| subscriber_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "subscribers": subscribers }))
}

async fn get_subscriber(
    State(state): State<Arc<SecurityLakeState>>,
    Path(id): Path<String>,
) -> Response {
    match state.subscribers.get(&id) {
        Some(s) => rest_json::ok(json!({ "subscriber": subscriber_to_json(s.value()) })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Subscriber '{}' not found",
            id
        ))),
    }
}
