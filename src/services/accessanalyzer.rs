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
pub struct Analyzer {
    pub name: String,
    pub arn: String,
    pub analyzer_type: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub id: String,
    pub analyzer_arn: String,
    pub resource_type: String,
    pub resource_arn: String,
    pub status: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct AccessAnalyzerState {
    pub analyzers: DashMap<String, Analyzer>,
    pub findings: DashMap<String, Finding>,
}

impl Default for AccessAnalyzerState {
    fn default() -> Self {
        Self {
            analyzers: DashMap::new(),
            findings: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &AccessAnalyzerState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target.strip_prefix("AccessAnalyzer.").unwrap_or(target);

    let result = match action {
        "CreateAnalyzer" => create_analyzer(state, payload),
        "DeleteAnalyzer" => delete_analyzer(state, payload),
        "ListAnalyzers" => list_analyzers(state),
        "GetAnalyzer" => get_analyzer(state, payload),
        "ListFindings" => list_findings(state, payload),
        "GetFinding" => get_finding(state, payload),
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

fn analyzer_to_json(a: &Analyzer) -> Value {
    json!({
        "name": a.name,
        "arn": a.arn,
        "type": a.analyzer_type,
        "status": a.status,
        "createdAt": a.created_at,
    })
}

fn finding_to_json(f: &Finding) -> Value {
    json!({
        "id": f.id,
        "analyzerArn": f.analyzer_arn,
        "resourceType": f.resource_type,
        "resource": f.resource_arn,
        "status": f.status,
        "createdAt": f.created_at,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_analyzer(state: &AccessAnalyzerState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["analyzerName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("analyzerName is required".to_string()))?
        .to_string();

    if state.analyzers.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Analyzer '{}' already exists",
            name
        )));
    }

    let analyzer_type = payload["type"].as_str().unwrap_or("ACCOUNT").to_string();

    let arn = format!("arn:aws:access-analyzer:{REGION}:{ACCOUNT_ID}:analyzer/{name}");
    let now = chrono::Utc::now().to_rfc3339();

    let analyzer = Analyzer {
        name: name.clone(),
        arn: arn.clone(),
        analyzer_type,
        status: "ACTIVE".to_string(),
        created_at: now,
    };

    let resp = analyzer_to_json(&analyzer);
    state.analyzers.insert(name, analyzer);

    Ok(json_response(json!({ "analyzer": resp })))
}

fn delete_analyzer(state: &AccessAnalyzerState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["analyzerName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("analyzerName is required".to_string()))?;

    state
        .analyzers
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Analyzer '{}' not found", name)))?;

    // Remove associated findings
    let arn_prefix = format!("arn:aws:access-analyzer:{REGION}:{ACCOUNT_ID}:analyzer/{name}");
    state.findings.retain(|_, f| f.analyzer_arn != arn_prefix);

    Ok(json_response(json!({})))
}

fn list_analyzers(state: &AccessAnalyzerState) -> Result<Response, LawsError> {
    let analyzers: Vec<Value> = state
        .analyzers
        .iter()
        .map(|entry| analyzer_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "analyzers": analyzers })))
}

fn get_analyzer(state: &AccessAnalyzerState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["analyzerName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("analyzerName is required".to_string()))?;

    let analyzer = state
        .analyzers
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("Analyzer '{}' not found", name)))?;

    Ok(json_response(
        json!({ "analyzer": analyzer_to_json(analyzer.value()) }),
    ))
}

fn list_findings(state: &AccessAnalyzerState, payload: &Value) -> Result<Response, LawsError> {
    let analyzer_arn = payload["analyzerArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("analyzerArn is required".to_string()))?;

    let findings: Vec<Value> = state
        .findings
        .iter()
        .filter(|entry| entry.value().analyzer_arn == analyzer_arn)
        .map(|entry| finding_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "findings": findings })))
}

fn get_finding(state: &AccessAnalyzerState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["id"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("id is required".to_string()))?;

    let finding = state
        .findings
        .get(id)
        .ok_or_else(|| LawsError::NotFound(format!("Finding '{}' not found", id)))?;

    Ok(json_response(
        json!({ "finding": finding_to_json(finding.value()) }),
    ))
}
