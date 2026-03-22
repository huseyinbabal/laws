use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{delete, get, post, put};
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
pub struct LexBot {
    pub name: String,
    pub arn: String,
    pub description: String,
    pub locale: String,
    pub child_directed: bool,
    pub status: String,
    pub created_date: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct LexIntent {
    pub name: String,
    pub description: String,
    pub sample_utterances: Vec<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct LexState {
    pub bots: DashMap<String, LexBot>,
    pub intents: DashMap<String, LexIntent>,
}

impl Default for LexState {
    fn default() -> Self {
        Self {
            bots: DashMap::new(),
            intents: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<LexState>) -> axum::Router {
    axum::Router::new()
        .route("/bots/{name}/versions/$LATEST", put(put_bot).get(get_bot))
        .route("/bots", get(get_bots))
        .route("/bots/{name}", delete(delete_bot))
        .route("/bots/{name}/text", post(post_text))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn put_bot(
    State(state): State<Arc<LexState>>,
    Path(name): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    let description = payload["description"].as_str().unwrap_or("").to_string();

    let locale = payload["locale"].as_str().unwrap_or("en-US").to_string();

    let child_directed = payload["childDirected"].as_bool().unwrap_or(false);

    let arn = format!("arn:aws:lex:{REGION}:{ACCOUNT_ID}:bot:{name}");
    let now = chrono::Utc::now().to_rfc3339();

    let bot = LexBot {
        name: name.clone(),
        arn,
        description,
        locale,
        child_directed,
        status: "READY".to_string(),
        created_date: now.clone(),
        version: "$LATEST".to_string(),
    };

    let resp = bot_to_json(&bot);
    state.bots.insert(name, bot);

    rest_json::ok(resp)
}

async fn get_bots(State(state): State<Arc<LexState>>) -> Response {
    let bots: Vec<Value> = state
        .bots
        .iter()
        .map(|entry| {
            let b = entry.value();
            json!({
                "name": b.name,
                "description": b.description,
                "status": b.status,
                "createdDate": b.created_date,
                "version": b.version,
            })
        })
        .collect();

    rest_json::ok(json!({ "bots": bots }))
}

async fn get_bot(State(state): State<Arc<LexState>>, Path(name): Path<String>) -> Response {
    match state.bots.get(&name) {
        Some(bot) => rest_json::ok(bot_to_json(bot.value())),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Bot '{}' not found", name)))
        }
    }
}

async fn delete_bot(State(state): State<Arc<LexState>>, Path(name): Path<String>) -> Response {
    match state.bots.remove(&name) {
        Some(_) => rest_json::no_content(),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Bot '{}' not found", name)))
        }
    }
}

async fn post_text(
    State(state): State<Arc<LexState>>,
    Path(name): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    if !state.bots.contains_key(&name) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "Bot '{}' not found",
            name
        )));
    }

    let input_text = payload["inputText"].as_str().unwrap_or("");

    rest_json::ok(json!({
        "intentName": "FallbackIntent",
        "dialogState": "Fulfilled",
        "message": format!("Mock response to: {}", input_text),
        "sessionAttributes": {},
    }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn bot_to_json(b: &LexBot) -> Value {
    json!({
        "name": b.name,
        "arn": b.arn,
        "description": b.description,
        "locale": b.locale,
        "childDirected": b.child_directed,
        "status": b.status,
        "createdDate": b.created_date,
        "version": b.version,
    })
}
