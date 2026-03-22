use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{delete, get, put};
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
pub struct BackupVault {
    pub name: String,
    pub arn: String,
    pub creation_date: String,
    pub number_of_recovery_points: u64,
}

#[derive(Debug, Clone)]
pub struct BackupPlan {
    pub plan_id: String,
    pub plan_name: String,
    pub arn: String,
    pub creation_date: String,
    pub rules: Value,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct BackupState {
    pub vaults: DashMap<String, BackupVault>,
    pub plans: DashMap<String, BackupPlan>,
}

impl Default for BackupState {
    fn default() -> Self {
        Self {
            vaults: DashMap::new(),
            plans: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<BackupState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/backup-vaults/{name}",
            put(create_backup_vault)
                .delete(delete_backup_vault)
                .get(describe_backup_vault),
        )
        .route("/backup-vaults", get(list_backup_vaults))
        .route(
            "/backup/plans",
            put(create_backup_plan).get(list_backup_plans),
        )
        .route("/backup/plans/{id}", delete(delete_backup_plan))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn vault_to_json(v: &BackupVault) -> Value {
    json!({
        "BackupVaultName": v.name,
        "BackupVaultArn": v.arn,
        "CreationDate": v.creation_date,
        "NumberOfRecoveryPoints": v.number_of_recovery_points,
    })
}

fn plan_to_json(p: &BackupPlan) -> Value {
    json!({
        "BackupPlanId": p.plan_id,
        "BackupPlanName": p.plan_name,
        "BackupPlanArn": p.arn,
        "CreationDate": p.creation_date,
        "BackupPlanRules": p.rules,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_backup_vault(
    State(state): State<Arc<BackupState>>,
    Path(name): Path<String>,
    Json(_payload): Json<Value>,
) -> Response {
    let arn = format!(
        "arn:aws:backup:{REGION}:{ACCOUNT_ID}:backup-vault:{name}"
    );
    let now = chrono::Utc::now().to_rfc3339();

    let vault = BackupVault {
        name: name.clone(),
        arn: arn.clone(),
        creation_date: now.clone(),
        number_of_recovery_points: 0,
    };

    let resp = vault_to_json(&vault);
    state.vaults.insert(name, vault);

    rest_json::created(resp)
}

async fn delete_backup_vault(
    State(state): State<Arc<BackupState>>,
    Path(name): Path<String>,
) -> Response {
    match state.vaults.remove(&name) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Backup vault '{}' not found",
            name
        ))),
    }
}

async fn list_backup_vaults(State(state): State<Arc<BackupState>>) -> Response {
    let vaults: Vec<Value> = state
        .vaults
        .iter()
        .map(|entry| vault_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "BackupVaultList": vaults }))
}

async fn describe_backup_vault(
    State(state): State<Arc<BackupState>>,
    Path(name): Path<String>,
) -> Response {
    match state.vaults.get(&name) {
        Some(vault) => rest_json::ok(vault_to_json(vault.value())),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Backup vault '{}' not found",
            name
        ))),
    }
}

async fn create_backup_plan(
    State(state): State<Arc<BackupState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let backup_plan = payload
            .get("BackupPlan")
            .ok_or_else(|| LawsError::InvalidRequest("Missing BackupPlan".into()))?;

        let plan_name = backup_plan["BackupPlanName"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing BackupPlanName".into()))?
            .to_string();

        let rules = backup_plan
            .get("Rules")
            .cloned()
            .unwrap_or(json!([]));

        let plan_id = uuid::Uuid::new_v4().to_string();
        let arn = format!(
            "arn:aws:backup:{REGION}:{ACCOUNT_ID}:backup-plan:{plan_id}"
        );
        let now = chrono::Utc::now().to_rfc3339();

        let plan = BackupPlan {
            plan_id: plan_id.clone(),
            plan_name,
            arn: arn.clone(),
            creation_date: now,
            rules,
        };

        let resp = json!({
            "BackupPlanId": plan_id,
            "BackupPlanArn": arn,
        });

        state.plans.insert(plan_id, plan);

        Ok(rest_json::created(resp))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_backup_plans(State(state): State<Arc<BackupState>>) -> Response {
    let plans: Vec<Value> = state
        .plans
        .iter()
        .map(|entry| plan_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "BackupPlansList": plans }))
}

async fn delete_backup_plan(
    State(state): State<Arc<BackupState>>,
    Path(id): Path<String>,
) -> Response {
    match state.plans.remove(&id) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Backup plan '{}' not found",
            id
        ))),
    }
}
