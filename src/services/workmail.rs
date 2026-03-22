use axum::response::{IntoResponse, Response};
use chrono::Utc;
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
pub struct Organization {
    pub organization_id: String,
    pub alias: String,
    pub default_mail_domain: String,
    pub state: String,
    pub arn: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct WorkMailUser {
    pub user_id: String,
    pub organization_id: String,
    pub name: String,
    pub display_name: String,
    pub email: String,
    pub state: String,
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct WorkMailState {
    pub organizations: DashMap<String, Organization>,
    pub users: DashMap<String, WorkMailUser>,
}

impl Default for WorkMailState {
    fn default() -> Self {
        Self {
            organizations: DashMap::new(),
            users: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &WorkMailState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("WorkMailService.")
        .unwrap_or(target);

    let result = match action {
        "CreateOrganization" => create_organization(state, payload),
        "DeleteOrganization" => delete_organization(state, payload),
        "ListOrganizations" => list_organizations(state),
        "DescribeOrganization" => describe_organization(state, payload),
        "CreateUser" => create_user(state, payload),
        "ListUsers" => list_users(state, payload),
        "RegisterToWorkMail" => register_to_workmail(state, payload),
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

fn json_response(status: StatusCode, body: Value) -> Response {
    (status, axum::Json(body)).into_response()
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_organization(state: &WorkMailState, payload: &Value) -> Result<Response, LawsError> {
    let alias = payload["Alias"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Alias".into()))?
        .to_string();

    let organization_id = format!("m-{}", uuid::Uuid::new_v4().simple());
    let arn = format!(
        "arn:aws:workmail:{REGION}:{ACCOUNT_ID}:organization/{organization_id}"
    );
    let now = Utc::now().to_rfc3339();

    let org = Organization {
        organization_id: organization_id.clone(),
        alias: alias.clone(),
        default_mail_domain: format!("{alias}.awsapps.com"),
        state: "Active".to_string(),
        arn,
        created_at: now,
    };

    state.organizations.insert(organization_id.clone(), org);

    Ok(json_response(
        StatusCode::OK,
        json!({ "OrganizationId": organization_id }),
    ))
}

fn delete_organization(state: &WorkMailState, payload: &Value) -> Result<Response, LawsError> {
    let organization_id = payload["OrganizationId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing OrganizationId".into()))?;

    state
        .organizations
        .remove(organization_id)
        .ok_or_else(|| {
            LawsError::NotFound(format!(
                "Organization not found: {organization_id}"
            ))
        })?;

    Ok(json_response(
        StatusCode::OK,
        json!({ "OrganizationId": organization_id, "State": "Deleted" }),
    ))
}

fn list_organizations(state: &WorkMailState) -> Result<Response, LawsError> {
    let orgs: Vec<Value> = state
        .organizations
        .iter()
        .map(|entry| {
            let o = entry.value();
            json!({
                "OrganizationId": o.organization_id,
                "Alias": o.alias,
                "DefaultMailDomain": o.default_mail_domain,
                "State": o.state,
            })
        })
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "OrganizationSummaries": orgs }),
    ))
}

fn describe_organization(state: &WorkMailState, payload: &Value) -> Result<Response, LawsError> {
    let organization_id = payload["OrganizationId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing OrganizationId".into()))?;

    let org = state
        .organizations
        .get(organization_id)
        .ok_or_else(|| {
            LawsError::NotFound(format!(
                "Organization not found: {organization_id}"
            ))
        })?;

    Ok(json_response(
        StatusCode::OK,
        json!({
            "OrganizationId": org.organization_id,
            "Alias": org.alias,
            "DefaultMailDomain": org.default_mail_domain,
            "State": org.state,
            "ARN": org.arn,
            "CompletedDate": org.created_at,
        }),
    ))
}

fn create_user(state: &WorkMailState, payload: &Value) -> Result<Response, LawsError> {
    let organization_id = payload["OrganizationId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing OrganizationId".into()))?
        .to_string();

    if !state.organizations.contains_key(&organization_id) {
        return Err(LawsError::NotFound(format!(
            "Organization not found: {organization_id}"
        )));
    }

    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
        .to_string();
    let display_name = payload["DisplayName"]
        .as_str()
        .unwrap_or(&name)
        .to_string();

    let user_id = uuid::Uuid::new_v4().to_string();

    let user = WorkMailUser {
        user_id: user_id.clone(),
        organization_id,
        name,
        display_name,
        email: String::new(),
        state: "ENABLED".to_string(),
        enabled: true,
    };

    state.users.insert(user_id.clone(), user);

    Ok(json_response(
        StatusCode::OK,
        json!({ "UserId": user_id }),
    ))
}

fn list_users(state: &WorkMailState, payload: &Value) -> Result<Response, LawsError> {
    let organization_id = payload["OrganizationId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing OrganizationId".into()))?;

    let users: Vec<Value> = state
        .users
        .iter()
        .filter(|entry| entry.value().organization_id == organization_id)
        .map(|entry| {
            let u = entry.value();
            json!({
                "Id": u.user_id,
                "Name": u.name,
                "DisplayName": u.display_name,
                "Email": u.email,
                "State": u.state,
                "UserRole": "USER",
            })
        })
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "Users": users }),
    ))
}

fn register_to_workmail(state: &WorkMailState, payload: &Value) -> Result<Response, LawsError> {
    let organization_id = payload["OrganizationId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing OrganizationId".into()))?;
    let entity_id = payload["EntityId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing EntityId".into()))?;
    let email = payload["Email"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Email".into()))?
        .to_string();

    if !state.organizations.contains_key(organization_id) {
        return Err(LawsError::NotFound(format!(
            "Organization not found: {organization_id}"
        )));
    }

    let mut user = state
        .users
        .get_mut(entity_id)
        .ok_or_else(|| LawsError::NotFound(format!("User not found: {entity_id}")))?;

    user.email = email;
    user.state = "REGISTERED".to_string();

    Ok(json_response(StatusCode::OK, json!({})))
}
