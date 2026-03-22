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
pub struct FhirDatastore {
    pub datastore_id: String,
    pub datastore_arn: String,
    pub datastore_name: String,
    pub datastore_status: String,
    pub datastore_type_version: String,
    pub datastore_endpoint: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct FhirImportJob {
    pub job_id: String,
    pub datastore_id: String,
    pub job_name: String,
    pub job_status: String,
    pub input_data_config: Value,
    pub submitted_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct HealthLakeState {
    pub datastores: DashMap<String, FhirDatastore>,
    pub import_jobs: DashMap<String, FhirImportJob>,
}

impl Default for HealthLakeState {
    fn default() -> Self {
        Self {
            datastores: DashMap::new(),
            import_jobs: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &HealthLakeState, target: &str, payload: &Value) -> Response {
    let action = target.strip_prefix("HealthLake.").unwrap_or(target);

    let result = match action {
        "CreateFHIRDatastore" => create_fhir_datastore(state, payload),
        "DeleteFHIRDatastore" => delete_fhir_datastore(state, payload),
        "DescribeFHIRDatastore" => describe_fhir_datastore(state, payload),
        "ListFHIRDatastores" => list_fhir_datastores(state),
        "StartFHIRImportJob" => start_fhir_import_job(state, payload),
        "ListFHIRImportJobs" => list_fhir_import_jobs(state, payload),
        other => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            other
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
        [("Content-Type", "application/x-amz-json-1.0")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_fhir_datastore(state: &HealthLakeState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["DatastoreName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DatastoreName".into()))?
        .to_string();

    let type_version = payload["DatastoreTypeVersion"]
        .as_str()
        .unwrap_or("R4")
        .to_string();

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:healthlake:{REGION}:{ACCOUNT_ID}:datastore/fhir/{id}");
    let endpoint = format!("https://healthlake.{REGION}.amazonaws.com/datastore/{id}/r4/");
    let now = chrono::Utc::now().to_rfc3339();

    let datastore = FhirDatastore {
        datastore_id: id.clone(),
        datastore_arn: arn.clone(),
        datastore_name: name.clone(),
        datastore_status: "ACTIVE".into(),
        datastore_type_version: type_version.clone(),
        datastore_endpoint: endpoint.clone(),
        created_at: now.clone(),
    };

    state.datastores.insert(id.clone(), datastore);

    Ok(json_response(json!({
        "DatastoreId": id,
        "DatastoreArn": arn,
        "DatastoreStatus": "CREATING",
        "DatastoreEndpoint": endpoint
    })))
}

fn delete_fhir_datastore(state: &HealthLakeState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["DatastoreId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DatastoreId".into()))?;

    let (_, ds) = state
        .datastores
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("Datastore '{}' not found", id)))?;

    Ok(json_response(json!({
        "DatastoreId": ds.datastore_id,
        "DatastoreArn": ds.datastore_arn,
        "DatastoreStatus": "DELETING",
        "DatastoreEndpoint": ds.datastore_endpoint
    })))
}

fn describe_fhir_datastore(
    state: &HealthLakeState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let id = payload["DatastoreId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DatastoreId".into()))?;

    let ds = state
        .datastores
        .get(id)
        .ok_or_else(|| LawsError::NotFound(format!("Datastore '{}' not found", id)))?;

    Ok(json_response(json!({
        "DatastoreProperties": {
            "DatastoreId": ds.datastore_id,
            "DatastoreArn": ds.datastore_arn,
            "DatastoreName": ds.datastore_name,
            "DatastoreStatus": ds.datastore_status,
            "DatastoreTypeVersion": ds.datastore_type_version,
            "DatastoreEndpoint": ds.datastore_endpoint,
            "CreatedAt": ds.created_at
        }
    })))
}

fn list_fhir_datastores(state: &HealthLakeState) -> Result<Response, LawsError> {
    let datastores: Vec<Value> = state
        .datastores
        .iter()
        .map(|e| {
            let ds = e.value();
            json!({
                "DatastoreId": ds.datastore_id,
                "DatastoreArn": ds.datastore_arn,
                "DatastoreName": ds.datastore_name,
                "DatastoreStatus": ds.datastore_status,
                "DatastoreTypeVersion": ds.datastore_type_version,
                "DatastoreEndpoint": ds.datastore_endpoint,
                "CreatedAt": ds.created_at
            })
        })
        .collect();

    Ok(json_response(json!({
        "DatastorePropertiesList": datastores
    })))
}

fn start_fhir_import_job(state: &HealthLakeState, payload: &Value) -> Result<Response, LawsError> {
    let datastore_id = payload["DatastoreId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DatastoreId".into()))?
        .to_string();

    let job_name = payload["JobName"]
        .as_str()
        .unwrap_or("import-job")
        .to_string();

    let input_data_config = payload["InputDataConfig"].clone();

    let job_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let job = FhirImportJob {
        job_id: job_id.clone(),
        datastore_id: datastore_id.clone(),
        job_name: job_name.clone(),
        job_status: "SUBMITTED".into(),
        input_data_config,
        submitted_at: now.clone(),
    };

    state.import_jobs.insert(job_id.clone(), job);

    Ok(json_response(json!({
        "JobId": job_id,
        "JobStatus": "SUBMITTED",
        "DatastoreId": datastore_id
    })))
}

fn list_fhir_import_jobs(state: &HealthLakeState, payload: &Value) -> Result<Response, LawsError> {
    let datastore_id = payload["DatastoreId"].as_str().unwrap_or("");

    let jobs: Vec<Value> = state
        .import_jobs
        .iter()
        .filter(|e| datastore_id.is_empty() || e.value().datastore_id == datastore_id)
        .map(|e| {
            let j = e.value();
            json!({
                "JobId": j.job_id,
                "DatastoreId": j.datastore_id,
                "JobName": j.job_name,
                "JobStatus": j.job_status,
                "SubmitTime": j.submitted_at
            })
        })
        .collect();

    Ok(json_response(json!({
        "ImportJobPropertiesList": jobs
    })))
}
