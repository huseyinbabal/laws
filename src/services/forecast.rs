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
pub struct ForecastDataset {
    pub dataset_arn: String,
    pub dataset_name: String,
    pub dataset_type: String,
    pub domain: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct Predictor {
    pub predictor_arn: String,
    pub predictor_name: String,
    pub status: String,
    pub algorithm_arn: String,
}

#[derive(Debug, Clone)]
pub struct Forecast {
    pub forecast_arn: String,
    pub forecast_name: String,
    pub predictor_arn: String,
    pub status: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ForecastState {
    pub datasets: DashMap<String, ForecastDataset>,
    pub predictors: DashMap<String, Predictor>,
    pub forecasts: DashMap<String, Forecast>,
}

impl Default for ForecastState {
    fn default() -> Self {
        Self {
            datasets: DashMap::new(),
            predictors: DashMap::new(),
            forecasts: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &ForecastState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("AmazonForecast.")
        .unwrap_or(target);

    let result = match action {
        "CreateDataset" => create_dataset(state, payload),
        "DeleteDataset" => delete_dataset(state, payload),
        "DescribeDataset" => describe_dataset(state, payload),
        "ListDatasets" => list_datasets(state),
        "CreatePredictor" => create_predictor(state, payload),
        "DeletePredictor" => delete_predictor(state, payload),
        "ListPredictors" => list_predictors(state),
        "CreateForecast" => create_forecast(state, payload),
        "DescribeForecast" => describe_forecast(state, payload),
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

fn create_dataset(state: &ForecastState, body: &Value) -> Result<Response, LawsError> {
    let dataset_name = require_str(body, "DatasetName")?.to_owned();
    let dataset_type = body.get("DatasetType").and_then(|v| v.as_str()).unwrap_or("TARGET_TIME_SERIES").to_owned();
    let domain = body.get("Domain").and_then(|v| v.as_str()).unwrap_or("CUSTOM").to_owned();
    let dataset_arn = format!("arn:aws:forecast:{REGION}:{ACCOUNT_ID}:dataset/{dataset_name}");

    let dataset = ForecastDataset {
        dataset_arn: dataset_arn.clone(),
        dataset_name: dataset_name.clone(),
        dataset_type,
        domain,
        status: "ACTIVE".into(),
    };

    state.datasets.insert(dataset_name, dataset);

    Ok(json_response(json!({
        "DatasetArn": dataset_arn
    })))
}

fn delete_dataset(state: &ForecastState, body: &Value) -> Result<Response, LawsError> {
    let dataset_arn = require_str(body, "DatasetArn")?;
    let name = dataset_arn.rsplit('/').next().unwrap_or(dataset_arn);
    state.datasets.remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("dataset not found: {dataset_arn}")))?;

    Ok(json_response(json!({})))
}

fn describe_dataset(state: &ForecastState, body: &Value) -> Result<Response, LawsError> {
    let dataset_arn = require_str(body, "DatasetArn")?;
    let name = dataset_arn.rsplit('/').next().unwrap_or(dataset_arn);

    let ds = state.datasets.get(name)
        .ok_or_else(|| LawsError::NotFound(format!("dataset not found: {dataset_arn}")))?;

    Ok(json_response(json!({
        "DatasetArn": ds.dataset_arn,
        "DatasetName": ds.dataset_name,
        "DatasetType": ds.dataset_type,
        "Domain": ds.domain,
        "Status": ds.status
    })))
}

fn list_datasets(state: &ForecastState) -> Result<Response, LawsError> {
    let datasets: Vec<Value> = state.datasets.iter().map(|entry| {
        let ds = entry.value();
        json!({
            "DatasetArn": ds.dataset_arn,
            "DatasetName": ds.dataset_name,
            "DatasetType": ds.dataset_type,
            "Domain": ds.domain
        })
    }).collect();

    Ok(json_response(json!({
        "Datasets": datasets
    })))
}

fn create_predictor(state: &ForecastState, body: &Value) -> Result<Response, LawsError> {
    let predictor_name = require_str(body, "PredictorName")?.to_owned();
    let algorithm_arn = body.get("AlgorithmArn").and_then(|v| v.as_str()).unwrap_or("arn:aws:forecast:::algorithm/NPTS").to_owned();
    let predictor_arn = format!("arn:aws:forecast:{REGION}:{ACCOUNT_ID}:predictor/{predictor_name}");

    let predictor = Predictor {
        predictor_arn: predictor_arn.clone(),
        predictor_name: predictor_name.clone(),
        status: "ACTIVE".into(),
        algorithm_arn,
    };

    state.predictors.insert(predictor_name, predictor);

    Ok(json_response(json!({
        "PredictorArn": predictor_arn
    })))
}

fn delete_predictor(state: &ForecastState, body: &Value) -> Result<Response, LawsError> {
    let predictor_arn = require_str(body, "PredictorArn")?;
    let name = predictor_arn.rsplit('/').next().unwrap_or(predictor_arn);
    state.predictors.remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("predictor not found: {predictor_arn}")))?;

    Ok(json_response(json!({})))
}

fn list_predictors(state: &ForecastState) -> Result<Response, LawsError> {
    let predictors: Vec<Value> = state.predictors.iter().map(|entry| {
        let p = entry.value();
        json!({
            "PredictorArn": p.predictor_arn,
            "PredictorName": p.predictor_name,
            "Status": p.status
        })
    }).collect();

    Ok(json_response(json!({
        "Predictors": predictors
    })))
}

fn create_forecast(state: &ForecastState, body: &Value) -> Result<Response, LawsError> {
    let forecast_name = require_str(body, "ForecastName")?.to_owned();
    let predictor_arn = require_str(body, "PredictorArn")?.to_owned();
    let forecast_arn = format!("arn:aws:forecast:{REGION}:{ACCOUNT_ID}:forecast/{forecast_name}");

    let forecast = Forecast {
        forecast_arn: forecast_arn.clone(),
        forecast_name: forecast_name.clone(),
        predictor_arn,
        status: "ACTIVE".into(),
    };

    state.forecasts.insert(forecast_name, forecast);

    Ok(json_response(json!({
        "ForecastArn": forecast_arn
    })))
}

fn describe_forecast(state: &ForecastState, body: &Value) -> Result<Response, LawsError> {
    let forecast_arn = require_str(body, "ForecastArn")?;
    let name = forecast_arn.rsplit('/').next().unwrap_or(forecast_arn);

    let f = state.forecasts.get(name)
        .ok_or_else(|| LawsError::NotFound(format!("forecast not found: {forecast_arn}")))?;

    Ok(json_response(json!({
        "ForecastArn": f.forecast_arn,
        "ForecastName": f.forecast_name,
        "PredictorArn": f.predictor_arn,
        "Status": f.status
    })))
}
