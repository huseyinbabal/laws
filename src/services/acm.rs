use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use std::sync::Arc;

use crate::error::LawsError;

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct AcmCertificate {
    pub arn: String,
    pub domain_name: String,
    pub sans: Vec<String>,
    pub status: String,
    pub created_at: f64,
    pub tags: Vec<(String, String)>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct AcmState {
    pub certificates: Arc<DashMap<String, AcmCertificate>>,
}

impl Default for AcmState {
    fn default() -> Self {
        Self {
            certificates: Arc::new(DashMap::new()),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &AcmState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target.strip_prefix("CertificateManager.").unwrap_or(target);

    let result = match action {
        "RequestCertificate" => request_certificate(state, payload).await,
        "DescribeCertificate" => describe_certificate(state, payload).await,
        "ListCertificates" => list_certificates(state).await,
        "DeleteCertificate" => delete_certificate(state, payload).await,
        "AddTagsToCertificate" => add_tags_to_certificate(state, payload).await,
        "ListTagsForCertificate" => list_tags_for_certificate(state, payload).await,
        "GetCertificate" => get_certificate(state, payload).await,
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

fn json_response(body: serde_json::Value) -> Response {
    (
        StatusCode::OK,
        [("Content-Type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn find_certificate(state: &AcmState, arn: &str) -> Result<AcmCertificate, LawsError> {
    state
        .certificates
        .get(arn)
        .map(|entry| entry.clone())
        .ok_or_else(|| LawsError::NotFound(format!("Certificate '{}' not found", arn)))
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

async fn request_certificate(
    state: &AcmState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let domain_name = payload["DomainName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DomainName is required".to_string()))?;

    let sans: Vec<String> = match payload["SubjectAlternativeNames"].as_array() {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        None => vec![domain_name.to_string()],
    };

    let _validation_method = payload["ValidationMethod"].as_str().unwrap_or("DNS");

    let cert_id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:acm:us-east-1:000000000000:certificate/{}", cert_id);
    let now = chrono::Utc::now().timestamp() as f64;

    let cert = AcmCertificate {
        arn: arn.clone(),
        domain_name: domain_name.to_string(),
        sans,
        status: "ISSUED".to_string(),
        created_at: now,
        tags: Vec::new(),
    };

    state.certificates.insert(arn.clone(), cert);

    Ok(json_response(serde_json::json!({
        "CertificateArn": arn,
    })))
}

async fn describe_certificate(
    state: &AcmState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let arn = payload["CertificateArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("CertificateArn is required".to_string()))?;

    let cert = find_certificate(state, arn)?;

    let not_after = cert.created_at + (365.0 * 86400.0);
    let serial = hex::encode(uuid::Uuid::new_v4().as_bytes());

    Ok(json_response(serde_json::json!({
        "Certificate": {
            "CertificateArn": cert.arn,
            "DomainName": cert.domain_name,
            "SubjectAlternativeNames": cert.sans,
            "Status": cert.status,
            "Type": "AMAZON_ISSUED",
            "CreatedAt": cert.created_at,
            "IssuedAt": cert.created_at,
            "NotBefore": cert.created_at,
            "NotAfter": not_after,
            "Serial": serial,
            "InUseBy": [],
        }
    })))
}

async fn list_certificates(state: &AcmState) -> Result<Response, LawsError> {
    let certs: Vec<serde_json::Value> = state
        .certificates
        .iter()
        .map(|entry| {
            let cert = entry.value();
            serde_json::json!({
                "CertificateArn": cert.arn,
                "DomainName": cert.domain_name,
                "Status": cert.status,
            })
        })
        .collect();

    Ok(json_response(serde_json::json!({
        "CertificateSummaryList": certs,
    })))
}

async fn delete_certificate(
    state: &AcmState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let arn = payload["CertificateArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("CertificateArn is required".to_string()))?;

    state
        .certificates
        .remove(arn)
        .ok_or_else(|| LawsError::NotFound(format!("Certificate '{}' not found", arn)))?;

    Ok(json_response(serde_json::json!({})))
}

async fn add_tags_to_certificate(
    state: &AcmState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let arn = payload["CertificateArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("CertificateArn is required".to_string()))?;

    let tags = payload["Tags"]
        .as_array()
        .ok_or_else(|| LawsError::InvalidRequest("Tags is required".to_string()))?;

    let new_tags: Vec<(String, String)> = tags
        .iter()
        .filter_map(|t| {
            let key = t["Key"].as_str()?;
            let value = t["Value"].as_str().unwrap_or("");
            Some((key.to_string(), value.to_string()))
        })
        .collect();

    let mut entry = state
        .certificates
        .get_mut(arn)
        .ok_or_else(|| LawsError::NotFound(format!("Certificate '{}' not found", arn)))?;

    entry.tags.extend(new_tags);

    Ok(json_response(serde_json::json!({})))
}

async fn list_tags_for_certificate(
    state: &AcmState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let arn = payload["CertificateArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("CertificateArn is required".to_string()))?;

    let cert = find_certificate(state, arn)?;

    let tags: Vec<serde_json::Value> = cert
        .tags
        .iter()
        .map(|(k, v)| {
            serde_json::json!({
                "Key": k,
                "Value": v,
            })
        })
        .collect();

    Ok(json_response(serde_json::json!({
        "Tags": tags,
    })))
}

async fn get_certificate(
    state: &AcmState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let arn = payload["CertificateArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("CertificateArn is required".to_string()))?;

    // Verify certificate exists
    let _cert = find_certificate(state, arn)?;

    Ok(json_response(serde_json::json!({
        "Certificate": "-----BEGIN CERTIFICATE-----\nMOCK\n-----END CERTIFICATE-----",
        "CertificateChain": "-----BEGIN CERTIFICATE-----\nMOCK_CHAIN\n-----END CERTIFICATE-----",
    })))
}
