use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use rand::Rng;
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
pub struct OrgAccount {
    pub id: String,
    pub name: String,
    pub email: String,
    pub status: String,
    pub arn: String,
    pub joined_method: String,
}

#[derive(Debug, Clone)]
pub struct OrganizationalUnit {
    pub id: String,
    pub name: String,
    pub arn: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct OrganizationsState {
    pub organization: DashMap<String, Value>,
    pub accounts: DashMap<String, OrgAccount>,
    pub organizational_units: DashMap<String, OrganizationalUnit>,
}

impl Default for OrganizationsState {
    fn default() -> Self {
        Self {
            organization: DashMap::new(),
            accounts: DashMap::new(),
            organizational_units: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &OrganizationsState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("AWSOrganizationsV20161128.")
        .unwrap_or(target);

    let result = match action {
        "CreateOrganization" => create_organization(state, payload),
        "DescribeOrganization" => describe_organization(state),
        "ListAccounts" => list_accounts(state),
        "CreateAccount" => create_account(state, payload),
        "DescribeAccount" => describe_account(state, payload),
        "CreateOrganizationalUnit" => create_organizational_unit(state, payload),
        "ListOrganizationalUnitsForParent" => list_organizational_units_for_parent(state),
        "ListRoots" => list_roots(state),
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

fn random_org_id() -> String {
    let suffix: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(10)
        .map(|c| char::from(c).to_ascii_lowercase())
        .collect();
    format!("o-{suffix}")
}

fn random_ou_id() -> String {
    let suffix: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(10)
        .map(|c| char::from(c).to_ascii_lowercase())
        .collect();
    format!("ou-{suffix}")
}

fn random_account_id() -> String {
    let mut rng = rand::thread_rng();
    format!("{:012}", rng.gen_range(100000000000u64..999999999999u64))
}

fn account_to_json(a: &OrgAccount) -> Value {
    json!({
        "Id": a.id,
        "Name": a.name,
        "Email": a.email,
        "Status": a.status,
        "Arn": a.arn,
        "JoinedMethod": a.joined_method,
    })
}

fn ou_to_json(ou: &OrganizationalUnit) -> Value {
    json!({
        "Id": ou.id,
        "Name": ou.name,
        "Arn": ou.arn,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_organization(
    state: &OrganizationsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    if state.organization.contains_key("default") {
        return Err(LawsError::AlreadyExists(
            "Organization already exists".into(),
        ));
    }

    let feature_set = payload["FeatureSet"]
        .as_str()
        .unwrap_or("ALL")
        .to_string();

    let org_id = random_org_id();
    let arn = format!("arn:aws:organizations::{ACCOUNT_ID}:organization/{org_id}");

    let org = json!({
        "Id": org_id,
        "Arn": arn,
        "MasterAccountId": ACCOUNT_ID,
        "MasterAccountArn": format!("arn:aws:organizations::{ACCOUNT_ID}:account/{ACCOUNT_ID}"),
        "MasterAccountEmail": "master@example.com",
        "FeatureSet": feature_set,
    });

    state.organization.insert("default".to_string(), org.clone());

    Ok(json_response(json!({ "Organization": org })))
}

fn describe_organization(state: &OrganizationsState) -> Result<Response, LawsError> {
    let org = state
        .organization
        .get("default")
        .ok_or_else(|| LawsError::NotFound("Organization not found".into()))?;

    Ok(json_response(json!({ "Organization": org.value().clone() })))
}

fn list_accounts(state: &OrganizationsState) -> Result<Response, LawsError> {
    let accounts: Vec<Value> = state
        .accounts
        .iter()
        .map(|entry| account_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "Accounts": accounts })))
}

fn create_account(
    state: &OrganizationsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["AccountName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing AccountName".into()))?
        .to_string();

    let email = payload["Email"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Email".into()))?
        .to_string();

    let account_id = random_account_id();
    let arn = format!("arn:aws:organizations::{ACCOUNT_ID}:account/{account_id}");

    let account = OrgAccount {
        id: account_id.clone(),
        name,
        email,
        status: "ACTIVE".to_string(),
        arn,
        joined_method: "CREATED".to_string(),
    };

    let resp = json!({
        "CreateAccountStatus": {
            "Id": uuid::Uuid::new_v4().to_string(),
            "AccountId": account.id,
            "AccountName": account.name,
            "State": "SUCCEEDED",
        }
    });

    state.accounts.insert(account_id, account);

    Ok(json_response(resp))
}

fn describe_account(
    state: &OrganizationsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let account_id = payload["AccountId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing AccountId".into()))?;

    let account = state
        .accounts
        .get(account_id)
        .ok_or_else(|| LawsError::NotFound(format!("Account '{}' not found", account_id)))?;

    Ok(json_response(json!({
        "Account": account_to_json(account.value())
    })))
}

fn create_organizational_unit(
    state: &OrganizationsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
        .to_string();

    let _parent_id = payload["ParentId"]
        .as_str()
        .unwrap_or("r-root");

    let ou_id = random_ou_id();
    let arn = format!("arn:aws:organizations::{ACCOUNT_ID}:ou/{ou_id}");

    let ou = OrganizationalUnit {
        id: ou_id.clone(),
        name,
        arn,
    };

    let resp = ou_to_json(&ou);
    state.organizational_units.insert(ou_id, ou);

    Ok(json_response(json!({ "OrganizationalUnit": resp })))
}

fn list_organizational_units_for_parent(
    state: &OrganizationsState,
) -> Result<Response, LawsError> {
    let ous: Vec<Value> = state
        .organizational_units
        .iter()
        .map(|entry| ou_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({
        "OrganizationalUnits": ous
    })))
}

fn list_roots(state: &OrganizationsState) -> Result<Response, LawsError> {
    let _org = state
        .organization
        .get("default")
        .ok_or_else(|| LawsError::NotFound("Organization not found".into()))?;

    let root = json!({
        "Id": "r-root",
        "Arn": format!("arn:aws:organizations::{ACCOUNT_ID}:root/r-root"),
        "Name": "Root",
        "PolicyTypes": [],
    });

    Ok(json_response(json!({ "Roots": [root] })))
}
