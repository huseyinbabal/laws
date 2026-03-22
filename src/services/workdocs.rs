use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{delete, get, post};
use axum::Json;
use chrono::Utc;
use dashmap::DashMap;
use serde::Deserialize;
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
pub struct Folder {
    pub folder_id: String,
    pub name: String,
    pub parent_folder_id: String,
    pub creator_id: String,
    pub created_timestamp: String,
    pub modified_timestamp: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub document_id: String,
    pub name: String,
    pub parent_folder_id: String,
    pub creator_id: String,
    pub created_timestamp: String,
    pub modified_timestamp: String,
    pub latest_version_status: String,
}

#[derive(Debug, Clone)]
pub struct User {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub given_name: String,
    pub surname: String,
    pub status: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct WorkDocsState {
    pub folders: DashMap<String, Folder>,
    pub documents: DashMap<String, Document>,
    pub users: DashMap<String, User>,
}

impl Default for WorkDocsState {
    fn default() -> Self {
        Self {
            folders: DashMap::new(),
            documents: DashMap::new(),
            users: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<WorkDocsState>) -> axum::Router {
    axum::Router::new()
        .route("/api/v1/folders", post(create_folder))
        .route(
            "/api/v1/folders/{folder_id}",
            delete(delete_folder),
        )
        .route(
            "/api/v1/folders/{folder_id}/contents",
            get(describe_folder_contents),
        )
        .route(
            "/api/v1/documents",
            post(initiate_document_version_upload),
        )
        .route("/api/v1/documents/{document_id}", get(get_document))
        .route("/api/v1/users", get(describe_users))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateFolderRequest {
    #[serde(alias = "Name")]
    name: String,
    #[serde(alias = "ParentFolderId")]
    parent_folder_id: String,
}

async fn create_folder(
    State(state): State<Arc<WorkDocsState>>,
    Json(req): Json<CreateFolderRequest>,
) -> Response {
    let folder_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let folder = Folder {
        folder_id: folder_id.clone(),
        name: req.name.clone(),
        parent_folder_id: req.parent_folder_id.clone(),
        creator_id: "anonymous".to_string(),
        created_timestamp: now.clone(),
        modified_timestamp: now.clone(),
        status: "ACTIVE".to_string(),
    };

    let resp = json!({
        "Metadata": {
            "Id": folder.folder_id,
            "Name": folder.name,
            "ParentFolderId": folder.parent_folder_id,
            "CreatorId": folder.creator_id,
            "CreatedTimestamp": folder.created_timestamp,
            "ModifiedTimestamp": folder.modified_timestamp,
            "ResourceState": folder.status,
        }
    });

    state.folders.insert(folder_id, folder);
    rest_json::created(resp)
}

async fn delete_folder(
    State(state): State<Arc<WorkDocsState>>,
    Path(folder_id): Path<String>,
) -> Response {
    match state.folders.remove(&folder_id) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Folder not found: {folder_id}"
        ))),
    }
}

async fn describe_folder_contents(
    State(state): State<Arc<WorkDocsState>>,
    Path(folder_id): Path<String>,
) -> Response {
    if !state.folders.contains_key(&folder_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "Folder not found: {folder_id}"
        )));
    }

    let folders: Vec<Value> = state
        .folders
        .iter()
        .filter(|entry| entry.value().parent_folder_id == folder_id)
        .map(|entry| {
            let f = entry.value();
            json!({
                "Id": f.folder_id,
                "Name": f.name,
                "ResourceState": f.status,
            })
        })
        .collect();

    let documents: Vec<Value> = state
        .documents
        .iter()
        .filter(|entry| entry.value().parent_folder_id == folder_id)
        .map(|entry| {
            let d = entry.value();
            json!({
                "Id": d.document_id,
                "Name": d.name,
                "CreatedTimestamp": d.created_timestamp,
            })
        })
        .collect();

    rest_json::ok(json!({
        "Folders": folders,
        "Documents": documents,
    }))
}

#[derive(Deserialize)]
struct InitiateDocumentVersionUploadRequest {
    #[serde(alias = "Name")]
    name: String,
    #[serde(alias = "ParentFolderId")]
    parent_folder_id: String,
}

async fn initiate_document_version_upload(
    State(state): State<Arc<WorkDocsState>>,
    Json(req): Json<InitiateDocumentVersionUploadRequest>,
) -> Response {
    let document_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let document = Document {
        document_id: document_id.clone(),
        name: req.name.clone(),
        parent_folder_id: req.parent_folder_id.clone(),
        creator_id: "anonymous".to_string(),
        created_timestamp: now.clone(),
        modified_timestamp: now.clone(),
        latest_version_status: "ACTIVE".to_string(),
    };

    let upload_url = format!(
        "https://workdocs.{REGION}.amazonaws.com/upload/{document_id}"
    );
    let version_id = uuid::Uuid::new_v4().to_string();

    let resp = json!({
        "Metadata": {
            "Id": document.document_id,
            "Name": document.name,
            "ParentFolderId": document.parent_folder_id,
            "CreatorId": document.creator_id,
            "CreatedTimestamp": document.created_timestamp,
            "ModifiedTimestamp": document.modified_timestamp,
        },
        "UploadMetadata": {
            "UploadUrl": upload_url,
            "SignedHeaders": {},
        },
        "VersionId": version_id,
    });

    state.documents.insert(document_id, document);
    rest_json::created(resp)
}

async fn get_document(
    State(state): State<Arc<WorkDocsState>>,
    Path(document_id): Path<String>,
) -> Response {
    match state.documents.get(&document_id) {
        Some(d) => rest_json::ok(json!({
            "Metadata": {
                "Id": d.document_id,
                "Name": d.name,
                "ParentFolderId": d.parent_folder_id,
                "CreatorId": d.creator_id,
                "CreatedTimestamp": d.created_timestamp,
                "ModifiedTimestamp": d.modified_timestamp,
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Document not found: {document_id}"
        ))),
    }
}

async fn describe_users(State(state): State<Arc<WorkDocsState>>) -> Response {
    let users: Vec<Value> = state
        .users
        .iter()
        .map(|entry| {
            let u = entry.value();
            json!({
                "Id": u.user_id,
                "Username": u.username,
                "EmailAddress": u.email,
                "GivenName": u.given_name,
                "Surname": u.surname,
                "Status": u.status,
            })
        })
        .collect();

    rest_json::ok(json!({ "Users": users }))
}
