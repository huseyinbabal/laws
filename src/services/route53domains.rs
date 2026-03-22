use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

#[allow(dead_code)]
const ACCOUNT_ID: &str = "000000000000";
#[allow(dead_code)]
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Domain {
    pub domain_name: String,
    pub auto_renew: bool,
    pub transfer_lock: bool,
    pub expiry: String,
    pub registrant_contact: Value,
    pub admin_contact: Value,
    pub tech_contact: Value,
    pub created_at: String,
    pub operation_id: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct Route53DomainsState {
    pub domains: DashMap<String, Domain>,
}

impl Default for Route53DomainsState {
    fn default() -> Self {
        Self {
            domains: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &Route53DomainsState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("Route53Domains_v20140515.")
        .unwrap_or(target);

    let result = match action {
        "RegisterDomain" => register_domain(state, payload),
        "GetDomainDetail" => get_domain_detail(state, payload),
        "ListDomains" => list_domains(state),
        "CheckDomainAvailability" => check_domain_availability(state, payload),
        "TransferDomain" => transfer_domain(state, payload),
        "RenewDomain" => renew_domain(state, payload),
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

fn default_contact() -> Value {
    json!({
        "FirstName": "John",
        "LastName": "Doe",
        "ContactType": "PERSON",
        "Email": "admin@example.com",
        "PhoneNumber": "+1.5555555555",
        "CountryCode": "US",
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn register_domain(
    state: &Route53DomainsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let domain_name = payload["DomainName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DomainName".into()))?
        .to_string();

    let auto_renew = payload["AutoRenew"]
        .as_bool()
        .unwrap_or(true);

    let duration_years = payload["DurationInYears"]
        .as_i64()
        .unwrap_or(1);

    let registrant_contact = payload["RegistrantContact"]
        .clone();
    let admin_contact = payload["AdminContact"]
        .clone();
    let tech_contact = payload["TechContact"]
        .clone();

    let operation_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let created_at = now.to_rfc3339();
    let expiry = (now + chrono::Duration::days(365 * duration_years))
        .to_rfc3339();

    let domain = Domain {
        domain_name: domain_name.clone(),
        auto_renew,
        transfer_lock: true,
        expiry,
        registrant_contact: if registrant_contact.is_null() {
            default_contact()
        } else {
            registrant_contact
        },
        admin_contact: if admin_contact.is_null() {
            default_contact()
        } else {
            admin_contact
        },
        tech_contact: if tech_contact.is_null() {
            default_contact()
        } else {
            tech_contact
        },
        created_at,
        operation_id: operation_id.clone(),
    };

    state.domains.insert(domain_name, domain);

    Ok(json_response(json!({
        "OperationId": operation_id,
    })))
}

fn get_domain_detail(
    state: &Route53DomainsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let domain_name = payload["DomainName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DomainName".into()))?;

    let domain = state
        .domains
        .get(domain_name)
        .ok_or_else(|| LawsError::NotFound(format!("Domain '{}' not found", domain_name)))?;

    Ok(json_response(json!({
        "DomainName": domain.domain_name,
        "AutoRenew": domain.auto_renew,
        "TransferLock": domain.transfer_lock,
        "ExpirationDate": domain.expiry,
        "RegistrantContact": domain.registrant_contact,
        "AdminContact": domain.admin_contact,
        "TechContact": domain.tech_contact,
        "CreationDate": domain.created_at,
    })))
}

fn list_domains(state: &Route53DomainsState) -> Result<Response, LawsError> {
    let domains: Vec<Value> = state
        .domains
        .iter()
        .map(|entry| {
            let d = entry.value();
            json!({
                "DomainName": d.domain_name,
                "AutoRenew": d.auto_renew,
                "TransferLock": d.transfer_lock,
                "Expiry": d.expiry,
            })
        })
        .collect();

    Ok(json_response(json!({
        "Domains": domains,
    })))
}

fn check_domain_availability(
    state: &Route53DomainsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let domain_name = payload["DomainName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DomainName".into()))?;

    let availability = if state.domains.contains_key(domain_name) {
        "UNAVAILABLE"
    } else {
        "AVAILABLE"
    };

    Ok(json_response(json!({
        "Availability": availability,
    })))
}

fn transfer_domain(
    state: &Route53DomainsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let domain_name = payload["DomainName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DomainName".into()))?
        .to_string();

    let operation_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let created_at = now.to_rfc3339();
    let expiry = (now + chrono::Duration::days(365)).to_rfc3339();

    let domain = Domain {
        domain_name: domain_name.clone(),
        auto_renew: true,
        transfer_lock: false,
        expiry,
        registrant_contact: default_contact(),
        admin_contact: default_contact(),
        tech_contact: default_contact(),
        created_at,
        operation_id: operation_id.clone(),
    };

    state.domains.insert(domain_name, domain);

    Ok(json_response(json!({
        "OperationId": operation_id,
    })))
}

fn renew_domain(
    state: &Route53DomainsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let domain_name = payload["DomainName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DomainName".into()))?;

    let duration_years = payload["DurationInYears"]
        .as_i64()
        .unwrap_or(1);

    let mut domain = state
        .domains
        .get_mut(domain_name)
        .ok_or_else(|| LawsError::NotFound(format!("Domain '{}' not found", domain_name)))?;

    let new_expiry = (chrono::Utc::now() + chrono::Duration::days(365 * duration_years))
        .to_rfc3339();
    domain.expiry = new_expiry;

    let operation_id = uuid::Uuid::new_v4().to_string();
    domain.operation_id = operation_id.clone();

    Ok(json_response(json!({
        "OperationId": operation_id,
    })))
}
