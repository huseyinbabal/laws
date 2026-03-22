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
pub struct FsxFileSystem {
    pub file_system_id: String,
    pub arn: String,
    pub file_system_type: String,
    pub storage_capacity: u64,
    pub storage_type: String,
    pub lifecycle: String,
    pub created_at: f64,
}

#[derive(Debug, Clone)]
pub struct FsxBackup {
    pub backup_id: String,
    pub arn: String,
    pub file_system_id: String,
    pub backup_type: String,
    pub lifecycle: String,
    pub created_at: f64,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct FsxState {
    pub file_systems: DashMap<String, FsxFileSystem>,
    pub backups: DashMap<String, FsxBackup>,
}

impl Default for FsxState {
    fn default() -> Self {
        Self {
            file_systems: DashMap::new(),
            backups: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &FsxState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("AWSSimbaAPIService_v20180301.")
        .unwrap_or(target);

    let result = match action {
        "CreateFileSystem" => create_file_system(state, payload),
        "DeleteFileSystem" => delete_file_system(state, payload),
        "DescribeFileSystems" => describe_file_systems(state, payload),
        "UpdateFileSystem" => update_file_system(state, payload),
        "CreateBackup" => create_backup(state, payload),
        "DeleteBackup" => delete_backup(state, payload),
        "DescribeBackups" => describe_backups(state, payload),
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

fn file_system_to_json(fs: &FsxFileSystem) -> Value {
    json!({
        "FileSystemId": fs.file_system_id,
        "ResourceARN": fs.arn,
        "FileSystemType": fs.file_system_type,
        "StorageCapacity": fs.storage_capacity,
        "StorageType": fs.storage_type,
        "Lifecycle": fs.lifecycle,
        "CreationTime": fs.created_at,
        "OwnerId": ACCOUNT_ID,
    })
}

fn backup_to_json(backup: &FsxBackup) -> Value {
    json!({
        "BackupId": backup.backup_id,
        "ResourceARN": backup.arn,
        "FileSystemId": backup.file_system_id,
        "Type": backup.backup_type,
        "Lifecycle": backup.lifecycle,
        "CreationTime": backup.created_at,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_file_system(state: &FsxState, payload: &Value) -> Result<Response, LawsError> {
    let file_system_type = payload["FileSystemType"]
        .as_str()
        .unwrap_or("LUSTRE")
        .to_string();

    let storage_capacity = payload["StorageCapacity"].as_u64().unwrap_or(1200);

    let storage_type = payload["StorageType"].as_str().unwrap_or("SSD").to_string();

    let file_system_id = format!(
        "fs-{}",
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..17]
    );

    let arn = format!("arn:aws:fsx:{REGION}:{ACCOUNT_ID}:file-system/{file_system_id}");

    let created_at = now_epoch();

    let fs = FsxFileSystem {
        file_system_id: file_system_id.clone(),
        arn,
        file_system_type,
        storage_capacity,
        storage_type,
        lifecycle: "AVAILABLE".to_string(),
        created_at,
    };

    let resp = file_system_to_json(&fs);
    state.file_systems.insert(file_system_id, fs);

    Ok(json_response(json!({
        "FileSystem": resp,
    })))
}

fn delete_file_system(state: &FsxState, payload: &Value) -> Result<Response, LawsError> {
    let file_system_id = payload["FileSystemId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("FileSystemId is required".to_string()))?;

    let (_, fs) = state.file_systems.remove(file_system_id).ok_or_else(|| {
        LawsError::NotFound(format!("File system '{}' not found", file_system_id))
    })?;

    // Remove associated backups
    state
        .backups
        .retain(|_, b| b.file_system_id != file_system_id);

    Ok(json_response(json!({
        "FileSystemId": fs.file_system_id,
        "Lifecycle": "DELETING",
    })))
}

fn describe_file_systems(state: &FsxState, payload: &Value) -> Result<Response, LawsError> {
    let file_system_ids = payload["FileSystemIds"].as_array();

    let file_systems: Vec<Value> = state
        .file_systems
        .iter()
        .filter(|entry| match file_system_ids {
            Some(ids) => ids
                .iter()
                .any(|id| id.as_str() == Some(entry.key().as_str())),
            None => true,
        })
        .map(|entry| file_system_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "FileSystems": file_systems })))
}

fn update_file_system(state: &FsxState, payload: &Value) -> Result<Response, LawsError> {
    let file_system_id = payload["FileSystemId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("FileSystemId is required".to_string()))?;

    let mut fs = state.file_systems.get_mut(file_system_id).ok_or_else(|| {
        LawsError::NotFound(format!("File system '{}' not found", file_system_id))
    })?;

    if let Some(capacity) = payload["StorageCapacity"].as_u64() {
        fs.storage_capacity = capacity;
    }

    Ok(json_response(json!({
        "FileSystem": file_system_to_json(&fs),
    })))
}

fn create_backup(state: &FsxState, payload: &Value) -> Result<Response, LawsError> {
    let file_system_id = payload["FileSystemId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("FileSystemId is required".to_string()))?
        .to_string();

    if !state.file_systems.contains_key(&file_system_id) {
        return Err(LawsError::NotFound(format!(
            "File system '{}' not found",
            file_system_id
        )));
    }

    let backup_id = format!(
        "backup-{}",
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..17]
    );

    let arn = format!("arn:aws:fsx:{REGION}:{ACCOUNT_ID}:backup/{backup_id}");

    let created_at = now_epoch();

    let backup = FsxBackup {
        backup_id: backup_id.clone(),
        arn,
        file_system_id,
        backup_type: "USER_INITIATED".to_string(),
        lifecycle: "AVAILABLE".to_string(),
        created_at,
    };

    let resp = backup_to_json(&backup);
    state.backups.insert(backup_id, backup);

    Ok(json_response(json!({
        "Backup": resp,
    })))
}

fn delete_backup(state: &FsxState, payload: &Value) -> Result<Response, LawsError> {
    let backup_id = payload["BackupId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("BackupId is required".to_string()))?;

    state
        .backups
        .remove(backup_id)
        .ok_or_else(|| LawsError::NotFound(format!("Backup '{}' not found", backup_id)))?;

    Ok(json_response(json!({
        "BackupId": backup_id,
        "Lifecycle": "DELETED",
    })))
}

fn describe_backups(state: &FsxState, payload: &Value) -> Result<Response, LawsError> {
    let backup_ids = payload["BackupIds"].as_array();

    let backups: Vec<Value> = state
        .backups
        .iter()
        .filter(|entry| match backup_ids {
            Some(ids) => ids
                .iter()
                .any(|id| id.as_str() == Some(entry.key().as_str())),
            None => true,
        })
        .map(|entry| backup_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "Backups": backups })))
}
