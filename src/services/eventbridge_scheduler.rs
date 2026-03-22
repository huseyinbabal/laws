use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Json;
use chrono::Utc;
use dashmap::DashMap;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::LawsError;
use crate::protocol::rest_json;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Schedule {
    pub name: String,
    pub arn: String,
    pub group_name: String,
    pub schedule_expression: String,
    pub state: String,
    pub target: ScheduleTarget,
    pub flexible_time_window: Value,
    pub created_at: String,
    pub last_modified_at: String,
}

#[derive(Debug, Clone)]
pub struct ScheduleTarget {
    pub arn: String,
    pub role_arn: String,
    pub input: String,
}

#[derive(Debug, Clone)]
pub struct ScheduleGroup {
    pub name: String,
    pub arn: String,
    pub state: String,
    pub created_at: String,
    pub last_modified_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct EventBridgeSchedulerState {
    pub schedules: DashMap<String, Schedule>,
    pub schedule_groups: DashMap<String, ScheduleGroup>,
}

impl Default for EventBridgeSchedulerState {
    fn default() -> Self {
        let schedule_groups = DashMap::new();
        // Seed default group
        let now = Utc::now().to_rfc3339();
        schedule_groups.insert(
            "default".to_string(),
            ScheduleGroup {
                name: "default".to_string(),
                arn: format!("arn:aws:scheduler:{REGION}:{ACCOUNT_ID}:schedule-group/default"),
                state: "ACTIVE".to_string(),
                created_at: now.clone(),
                last_modified_at: now,
            },
        );

        Self {
            schedules: DashMap::new(),
            schedule_groups,
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<EventBridgeSchedulerState>) -> axum::Router {
    axum::Router::new()
        .route("/schedules", get(list_schedules))
        .route(
            "/schedules/{name}",
            post(create_schedule)
                .get(get_schedule)
                .put(update_schedule)
                .delete(delete_schedule),
        )
        .route("/schedule-groups", get(list_schedule_groups))
        .route("/schedule-groups/{name}", post(create_schedule_group))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateScheduleRequest {
    #[serde(alias = "ScheduleExpression")]
    schedule_expression: String,
    #[serde(alias = "Target")]
    target: TargetInput,
    #[serde(alias = "FlexibleTimeWindow", default)]
    flexible_time_window: Option<Value>,
    #[serde(alias = "GroupName", default)]
    group_name: Option<String>,
    #[serde(alias = "State", default)]
    state: Option<String>,
}

#[derive(Deserialize)]
struct TargetInput {
    #[serde(alias = "Arn")]
    arn: String,
    #[serde(alias = "RoleArn", default)]
    role_arn: Option<String>,
    #[serde(alias = "Input", default)]
    input: Option<String>,
}

async fn create_schedule(
    State(state): State<Arc<EventBridgeSchedulerState>>,
    Path(name): Path<String>,
    Json(req): Json<CreateScheduleRequest>,
) -> Response {
    let group_name = req.group_name.unwrap_or_else(|| "default".into());
    let arn = format!("arn:aws:scheduler:{REGION}:{ACCOUNT_ID}:schedule/{group_name}/{name}");
    let now = Utc::now().to_rfc3339();

    let schedule = Schedule {
        name: name.clone(),
        arn: arn.clone(),
        group_name,
        schedule_expression: req.schedule_expression,
        state: req.state.unwrap_or_else(|| "ENABLED".into()),
        target: ScheduleTarget {
            arn: req.target.arn,
            role_arn: req
                .target
                .role_arn
                .unwrap_or_else(|| format!("arn:aws:iam::{ACCOUNT_ID}:role/scheduler-role")),
            input: req.target.input.unwrap_or_default(),
        },
        flexible_time_window: req.flexible_time_window.unwrap_or(json!({ "Mode": "OFF" })),
        created_at: now.clone(),
        last_modified_at: now,
    };

    state.schedules.insert(name, schedule);

    rest_json::ok(json!({ "ScheduleArn": arn }))
}

async fn get_schedule(
    State(state): State<Arc<EventBridgeSchedulerState>>,
    Path(name): Path<String>,
) -> Response {
    match state.schedules.get(&name) {
        Some(s) => rest_json::ok(schedule_to_json(&s)),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Schedule not found: {name}")))
        }
    }
}

async fn list_schedules(State(state): State<Arc<EventBridgeSchedulerState>>) -> Response {
    let schedules: Vec<Value> = state
        .schedules
        .iter()
        .map(|entry| {
            let s = entry.value();
            json!({
                "Name": s.name,
                "Arn": s.arn,
                "GroupName": s.group_name,
                "State": s.state,
                "CreationDate": s.created_at,
                "LastModificationDate": s.last_modified_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "Schedules": schedules }))
}

async fn delete_schedule(
    State(state): State<Arc<EventBridgeSchedulerState>>,
    Path(name): Path<String>,
) -> Response {
    match state.schedules.remove(&name) {
        Some(_) => rest_json::ok(json!({})),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Schedule not found: {name}")))
        }
    }
}

async fn update_schedule(
    State(state): State<Arc<EventBridgeSchedulerState>>,
    Path(name): Path<String>,
    Json(req): Json<CreateScheduleRequest>,
) -> Response {
    match state.schedules.get_mut(&name) {
        Some(mut s) => {
            s.schedule_expression = req.schedule_expression;
            s.target = ScheduleTarget {
                arn: req.target.arn,
                role_arn: req
                    .target
                    .role_arn
                    .unwrap_or_else(|| s.target.role_arn.clone()),
                input: req.target.input.unwrap_or_else(|| s.target.input.clone()),
            };
            if let Some(state_val) = req.state {
                s.state = state_val;
            }
            if let Some(ftw) = req.flexible_time_window {
                s.flexible_time_window = ftw;
            }
            if let Some(gn) = req.group_name {
                s.group_name = gn;
            }
            s.last_modified_at = Utc::now().to_rfc3339();

            rest_json::ok(json!({ "ScheduleArn": s.arn }))
        }
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Schedule not found: {name}")))
        }
    }
}

#[derive(Deserialize)]
struct CreateScheduleGroupRequest {
    #[serde(default)]
    _placeholder: Option<String>,
}

async fn create_schedule_group(
    State(state): State<Arc<EventBridgeSchedulerState>>,
    Path(name): Path<String>,
    Json(_req): Json<CreateScheduleGroupRequest>,
) -> Response {
    if state.schedule_groups.contains_key(&name) {
        return rest_json::error_response(&LawsError::AlreadyExists(format!(
            "ScheduleGroup already exists: {name}"
        )));
    }

    let arn = format!("arn:aws:scheduler:{REGION}:{ACCOUNT_ID}:schedule-group/{name}");
    let now = Utc::now().to_rfc3339();

    let group = ScheduleGroup {
        name: name.clone(),
        arn: arn.clone(),
        state: "ACTIVE".to_string(),
        created_at: now.clone(),
        last_modified_at: now,
    };

    state.schedule_groups.insert(name, group);

    rest_json::ok(json!({ "ScheduleGroupArn": arn }))
}

async fn list_schedule_groups(State(state): State<Arc<EventBridgeSchedulerState>>) -> Response {
    let groups: Vec<Value> = state
        .schedule_groups
        .iter()
        .map(|entry| {
            let g = entry.value();
            json!({
                "Name": g.name,
                "Arn": g.arn,
                "State": g.state,
                "CreationDate": g.created_at,
                "LastModificationDate": g.last_modified_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "ScheduleGroups": groups }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn schedule_to_json(s: &Schedule) -> Value {
    json!({
        "Name": s.name,
        "Arn": s.arn,
        "GroupName": s.group_name,
        "ScheduleExpression": s.schedule_expression,
        "State": s.state,
        "Target": {
            "Arn": s.target.arn,
            "RoleArn": s.target.role_arn,
            "Input": s.target.input,
        },
        "FlexibleTimeWindow": s.flexible_time_window,
        "CreationDate": s.created_at,
        "LastModificationDate": s.last_modified_at,
    })
}
