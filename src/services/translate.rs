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
pub struct Terminology {
    pub name: String,
    pub arn: String,
    pub source_language: String,
    pub target_languages: Vec<String>,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct TranslateState {
    pub terminologies: DashMap<String, Terminology>,
}

impl Default for TranslateState {
    fn default() -> Self {
        Self {
            terminologies: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &TranslateState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target
        .strip_prefix("AWSShineFrontendService_20170701.")
        .unwrap_or(target);

    let result = match action {
        "TranslateText" => translate_text(payload),
        "ListTerminologies" => list_terminologies(state),
        "ImportTerminology" => import_terminology(state, payload),
        "DeleteTerminology" => delete_terminology(state, payload),
        "ListLanguages" => list_languages(),
        other => Err(LawsError::InvalidRequest(format!(
            "unknown action: {other}"
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

fn require_str<'a>(body: &'a Value, field: &str) -> Result<&'a str, LawsError> {
    body.get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest(format!("missing required field: {field}")))
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn translate_text(body: &Value) -> Result<Response, LawsError> {
    let text = require_str(body, "Text")?;
    let source_language = require_str(body, "SourceLanguageCode")?;
    let target_language = require_str(body, "TargetLanguageCode")?;

    Ok(json_response(json!({
        "TranslatedText": text,
        "SourceLanguageCode": source_language,
        "TargetLanguageCode": target_language
    })))
}

fn list_terminologies(state: &TranslateState) -> Result<Response, LawsError> {
    let terminology_list: Vec<Value> = state
        .terminologies
        .iter()
        .map(|entry| {
            let t = entry.value();
            json!({
                "Name": t.name,
                "Arn": t.arn,
                "SourceLanguageCode": t.source_language,
                "TargetLanguageCodes": t.target_languages,
                "CreatedAt": t.created_at
            })
        })
        .collect();

    Ok(json_response(json!({
        "TerminologyPropertiesList": terminology_list
    })))
}

fn import_terminology(state: &TranslateState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "Name")?.to_owned();
    let source_language = body
        .get("SourceLanguageCode")
        .and_then(|v| v.as_str())
        .unwrap_or("en")
        .to_owned();
    let target_languages: Vec<String> = body
        .get("TargetLanguageCodes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                .collect()
        })
        .unwrap_or_default();

    let arn = format!("arn:aws:translate:{REGION}:{ACCOUNT_ID}:terminology/{name}");
    let created_at = chrono::Utc::now().to_rfc3339();

    let terminology = Terminology {
        name: name.clone(),
        arn: arn.clone(),
        source_language: source_language.clone(),
        target_languages: target_languages.clone(),
        created_at: created_at.clone(),
    };

    state.terminologies.insert(name.clone(), terminology);

    Ok(json_response(json!({
        "TerminologyProperties": {
            "Name": name,
            "Arn": arn,
            "SourceLanguageCode": source_language,
            "TargetLanguageCodes": target_languages,
            "CreatedAt": created_at
        }
    })))
}

fn delete_terminology(state: &TranslateState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "Name")?;
    state
        .terminologies
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("terminology not found: {name}")))?;
    Ok(json_response(json!({})))
}

fn list_languages() -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "Languages": [
            {"LanguageName": "English", "LanguageCode": "en"},
            {"LanguageName": "Spanish", "LanguageCode": "es"},
            {"LanguageName": "French", "LanguageCode": "fr"},
            {"LanguageName": "German", "LanguageCode": "de"},
            {"LanguageName": "Turkish", "LanguageCode": "tr"}
        ]
    })))
}
