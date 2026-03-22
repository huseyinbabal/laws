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
pub struct WebAcl {
    pub name: String,
    pub id: String,
    pub arn: String,
    pub default_action: String,
    pub rules: Vec<Value>,
    pub capacity: u32,
}

#[derive(Debug, Clone)]
pub struct WafRuleGroup {
    pub name: String,
    pub id: String,
    pub arn: String,
    pub capacity: u32,
    pub rules: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct IpSet {
    pub name: String,
    pub id: String,
    pub arn: String,
    pub ip_version: String,
    pub addresses: Vec<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct WafState {
    pub web_acls: DashMap<String, WebAcl>,
    pub rule_groups: DashMap<String, WafRuleGroup>,
    pub ip_sets: DashMap<String, IpSet>,
}

impl Default for WafState {
    fn default() -> Self {
        Self {
            web_acls: DashMap::new(),
            rule_groups: DashMap::new(),
            ip_sets: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &WafState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target.strip_prefix("AWSWAF_20190729.").unwrap_or(target);

    let result = match action {
        "CreateWebACL" => create_web_acl(state, payload).await,
        "DeleteWebACL" => delete_web_acl(state, payload).await,
        "GetWebACL" => get_web_acl(state, payload).await,
        "ListWebACLs" => list_web_acls(state).await,
        "CreateRuleGroup" => create_rule_group(state, payload).await,
        "DeleteRuleGroup" => delete_rule_group(state, payload).await,
        "ListRuleGroups" => list_rule_groups(state).await,
        "CreateIPSet" => create_ip_set(state, payload).await,
        "DeleteIPSet" => delete_ip_set(state, payload).await,
        "ListIPSets" => list_ip_sets(state).await,
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

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

async fn create_web_acl(
    state: &WafState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:wafv2:{}:{}:regional/webacl/{}/{}",
        REGION, ACCOUNT_ID, name, id
    );

    let default_action = if payload["DefaultAction"]["Allow"].is_object()
        || payload["DefaultAction"]["Allow"].is_null()
            && !payload["DefaultAction"]["Block"].is_object()
    {
        "ALLOW"
    } else {
        "BLOCK"
    };

    let rules = payload["Rules"].as_array().cloned().unwrap_or_default();

    let capacity = payload["Capacity"].as_u64().unwrap_or(100) as u32;

    let acl = WebAcl {
        name: name.clone(),
        id: id.clone(),
        arn: arn.clone(),
        default_action: default_action.to_string(),
        rules,
        capacity,
    };

    state.web_acls.insert(id.clone(), acl);

    Ok(json_response(json!({
        "Summary": {
            "Name": name,
            "Id": id,
            "ARN": arn,
        }
    })))
}

async fn delete_web_acl(
    state: &WafState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let id = payload["Id"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Id is required".to_string()))?;

    state
        .web_acls
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("WebACL not found: {}", id)))?;

    Ok(json_response(json!({})))
}

async fn get_web_acl(state: &WafState, payload: &serde_json::Value) -> Result<Response, LawsError> {
    let id = payload["Id"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Id is required".to_string()))?;

    let acl = state
        .web_acls
        .get(id)
        .ok_or_else(|| LawsError::NotFound(format!("WebACL not found: {}", id)))?;

    Ok(json_response(json!({
        "WebACL": {
            "Name": acl.name,
            "Id": acl.id,
            "ARN": acl.arn,
            "DefaultAction": {
                acl.default_action.clone(): {},
            },
            "Rules": acl.rules,
            "Capacity": acl.capacity,
        },
        "LockToken": uuid::Uuid::new_v4().to_string(),
    })))
}

async fn list_web_acls(state: &WafState) -> Result<Response, LawsError> {
    let acls: Vec<Value> = state
        .web_acls
        .iter()
        .map(|entry| {
            let acl = entry.value();
            json!({
                "Name": acl.name,
                "Id": acl.id,
                "ARN": acl.arn,
            })
        })
        .collect();

    Ok(json_response(json!({
        "WebACLs": acls,
    })))
}

async fn create_rule_group(
    state: &WafState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:wafv2:{}:{}:regional/rulegroup/{}/{}",
        REGION, ACCOUNT_ID, name, id
    );
    let capacity = payload["Capacity"].as_u64().unwrap_or(100) as u32;
    let rules = payload["Rules"].as_array().cloned().unwrap_or_default();

    let rule_group = WafRuleGroup {
        name: name.clone(),
        id: id.clone(),
        arn: arn.clone(),
        capacity,
        rules,
    };

    state.rule_groups.insert(id.clone(), rule_group);

    Ok(json_response(json!({
        "Summary": {
            "Name": name,
            "Id": id,
            "ARN": arn,
        }
    })))
}

async fn delete_rule_group(
    state: &WafState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let id = payload["Id"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Id is required".to_string()))?;

    state
        .rule_groups
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("RuleGroup not found: {}", id)))?;

    Ok(json_response(json!({})))
}

async fn list_rule_groups(state: &WafState) -> Result<Response, LawsError> {
    let groups: Vec<Value> = state
        .rule_groups
        .iter()
        .map(|entry| {
            let rg = entry.value();
            json!({
                "Name": rg.name,
                "Id": rg.id,
                "ARN": rg.arn,
            })
        })
        .collect();

    Ok(json_response(json!({
        "RuleGroups": groups,
    })))
}

async fn create_ip_set(
    state: &WafState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:wafv2:{}:{}:regional/ipset/{}/{}",
        REGION, ACCOUNT_ID, name, id
    );
    let ip_version = payload["IPAddressVersion"]
        .as_str()
        .unwrap_or("IPV4")
        .to_string();
    let addresses: Vec<String> = payload["Addresses"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let ip_set = IpSet {
        name: name.clone(),
        id: id.clone(),
        arn: arn.clone(),
        ip_version,
        addresses,
    };

    state.ip_sets.insert(id.clone(), ip_set);

    Ok(json_response(json!({
        "Summary": {
            "Name": name,
            "Id": id,
            "ARN": arn,
        }
    })))
}

async fn delete_ip_set(
    state: &WafState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let id = payload["Id"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Id is required".to_string()))?;

    state
        .ip_sets
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("IPSet not found: {}", id)))?;

    Ok(json_response(json!({})))
}

async fn list_ip_sets(state: &WafState) -> Result<Response, LawsError> {
    let sets: Vec<Value> = state
        .ip_sets
        .iter()
        .map(|entry| {
            let ip = entry.value();
            json!({
                "Name": ip.name,
                "Id": ip.id,
                "ARN": ip.arn,
            })
        })
        .collect();

    Ok(json_response(json!({
        "IPSets": sets,
    })))
}
