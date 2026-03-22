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
pub struct Fleet {
    pub fleet_id: String,
    pub arn: String,
    pub name: String,
    pub build_id: String,
    pub status: String,
    pub instance_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct GameSession {
    pub game_session_id: String,
    pub arn: String,
    pub fleet_id: String,
    pub name: String,
    pub status: String,
    pub max_players: u32,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct GameSessionQueue {
    pub name: String,
    pub arn: String,
    pub timeout_in_seconds: u32,
}

#[derive(Debug, Clone)]
pub struct MatchmakingConfiguration {
    pub name: String,
    pub arn: String,
    pub rule_set_name: String,
    pub acceptance_required: bool,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct GameLiftState {
    pub fleets: DashMap<String, Fleet>,
    pub game_sessions: DashMap<String, GameSession>,
    pub queues: DashMap<String, GameSessionQueue>,
    pub matchmaking_configs: DashMap<String, MatchmakingConfiguration>,
}

impl Default for GameLiftState {
    fn default() -> Self {
        Self {
            fleets: DashMap::new(),
            game_sessions: DashMap::new(),
            queues: DashMap::new(),
            matchmaking_configs: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &GameLiftState, target: &str, payload: &Value) -> Response {
    let action = target.strip_prefix("GameLift.").unwrap_or(target);

    let result = match action {
        "CreateFleet" => create_fleet(state, payload),
        "DeleteFleet" => delete_fleet(state, payload),
        "DescribeFleetAttributes" => describe_fleet_attributes(state, payload),
        "ListFleets" => list_fleets(state),
        "CreateGameSessionQueue" => create_game_session_queue(state, payload),
        "DescribeGameSessions" => describe_game_sessions(state, payload),
        "CreateMatchmakingConfiguration" => create_matchmaking_configuration(state, payload),
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

fn create_fleet(state: &GameLiftState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
        .to_string();

    let build_id = payload["BuildId"]
        .as_str()
        .unwrap_or("build-00000000")
        .to_string();

    let instance_type = payload["EC2InstanceType"]
        .as_str()
        .unwrap_or("c5.large")
        .to_string();

    let fleet_id = format!("fleet-{}", uuid::Uuid::new_v4());
    let arn = format!("arn:aws:gamelift:{REGION}:{ACCOUNT_ID}:fleet/{fleet_id}");
    let now = chrono::Utc::now().to_rfc3339();

    let fleet = Fleet {
        fleet_id: fleet_id.clone(),
        arn: arn.clone(),
        name: name.clone(),
        build_id: build_id.clone(),
        status: "ACTIVE".into(),
        instance_type: instance_type.clone(),
        created_at: now.clone(),
    };

    state.fleets.insert(fleet_id.clone(), fleet);

    Ok(json_response(json!({
        "FleetAttributes": {
            "FleetId": fleet_id,
            "FleetArn": arn,
            "Name": name,
            "BuildId": build_id,
            "Status": "ACTIVE",
            "EC2InstanceType": instance_type,
            "CreationTime": now
        }
    })))
}

fn delete_fleet(state: &GameLiftState, payload: &Value) -> Result<Response, LawsError> {
    let fleet_id = payload["FleetId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing FleetId".into()))?;

    state
        .fleets
        .remove(fleet_id)
        .ok_or_else(|| LawsError::NotFound(format!("Fleet '{}' not found", fleet_id)))?;

    Ok(json_response(json!({})))
}

fn describe_fleet_attributes(
    state: &GameLiftState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let fleet_ids: Vec<&str> = payload["FleetIds"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let attrs: Vec<Value> = if fleet_ids.is_empty() {
        state
            .fleets
            .iter()
            .map(|e| fleet_to_json(e.value()))
            .collect()
    } else {
        fleet_ids
            .iter()
            .filter_map(|id| state.fleets.get(*id).map(|e| fleet_to_json(e.value())))
            .collect()
    };

    Ok(json_response(json!({
        "FleetAttributes": attrs
    })))
}

fn fleet_to_json(f: &Fleet) -> Value {
    json!({
        "FleetId": f.fleet_id,
        "FleetArn": f.arn,
        "Name": f.name,
        "BuildId": f.build_id,
        "Status": f.status,
        "EC2InstanceType": f.instance_type,
        "CreationTime": f.created_at,
    })
}

fn list_fleets(state: &GameLiftState) -> Result<Response, LawsError> {
    let fleet_ids: Vec<String> = state.fleets.iter().map(|e| e.key().clone()).collect();
    Ok(json_response(json!({
        "FleetIds": fleet_ids
    })))
}

fn create_game_session_queue(
    state: &GameLiftState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
        .to_string();

    let timeout = payload["TimeoutInSeconds"].as_u64().unwrap_or(300) as u32;

    let arn = format!("arn:aws:gamelift:{REGION}:{ACCOUNT_ID}:gamesessionqueue/{name}");

    let queue = GameSessionQueue {
        name: name.clone(),
        arn: arn.clone(),
        timeout_in_seconds: timeout,
    };

    state.queues.insert(name.clone(), queue);

    Ok(json_response(json!({
        "GameSessionQueue": {
            "Name": name,
            "GameSessionQueueArn": arn,
            "TimeoutInSeconds": timeout
        }
    })))
}

fn describe_game_sessions(state: &GameLiftState, payload: &Value) -> Result<Response, LawsError> {
    let fleet_id = payload["FleetId"].as_str();

    let sessions: Vec<Value> = state
        .game_sessions
        .iter()
        .filter(|e| fleet_id.map_or(true, |fid| e.value().fleet_id == fid))
        .map(|e| {
            let s = e.value();
            json!({
                "GameSessionId": s.game_session_id,
                "GameSessionArn": s.arn,
                "FleetId": s.fleet_id,
                "Name": s.name,
                "Status": s.status,
                "MaximumPlayerSessionCount": s.max_players,
                "CreationTime": s.created_at,
            })
        })
        .collect();

    Ok(json_response(json!({
        "GameSessions": sessions
    })))
}

fn create_matchmaking_configuration(
    state: &GameLiftState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
        .to_string();

    let rule_set_name = payload["RuleSetName"]
        .as_str()
        .unwrap_or("default")
        .to_string();

    let acceptance_required = payload["AcceptanceRequired"].as_bool().unwrap_or(false);

    let arn = format!("arn:aws:gamelift:{REGION}:{ACCOUNT_ID}:matchmakingconfiguration/{name}");

    let config = MatchmakingConfiguration {
        name: name.clone(),
        arn: arn.clone(),
        rule_set_name: rule_set_name.clone(),
        acceptance_required,
    };

    state.matchmaking_configs.insert(name.clone(), config);

    Ok(json_response(json!({
        "Configuration": {
            "Name": name,
            "ConfigurationArn": arn,
            "RuleSetName": rule_set_name,
            "AcceptanceRequired": acceptance_required
        }
    })))
}
