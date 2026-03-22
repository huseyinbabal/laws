use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
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
pub struct Vault {
    pub vault_name: String,
    pub vault_arn: String,
    pub creation_date: String,
    pub number_of_archives: u64,
    pub size_in_bytes: u64,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct GlacierState {
    pub vaults: DashMap<String, Vault>,
}

impl Default for GlacierState {
    fn default() -> Self {
        Self {
            vaults: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<GlacierState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/{accountId}/vaults/{name}",
            axum::routing::put(create_vault)
                .get(describe_vault)
                .delete(delete_vault),
        )
        .route("/{accountId}/vaults", axum::routing::get(list_vaults))
        .route(
            "/{accountId}/vaults/{name}/archives",
            axum::routing::post(upload_archive),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_vault(
    State(state): State<Arc<GlacierState>>,
    Path((_account_id, name)): Path<(String, String)>,
) -> Response {
    let vault_arn = format!("arn:aws:glacier:{REGION}:{ACCOUNT_ID}:vaults/{name}");
    let creation_date = chrono::Utc::now().to_rfc3339();

    let vault = Vault {
        vault_name: name.clone(),
        vault_arn: vault_arn.clone(),
        creation_date,
        number_of_archives: 0,
        size_in_bytes: 0,
    };

    state.vaults.insert(name.clone(), vault);

    rest_json::created(json!({
        "location": format!("/{ACCOUNT_ID}/vaults/{name}")
    }))
}

async fn list_vaults(
    State(state): State<Arc<GlacierState>>,
    Path(_account_id): Path<String>,
) -> Response {
    let vault_list: Vec<Value> = state
        .vaults
        .iter()
        .map(|entry| {
            let v = entry.value();
            json!({
                "VaultName": v.vault_name,
                "VaultARN": v.vault_arn,
                "CreationDate": v.creation_date,
                "NumberOfArchives": v.number_of_archives,
                "SizeInBytes": v.size_in_bytes
            })
        })
        .collect();

    rest_json::ok(json!({
        "VaultList": vault_list
    }))
}

async fn describe_vault(
    State(state): State<Arc<GlacierState>>,
    Path((_account_id, name)): Path<(String, String)>,
) -> Response {
    match state.vaults.get(&name) {
        Some(v) => rest_json::ok(json!({
            "VaultName": v.vault_name,
            "VaultARN": v.vault_arn,
            "CreationDate": v.creation_date,
            "NumberOfArchives": v.number_of_archives,
            "SizeInBytes": v.size_in_bytes
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!("vault not found: {name}"))),
    }
}

async fn delete_vault(
    State(state): State<Arc<GlacierState>>,
    Path((_account_id, name)): Path<(String, String)>,
) -> Response {
    match state.vaults.remove(&name) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!("vault not found: {name}"))),
    }
}

async fn upload_archive(
    State(state): State<Arc<GlacierState>>,
    Path((_account_id, name)): Path<(String, String)>,
) -> Response {
    match state.vaults.get_mut(&name) {
        Some(mut v) => {
            v.number_of_archives += 1;
            let archive_id = uuid::Uuid::new_v4().to_string();

            rest_json::created(json!({
                "location": format!("/{ACCOUNT_ID}/vaults/{name}/archives/{archive_id}"),
                "archiveId": archive_id,
                "x-amz-sha256-tree-hash": "0000000000000000000000000000000000000000000000000000000000000000"
            }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!("vault not found: {name}"))),
    }
}
