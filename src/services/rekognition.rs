use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RekCollection {
    pub collection_id: String,
    pub arn: String,
    pub face_count: u64,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct RekognitionState {
    pub collections: DashMap<String, RekCollection>,
}

impl Default for RekognitionState {
    fn default() -> Self {
        Self {
            collections: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &RekognitionState, target: &str, payload: &serde_json::Value) -> Response {
    let action = target
        .strip_prefix("RekognitionService.")
        .unwrap_or(target);

    let result = match action {
        "DetectLabels" => detect_labels(),
        "DetectFaces" => detect_faces(),
        "DetectText" => detect_text(),
        "CompareFaces" => compare_faces(),
        "CreateCollection" => create_collection(state, payload),
        "DeleteCollection" => delete_collection(state, payload),
        "ListCollections" => list_collections(state),
        "IndexFaces" => index_faces(state, payload),
        "SearchFacesByImage" => search_faces_by_image(state, payload),
        other => Err(LawsError::InvalidRequest(format!("unknown action: {other}"))),
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
    (StatusCode::OK, [("Content-Type", "application/x-amz-json-1.1")], serde_json::to_string(&body).unwrap_or_default()).into_response()
}

fn require_str<'a>(body: &'a Value, field: &str) -> Result<&'a str, LawsError> {
    body.get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest(format!("missing required field: {field}")))
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn detect_labels() -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "Labels": [{
            "Name": "Object",
            "Confidence": 99.5,
            "Instances": [],
            "Parents": []
        }],
        "LabelModelVersion": "3.0"
    })))
}

fn detect_faces() -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "FaceDetails": []
    })))
}

fn detect_text() -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "TextDetections": []
    })))
}

fn compare_faces() -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "FaceMatches": [],
        "UnmatchedFaces": [],
        "SourceImageFace": {
            "Confidence": 99.9,
            "BoundingBox": {
                "Width": 0.5,
                "Height": 0.5,
                "Left": 0.1,
                "Top": 0.1
            }
        }
    })))
}

fn create_collection(state: &RekognitionState, body: &Value) -> Result<Response, LawsError> {
    let collection_id = require_str(body, "CollectionId")?.to_owned();

    if state.collections.contains_key(&collection_id) {
        return Err(LawsError::AlreadyExists(format!(
            "collection already exists: {collection_id}"
        )));
    }

    let arn = format!(
        "arn:aws:rekognition:{REGION}:{ACCOUNT_ID}:collection/{collection_id}"
    );
    let created_at = chrono::Utc::now().to_rfc3339();

    let collection = RekCollection {
        collection_id: collection_id.clone(),
        arn: arn.clone(),
        face_count: 0,
        created_at,
    };

    state.collections.insert(collection_id, collection);

    Ok(json_response(json!({
        "CollectionArn": arn,
        "FaceModelVersion": "6.0",
        "StatusCode": 200
    })))
}

fn delete_collection(state: &RekognitionState, body: &Value) -> Result<Response, LawsError> {
    let collection_id = require_str(body, "CollectionId")?;
    state
        .collections
        .remove(collection_id)
        .ok_or_else(|| LawsError::NotFound(format!("collection not found: {collection_id}")))?;

    Ok(json_response(json!({
        "StatusCode": 200
    })))
}

fn list_collections(state: &RekognitionState) -> Result<Response, LawsError> {
    let ids: Vec<String> = state
        .collections
        .iter()
        .map(|entry| entry.key().clone())
        .collect();

    Ok(json_response(json!({
        "CollectionIds": ids,
        "FaceModelVersions": ids.iter().map(|_| "6.0").collect::<Vec<_>>()
    })))
}

fn index_faces(state: &RekognitionState, body: &Value) -> Result<Response, LawsError> {
    let collection_id = require_str(body, "CollectionId")?;

    let mut collection = state
        .collections
        .get_mut(collection_id)
        .ok_or_else(|| LawsError::NotFound(format!("collection not found: {collection_id}")))?;

    let face_id = uuid::Uuid::new_v4().to_string();
    collection.face_count += 1;

    Ok(json_response(json!({
        "FaceRecords": [{
            "Face": {
                "FaceId": face_id,
                "BoundingBox": {
                    "Width": 0.5,
                    "Height": 0.5,
                    "Left": 0.1,
                    "Top": 0.1
                },
                "Confidence": 99.9,
                "ImageId": uuid::Uuid::new_v4().to_string()
            },
            "FaceDetail": {
                "BoundingBox": {
                    "Width": 0.5,
                    "Height": 0.5,
                    "Left": 0.1,
                    "Top": 0.1
                },
                "Confidence": 99.9
            }
        }],
        "FaceModelVersion": "6.0",
        "UnindexedFaces": []
    })))
}

fn search_faces_by_image(state: &RekognitionState, body: &Value) -> Result<Response, LawsError> {
    let collection_id = require_str(body, "CollectionId")?;

    if !state.collections.contains_key(collection_id) {
        return Err(LawsError::NotFound(format!(
            "collection not found: {collection_id}"
        )));
    }

    Ok(json_response(json!({
        "SearchedFaceBoundingBox": {
            "Width": 0.5,
            "Height": 0.5,
            "Left": 0.1,
            "Top": 0.1
        },
        "SearchedFaceConfidence": 99.9,
        "FaceMatches": [],
        "FaceModelVersion": "6.0"
    })))
}
