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
pub struct License {
    pub license_arn: String,
    pub license_name: String,
    pub product_name: String,
    pub issuer: String,
    pub status: String,
    pub validity_start: String,
    pub validity_end: String,
}

#[derive(Debug, Clone)]
pub struct LicenseConfiguration {
    pub license_configuration_arn: String,
    pub license_configuration_id: String,
    pub name: String,
    pub license_counting_type: String,
    pub license_count: u64,
    pub status: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct LicenseManagerState {
    pub licenses: DashMap<String, License>,
    pub configurations: DashMap<String, LicenseConfiguration>,
}

impl Default for LicenseManagerState {
    fn default() -> Self {
        Self {
            licenses: DashMap::new(),
            configurations: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &LicenseManagerState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("AWSLicenseManager.")
        .unwrap_or(target);

    let result = match action {
        "CreateLicense" => create_license(state, payload),
        "GetLicense" => get_license(state, payload),
        "ListReceivedLicenses" => list_received_licenses(state),
        "DeleteLicense" => delete_license(state, payload),
        "CreateLicenseConfiguration" => create_license_configuration(state, payload),
        "ListLicenseConfigurations" => list_license_configurations(state),
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
        [("Content-Type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_license(
    state: &LicenseManagerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let license_name = payload["LicenseName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing LicenseName".into()))?
        .to_string();

    let product_name = payload["ProductName"]
        .as_str()
        .unwrap_or("Product")
        .to_string();

    let issuer = payload["Issuer"]["Name"]
        .as_str()
        .unwrap_or("Self")
        .to_string();

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:license-manager:{REGION}:{ACCOUNT_ID}:license/{id}");
    let now = chrono::Utc::now().to_rfc3339();

    let license = License {
        license_arn: arn.clone(),
        license_name,
        product_name,
        issuer,
        status: "AVAILABLE".into(),
        validity_start: now.clone(),
        validity_end: "2099-12-31T23:59:59Z".into(),
    };

    state.licenses.insert(arn.clone(), license);

    Ok(json_response(json!({
        "LicenseArn": arn,
        "Status": "AVAILABLE",
        "Version": "1"
    })))
}

fn get_license(
    state: &LicenseManagerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let arn = payload["LicenseArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing LicenseArn".into()))?;

    let lic = state
        .licenses
        .get(arn)
        .ok_or_else(|| LawsError::NotFound(format!("License '{}' not found", arn)))?;

    Ok(json_response(json!({
        "License": {
            "LicenseArn": lic.license_arn,
            "LicenseName": lic.license_name,
            "ProductName": lic.product_name,
            "Issuer": { "Name": lic.issuer },
            "Status": lic.status,
            "Validity": {
                "Begin": lic.validity_start,
                "End": lic.validity_end
            }
        }
    })))
}

fn list_received_licenses(
    state: &LicenseManagerState,
) -> Result<Response, LawsError> {
    let licenses: Vec<Value> = state
        .licenses
        .iter()
        .map(|e| {
            let lic = e.value();
            json!({
                "LicenseArn": lic.license_arn,
                "LicenseName": lic.license_name,
                "ProductName": lic.product_name,
                "Issuer": { "Name": lic.issuer },
                "Status": lic.status
            })
        })
        .collect();

    Ok(json_response(json!({
        "Licenses": licenses
    })))
}

fn delete_license(
    state: &LicenseManagerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let arn = payload["LicenseArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing LicenseArn".into()))?;

    state
        .licenses
        .remove(arn)
        .ok_or_else(|| LawsError::NotFound(format!("License '{}' not found", arn)))?;

    Ok(json_response(json!({
        "Status": "DELETED",
        "DeletionDate": chrono::Utc::now().to_rfc3339()
    })))
}

fn create_license_configuration(
    state: &LicenseManagerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
        .to_string();

    let counting_type = payload["LicenseCountingType"]
        .as_str()
        .unwrap_or("vCPU")
        .to_string();

    let license_count = payload["LicenseCount"]
        .as_u64()
        .unwrap_or(10);

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:license-manager:{REGION}:{ACCOUNT_ID}:license-configuration/{id}"
    );

    let config = LicenseConfiguration {
        license_configuration_arn: arn.clone(),
        license_configuration_id: id,
        name,
        license_counting_type: counting_type,
        license_count,
        status: "AVAILABLE".into(),
    };

    state.configurations.insert(arn.clone(), config);

    Ok(json_response(json!({
        "LicenseConfigurationArn": arn
    })))
}

fn list_license_configurations(
    state: &LicenseManagerState,
) -> Result<Response, LawsError> {
    let configs: Vec<Value> = state
        .configurations
        .iter()
        .map(|e| {
            let c = e.value();
            json!({
                "LicenseConfigurationArn": c.license_configuration_arn,
                "LicenseConfigurationId": c.license_configuration_id,
                "Name": c.name,
                "LicenseCountingType": c.license_counting_type,
                "LicenseCount": c.license_count,
                "Status": c.status
            })
        })
        .collect();

    Ok(json_response(json!({
        "LicenseConfigurations": configs
    })))
}
