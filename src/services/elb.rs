use axum::body::Bytes;
use axum::http::{HeaderMap, Uri};
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;
use crate::protocol::query::{parse_query_request, xml_error_response, xml_response};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LoadBalancer {
    pub name: String,
    pub arn: String,
    pub dns_name: String,
    pub type_: String,
    pub scheme: String,
    pub state: String,
    pub vpc_id: String,
}

#[derive(Debug, Clone)]
pub struct TargetGroup {
    pub name: String,
    pub arn: String,
    pub protocol: String,
    pub port: u16,
    pub vpc_id: String,
    pub health_check_path: String,
    pub targets: Vec<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ElbState {
    pub load_balancers: DashMap<String, LoadBalancer>,
    pub target_groups: DashMap<String, TargetGroup>,
}

impl Default for ElbState {
    fn default() -> Self {
        Self {
            load_balancers: DashMap::new(),
            target_groups: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &ElbState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("ElasticLoadBalancingV2.")
        .unwrap_or(target);

    let result = match action {
        "CreateLoadBalancer" => create_load_balancer(state, payload),
        "DeleteLoadBalancer" => delete_load_balancer(state, payload),
        "DescribeLoadBalancers" => describe_load_balancers(state, payload),
        "CreateTargetGroup" => create_target_group(state, payload),
        "DeleteTargetGroup" => delete_target_group(state, payload),
        "DescribeTargetGroups" => describe_target_groups(state, payload),
        "RegisterTargets" => register_targets(state, payload),
        "DeregisterTargets" => deregister_targets(state, payload),
        "CreateListener" => create_listener(state, payload),
        "DescribeListeners" => describe_listeners(state, payload),
        "DeleteListener" => delete_listener(state, payload),
        "DescribeRules" => describe_rules(state, payload),
        "DeleteRule" => delete_rule(state, payload),
        "DescribeTargetHealth" => describe_target_health(state, payload),
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

fn random_hex8() -> String {
    let bytes: [u8; 4] = rand::random();
    hex::encode(bytes)
}

fn lb_to_json(lb: &LoadBalancer) -> Value {
    json!({
        "LoadBalancerName": lb.name,
        "LoadBalancerArn": lb.arn,
        "DNSName": lb.dns_name,
        "Type": lb.type_,
        "Scheme": lb.scheme,
        "State": { "Code": lb.state },
        "VpcId": lb.vpc_id,
        "AvailabilityZones": [],
        "SecurityGroups": []
    })
}

fn tg_to_json(tg: &TargetGroup) -> Value {
    json!({
        "TargetGroupName": tg.name,
        "TargetGroupArn": tg.arn,
        "Protocol": tg.protocol,
        "Port": tg.port,
        "VpcId": tg.vpc_id,
        "HealthCheckPath": tg.health_check_path,
        "TargetType": "instance",
        "HealthCheckProtocol": "HTTP"
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_load_balancer(state: &ElbState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload
        .get("Name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest("missing required field: Name".into()))?
        .to_owned();

    if state.load_balancers.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "load balancer already exists: {name}"
        )));
    }

    let hex = random_hex8();
    let arn =
        format!("arn:aws:elasticloadbalancing:{REGION}:{ACCOUNT_ID}:loadbalancer/app/{name}/{hex}");
    let dns_name = format!("{name}-{hex}.{REGION}.elb.amazonaws.com");

    let lb = LoadBalancer {
        name: name.clone(),
        arn,
        dns_name,
        type_: "application".into(),
        scheme: payload
            .get("Scheme")
            .and_then(|v| v.as_str())
            .unwrap_or("internet-facing")
            .to_owned(),
        state: "active".into(),
        vpc_id: "vpc-12345678".into(),
    };

    let resp = lb_to_json(&lb);
    state.load_balancers.insert(name, lb);

    Ok(json_response(json!({
        "LoadBalancers": [resp]
    })))
}

fn delete_load_balancer(state: &ElbState, payload: &Value) -> Result<Response, LawsError> {
    let arn = payload
        .get("LoadBalancerArn")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            LawsError::InvalidRequest("missing required field: LoadBalancerArn".into())
        })?;

    let name = state
        .load_balancers
        .iter()
        .find(|entry| entry.value().arn == arn)
        .map(|entry| entry.key().clone())
        .ok_or_else(|| LawsError::NotFound(format!("load balancer not found: {arn}")))?;

    state.load_balancers.remove(&name);

    Ok(json_response(json!({})))
}

fn describe_load_balancers(state: &ElbState, payload: &Value) -> Result<Response, LawsError> {
    let lbs: Vec<Value> = if let Some(names) = payload.get("Names").and_then(|v| v.as_array()) {
        names
            .iter()
            .filter_map(|n| {
                let name = n.as_str()?;
                state.load_balancers.get(name).map(|lb| lb_to_json(&lb))
            })
            .collect()
    } else {
        state
            .load_balancers
            .iter()
            .map(|entry| lb_to_json(entry.value()))
            .collect()
    };

    Ok(json_response(json!({
        "LoadBalancers": lbs
    })))
}

fn create_target_group(state: &ElbState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload
        .get("Name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest("missing required field: Name".into()))?
        .to_owned();

    if state.target_groups.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "target group already exists: {name}"
        )));
    }

    let hex = random_hex8();
    let arn =
        format!("arn:aws:elasticloadbalancing:{REGION}:{ACCOUNT_ID}:targetgroup/{name}/{hex}");

    let port = payload.get("Port").and_then(|v| v.as_u64()).unwrap_or(80) as u16;

    let tg = TargetGroup {
        name: name.clone(),
        arn,
        protocol: payload
            .get("Protocol")
            .and_then(|v| v.as_str())
            .unwrap_or("HTTP")
            .to_owned(),
        port,
        vpc_id: payload
            .get("VpcId")
            .and_then(|v| v.as_str())
            .unwrap_or("vpc-12345678")
            .to_owned(),
        health_check_path: payload
            .get("HealthCheckPath")
            .and_then(|v| v.as_str())
            .unwrap_or("/")
            .to_owned(),
        targets: Vec::new(),
    };

    let resp = tg_to_json(&tg);
    state.target_groups.insert(name, tg);

    Ok(json_response(json!({
        "TargetGroups": [resp]
    })))
}

fn delete_target_group(state: &ElbState, payload: &Value) -> Result<Response, LawsError> {
    let arn = payload
        .get("TargetGroupArn")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            LawsError::InvalidRequest("missing required field: TargetGroupArn".into())
        })?;

    let name = state
        .target_groups
        .iter()
        .find(|entry| entry.value().arn == arn)
        .map(|entry| entry.key().clone())
        .ok_or_else(|| LawsError::NotFound(format!("target group not found: {arn}")))?;

    state.target_groups.remove(&name);

    Ok(json_response(json!({})))
}

fn describe_target_groups(state: &ElbState, payload: &Value) -> Result<Response, LawsError> {
    let tgs: Vec<Value> = if let Some(names) = payload.get("Names").and_then(|v| v.as_array()) {
        names
            .iter()
            .filter_map(|n| {
                let name = n.as_str()?;
                state.target_groups.get(name).map(|tg| tg_to_json(&tg))
            })
            .collect()
    } else {
        state
            .target_groups
            .iter()
            .map(|entry| tg_to_json(entry.value()))
            .collect()
    };

    Ok(json_response(json!({
        "TargetGroups": tgs
    })))
}

fn register_targets(state: &ElbState, payload: &Value) -> Result<Response, LawsError> {
    let arn = payload
        .get("TargetGroupArn")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            LawsError::InvalidRequest("missing required field: TargetGroupArn".into())
        })?;

    let name = state
        .target_groups
        .iter()
        .find(|entry| entry.value().arn == arn)
        .map(|entry| entry.key().clone())
        .ok_or_else(|| LawsError::NotFound(format!("target group not found: {arn}")))?;

    let targets = payload
        .get("Targets")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut tg = state
        .target_groups
        .get_mut(&name)
        .ok_or_else(|| LawsError::NotFound(format!("target group not found: {name}")))?;

    for target in &targets {
        if let Some(id) = target.get("Id").and_then(|v| v.as_str()) {
            tg.targets.push(id.to_owned());
        }
    }

    Ok(json_response(json!({})))
}

fn create_listener(state: &ElbState, payload: &Value) -> Result<Response, LawsError> {
    let lb_arn = payload
        .get("LoadBalancerArn")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            LawsError::InvalidRequest("missing required field: LoadBalancerArn".into())
        })?;

    // Verify load balancer exists
    let _lb = state
        .load_balancers
        .iter()
        .find(|entry| entry.value().arn == lb_arn)
        .ok_or_else(|| LawsError::NotFound(format!("load balancer not found: {lb_arn}")))?;

    let port = payload.get("Port").and_then(|v| v.as_u64()).unwrap_or(80);

    let protocol = payload
        .get("Protocol")
        .and_then(|v| v.as_str())
        .unwrap_or("HTTP");

    let listener_arn = format!(
        "arn:aws:elasticloadbalancing:{REGION}:{ACCOUNT_ID}:listener/app/{}/{}",
        random_hex8(),
        random_hex8()
    );

    Ok(json_response(json!({
        "Listeners": [{
            "ListenerArn": listener_arn,
            "LoadBalancerArn": lb_arn,
            "Port": port,
            "Protocol": protocol,
            "DefaultActions": payload.get("DefaultActions").cloned().unwrap_or(json!([]))
        }]
    })))
}

fn deregister_targets(state: &ElbState, payload: &Value) -> Result<Response, LawsError> {
    let arn = payload
        .get("TargetGroupArn")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            LawsError::InvalidRequest("missing required field: TargetGroupArn".into())
        })?;

    let name = state
        .target_groups
        .iter()
        .find(|entry| entry.value().arn == arn)
        .map(|entry| entry.key().clone())
        .ok_or_else(|| LawsError::NotFound(format!("target group not found: {arn}")))?;

    let targets_to_remove: Vec<String> = payload
        .get("Targets")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.get("Id").and_then(|v| v.as_str()).map(|s| s.to_owned()))
                .collect()
        })
        .unwrap_or_default();

    if let Some(mut tg) = state.target_groups.get_mut(&name) {
        tg.targets.retain(|t| !targets_to_remove.contains(t));
    }

    Ok(json_response(json!({})))
}

fn describe_listeners(_state: &ElbState, payload: &Value) -> Result<Response, LawsError> {
    let lb_arn = payload.get("LoadBalancerArn").and_then(|v| v.as_str());
    let listener_arns = payload.get("ListenerArns").and_then(|v| v.as_array());

    // Mock: return a default listener if a load balancer ARN is specified
    let listeners: Vec<Value> = if let Some(arn) = lb_arn {
        vec![json!({
            "ListenerArn": format!("arn:aws:elasticloadbalancing:{REGION}:{ACCOUNT_ID}:listener/app/{}/{}", random_hex8(), random_hex8()),
            "LoadBalancerArn": arn,
            "Port": 80,
            "Protocol": "HTTP",
            "DefaultActions": []
        })]
    } else if let Some(_arns) = listener_arns {
        // Return empty — we don't persist listeners in this mock
        vec![]
    } else {
        vec![]
    };

    Ok(json_response(json!({ "Listeners": listeners })))
}

fn delete_listener(_state: &ElbState, payload: &Value) -> Result<Response, LawsError> {
    let _listener_arn = payload
        .get("ListenerArn")
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest("missing required field: ListenerArn".into()))?;
    // Mock: just acknowledge the delete
    Ok(json_response(json!({})))
}

fn describe_rules(_state: &ElbState, payload: &Value) -> Result<Response, LawsError> {
    let _listener_arn = payload.get("ListenerArn").and_then(|v| v.as_str());
    // Mock: return empty rules list
    Ok(json_response(json!({ "Rules": [] })))
}

fn delete_rule(_state: &ElbState, payload: &Value) -> Result<Response, LawsError> {
    let _rule_arn = payload
        .get("RuleArn")
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest("missing required field: RuleArn".into()))?;
    // Mock: just acknowledge the delete
    Ok(json_response(json!({})))
}

fn describe_target_health(state: &ElbState, payload: &Value) -> Result<Response, LawsError> {
    let arn = payload
        .get("TargetGroupArn")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            LawsError::InvalidRequest("missing required field: TargetGroupArn".into())
        })?;

    let tg = state
        .target_groups
        .iter()
        .find(|entry| entry.value().arn == arn)
        .ok_or_else(|| LawsError::NotFound(format!("target group not found: {arn}")))?;

    let descriptions: Vec<Value> = tg
        .targets
        .iter()
        .map(|id| {
            json!({
                "Target": { "Id": id, "Port": tg.port },
                "TargetHealth": { "State": "healthy" }
            })
        })
        .collect();

    Ok(json_response(json!({
        "TargetHealthDescriptions": descriptions
    })))
}

// ---------------------------------------------------------------------------
// XML helpers for query protocol
// ---------------------------------------------------------------------------

fn lb_to_xml(lb: &LoadBalancer) -> String {
    format!(
        "<member>\
            <LoadBalancerName>{name}</LoadBalancerName>\
            <LoadBalancerArn>{arn}</LoadBalancerArn>\
            <DNSName>{dns}</DNSName>\
            <Type>{type_}</Type>\
            <Scheme>{scheme}</Scheme>\
            <State><Code>{state}</Code></State>\
            <VpcId>{vpc}</VpcId>\
            <AvailabilityZones/>\
            <SecurityGroups/>\
        </member>",
        name = lb.name,
        arn = lb.arn,
        dns = lb.dns_name,
        type_ = lb.type_,
        scheme = lb.scheme,
        state = lb.state,
        vpc = lb.vpc_id,
    )
}

fn tg_to_xml(tg: &TargetGroup) -> String {
    format!(
        "<member>\
            <TargetGroupName>{name}</TargetGroupName>\
            <TargetGroupArn>{arn}</TargetGroupArn>\
            <Protocol>{protocol}</Protocol>\
            <Port>{port}</Port>\
            <VpcId>{vpc}</VpcId>\
            <HealthCheckPath>{hc}</HealthCheckPath>\
            <TargetType>instance</TargetType>\
            <HealthCheckProtocol>HTTP</HealthCheckProtocol>\
        </member>",
        name = tg.name,
        arn = tg.arn,
        protocol = tg.protocol,
        port = tg.port,
        vpc = tg.vpc_id,
        hc = tg.health_check_path,
    )
}

// ---------------------------------------------------------------------------
// Query protocol handler (XML responses for taws compatibility)
// ---------------------------------------------------------------------------

pub fn handle_query_request(
    state: &ElbState,
    headers: &HeaderMap,
    body: &Bytes,
    uri: &Uri,
) -> Response {
    let req = match parse_query_request(uri, headers, body) {
        Ok(r) => r,
        Err(e) => return xml_error_response(&e),
    };

    let result = match req.action.as_str() {
        "CreateLoadBalancer" => query_create_load_balancer(state, &req.params),
        "DeleteLoadBalancer" => query_delete_load_balancer(state, &req.params),
        "DescribeLoadBalancers" => query_describe_load_balancers(state, &req.params),
        "CreateTargetGroup" => query_create_target_group(state, &req.params),
        "DeleteTargetGroup" => query_delete_target_group(state, &req.params),
        "DescribeTargetGroups" => query_describe_target_groups(state, &req.params),
        "RegisterTargets" => query_register_targets(state, &req.params),
        "DeregisterTargets" => query_deregister_targets(state, &req.params),
        "CreateListener" => query_create_listener(state, &req.params),
        "DescribeListeners" => query_describe_listeners(state, &req.params),
        "DeleteListener" => query_delete_listener(state, &req.params),
        "DescribeRules" => query_describe_rules(state, &req.params),
        "DeleteRule" => query_delete_rule(state, &req.params),
        "DescribeTargetHealth" => query_describe_target_health(state, &req.params),
        _ => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            req.action
        ))),
    };

    match result {
        Ok(resp) => resp,
        Err(e) => xml_error_response(&e),
    }
}

// ---------------------------------------------------------------------------
// Query protocol operations (XML)
// ---------------------------------------------------------------------------

fn query_create_load_balancer(
    state: &ElbState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let name = params
        .get("Name")
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    if state.load_balancers.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "load balancer already exists: {name}"
        )));
    }

    let hex = random_hex8();
    let arn =
        format!("arn:aws:elasticloadbalancing:{REGION}:{ACCOUNT_ID}:loadbalancer/app/{name}/{hex}");
    let dns_name = format!("{name}-{hex}.{REGION}.elb.amazonaws.com");

    let lb = LoadBalancer {
        name: name.clone(),
        arn,
        dns_name,
        type_: "application".into(),
        scheme: params
            .get("Scheme")
            .cloned()
            .unwrap_or_else(|| "internet-facing".into()),
        state: "active".into(),
        vpc_id: "vpc-12345678".into(),
    };

    let xml = lb_to_xml(&lb);
    state.load_balancers.insert(name, lb);

    Ok(xml_response(
        "CreateLoadBalancer",
        &format!("<LoadBalancers>{xml}</LoadBalancers>"),
    ))
}

fn query_delete_load_balancer(
    state: &ElbState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let arn = params
        .get("LoadBalancerArn")
        .ok_or_else(|| LawsError::InvalidRequest("LoadBalancerArn is required".to_string()))?;

    let name = state
        .load_balancers
        .iter()
        .find(|entry| entry.value().arn == arn.as_str())
        .map(|entry| entry.key().clone())
        .ok_or_else(|| LawsError::NotFound(format!("load balancer not found: {arn}")))?;

    state.load_balancers.remove(&name);

    Ok(xml_response("DeleteLoadBalancer", ""))
}

fn query_describe_load_balancers(
    state: &ElbState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let filter_name = params.get("Names.member.1").map(|s| s.as_str());

    let lbs: Vec<String> = state
        .load_balancers
        .iter()
        .filter(|entry| filter_name.map(|n| entry.key() == n).unwrap_or(true))
        .map(|entry| lb_to_xml(entry.value()))
        .collect();

    let inner = format!("<LoadBalancers>{}</LoadBalancers>", lbs.join(""));
    Ok(xml_response("DescribeLoadBalancers", &inner))
}

fn query_create_target_group(
    state: &ElbState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let name = params
        .get("Name")
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    if state.target_groups.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "target group already exists: {name}"
        )));
    }

    let hex = random_hex8();
    let arn =
        format!("arn:aws:elasticloadbalancing:{REGION}:{ACCOUNT_ID}:targetgroup/{name}/{hex}");
    let port = params
        .get("Port")
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(80);

    let tg = TargetGroup {
        name: name.clone(),
        arn,
        protocol: params
            .get("Protocol")
            .cloned()
            .unwrap_or_else(|| "HTTP".into()),
        port,
        vpc_id: params
            .get("VpcId")
            .cloned()
            .unwrap_or_else(|| "vpc-12345678".into()),
        health_check_path: params
            .get("HealthCheckPath")
            .cloned()
            .unwrap_or_else(|| "/".into()),
        targets: Vec::new(),
    };

    let xml = tg_to_xml(&tg);
    state.target_groups.insert(name, tg);

    Ok(xml_response(
        "CreateTargetGroup",
        &format!("<TargetGroups>{xml}</TargetGroups>"),
    ))
}

fn query_delete_target_group(
    state: &ElbState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let arn = params
        .get("TargetGroupArn")
        .ok_or_else(|| LawsError::InvalidRequest("TargetGroupArn is required".to_string()))?;

    let name = state
        .target_groups
        .iter()
        .find(|entry| entry.value().arn == arn.as_str())
        .map(|entry| entry.key().clone())
        .ok_or_else(|| LawsError::NotFound(format!("target group not found: {arn}")))?;

    state.target_groups.remove(&name);

    Ok(xml_response("DeleteTargetGroup", ""))
}

fn query_describe_target_groups(
    state: &ElbState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let filter_name = params.get("Names.member.1").map(|s| s.as_str());

    let tgs: Vec<String> = state
        .target_groups
        .iter()
        .filter(|entry| filter_name.map(|n| entry.key() == n).unwrap_or(true))
        .map(|entry| tg_to_xml(entry.value()))
        .collect();

    let inner = format!("<TargetGroups>{}</TargetGroups>", tgs.join(""));
    Ok(xml_response("DescribeTargetGroups", &inner))
}

fn query_register_targets(
    state: &ElbState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let arn = params
        .get("TargetGroupArn")
        .ok_or_else(|| LawsError::InvalidRequest("TargetGroupArn is required".to_string()))?;

    let name = state
        .target_groups
        .iter()
        .find(|entry| entry.value().arn == arn.as_str())
        .map(|entry| entry.key().clone())
        .ok_or_else(|| LawsError::NotFound(format!("target group not found: {arn}")))?;

    // Collect targets from Targets.member.N.Id
    let mut targets = Vec::new();
    for i in 1.. {
        let key = format!("Targets.member.{i}.Id");
        if let Some(id) = params.get(&key) {
            targets.push(id.clone());
        } else {
            break;
        }
    }

    if let Some(mut tg) = state.target_groups.get_mut(&name) {
        tg.targets.extend(targets);
    }

    Ok(xml_response("RegisterTargets", ""))
}

fn query_deregister_targets(
    state: &ElbState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let arn = params
        .get("TargetGroupArn")
        .ok_or_else(|| LawsError::InvalidRequest("TargetGroupArn is required".to_string()))?;

    let name = state
        .target_groups
        .iter()
        .find(|entry| entry.value().arn == arn.as_str())
        .map(|entry| entry.key().clone())
        .ok_or_else(|| LawsError::NotFound(format!("target group not found: {arn}")))?;

    let mut targets_to_remove = Vec::new();
    for i in 1.. {
        let key = format!("Targets.member.{i}.Id");
        if let Some(id) = params.get(&key) {
            targets_to_remove.push(id.clone());
        } else {
            break;
        }
    }

    if let Some(mut tg) = state.target_groups.get_mut(&name) {
        tg.targets.retain(|t| !targets_to_remove.contains(t));
    }

    Ok(xml_response("DeregisterTargets", ""))
}

fn query_create_listener(
    state: &ElbState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let lb_arn = params
        .get("LoadBalancerArn")
        .ok_or_else(|| LawsError::InvalidRequest("LoadBalancerArn is required".to_string()))?;

    let _lb = state
        .load_balancers
        .iter()
        .find(|entry| entry.value().arn == lb_arn.as_str())
        .ok_or_else(|| LawsError::NotFound(format!("load balancer not found: {lb_arn}")))?;

    let port = params
        .get("Port")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(80);
    let protocol = params.get("Protocol").map(|s| s.as_str()).unwrap_or("HTTP");
    let listener_arn = format!(
        "arn:aws:elasticloadbalancing:{REGION}:{ACCOUNT_ID}:listener/app/{}/{}",
        random_hex8(),
        random_hex8()
    );

    let inner = format!(
        "<Listeners><member>\
            <ListenerArn>{listener_arn}</ListenerArn>\
            <LoadBalancerArn>{lb_arn}</LoadBalancerArn>\
            <Port>{port}</Port>\
            <Protocol>{protocol}</Protocol>\
            <DefaultActions/>\
        </member></Listeners>"
    );
    Ok(xml_response("CreateListener", &inner))
}

fn query_describe_listeners(
    _state: &ElbState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let lb_arn = params.get("LoadBalancerArn");

    let inner = if let Some(arn) = lb_arn {
        let listener_arn = format!(
            "arn:aws:elasticloadbalancing:{REGION}:{ACCOUNT_ID}:listener/app/{}/{}",
            random_hex8(),
            random_hex8()
        );
        format!(
            "<Listeners><member>\
                <ListenerArn>{listener_arn}</ListenerArn>\
                <LoadBalancerArn>{arn}</LoadBalancerArn>\
                <Port>80</Port>\
                <Protocol>HTTP</Protocol>\
                <DefaultActions/>\
            </member></Listeners>"
        )
    } else {
        "<Listeners/>".to_string()
    };

    Ok(xml_response("DescribeListeners", &inner))
}

fn query_delete_listener(
    _state: &ElbState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let _arn = params
        .get("ListenerArn")
        .ok_or_else(|| LawsError::InvalidRequest("ListenerArn is required".to_string()))?;
    Ok(xml_response("DeleteListener", ""))
}

fn query_describe_rules(
    _state: &ElbState,
    _params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    Ok(xml_response("DescribeRules", "<Rules/>"))
}

fn query_delete_rule(
    _state: &ElbState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let _arn = params
        .get("RuleArn")
        .ok_or_else(|| LawsError::InvalidRequest("RuleArn is required".to_string()))?;
    Ok(xml_response("DeleteRule", ""))
}

fn query_describe_target_health(
    state: &ElbState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let arn = params
        .get("TargetGroupArn")
        .ok_or_else(|| LawsError::InvalidRequest("TargetGroupArn is required".to_string()))?;

    let tg = state
        .target_groups
        .iter()
        .find(|entry| entry.value().arn == arn.as_str())
        .ok_or_else(|| LawsError::NotFound(format!("target group not found: {arn}")))?;

    let members: Vec<String> = tg
        .targets
        .iter()
        .map(|id| {
            format!(
                "<member>\
                    <Target><Id>{id}</Id><Port>{port}</Port></Target>\
                    <TargetHealth><State>healthy</State></TargetHealth>\
                </member>",
                port = tg.port
            )
        })
        .collect();

    let inner = format!(
        "<TargetHealthDescriptions>{}</TargetHealthDescriptions>",
        members.join("")
    );
    Ok(xml_response("DescribeTargetHealth", &inner))
}
