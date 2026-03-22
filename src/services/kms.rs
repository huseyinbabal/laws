use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use std::sync::Arc;

use base64::Engine;

use crate::error::LawsError;

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct KmsKey {
    pub key_id: String,
    pub arn: String,
    pub description: String,
    pub key_usage: String,
    pub key_spec: String,
    pub key_state: String,
    pub creation_date: f64,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub struct KmsAlias {
    pub alias_name: String,
    pub alias_arn: String,
    pub target_key_id: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct KmsState {
    pub keys: Arc<DashMap<String, KmsKey>>,
    pub aliases: Arc<DashMap<String, KmsAlias>>,
}

impl Default for KmsState {
    fn default() -> Self {
        Self {
            keys: Arc::new(DashMap::new()),
            aliases: Arc::new(DashMap::new()),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &KmsState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target.strip_prefix("TrentService.").unwrap_or(target);

    let result = match action {
        "CreateKey" => create_key(state, payload).await,
        "DescribeKey" => describe_key(state, payload).await,
        "ListKeys" => list_keys(state).await,
        "EnableKey" => enable_key(state, payload).await,
        "DisableKey" => disable_key(state, payload).await,
        "ScheduleKeyDeletion" => schedule_key_deletion(state, payload).await,
        "Encrypt" => encrypt(state, payload).await,
        "Decrypt" => decrypt(state, payload).await,
        "GenerateDataKey" => generate_data_key(state, payload).await,
        "CreateAlias" => create_alias(state, payload).await,
        "ListAliases" => list_aliases(state).await,
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

fn json_response(body: serde_json::Value) -> Response {
    (
        StatusCode::OK,
        [("Content-Type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn make_arn(key_id: &str) -> String {
    format!("arn:aws:kms:us-east-1:000000000000:key/{}", key_id)
}

/// Resolve a KeyId parameter which may be a raw key ID, an ARN, or an alias name.
fn resolve_key(state: &KmsState, key_id_input: &str) -> Result<KmsKey, LawsError> {
    // Direct key ID lookup
    if let Some(key) = state.keys.get(key_id_input) {
        return Ok(key.clone());
    }

    // ARN lookup
    for entry in state.keys.iter() {
        if entry.value().arn == key_id_input {
            return Ok(entry.value().clone());
        }
    }

    // Alias lookup
    if let Some(alias) = state.aliases.get(key_id_input) {
        if let Some(key) = state.keys.get(&alias.target_key_id) {
            return Ok(key.clone());
        }
    }

    Err(LawsError::NotFound(format!(
        "Key '{}' not found",
        key_id_input
    )))
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

async fn create_key(state: &KmsState, payload: &serde_json::Value) -> Result<Response, LawsError> {
    let description = payload["Description"].as_str().unwrap_or("").to_string();
    let key_usage = payload["KeyUsage"]
        .as_str()
        .unwrap_or("ENCRYPT_DECRYPT")
        .to_string();
    let key_spec = payload["KeySpec"]
        .as_str()
        .unwrap_or("SYMMETRIC_DEFAULT")
        .to_string();

    let key_id = uuid::Uuid::new_v4().to_string();
    let arn = make_arn(&key_id);
    let now = chrono::Utc::now().timestamp() as f64;

    let key = KmsKey {
        key_id: key_id.clone(),
        arn: arn.clone(),
        description: description.clone(),
        key_usage: key_usage.clone(),
        key_spec: key_spec.clone(),
        key_state: "Enabled".to_string(),
        creation_date: now,
        enabled: true,
    };

    state.keys.insert(key_id.clone(), key);

    Ok(json_response(serde_json::json!({
        "KeyMetadata": {
            "KeyId": key_id,
            "Arn": arn,
            "Description": description,
            "KeyUsage": key_usage,
            "KeySpec": key_spec,
            "KeyState": "Enabled",
            "CreationDate": now,
            "Enabled": true,
            "KeyManager": "CUSTOMER"
        }
    })))
}

async fn describe_key(
    state: &KmsState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let key_id_input = payload["KeyId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("KeyId is required".to_string()))?;

    let key = resolve_key(state, key_id_input)?;

    Ok(json_response(serde_json::json!({
        "KeyMetadata": {
            "KeyId": key.key_id,
            "Arn": key.arn,
            "Description": key.description,
            "KeyUsage": key.key_usage,
            "KeySpec": key.key_spec,
            "KeyState": key.key_state,
            "CreationDate": key.creation_date,
            "Enabled": key.enabled,
            "KeyManager": "CUSTOMER"
        }
    })))
}

async fn list_keys(state: &KmsState) -> Result<Response, LawsError> {
    let keys: Vec<serde_json::Value> = state
        .keys
        .iter()
        .map(|entry| {
            let key = entry.value();
            serde_json::json!({
                "KeyId": key.key_id,
                "KeyArn": key.arn,
            })
        })
        .collect();

    Ok(json_response(serde_json::json!({
        "Keys": keys,
        "Truncated": false,
    })))
}

async fn enable_key(state: &KmsState, payload: &serde_json::Value) -> Result<Response, LawsError> {
    let key_id_input = payload["KeyId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("KeyId is required".to_string()))?;

    let key = resolve_key(state, key_id_input)?;

    if let Some(mut entry) = state.keys.get_mut(&key.key_id) {
        entry.key_state = "Enabled".to_string();
        entry.enabled = true;
    }

    Ok(json_response(serde_json::json!({})))
}

async fn disable_key(state: &KmsState, payload: &serde_json::Value) -> Result<Response, LawsError> {
    let key_id_input = payload["KeyId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("KeyId is required".to_string()))?;

    let key = resolve_key(state, key_id_input)?;

    if let Some(mut entry) = state.keys.get_mut(&key.key_id) {
        entry.key_state = "Disabled".to_string();
        entry.enabled = false;
    }

    Ok(json_response(serde_json::json!({})))
}

async fn schedule_key_deletion(
    state: &KmsState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let key_id_input = payload["KeyId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("KeyId is required".to_string()))?;

    let pending_days = payload["PendingWindowInDays"].as_u64().unwrap_or(30);

    let key = resolve_key(state, key_id_input)?;

    if let Some(mut entry) = state.keys.get_mut(&key.key_id) {
        entry.key_state = "PendingDeletion".to_string();
        entry.enabled = false;
    }

    let deletion_date = chrono::Utc::now().timestamp() as f64 + (pending_days as f64 * 86400.0);

    Ok(json_response(serde_json::json!({
        "KeyId": key.key_id,
        "KeyState": "PendingDeletion",
        "DeletionDate": deletion_date,
    })))
}

async fn encrypt(state: &KmsState, payload: &serde_json::Value) -> Result<Response, LawsError> {
    let key_id_input = payload["KeyId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("KeyId is required".to_string()))?;
    let plaintext = payload["Plaintext"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Plaintext is required".to_string()))?;

    let key = resolve_key(state, key_id_input)?;

    let combined = format!("ENCRYPTED:{}:{}", key.key_id, plaintext);
    let ciphertext_blob = base64::engine::general_purpose::STANDARD.encode(combined.as_bytes());

    Ok(json_response(serde_json::json!({
        "CiphertextBlob": ciphertext_blob,
        "KeyId": key.key_id,
        "EncryptionAlgorithm": "SYMMETRIC_DEFAULT",
    })))
}

async fn decrypt(state: &KmsState, payload: &serde_json::Value) -> Result<Response, LawsError> {
    let ciphertext_blob = payload["CiphertextBlob"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("CiphertextBlob is required".to_string()))?;

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(ciphertext_blob)
        .map_err(|e| LawsError::InvalidRequest(format!("Invalid base64: {}", e)))?;

    let decoded_str = String::from_utf8(decoded)
        .map_err(|e| LawsError::InvalidRequest(format!("Invalid UTF-8: {}", e)))?;

    // Expected format: ENCRYPTED:{key_id}:{original_plaintext}
    let parts: Vec<&str> = decoded_str.splitn(3, ':').collect();
    if parts.len() != 3 || parts[0] != "ENCRYPTED" {
        return Err(LawsError::InvalidRequest(
            "Invalid ciphertext format".to_string(),
        ));
    }

    let key_id = parts[1];
    let original_plaintext = parts[2];

    // Verify key exists
    let _key = resolve_key(state, key_id)?;

    Ok(json_response(serde_json::json!({
        "Plaintext": original_plaintext,
        "KeyId": key_id,
        "EncryptionAlgorithm": "SYMMETRIC_DEFAULT",
    })))
}

async fn generate_data_key(
    state: &KmsState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let key_id_input = payload["KeyId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("KeyId is required".to_string()))?;
    let key_spec = payload["KeySpec"].as_str().unwrap_or("AES_256");

    let key = resolve_key(state, key_id_input)?;

    let num_bytes: usize = match key_spec {
        "AES_128" => 16,
        "AES_256" => 32,
        _ => {
            return Err(LawsError::InvalidRequest(format!(
                "Unsupported KeySpec: {}",
                key_spec
            )))
        }
    };

    use rand::Rng;
    let mut plaintext_bytes = vec![0u8; num_bytes];
    rand::rng().fill_bytes(&mut plaintext_bytes);

    let plaintext_b64 = base64::engine::general_purpose::STANDARD.encode(&plaintext_bytes);

    // Mock ciphertext: just the encrypted version of the plaintext
    let combined = format!("ENCRYPTED:{}:{}", key.key_id, plaintext_b64);
    let ciphertext_b64 = base64::engine::general_purpose::STANDARD.encode(combined.as_bytes());

    Ok(json_response(serde_json::json!({
        "CiphertextBlob": ciphertext_b64,
        "Plaintext": plaintext_b64,
        "KeyId": key.key_id,
    })))
}

async fn create_alias(
    state: &KmsState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let alias_name = payload["AliasName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("AliasName is required".to_string()))?;
    let target_key_id = payload["TargetKeyId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("TargetKeyId is required".to_string()))?;

    // Verify target key exists
    let _key = resolve_key(state, target_key_id)?;

    let alias_arn = format!("arn:aws:kms:us-east-1:000000000000:{}", alias_name);

    let alias = KmsAlias {
        alias_name: alias_name.to_string(),
        alias_arn,
        target_key_id: target_key_id.to_string(),
    };

    state.aliases.insert(alias_name.to_string(), alias);

    Ok(json_response(serde_json::json!({})))
}

async fn list_aliases(state: &KmsState) -> Result<Response, LawsError> {
    let aliases: Vec<serde_json::Value> = state
        .aliases
        .iter()
        .map(|entry| {
            let alias = entry.value();
            serde_json::json!({
                "AliasName": alias.alias_name,
                "AliasArn": alias.alias_arn,
                "TargetKeyId": alias.target_key_id,
            })
        })
        .collect();

    Ok(json_response(serde_json::json!({
        "Aliases": aliases,
        "Truncated": false,
    })))
}
