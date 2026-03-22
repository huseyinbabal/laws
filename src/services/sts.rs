use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use http::StatusCode;
use std::sync::Arc;

use crate::error::LawsError;

#[derive(Clone)]
pub struct StsState {
    pub account_id: String,
    pub region: String,
}

impl Default for StsState {
    fn default() -> Self {
        Self {
            account_id: "000000000000".to_string(),
            region: "us-east-1".to_string(),
        }
    }
}

pub fn router(state: Arc<StsState>) -> Router {
    Router::new()
        .route("/", post(handle_sts_action))
        .with_state(state)
}

pub fn handle_request(state: &StsState, body: &[u8]) -> Response {
    let body_str = std::str::from_utf8(body).unwrap_or("");
    let params: Vec<(String, String)> = form_urlencoded::parse(body_str.as_bytes())
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    let action = params
        .iter()
        .find(|(k, _)| k == "Action")
        .map(|(_, v)| v.as_str())
        .unwrap_or("");

    let result = match action {
        "GetCallerIdentity" => get_caller_identity(state),
        "AssumeRole" => {
            let role_arn = params
                .iter()
                .find(|(k, _)| k == "RoleArn")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            let role_session_name = params
                .iter()
                .find(|(k, _)| k == "RoleSessionName")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            assume_role(state, &role_arn, &role_session_name)
        }
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

async fn handle_sts_action(State(state): State<Arc<StsState>>, body: String) -> Response {
    handle_request(&state, body.as_bytes())
}

fn get_caller_identity(state: &StsState) -> Result<Response, LawsError> {
    let request_id = uuid::Uuid::new_v4();
    let xml = format!(
        r#"<GetCallerIdentityResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
  <GetCallerIdentityResult>
    <Account>{account_id}</Account>
    <Arn>arn:aws:iam::{account_id}:root</Arn>
    <UserId>AKIAIOSFODNN7EXAMPLE</UserId>
  </GetCallerIdentityResult>
  <ResponseMetadata>
    <RequestId>{request_id}</RequestId>
  </ResponseMetadata>
</GetCallerIdentityResponse>"#,
        account_id = state.account_id,
        request_id = request_id,
    );

    Ok((StatusCode::OK, [("Content-Type", "text/xml")], xml).into_response())
}

fn assume_role(
    state: &StsState,
    role_arn: &str,
    role_session_name: &str,
) -> Result<Response, LawsError> {
    if role_arn.is_empty() {
        return Err(LawsError::InvalidRequest("RoleArn is required".to_string()));
    }
    if role_session_name.is_empty() {
        return Err(LawsError::InvalidRequest(
            "RoleSessionName is required".to_string(),
        ));
    }

    let request_id = uuid::Uuid::new_v4();
    let session_token = format!("FwoGZXIvYXdzE{}", uuid::Uuid::new_v4().simple());
    let access_key_id =
        format!("ASIA{}", &uuid::Uuid::new_v4().simple().to_string()[..16]).to_uppercase();
    let secret_access_key = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    );
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(1))
        .unwrap_or_else(chrono::Utc::now)
        .format("%Y-%m-%dT%H:%M:%SZ");

    let assumed_role_id = format!(
        "AROA{}:{}",
        &uuid::Uuid::new_v4().simple().to_string()[..16].to_uppercase(),
        role_session_name
    );

    let assumed_role_arn = format!(
        "arn:aws:sts::{}:assumed-role/{}/{}",
        state.account_id,
        role_arn.rsplit('/').next().unwrap_or("role"),
        role_session_name
    );

    let xml = format!(
        r#"<AssumeRoleResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
  <AssumeRoleResult>
    <Credentials>
      <AccessKeyId>{access_key_id}</AccessKeyId>
      <SecretAccessKey>{secret_access_key}</SecretAccessKey>
      <SessionToken>{session_token}</SessionToken>
      <Expiration>{expiration}</Expiration>
    </Credentials>
    <AssumedRoleUser>
      <AssumedRoleId>{assumed_role_id}</AssumedRoleId>
      <Arn>{assumed_role_arn}</Arn>
    </AssumedRoleUser>
  </AssumeRoleResult>
  <ResponseMetadata>
    <RequestId>{request_id}</RequestId>
  </ResponseMetadata>
</AssumeRoleResponse>"#,
        access_key_id = access_key_id,
        secret_access_key = secret_access_key,
        session_token = session_token,
        expiration = expiration,
        assumed_role_id = assumed_role_id,
        assumed_role_arn = assumed_role_arn,
        request_id = request_id,
    );

    Ok((StatusCode::OK, [("Content-Type", "text/xml")], xml).into_response())
}
