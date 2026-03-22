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
pub struct CertificateAuthority {
    pub arn: String,
    pub ca_type: String,
    pub status: String,
    pub subject: Value,
    pub created_at: String,
    pub certificates: DashMap<String, Certificate>,
}

#[derive(Debug, Clone)]
pub struct Certificate {
    pub arn: String,
    pub ca_arn: String,
    pub status: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct AcmPcaState {
    pub certificate_authorities: DashMap<String, CertificateAuthority>,
}

impl Default for AcmPcaState {
    fn default() -> Self {
        Self {
            certificate_authorities: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &AcmPcaState, target: &str, payload: &Value) -> Response {
    let action = target.strip_prefix("ACMPrivateCA.").unwrap_or(target);

    let result = match action {
        "CreateCertificateAuthority" => create_certificate_authority(state, payload),
        "DeleteCertificateAuthority" => delete_certificate_authority(state, payload),
        "DescribeCertificateAuthority" => describe_certificate_authority(state, payload),
        "ListCertificateAuthorities" => list_certificate_authorities(state),
        "IssueCertificate" => issue_certificate(state, payload),
        "GetCertificate" => get_certificate(state, payload),
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

fn ca_to_json(ca: &CertificateAuthority) -> Value {
    json!({
        "Arn": ca.arn,
        "Type": ca.ca_type,
        "Status": ca.status,
        "Subject": ca.subject,
        "CreatedAt": ca.created_at,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_certificate_authority(
    state: &AcmPcaState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let ca_type = payload["CertificateAuthorityType"]
        .as_str()
        .unwrap_or("ROOT")
        .to_string();

    let subject = payload["CertificateAuthorityConfiguration"]["Subject"].clone();

    let ca_id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:acm-pca:{REGION}:{ACCOUNT_ID}:certificate-authority/{ca_id}");
    let now = chrono::Utc::now().to_rfc3339();

    let ca = CertificateAuthority {
        arn: arn.clone(),
        ca_type,
        status: "PENDING_CERTIFICATE".to_string(),
        subject,
        created_at: now,
        certificates: DashMap::new(),
    };

    state.certificate_authorities.insert(arn.clone(), ca);

    Ok(json_response(json!({
        "CertificateAuthorityArn": arn,
    })))
}

fn delete_certificate_authority(
    state: &AcmPcaState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let arn = payload["CertificateAuthorityArn"].as_str().ok_or_else(|| {
        LawsError::InvalidRequest("CertificateAuthorityArn is required".to_string())
    })?;

    state
        .certificate_authorities
        .remove(arn)
        .ok_or_else(|| LawsError::NotFound(format!("Certificate authority '{}' not found", arn)))?;

    Ok(json_response(json!({})))
}

fn describe_certificate_authority(
    state: &AcmPcaState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let arn = payload["CertificateAuthorityArn"].as_str().ok_or_else(|| {
        LawsError::InvalidRequest("CertificateAuthorityArn is required".to_string())
    })?;

    let ca = state
        .certificate_authorities
        .get(arn)
        .ok_or_else(|| LawsError::NotFound(format!("Certificate authority '{}' not found", arn)))?;

    Ok(json_response(json!({
        "CertificateAuthority": ca_to_json(ca.value()),
    })))
}

fn list_certificate_authorities(state: &AcmPcaState) -> Result<Response, LawsError> {
    let cas: Vec<Value> = state
        .certificate_authorities
        .iter()
        .map(|entry| ca_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({
        "CertificateAuthorities": cas,
    })))
}

fn issue_certificate(state: &AcmPcaState, payload: &Value) -> Result<Response, LawsError> {
    let ca_arn = payload["CertificateAuthorityArn"].as_str().ok_or_else(|| {
        LawsError::InvalidRequest("CertificateAuthorityArn is required".to_string())
    })?;

    let ca = state.certificate_authorities.get(ca_arn).ok_or_else(|| {
        LawsError::NotFound(format!("Certificate authority '{}' not found", ca_arn))
    })?;

    let cert_id = uuid::Uuid::new_v4().to_string();
    let cert_arn = format!(
        "arn:aws:acm-pca:{REGION}:{ACCOUNT_ID}:certificate-authority/{}/certificate/{cert_id}",
        ca_arn.rsplit('/').next().unwrap_or("")
    );
    let now = chrono::Utc::now().to_rfc3339();

    let certificate = Certificate {
        arn: cert_arn.clone(),
        ca_arn: ca_arn.to_string(),
        status: "ISSUED".to_string(),
        created_at: now,
    };

    ca.certificates.insert(cert_arn.clone(), certificate);

    Ok(json_response(json!({
        "CertificateArn": cert_arn,
    })))
}

fn get_certificate(state: &AcmPcaState, payload: &Value) -> Result<Response, LawsError> {
    let ca_arn = payload["CertificateAuthorityArn"].as_str().ok_or_else(|| {
        LawsError::InvalidRequest("CertificateAuthorityArn is required".to_string())
    })?;

    let cert_arn = payload["CertificateArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("CertificateArn is required".to_string()))?;

    let ca = state.certificate_authorities.get(ca_arn).ok_or_else(|| {
        LawsError::NotFound(format!("Certificate authority '{}' not found", ca_arn))
    })?;

    let _cert = ca
        .certificates
        .get(cert_arn)
        .ok_or_else(|| LawsError::NotFound(format!("Certificate '{}' not found", cert_arn)))?;

    // Return a mock PEM certificate
    Ok(json_response(json!({
        "Certificate": "-----BEGIN CERTIFICATE-----\nMIICmock...\n-----END CERTIFICATE-----",
        "CertificateChain": "-----BEGIN CERTIFICATE-----\nMIICmockchain...\n-----END CERTIFICATE-----",
    })))
}
