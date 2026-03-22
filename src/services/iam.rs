use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, Uri};
use axum::response::Response;
use axum::routing::post;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::LawsError;
use crate::protocol::query::{parse_query_request, xml_error_response, xml_response};
use crate::storage::mem::MemoryStore;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize, Deserialize)]
pub struct IamUser {
    pub user_name: String,
    pub user_id: String,
    pub arn: String,
    pub create_date: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct IamRole {
    pub role_name: String,
    pub role_id: String,
    pub arn: String,
    pub assume_role_policy_document: String,
    pub attached_policies: Vec<String>,
    pub create_date: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct IamPolicy {
    pub policy_name: String,
    pub policy_id: String,
    pub arn: String,
    pub document: String,
    pub create_date: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct IamState {
    pub users: MemoryStore<IamUser>,
    pub roles: MemoryStore<IamRole>,
    pub policies: MemoryStore<IamPolicy>,
}

impl Default for IamState {
    fn default() -> Self {
        Self {
            users: MemoryStore::new(),
            roles: MemoryStore::new(),
            policies: MemoryStore::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<IamState>) -> axum::Router {
    axum::Router::new()
        .route("/", post(handle))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Main dispatch handler
// ---------------------------------------------------------------------------

pub fn handle_request(
    state: &IamState,
    headers: &axum::http::HeaderMap,
    body: &axum::body::Bytes,
    uri: &axum::http::Uri,
) -> Response {
    let req = match parse_query_request(uri, headers, body) {
        Ok(r) => r,
        Err(e) => return xml_error_response(&e),
    };

    let result = match req.action.as_str() {
        "CreateUser" => create_user(state, &req.params),
        "DeleteUser" => delete_user(state, &req.params),
        "GetUser" => get_user(state, &req.params),
        "ListUsers" => list_users(state),
        "CreateRole" => create_role(state, &req.params),
        "DeleteRole" => delete_role(state, &req.params),
        "GetRole" => get_role(state, &req.params),
        "ListRoles" => list_roles(state),
        "ListAttachedRolePolicies" => list_attached_role_policies(state, &req.params),
        "CreatePolicy" => create_policy(state, &req.params),
        "DeletePolicy" => delete_policy(state, &req.params),
        "ListPolicies" => list_policies(state),
        "AttachRolePolicy" => attach_role_policy(state, &req.params),
        "DetachRolePolicy" => detach_role_policy(state, &req.params),
        "ListAttachedUserPolicies" => list_attached_user_policies(&req.params),
        "ListGroupsForUser" => list_groups_for_user(&req.params),
        "ListAccessKeys" => list_access_keys(&req.params),
        "ListGroups" => list_groups(),
        "GetGroup" => get_group(&req.params),
        other => Err(LawsError::InvalidRequest(format!(
            "unknown action: {other}"
        ))),
    };

    match result {
        Ok((action, inner_xml)) => xml_response(&action, &inner_xml),
        Err(e) => xml_error_response(&e),
    }
}

async fn handle(
    State(state): State<Arc<IamState>>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_request(&state, &headers, &body, &uri)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type ActionResult = Result<(String, String), LawsError>;

fn require_param<'a>(
    params: &'a std::collections::HashMap<String, String>,
    key: &str,
) -> Result<&'a str, LawsError> {
    params
        .get(key)
        .map(|s| s.as_str())
        .ok_or_else(|| LawsError::InvalidRequest(format!("missing required parameter: {key}")))
}

fn escape(s: &str) -> String {
    quick_xml::escape::escape(s).to_string()
}

fn user_xml(user: &IamUser) -> String {
    format!(
        r#"<UserName>{}</UserName>
      <UserId>{}</UserId>
      <Arn>{}</Arn>
      <CreateDate>{}</CreateDate>"#,
        escape(&user.user_name),
        escape(&user.user_id),
        escape(&user.arn),
        escape(&user.create_date),
    )
}

fn role_xml(role: &IamRole) -> String {
    let policies_xml: String = role
        .attached_policies
        .iter()
        .map(|arn| format!("<member>{}</member>", escape(arn)))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<RoleName>{}</RoleName>
      <RoleId>{}</RoleId>
      <Arn>{}</Arn>
      <AssumeRolePolicyDocument>{}</AssumeRolePolicyDocument>
      <AttachedPolicies>{}</AttachedPolicies>
      <CreateDate>{}</CreateDate>"#,
        escape(&role.role_name),
        escape(&role.role_id),
        escape(&role.arn),
        escape(&role.assume_role_policy_document),
        policies_xml,
        escape(&role.create_date),
    )
}

fn policy_xml(policy: &IamPolicy) -> String {
    format!(
        r#"<PolicyName>{}</PolicyName>
      <PolicyId>{}</PolicyId>
      <Arn>{}</Arn>
      <PolicyDocument>{}</PolicyDocument>
      <CreateDate>{}</CreateDate>"#,
        escape(&policy.policy_name),
        escape(&policy.policy_id),
        escape(&policy.arn),
        escape(&policy.document),
        escape(&policy.create_date),
    )
}

// ---------------------------------------------------------------------------
// User operations
// ---------------------------------------------------------------------------

fn create_user(
    state: &IamState,
    params: &std::collections::HashMap<String, String>,
) -> ActionResult {
    let user_name = require_param(params, "UserName")?;

    if state.users.contains(user_name) {
        return Err(LawsError::AlreadyExists(format!(
            "user already exists: {user_name}"
        )));
    }

    let user = IamUser {
        user_name: user_name.to_owned(),
        user_id: Uuid::new_v4().to_string(),
        arn: format!("arn:aws:iam::{ACCOUNT_ID}:user/{user_name}"),
        create_date: Utc::now().to_rfc3339(),
    };

    let xml = format!("<User>{}</User>", user_xml(&user));
    state.users.insert(user_name.to_owned(), user);

    Ok(("CreateUser".into(), xml))
}

fn delete_user(
    state: &IamState,
    params: &std::collections::HashMap<String, String>,
) -> ActionResult {
    let user_name = require_param(params, "UserName")?;

    state
        .users
        .remove(user_name)
        .ok_or_else(|| LawsError::NotFound(format!("user not found: {user_name}")))?;

    Ok(("DeleteUser".into(), String::new()))
}

fn get_user(state: &IamState, params: &std::collections::HashMap<String, String>) -> ActionResult {
    let user_name = require_param(params, "UserName")?;

    let user = state
        .users
        .get(user_name)
        .ok_or_else(|| LawsError::NotFound(format!("user not found: {user_name}")))?;

    let xml = format!("<User>{}</User>", user_xml(&user));
    Ok(("GetUser".into(), xml))
}

fn list_users(state: &IamState) -> ActionResult {
    let users = state.users.list_values();
    let members: String = users
        .iter()
        .map(|u| format!("<member>{}</member>", user_xml(u)))
        .collect::<Vec<_>>()
        .join("\n");

    let xml = format!("<Users>{members}</Users>");
    Ok(("ListUsers".into(), xml))
}

// ---------------------------------------------------------------------------
// Role operations
// ---------------------------------------------------------------------------

fn create_role(
    state: &IamState,
    params: &std::collections::HashMap<String, String>,
) -> ActionResult {
    let role_name = require_param(params, "RoleName")?;
    let assume_role_doc = params
        .get("AssumeRolePolicyDocument")
        .cloned()
        .unwrap_or_default();

    if state.roles.contains(role_name) {
        return Err(LawsError::AlreadyExists(format!(
            "role already exists: {role_name}"
        )));
    }

    let role = IamRole {
        role_name: role_name.to_owned(),
        role_id: Uuid::new_v4().to_string(),
        arn: format!("arn:aws:iam::{ACCOUNT_ID}:role/{role_name}"),
        assume_role_policy_document: assume_role_doc,
        attached_policies: Vec::new(),
        create_date: Utc::now().to_rfc3339(),
    };

    let xml = format!("<Role>{}</Role>", role_xml(&role));
    state.roles.insert(role_name.to_owned(), role);

    Ok(("CreateRole".into(), xml))
}

fn delete_role(
    state: &IamState,
    params: &std::collections::HashMap<String, String>,
) -> ActionResult {
    let role_name = require_param(params, "RoleName")?;

    state
        .roles
        .remove(role_name)
        .ok_or_else(|| LawsError::NotFound(format!("role not found: {role_name}")))?;

    Ok(("DeleteRole".into(), String::new()))
}

fn list_roles(state: &IamState) -> ActionResult {
    let roles = state.roles.list_values();
    let members: String = roles
        .iter()
        .map(|r| format!("<member>{}</member>", role_xml(r)))
        .collect::<Vec<_>>()
        .join("\n");

    let xml = format!("<Roles>{members}</Roles>");
    Ok(("ListRoles".into(), xml))
}

// ---------------------------------------------------------------------------
// Policy operations
// ---------------------------------------------------------------------------

fn create_policy(
    state: &IamState,
    params: &std::collections::HashMap<String, String>,
) -> ActionResult {
    let policy_name = require_param(params, "PolicyName")?;
    let document = params.get("PolicyDocument").cloned().unwrap_or_default();

    if state.policies.contains(policy_name) {
        return Err(LawsError::AlreadyExists(format!(
            "policy already exists: {policy_name}"
        )));
    }

    let policy = IamPolicy {
        policy_name: policy_name.to_owned(),
        policy_id: Uuid::new_v4().to_string(),
        arn: format!("arn:aws:iam::{ACCOUNT_ID}:policy/{policy_name}"),
        document,
        create_date: Utc::now().to_rfc3339(),
    };

    let xml = format!("<Policy>{}</Policy>", policy_xml(&policy));
    state.policies.insert(policy_name.to_owned(), policy);

    Ok(("CreatePolicy".into(), xml))
}

fn delete_policy(
    state: &IamState,
    params: &std::collections::HashMap<String, String>,
) -> ActionResult {
    let policy_arn = require_param(params, "PolicyArn")?;

    // Extract policy name from ARN for the store lookup.
    let policy_name = policy_arn
        .rsplit('/')
        .next()
        .ok_or_else(|| LawsError::InvalidRequest("invalid PolicyArn format".into()))?;

    state
        .policies
        .remove(policy_name)
        .ok_or_else(|| LawsError::NotFound(format!("policy not found: {policy_arn}")))?;

    Ok(("DeletePolicy".into(), String::new()))
}

fn list_policies(state: &IamState) -> ActionResult {
    let policies = state.policies.list_values();
    let members: String = policies
        .iter()
        .map(|p| format!("<member>{}</member>", policy_xml(p)))
        .collect::<Vec<_>>()
        .join("\n");

    let xml = format!("<Policies>{members}</Policies>");
    Ok(("ListPolicies".into(), xml))
}

// ---------------------------------------------------------------------------
// Role-policy attachment operations
// ---------------------------------------------------------------------------

fn attach_role_policy(
    state: &IamState,
    params: &std::collections::HashMap<String, String>,
) -> ActionResult {
    let role_name = require_param(params, "RoleName")?;
    let policy_arn = require_param(params, "PolicyArn")?;

    let mut role = state
        .roles
        .get(role_name)
        .ok_or_else(|| LawsError::NotFound(format!("role not found: {role_name}")))?;

    if !role.attached_policies.contains(&policy_arn.to_owned()) {
        role.attached_policies.push(policy_arn.to_owned());
    }

    state.roles.insert(role_name.to_owned(), role);
    Ok(("AttachRolePolicy".into(), String::new()))
}

fn detach_role_policy(
    state: &IamState,
    params: &std::collections::HashMap<String, String>,
) -> ActionResult {
    let role_name = require_param(params, "RoleName")?;
    let policy_arn = require_param(params, "PolicyArn")?;

    let mut role = state
        .roles
        .get(role_name)
        .ok_or_else(|| LawsError::NotFound(format!("role not found: {role_name}")))?;

    role.attached_policies.retain(|a| a != policy_arn);

    state.roles.insert(role_name.to_owned(), role);
    Ok(("DetachRolePolicy".into(), String::new()))
}

// ---------------------------------------------------------------------------
// GetRole
// ---------------------------------------------------------------------------

fn get_role(state: &IamState, params: &std::collections::HashMap<String, String>) -> ActionResult {
    let role_name = require_param(params, "RoleName")?;
    let role = state
        .roles
        .get(role_name)
        .ok_or_else(|| LawsError::NotFound(format!("role not found: {role_name}")))?;

    let xml = format!("<Role>{}</Role>", role_xml(&role));
    Ok(("GetRole".into(), xml))
}

// ---------------------------------------------------------------------------
// ListAttachedRolePolicies
// ---------------------------------------------------------------------------

fn list_attached_role_policies(
    state: &IamState,
    params: &std::collections::HashMap<String, String>,
) -> ActionResult {
    let role_name = require_param(params, "RoleName")?;
    let role = state
        .roles
        .get(role_name)
        .ok_or_else(|| LawsError::NotFound(format!("role not found: {role_name}")))?;

    let members: String = role
        .attached_policies
        .iter()
        .map(|arn| {
            let name = arn.rsplit('/').next().unwrap_or(arn);
            format!(
                "<member><PolicyName>{}</PolicyName><PolicyArn>{}</PolicyArn></member>",
                escape(name),
                escape(arn)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let xml =
        format!("<AttachedPolicies>{members}</AttachedPolicies><IsTruncated>false</IsTruncated>");
    Ok(("ListAttachedRolePolicies".into(), xml))
}

// ---------------------------------------------------------------------------
// ListAttachedUserPolicies — stub returning empty list
// ---------------------------------------------------------------------------

fn list_attached_user_policies(params: &std::collections::HashMap<String, String>) -> ActionResult {
    let _user_name = require_param(params, "UserName")?;
    let xml = "<AttachedPolicies></AttachedPolicies><IsTruncated>false</IsTruncated>".to_string();
    Ok(("ListAttachedUserPolicies".into(), xml))
}

// ---------------------------------------------------------------------------
// ListGroupsForUser — stub returning empty list
// ---------------------------------------------------------------------------

fn list_groups_for_user(params: &std::collections::HashMap<String, String>) -> ActionResult {
    let _user_name = require_param(params, "UserName")?;
    let xml = "<Groups></Groups><IsTruncated>false</IsTruncated>".to_string();
    Ok(("ListGroupsForUser".into(), xml))
}

// ---------------------------------------------------------------------------
// ListAccessKeys — stub returning empty list
// ---------------------------------------------------------------------------

fn list_access_keys(params: &std::collections::HashMap<String, String>) -> ActionResult {
    let _user_name = require_param(params, "UserName")?;
    let xml = "<AccessKeyMetadata></AccessKeyMetadata><IsTruncated>false</IsTruncated>".to_string();
    Ok(("ListAccessKeys".into(), xml))
}

// ---------------------------------------------------------------------------
// ListGroups — stub returning empty list
// ---------------------------------------------------------------------------

fn list_groups() -> ActionResult {
    let xml = "<Groups></Groups><IsTruncated>false</IsTruncated>".to_string();
    Ok(("ListGroups".into(), xml))
}

// ---------------------------------------------------------------------------
// GetGroup — stub returning group with empty users
// ---------------------------------------------------------------------------

fn get_group(params: &std::collections::HashMap<String, String>) -> ActionResult {
    let group_name = require_param(params, "GroupName")?;
    let xml = format!(
        r#"<Group>
  <GroupName>{gn}</GroupName>
  <GroupId>AGPA000000000000</GroupId>
  <Arn>arn:aws:iam::{ACCOUNT_ID}:group/{gn}</Arn>
  <CreateDate>2024-01-01T00:00:00Z</CreateDate>
</Group>
<Users></Users>
<IsTruncated>false</IsTruncated>"#,
        gn = escape(group_name),
    );
    Ok(("GetGroup".into(), xml))
}
