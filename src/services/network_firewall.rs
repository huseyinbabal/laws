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
pub struct Firewall {
    pub firewall_name: String,
    pub firewall_arn: String,
    pub firewall_id: String,
    pub firewall_policy_arn: String,
    pub vpc_id: String,
    pub subnet_mappings: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct FirewallPolicy {
    pub policy_name: String,
    pub policy_arn: String,
    pub policy_id: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct RuleGroup {
    pub rule_group_name: String,
    pub rule_group_arn: String,
    pub rule_group_id: String,
    pub rule_group_type: String,
    pub capacity: i64,
    pub description: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct NetworkFirewallState {
    pub firewalls: DashMap<String, Firewall>,
    pub policies: DashMap<String, FirewallPolicy>,
    pub rule_groups: DashMap<String, RuleGroup>,
}

impl Default for NetworkFirewallState {
    fn default() -> Self {
        Self {
            firewalls: DashMap::new(),
            policies: DashMap::new(),
            rule_groups: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &NetworkFirewallState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("NetworkFirewall_20201112.")
        .unwrap_or(target);

    let result = match action {
        "CreateFirewall" => create_firewall(state, payload),
        "DeleteFirewall" => delete_firewall(state, payload),
        "DescribeFirewall" => describe_firewall(state, payload),
        "ListFirewalls" => list_firewalls(state),
        "CreateFirewallPolicy" => create_firewall_policy(state, payload),
        "DescribeFirewallPolicy" => describe_firewall_policy(state, payload),
        "CreateRuleGroup" => create_rule_group(state, payload),
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

fn create_firewall(state: &NetworkFirewallState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["FirewallName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing FirewallName".into()))?
        .to_string();

    let firewall_policy_arn = payload["FirewallPolicyArn"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let vpc_id = payload["VpcId"]
        .as_str()
        .unwrap_or("vpc-00000000")
        .to_string();

    let subnet_mappings: Vec<String> = payload["SubnetMappings"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v["SubnetId"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let firewall_id = uuid::Uuid::new_v4().to_string();
    let firewall_arn = format!("arn:aws:network-firewall:{REGION}:{ACCOUNT_ID}:firewall/{name}");

    let firewall = Firewall {
        firewall_name: name.clone(),
        firewall_arn: firewall_arn.clone(),
        firewall_id: firewall_id.clone(),
        firewall_policy_arn: firewall_policy_arn.clone(),
        vpc_id: vpc_id.clone(),
        subnet_mappings: subnet_mappings.clone(),
        status: "PROVISIONING".to_string(),
    };

    let resp_name = name.clone();
    state.firewalls.insert(name, firewall);

    Ok(json_response(json!({
        "Firewall": {
            "FirewallName": resp_name,
            "FirewallArn": firewall_arn,
            "FirewallId": firewall_id,
            "FirewallPolicyArn": firewall_policy_arn,
            "VpcId": vpc_id,
            "SubnetMappings": subnet_mappings.iter().map(|s| json!({"SubnetId": s})).collect::<Vec<_>>(),
        },
        "FirewallStatus": {
            "Status": "PROVISIONING",
            "ConfigurationSyncStateSummary": "PENDING",
        }
    })))
}

fn delete_firewall(state: &NetworkFirewallState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["FirewallName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing FirewallName".into()))?;

    let (_, firewall) = state
        .firewalls
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Firewall '{}' not found", name)))?;

    Ok(json_response(json!({
        "Firewall": {
            "FirewallName": firewall.firewall_name,
            "FirewallArn": firewall.firewall_arn,
            "FirewallId": firewall.firewall_id,
        },
        "FirewallStatus": {
            "Status": "DELETING",
        }
    })))
}

fn describe_firewall(state: &NetworkFirewallState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["FirewallName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing FirewallName".into()))?;

    let fw = state
        .firewalls
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("Firewall '{}' not found", name)))?;

    Ok(json_response(json!({
        "Firewall": {
            "FirewallName": fw.firewall_name,
            "FirewallArn": fw.firewall_arn,
            "FirewallId": fw.firewall_id,
            "FirewallPolicyArn": fw.firewall_policy_arn,
            "VpcId": fw.vpc_id,
            "SubnetMappings": fw.subnet_mappings.iter().map(|s| json!({"SubnetId": s})).collect::<Vec<_>>(),
        },
        "FirewallStatus": {
            "Status": fw.status,
            "ConfigurationSyncStateSummary": "IN_SYNC",
        }
    })))
}

fn list_firewalls(state: &NetworkFirewallState) -> Result<Response, LawsError> {
    let firewalls: Vec<Value> = state
        .firewalls
        .iter()
        .map(|entry| {
            let fw = entry.value();
            json!({
                "FirewallName": fw.firewall_name,
                "FirewallArn": fw.firewall_arn,
            })
        })
        .collect();

    Ok(json_response(json!({ "Firewalls": firewalls })))
}

fn create_firewall_policy(
    state: &NetworkFirewallState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["FirewallPolicyName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing FirewallPolicyName".into()))?
        .to_string();

    let description = payload["Description"].as_str().unwrap_or("").to_string();

    let policy_id = uuid::Uuid::new_v4().to_string();
    let policy_arn =
        format!("arn:aws:network-firewall:{REGION}:{ACCOUNT_ID}:firewall-policy/{name}");

    let policy = FirewallPolicy {
        policy_name: name.clone(),
        policy_arn: policy_arn.clone(),
        policy_id: policy_id.clone(),
        description: description.clone(),
    };

    state.policies.insert(name.clone(), policy);

    Ok(json_response(json!({
        "FirewallPolicyResponse": {
            "FirewallPolicyName": name,
            "FirewallPolicyArn": policy_arn,
            "FirewallPolicyId": policy_id,
            "Description": description,
        },
        "UpdateToken": uuid::Uuid::new_v4().to_string(),
    })))
}

fn describe_firewall_policy(
    state: &NetworkFirewallState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["FirewallPolicyName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing FirewallPolicyName".into()))?;

    let policy = state
        .policies
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("FirewallPolicy '{}' not found", name)))?;

    Ok(json_response(json!({
        "FirewallPolicyResponse": {
            "FirewallPolicyName": policy.policy_name,
            "FirewallPolicyArn": policy.policy_arn,
            "FirewallPolicyId": policy.policy_id,
            "Description": policy.description,
        },
        "UpdateToken": uuid::Uuid::new_v4().to_string(),
    })))
}

fn create_rule_group(state: &NetworkFirewallState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["RuleGroupName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing RuleGroupName".into()))?
        .to_string();

    let rule_group_type = payload["Type"].as_str().unwrap_or("STATELESS").to_string();

    let capacity = payload["Capacity"].as_i64().unwrap_or(100);

    let description = payload["Description"].as_str().unwrap_or("").to_string();

    let rule_group_id = uuid::Uuid::new_v4().to_string();
    let rule_group_arn =
        format!("arn:aws:network-firewall:{REGION}:{ACCOUNT_ID}:stateless-rulegroup/{name}");

    let rg = RuleGroup {
        rule_group_name: name.clone(),
        rule_group_arn: rule_group_arn.clone(),
        rule_group_id: rule_group_id.clone(),
        rule_group_type: rule_group_type.clone(),
        capacity,
        description: description.clone(),
    };

    state.rule_groups.insert(name.clone(), rg);

    Ok(json_response(json!({
        "RuleGroupResponse": {
            "RuleGroupName": name,
            "RuleGroupArn": rule_group_arn,
            "RuleGroupId": rule_group_id,
            "Type": rule_group_type,
            "Capacity": capacity,
            "Description": description,
        },
        "UpdateToken": uuid::Uuid::new_v4().to_string(),
    })))
}
