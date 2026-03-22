use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use base64::Engine;
use rand::Rng;
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
pub struct EcrRepository {
    pub repository_name: String,
    pub registry_id: String,
    pub arn: String,
    pub uri: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct EcrImage {
    pub image_digest: String,
    pub image_tag: Option<String>,
    pub pushed_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct EcrState {
    pub repositories: DashMap<String, EcrRepository>,
    pub images: DashMap<String, Vec<EcrImage>>,
}

impl Default for EcrState {
    fn default() -> Self {
        Self {
            repositories: DashMap::new(),
            images: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &EcrState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target
        .strip_prefix("AmazonEC2ContainerRegistry_V20150921.")
        .unwrap_or(target);

    let result = match action {
        "CreateRepository" => create_repository(state, payload),
        "DeleteRepository" => delete_repository(state, payload),
        "DescribeRepositories" => describe_repositories(state, payload),
        "ListImages" => list_images(state, payload),
        "PutImage" => put_image(state, payload),
        "BatchGetImage" => batch_get_image(state, payload),
        "GetAuthorizationToken" => get_authorization_token(state, payload),
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

fn generate_image_digest() -> String {
    let hex_chars: Vec<char> = "0123456789abcdef".chars().collect();
    let mut rng = rand::thread_rng();
    let hash: String = (0..64)
        .map(|_| {
            let idx = rng.gen_range(0..hex_chars.len());
            hex_chars[idx]
        })
        .collect();
    format!("sha256:{hash}")
}

fn repository_to_json(repo: &EcrRepository) -> Value {
    json!({
        "repositoryName": repo.repository_name,
        "registryId": repo.registry_id,
        "repositoryArn": repo.arn,
        "repositoryUri": repo.uri,
        "createdAt": repo.created_at
    })
}

fn image_to_json(image: &EcrImage) -> Value {
    let mut v = json!({
        "imageDigest": image.image_digest,
        "pushedAt": image.pushed_at
    });
    if let Some(tag) = &image.image_tag {
        v["imageTag"] = json!(tag);
    }
    v
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_repository(state: &EcrState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["repositoryName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("repositoryName is required".to_string()))?
        .to_string();

    if state.repositories.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Repository '{}' already exists",
            name
        )));
    }

    let arn = format!("arn:aws:ecr:{REGION}:{ACCOUNT_ID}:repository/{name}");
    let uri = format!("{ACCOUNT_ID}.dkr.ecr.{REGION}.amazonaws.com/{name}");
    let now = chrono::Utc::now().to_rfc3339();

    let repo = EcrRepository {
        repository_name: name.clone(),
        registry_id: ACCOUNT_ID.to_string(),
        arn,
        uri,
        created_at: now,
    };

    let resp = repository_to_json(&repo);
    state.repositories.insert(name.clone(), repo);
    state.images.insert(name, Vec::new());

    Ok(json_response(json!({ "repository": resp })))
}

fn delete_repository(state: &EcrState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["repositoryName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("repositoryName is required".to_string()))?;

    let (_, repo) = state
        .repositories
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Repository '{}' not found", name)))?;

    state.images.remove(name);

    Ok(json_response(json!({ "repository": repository_to_json(&repo) })))
}

fn describe_repositories(state: &EcrState, payload: &Value) -> Result<Response, LawsError> {
    let filter_names: Option<Vec<&str>> = payload["repositoryNames"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect());

    let repos: Vec<Value> = state
        .repositories
        .iter()
        .filter(|entry| {
            filter_names
                .as_ref()
                .map(|names| names.contains(&entry.key().as_str()))
                .unwrap_or(true)
        })
        .map(|entry| repository_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "repositories": repos })))
}

fn list_images(state: &EcrState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["repositoryName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("repositoryName is required".to_string()))?;

    if !state.repositories.contains_key(name) {
        return Err(LawsError::NotFound(format!(
            "Repository '{}' not found",
            name
        )));
    }

    let image_ids: Vec<Value> = state
        .images
        .get(name)
        .map(|imgs| {
            imgs.iter()
                .map(|img| {
                    let mut id = json!({ "imageDigest": img.image_digest });
                    if let Some(tag) = &img.image_tag {
                        id["imageTag"] = json!(tag);
                    }
                    id
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(json_response(json!({ "imageIds": image_ids })))
}

fn put_image(state: &EcrState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["repositoryName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("repositoryName is required".to_string()))?;

    if !state.repositories.contains_key(name) {
        return Err(LawsError::NotFound(format!(
            "Repository '{}' not found",
            name
        )));
    }

    let image_tag = payload["imageTag"].as_str().map(|s| s.to_string());
    let image_digest = generate_image_digest();
    let now = chrono::Utc::now().to_rfc3339();

    let image = EcrImage {
        image_digest: image_digest.clone(),
        image_tag: image_tag.clone(),
        pushed_at: now,
    };

    let resp = image_to_json(&image);

    if let Some(mut imgs) = state.images.get_mut(name) {
        imgs.push(image);
    }

    Ok(json_response(json!({ "image": resp })))
}

fn batch_get_image(state: &EcrState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["repositoryName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("repositoryName is required".to_string()))?;

    if !state.repositories.contains_key(name) {
        return Err(LawsError::NotFound(format!(
            "Repository '{}' not found",
            name
        )));
    }

    let requested_ids = payload["imageIds"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut images = Vec::new();
    let mut failures = Vec::new();

    if let Some(repo_images) = state.images.get(name) {
        for req_id in &requested_ids {
            let digest = req_id["imageDigest"].as_str();
            let tag = req_id["imageTag"].as_str();

            let found = repo_images.iter().find(|img| {
                if let Some(d) = digest {
                    if img.image_digest == d {
                        return true;
                    }
                }
                if let Some(t) = tag {
                    if img.image_tag.as_deref() == Some(t) {
                        return true;
                    }
                }
                false
            });

            match found {
                Some(img) => images.push(image_to_json(img)),
                None => failures.push(json!({
                    "imageId": req_id,
                    "failureCode": "ImageNotFound",
                    "failureReason": "Requested image not found"
                })),
            }
        }
    }

    Ok(json_response(json!({
        "images": images,
        "failures": failures
    })))
}

fn get_authorization_token(_state: &EcrState, _payload: &Value) -> Result<Response, LawsError> {
    let token = base64::engine::general_purpose::STANDARD.encode(format!("AWS:{}", "mock-password"));
    let endpoint = format!("https://{ACCOUNT_ID}.dkr.ecr.{REGION}.amazonaws.com");

    Ok(json_response(json!({
        "authorizationData": [{
            "authorizationToken": token,
            "expiresAt": chrono::Utc::now().timestamp() + 43200,
            "proxyEndpoint": endpoint
        }]
    })))
}
