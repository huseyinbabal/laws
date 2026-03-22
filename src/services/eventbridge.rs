use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::post;
use dashmap::DashMap;
use serde_json::{json, Value};

use crate::error::LawsError;
use crate::protocol::json::{json_error_response, json_response, parse_target};

// ---------------------------------------------------------------------------
// State & data model
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct EventBus {
    pub name: String,
    pub arn: String,
}

#[derive(Clone, Debug)]
pub struct EventTarget {
    pub id: String,
    pub arn: String,
}

#[derive(Clone, Debug)]
pub struct EventRule {
    pub name: String,
    pub event_bus_name: String,
    pub event_pattern: Option<String>,
    pub schedule_expression: Option<String>,
    pub state: String,
    pub arn: String,
    pub targets: Vec<EventTarget>,
}

pub struct EventBridgeState {
    pub buses: DashMap<String, EventBus>,
    pub rules: DashMap<String, EventRule>,
}

impl Default for EventBridgeState {
    fn default() -> Self {
        let buses = DashMap::new();
        buses.insert(
            "default".to_owned(),
            EventBus {
                name: "default".to_owned(),
                arn: "arn:aws:events:us-east-1:000000000000:event-bus/default".to_owned(),
            },
        );
        Self {
            buses,
            rules: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<EventBridgeState>) -> axum::Router {
    axum::Router::new()
        .route("/", post(handle))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Main dispatch handler
// ---------------------------------------------------------------------------

pub fn handle_request(state: &EventBridgeState, target: &str, body: &[u8]) -> Response {
    let action = target.split('.').next_back().unwrap_or("");

    let body: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(e) => {
            return json_error_response(&LawsError::InvalidRequest(format!(
                "invalid JSON body: {e}"
            )))
        }
    };

    let result = match action {
        "CreateEventBus" => create_event_bus(state, &body),
        "DeleteEventBus" => delete_event_bus(state, &body),
        "ListEventBuses" => list_event_buses(state),
        "DescribeEventBus" => describe_event_bus(state, &body),
        "PutRule" => put_rule(state, &body),
        "DeleteRule" => delete_rule(state, &body),
        "ListRules" => list_rules(state, &body),
        "PutTargets" => put_targets(state, &body),
        "RemoveTargets" => remove_targets(state, &body),
        "PutEvents" => put_events(state, &body),
        other => Err(LawsError::InvalidRequest(format!(
            "unknown action: {other}"
        ))),
    };

    match result {
        Ok(v) => json_response(v),
        Err(e) => json_error_response(&e),
    }
}

async fn handle(
    State(state): State<Arc<EventBridgeState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target = match parse_target(&headers) {
        Ok(t) => t,
        Err(e) => return json_error_response(&e),
    };

    handle_request(&state, &target.action, &body)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_str<'a>(body: &'a Value, field: &str) -> Result<&'a str, LawsError> {
    body.get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest(format!("missing required field: {field}")))
}

/// Composite key for rules: "{event_bus_name}#{rule_name}"
fn rule_key(event_bus_name: &str, rule_name: &str) -> String {
    format!("{event_bus_name}#{rule_name}")
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_event_bus(state: &EventBridgeState, body: &Value) -> Result<Value, LawsError> {
    let name = require_str(body, "Name")?.to_owned();

    if state.buses.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "event bus already exists: {name}"
        )));
    }

    let arn = format!("arn:aws:events:us-east-1:000000000000:event-bus/{name}");

    let bus = EventBus {
        name: name.clone(),
        arn: arn.clone(),
    };
    state.buses.insert(name, bus);

    Ok(json!({ "EventBusArn": arn }))
}

fn delete_event_bus(state: &EventBridgeState, body: &Value) -> Result<Value, LawsError> {
    let name = require_str(body, "Name")?;

    if name == "default" {
        return Err(LawsError::InvalidRequest(
            "cannot delete the default event bus".into(),
        ));
    }

    state
        .buses
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("event bus not found: {name}")))?;

    Ok(json!({}))
}

fn list_event_buses(state: &EventBridgeState) -> Result<Value, LawsError> {
    let buses: Vec<Value> = state
        .buses
        .iter()
        .map(|entry| {
            let bus = entry.value();
            json!({
                "Name": bus.name,
                "Arn": bus.arn,
            })
        })
        .collect();

    Ok(json!({ "EventBuses": buses }))
}

fn describe_event_bus(state: &EventBridgeState, body: &Value) -> Result<Value, LawsError> {
    let name = body
        .get("Name")
        .and_then(|v| v.as_str())
        .unwrap_or("default");

    let bus = state
        .buses
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("event bus not found: {name}")))?;

    Ok(json!({
        "Name": bus.name,
        "Arn": bus.arn,
    }))
}

fn put_rule(state: &EventBridgeState, body: &Value) -> Result<Value, LawsError> {
    let name = require_str(body, "Name")?.to_owned();
    let event_bus_name = body
        .get("EventBusName")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_owned();

    if !state.buses.contains_key(&event_bus_name) {
        return Err(LawsError::NotFound(format!(
            "event bus not found: {event_bus_name}"
        )));
    }

    let event_pattern = body
        .get("EventPattern")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());
    let schedule_expression = body
        .get("ScheduleExpression")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());
    let rule_state = body
        .get("State")
        .and_then(|v| v.as_str())
        .unwrap_or("ENABLED")
        .to_owned();

    let arn = format!("arn:aws:events:us-east-1:000000000000:rule/{event_bus_name}/{name}");

    let key = rule_key(&event_bus_name, &name);

    // Preserve existing targets if rule already exists.
    let existing_targets = state
        .rules
        .get(&key)
        .map(|r| r.targets.clone())
        .unwrap_or_default();

    let rule = EventRule {
        name: name.clone(),
        event_bus_name,
        event_pattern,
        schedule_expression,
        state: rule_state,
        arn: arn.clone(),
        targets: existing_targets,
    };

    state.rules.insert(key, rule);

    Ok(json!({ "RuleArn": arn }))
}

fn delete_rule(state: &EventBridgeState, body: &Value) -> Result<Value, LawsError> {
    let name = require_str(body, "Name")?;
    let event_bus_name = body
        .get("EventBusName")
        .and_then(|v| v.as_str())
        .unwrap_or("default");

    let key = rule_key(event_bus_name, name);
    state
        .rules
        .remove(&key)
        .ok_or_else(|| LawsError::NotFound(format!("rule not found: {name}")))?;

    Ok(json!({}))
}

fn list_rules(state: &EventBridgeState, body: &Value) -> Result<Value, LawsError> {
    let event_bus_name = body
        .get("EventBusName")
        .and_then(|v| v.as_str())
        .unwrap_or("default");

    let rules: Vec<Value> = state
        .rules
        .iter()
        .filter(|entry| entry.value().event_bus_name == event_bus_name)
        .map(|entry| {
            let rule = entry.value();
            let mut obj = json!({
                "Name": rule.name,
                "Arn": rule.arn,
                "State": rule.state,
                "EventBusName": rule.event_bus_name,
            });
            if let Some(ref pattern) = rule.event_pattern {
                obj["EventPattern"] = json!(pattern);
            }
            if let Some(ref expr) = rule.schedule_expression {
                obj["ScheduleExpression"] = json!(expr);
            }
            obj
        })
        .collect();

    Ok(json!({ "Rules": rules }))
}

fn put_targets(state: &EventBridgeState, body: &Value) -> Result<Value, LawsError> {
    let rule_name = require_str(body, "Rule")?;
    let event_bus_name = body
        .get("EventBusName")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let targets_arr = body
        .get("Targets")
        .and_then(|v| v.as_array())
        .ok_or_else(|| LawsError::InvalidRequest("missing Targets array".into()))?;

    let key = rule_key(event_bus_name, rule_name);
    let mut rule = state
        .rules
        .get_mut(&key)
        .ok_or_else(|| LawsError::NotFound(format!("rule not found: {rule_name}")))?;

    for t in targets_arr {
        let id = t
            .get("Id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        let arn = t
            .get("Arn")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();

        // Replace existing target with same id, or add new one.
        if let Some(existing) = rule.targets.iter_mut().find(|et| et.id == id) {
            existing.arn = arn;
        } else {
            rule.targets.push(EventTarget { id, arn });
        }
    }

    Ok(json!({
        "FailedEntryCount": 0,
        "FailedEntries": [],
    }))
}

fn remove_targets(state: &EventBridgeState, body: &Value) -> Result<Value, LawsError> {
    let rule_name = require_str(body, "Rule")?;
    let event_bus_name = body
        .get("EventBusName")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let ids = body
        .get("Ids")
        .and_then(|v| v.as_array())
        .ok_or_else(|| LawsError::InvalidRequest("missing Ids array".into()))?;

    let ids_set: Vec<String> = ids
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_owned()))
        .collect();

    let key = rule_key(event_bus_name, rule_name);
    let mut rule = state
        .rules
        .get_mut(&key)
        .ok_or_else(|| LawsError::NotFound(format!("rule not found: {rule_name}")))?;

    rule.targets.retain(|t| !ids_set.contains(&t.id));

    Ok(json!({
        "FailedEntryCount": 0,
        "FailedEntries": [],
    }))
}

fn put_events(_state: &EventBridgeState, body: &Value) -> Result<Value, LawsError> {
    let entries = body
        .get("Entries")
        .and_then(|v| v.as_array())
        .ok_or_else(|| LawsError::InvalidRequest("missing Entries array".into()))?;

    let result_entries: Vec<Value> = entries
        .iter()
        .map(|_| {
            json!({
                "EventId": uuid::Uuid::new_v4().to_string(),
            })
        })
        .collect();

    Ok(json!({
        "FailedEntryCount": 0,
        "Entries": result_entries,
    }))
}
