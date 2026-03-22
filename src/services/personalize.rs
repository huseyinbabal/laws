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
pub struct PersonalizeDataset {
    pub dataset_arn: String,
    pub name: String,
    pub dataset_type: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct Solution {
    pub solution_arn: String,
    pub name: String,
    pub recipe_arn: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct Campaign {
    pub campaign_arn: String,
    pub name: String,
    pub solution_version_arn: String,
    pub status: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct PersonalizeState {
    pub datasets: DashMap<String, PersonalizeDataset>,
    pub solutions: DashMap<String, Solution>,
    pub campaigns: DashMap<String, Campaign>,
}

impl Default for PersonalizeState {
    fn default() -> Self {
        Self {
            datasets: DashMap::new(),
            solutions: DashMap::new(),
            campaigns: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &PersonalizeState, target: &str, payload: &Value) -> Response {
    let action = target.strip_prefix("AmazonPersonalize.").unwrap_or(target);

    let result = match action {
        "CreateDataset" => create_dataset(state, payload),
        "DeleteDataset" => delete_dataset(state, payload),
        "ListDatasets" => list_datasets(state),
        "CreateSolution" => create_solution(state, payload),
        "DeleteSolution" => delete_solution(state, payload),
        "ListSolutions" => list_solutions(state),
        "CreateCampaign" => create_campaign(state, payload),
        "DeleteCampaign" => delete_campaign(state, payload),
        "ListCampaigns" => list_campaigns(state),
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

fn create_dataset(state: &PersonalizeState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "Name")?.to_owned();
    let dataset_type = body
        .get("DatasetType")
        .and_then(|v| v.as_str())
        .unwrap_or("Interactions")
        .to_owned();
    let dataset_arn = format!("arn:aws:personalize:{REGION}:{ACCOUNT_ID}:dataset/{name}");

    let dataset = PersonalizeDataset {
        dataset_arn: dataset_arn.clone(),
        name: name.clone(),
        dataset_type,
        status: "ACTIVE".into(),
    };

    state.datasets.insert(name, dataset);

    Ok(json_response(json!({
        "DatasetArn": dataset_arn
    })))
}

fn delete_dataset(state: &PersonalizeState, body: &Value) -> Result<Response, LawsError> {
    let dataset_arn = require_str(body, "DatasetArn")?;
    let name = dataset_arn.rsplit('/').next().unwrap_or(dataset_arn);
    state
        .datasets
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("dataset not found: {dataset_arn}")))?;

    Ok(json_response(json!({})))
}

fn list_datasets(state: &PersonalizeState) -> Result<Response, LawsError> {
    let datasets: Vec<Value> = state
        .datasets
        .iter()
        .map(|entry| {
            let ds = entry.value();
            json!({
                "DatasetArn": ds.dataset_arn,
                "Name": ds.name,
                "DatasetType": ds.dataset_type,
                "Status": ds.status
            })
        })
        .collect();

    Ok(json_response(json!({
        "Datasets": datasets
    })))
}

fn create_solution(state: &PersonalizeState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "Name")?.to_owned();
    let recipe_arn = body
        .get("RecipeArn")
        .and_then(|v| v.as_str())
        .unwrap_or("arn:aws:personalize:::recipe/aws-hrnn")
        .to_owned();
    let solution_arn = format!("arn:aws:personalize:{REGION}:{ACCOUNT_ID}:solution/{name}");

    let solution = Solution {
        solution_arn: solution_arn.clone(),
        name: name.clone(),
        recipe_arn,
        status: "ACTIVE".into(),
    };

    state.solutions.insert(name, solution);

    Ok(json_response(json!({
        "SolutionArn": solution_arn
    })))
}

fn delete_solution(state: &PersonalizeState, body: &Value) -> Result<Response, LawsError> {
    let solution_arn = require_str(body, "SolutionArn")?;
    let name = solution_arn.rsplit('/').next().unwrap_or(solution_arn);
    state
        .solutions
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("solution not found: {solution_arn}")))?;

    Ok(json_response(json!({})))
}

fn list_solutions(state: &PersonalizeState) -> Result<Response, LawsError> {
    let solutions: Vec<Value> = state
        .solutions
        .iter()
        .map(|entry| {
            let s = entry.value();
            json!({
                "SolutionArn": s.solution_arn,
                "Name": s.name,
                "Status": s.status
            })
        })
        .collect();

    Ok(json_response(json!({
        "Solutions": solutions
    })))
}

fn create_campaign(state: &PersonalizeState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "Name")?.to_owned();
    let solution_version_arn = require_str(body, "SolutionVersionArn")?.to_owned();
    let campaign_arn = format!("arn:aws:personalize:{REGION}:{ACCOUNT_ID}:campaign/{name}");

    let campaign = Campaign {
        campaign_arn: campaign_arn.clone(),
        name: name.clone(),
        solution_version_arn,
        status: "ACTIVE".into(),
    };

    state.campaigns.insert(name, campaign);

    Ok(json_response(json!({
        "CampaignArn": campaign_arn
    })))
}

fn delete_campaign(state: &PersonalizeState, body: &Value) -> Result<Response, LawsError> {
    let campaign_arn = require_str(body, "CampaignArn")?;
    let name = campaign_arn.rsplit('/').next().unwrap_or(campaign_arn);
    state
        .campaigns
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("campaign not found: {campaign_arn}")))?;

    Ok(json_response(json!({})))
}

fn list_campaigns(state: &PersonalizeState) -> Result<Response, LawsError> {
    let campaigns: Vec<Value> = state
        .campaigns
        .iter()
        .map(|entry| {
            let c = entry.value();
            json!({
                "CampaignArn": c.campaign_arn,
                "Name": c.name,
                "Status": c.status
            })
        })
        .collect();

    Ok(json_response(json!({
        "Campaigns": campaigns
    })))
}
