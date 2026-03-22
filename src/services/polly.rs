use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::Json;
use dashmap::DashMap;
use http::StatusCode;
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
pub struct PollyLexicon {
    pub name: String,
    pub arn: String,
    pub content: String,
    pub alphabet: String,
    pub language_code: String,
    pub lexemes_count: u32,
    pub size: u32,
    pub last_modified: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct PollyState {
    pub lexicons: DashMap<String, PollyLexicon>,
}

impl Default for PollyState {
    fn default() -> Self {
        Self {
            lexicons: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<PollyState>) -> axum::Router {
    axum::Router::new()
        .route("/v1/lexicons", get(list_lexicons))
        .route(
            "/v1/lexicons/{name}",
            put(put_lexicon).get(get_lexicon).delete(delete_lexicon),
        )
        .route("/v1/speech", post(synthesize_speech))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn put_lexicon(
    State(state): State<Arc<PollyState>>,
    Path(name): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    let arn = format!("arn:aws:polly:{REGION}:{ACCOUNT_ID}:lexicon/{name}");

    let content = payload["Content"].as_str().unwrap_or("").to_string();

    let now = chrono::Utc::now().to_rfc3339();

    let lexicon = PollyLexicon {
        name: name.clone(),
        arn,
        content,
        alphabet: "ipa".to_string(),
        language_code: "en-US".to_string(),
        lexemes_count: 1,
        size: 0,
        last_modified: now,
    };

    state.lexicons.insert(name, lexicon);

    rest_json::ok(json!({}))
}

async fn list_lexicons(State(state): State<Arc<PollyState>>) -> Response {
    let lexicons: Vec<Value> = state
        .lexicons
        .iter()
        .map(|entry| {
            let l = entry.value();
            json!({
                "Name": l.name,
                "Attributes": {
                    "Alphabet": l.alphabet,
                    "LanguageCode": l.language_code,
                    "LastModified": l.last_modified,
                    "LexemesCount": l.lexemes_count,
                    "LexiconArn": l.arn,
                    "Size": l.size,
                }
            })
        })
        .collect();

    rest_json::ok(json!({ "Lexicons": lexicons }))
}

async fn get_lexicon(State(state): State<Arc<PollyState>>, Path(name): Path<String>) -> Response {
    match state.lexicons.get(&name) {
        Some(lexicon) => rest_json::ok(json!({
            "Lexicon": {
                "Name": lexicon.name,
                "Content": lexicon.content,
            },
            "LexiconAttributes": {
                "Alphabet": lexicon.alphabet,
                "LanguageCode": lexicon.language_code,
                "LastModified": lexicon.last_modified,
                "LexemesCount": lexicon.lexemes_count,
                "LexiconArn": lexicon.arn,
                "Size": lexicon.size,
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Lexicon '{}' not found",
            name
        ))),
    }
}

async fn delete_lexicon(
    State(state): State<Arc<PollyState>>,
    Path(name): Path<String>,
) -> Response {
    match state.lexicons.remove(&name) {
        Some(_) => rest_json::ok(json!({})),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Lexicon '{}' not found",
            name
        ))),
    }
}

async fn synthesize_speech(
    State(_state): State<Arc<PollyState>>,
    Json(_payload): Json<Value>,
) -> Response {
    (
        StatusCode::OK,
        [("Content-Type", "audio/mpeg")],
        Vec::<u8>::new(),
    )
        .into_response()
}
