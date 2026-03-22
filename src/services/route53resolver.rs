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
pub struct ResolverEndpoint {
    pub id: String,
    pub arn: String,
    pub name: String,
    pub direction: String,
    pub ip_address_count: i64,
    pub status: String,
    pub host_vpc_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ResolverRule {
    pub id: String,
    pub arn: String,
    pub name: String,
    pub domain_name: String,
    pub rule_type: String,
    pub resolver_endpoint_id: String,
    pub status: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct Route53ResolverState {
    pub endpoints: DashMap<String, ResolverEndpoint>,
    pub rules: DashMap<String, ResolverRule>,
}

impl Default for Route53ResolverState {
    fn default() -> Self {
        Self {
            endpoints: DashMap::new(),
            rules: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &Route53ResolverState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target.strip_prefix("Route53Resolver.").unwrap_or(target);

    let result = match action {
        "CreateResolverEndpoint" => create_resolver_endpoint(state, payload),
        "DeleteResolverEndpoint" => delete_resolver_endpoint(state, payload),
        "ListResolverEndpoints" => list_resolver_endpoints(state),
        "GetResolverEndpoint" => get_resolver_endpoint(state, payload),
        "CreateResolverRule" => create_resolver_rule(state, payload),
        "ListResolverRules" => list_resolver_rules(state),
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

fn random_id(prefix: &str) -> String {
    format!("{}-{}", prefix, &uuid::Uuid::new_v4().to_string()[..12])
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_resolver_endpoint(
    state: &Route53ResolverState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
        .to_string();

    let direction = payload["Direction"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Direction".into()))?
        .to_string();

    let host_vpc_id = payload["SecurityGroupIds"]
        .as_array()
        .and_then(|_| payload["Direction"].as_str())
        .map(|_| "vpc-00000000".to_string())
        .unwrap_or_else(|| "vpc-00000000".to_string());

    let ip_count = payload["IpAddresses"]
        .as_array()
        .map(|arr| arr.len() as i64)
        .unwrap_or(2);

    let id = random_id("rslvr-in");
    let arn = format!("arn:aws:route53resolver:{REGION}:{ACCOUNT_ID}:resolver-endpoint/{id}");
    let created_at = chrono::Utc::now().to_rfc3339();

    let endpoint = ResolverEndpoint {
        id: id.clone(),
        arn: arn.clone(),
        name: name.clone(),
        direction: direction.clone(),
        ip_address_count: ip_count,
        status: "OPERATIONAL".to_string(),
        host_vpc_id: host_vpc_id.clone(),
        created_at: created_at.clone(),
    };

    state.endpoints.insert(id.clone(), endpoint);

    Ok(json_response(json!({
        "ResolverEndpoint": {
            "Id": id,
            "Arn": arn,
            "Name": name,
            "Direction": direction,
            "IpAddressCount": ip_count,
            "Status": "OPERATIONAL",
            "HostVPCId": host_vpc_id,
            "CreationTime": created_at,
        }
    })))
}

fn delete_resolver_endpoint(
    state: &Route53ResolverState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let id = payload["ResolverEndpointId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ResolverEndpointId".into()))?;

    let (_, ep) = state
        .endpoints
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("ResolverEndpoint '{}' not found", id)))?;

    Ok(json_response(json!({
        "ResolverEndpoint": {
            "Id": ep.id,
            "Arn": ep.arn,
            "Name": ep.name,
            "Direction": ep.direction,
            "Status": "DELETING",
        }
    })))
}

fn list_resolver_endpoints(state: &Route53ResolverState) -> Result<Response, LawsError> {
    let endpoints: Vec<Value> = state
        .endpoints
        .iter()
        .map(|entry| {
            let ep = entry.value();
            json!({
                "Id": ep.id,
                "Arn": ep.arn,
                "Name": ep.name,
                "Direction": ep.direction,
                "IpAddressCount": ep.ip_address_count,
                "Status": ep.status,
                "HostVPCId": ep.host_vpc_id,
                "CreationTime": ep.created_at,
            })
        })
        .collect();

    Ok(json_response(json!({
        "ResolverEndpoints": endpoints,
        "MaxResults": 100,
    })))
}

fn get_resolver_endpoint(
    state: &Route53ResolverState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let id = payload["ResolverEndpointId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ResolverEndpointId".into()))?;

    let ep = state
        .endpoints
        .get(id)
        .ok_or_else(|| LawsError::NotFound(format!("ResolverEndpoint '{}' not found", id)))?;

    Ok(json_response(json!({
        "ResolverEndpoint": {
            "Id": ep.id,
            "Arn": ep.arn,
            "Name": ep.name,
            "Direction": ep.direction,
            "IpAddressCount": ep.ip_address_count,
            "Status": ep.status,
            "HostVPCId": ep.host_vpc_id,
            "CreationTime": ep.created_at,
        }
    })))
}

fn create_resolver_rule(
    state: &Route53ResolverState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
        .to_string();

    let domain_name = payload["DomainName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DomainName".into()))?
        .to_string();

    let rule_type = payload["RuleType"]
        .as_str()
        .unwrap_or("FORWARD")
        .to_string();

    let resolver_endpoint_id = payload["ResolverEndpointId"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let id = random_id("rslvr-rr");
    let arn = format!("arn:aws:route53resolver:{REGION}:{ACCOUNT_ID}:resolver-rule/{id}");
    let created_at = chrono::Utc::now().to_rfc3339();

    let rule = ResolverRule {
        id: id.clone(),
        arn: arn.clone(),
        name: name.clone(),
        domain_name: domain_name.clone(),
        rule_type: rule_type.clone(),
        resolver_endpoint_id: resolver_endpoint_id.clone(),
        status: "COMPLETE".to_string(),
        created_at: created_at.clone(),
    };

    state.rules.insert(id.clone(), rule);

    Ok(json_response(json!({
        "ResolverRule": {
            "Id": id,
            "Arn": arn,
            "Name": name,
            "DomainName": domain_name,
            "RuleType": rule_type,
            "ResolverEndpointId": resolver_endpoint_id,
            "Status": "COMPLETE",
            "CreationTime": created_at,
        }
    })))
}

fn list_resolver_rules(state: &Route53ResolverState) -> Result<Response, LawsError> {
    let rules: Vec<Value> = state
        .rules
        .iter()
        .map(|entry| {
            let r = entry.value();
            json!({
                "Id": r.id,
                "Arn": r.arn,
                "Name": r.name,
                "DomainName": r.domain_name,
                "RuleType": r.rule_type,
                "ResolverEndpointId": r.resolver_endpoint_id,
                "Status": r.status,
                "CreationTime": r.created_at,
            })
        })
        .collect();

    Ok(json_response(json!({
        "ResolverRules": rules,
        "MaxResults": 100,
    })))
}
