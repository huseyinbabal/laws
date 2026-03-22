use std::sync::Arc;

use axum::extract::State;
use axum::response::Response;
use axum::routing::{get, post};
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
pub struct ModelCustomizationJob {
    pub job_arn: String,
    pub job_name: String,
    pub base_model_identifier: String,
    pub custom_model_name: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct CustomModel {
    pub model_arn: String,
    pub model_name: String,
    pub base_model_arn: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct BedrockState {
    pub model_customization_jobs: DashMap<String, ModelCustomizationJob>,
    pub custom_models: DashMap<String, CustomModel>,
}

impl Default for BedrockState {
    fn default() -> Self {
        Self {
            model_customization_jobs: DashMap::new(),
            custom_models: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<BedrockState>) -> axum::Router {
    axum::Router::new()
        .route("/foundation-models", get(list_foundation_models))
        .route(
            "/model-customization-jobs",
            post(create_model_customization_job).get(list_model_customization_jobs),
        )
        .route("/custom-models", get(list_custom_models))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn list_foundation_models(State(_state): State<Arc<BedrockState>>) -> Response {
    let models = vec![
        foundation_model("anthropic.claude-v2", "Anthropic", "Claude v2"),
        foundation_model("anthropic.claude-v2:1", "Anthropic", "Claude v2.1"),
        foundation_model(
            "anthropic.claude-3-sonnet-20240229-v1:0",
            "Anthropic",
            "Claude 3 Sonnet",
        ),
        foundation_model(
            "anthropic.claude-3-haiku-20240307-v1:0",
            "Anthropic",
            "Claude 3 Haiku",
        ),
        foundation_model(
            "amazon.titan-text-express-v1",
            "Amazon",
            "Titan Text Express",
        ),
        foundation_model("amazon.titan-text-lite-v1", "Amazon", "Titan Text Lite"),
        foundation_model("amazon.titan-embed-text-v1", "Amazon", "Titan Embeddings"),
        foundation_model("meta.llama2-13b-chat-v1", "Meta", "Llama 2 Chat 13B"),
        foundation_model("cohere.command-text-v14", "Cohere", "Command"),
        foundation_model("ai21.j2-mid-v1", "AI21 Labs", "Jurassic-2 Mid"),
    ];

    rest_json::ok(json!({ "modelSummaries": models }))
}

async fn create_model_customization_job(
    State(state): State<Arc<BedrockState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let job_name = payload["jobName"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("jobName is required".into()))?
            .to_string();

        let base_model_identifier = payload["baseModelIdentifier"]
            .as_str()
            .unwrap_or("anthropic.claude-v2")
            .to_string();

        let custom_model_name = payload["customModelName"]
            .as_str()
            .unwrap_or(&job_name)
            .to_string();

        let job_arn =
            format!("arn:aws:bedrock:{REGION}:{ACCOUNT_ID}:model-customization-job/{job_name}");
        let now = chrono::Utc::now().to_rfc3339();

        let job = ModelCustomizationJob {
            job_arn: job_arn.clone(),
            job_name: job_name.clone(),
            base_model_identifier,
            custom_model_name: custom_model_name.clone(),
            status: "Completed".to_string(),
            created_at: now.clone(),
        };

        state.model_customization_jobs.insert(job_name.clone(), job);

        // Also create the custom model
        let model_arn =
            format!("arn:aws:bedrock:{REGION}:{ACCOUNT_ID}:custom-model/{custom_model_name}");

        let model = CustomModel {
            model_arn: model_arn.clone(),
            model_name: custom_model_name.clone(),
            base_model_arn: format!(
                "arn:aws:bedrock:{REGION}::foundation-model/anthropic.claude-v2"
            ),
            created_at: now,
        };

        state.custom_models.insert(custom_model_name, model);

        Ok(rest_json::created(json!({
            "jobArn": job_arn,
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_model_customization_jobs(State(state): State<Arc<BedrockState>>) -> Response {
    let summaries: Vec<Value> = state
        .model_customization_jobs
        .iter()
        .map(|entry| {
            let j = entry.value();
            json!({
                "jobArn": j.job_arn,
                "jobName": j.job_name,
                "baseModelIdentifier": j.base_model_identifier,
                "customModelName": j.custom_model_name,
                "status": j.status,
                "creationTime": j.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({
        "modelCustomizationJobSummaries": summaries,
    }))
}

async fn list_custom_models(State(state): State<Arc<BedrockState>>) -> Response {
    let summaries: Vec<Value> = state
        .custom_models
        .iter()
        .map(|entry| {
            let m = entry.value();
            json!({
                "modelArn": m.model_arn,
                "modelName": m.model_name,
                "baseModelArn": m.base_model_arn,
                "creationTime": m.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "modelSummaries": summaries }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn foundation_model(model_id: &str, provider: &str, name: &str) -> Value {
    json!({
        "modelId": model_id,
        "modelName": name,
        "providerName": provider,
        "modelArn": format!("arn:aws:bedrock:{REGION}::foundation-model/{model_id}"),
        "inputModalities": ["TEXT"],
        "outputModalities": ["TEXT"],
        "modelLifecycle": { "status": "ACTIVE" },
    })
}
