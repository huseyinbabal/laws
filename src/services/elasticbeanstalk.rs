use axum::body::Bytes;
use axum::http::{HeaderMap, Uri};
use axum::response::Response;
use dashmap::DashMap;

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
pub struct BeanstalkApplication {
    pub name: String,
    pub arn: String,
    pub description: String,
    pub date_created: String,
    pub date_updated: String,
}

#[derive(Debug, Clone)]
pub struct BeanstalkEnvironment {
    pub environment_id: String,
    pub environment_name: String,
    pub application_name: String,
    pub arn: String,
    pub status: String,
    pub health: String,
    pub solution_stack_name: String,
    pub cname: String,
    pub date_created: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ElasticBeanstalkState {
    pub applications: DashMap<String, BeanstalkApplication>,
    pub environments: DashMap<String, BeanstalkEnvironment>,
}

impl Default for ElasticBeanstalkState {
    fn default() -> Self {
        Self {
            applications: DashMap::new(),
            environments: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub fn handle_request(
    state: &ElasticBeanstalkState,
    headers: &HeaderMap,
    body: &Bytes,
    uri: &Uri,
) -> Response {
    let req = match parse_query_request(uri, headers, body) {
        Ok(r) => r,
        Err(e) => return xml_error_response(&e),
    };

    let result = match req.action.as_str() {
        "CreateApplication" => create_application(state, &req.params),
        "DeleteApplication" => delete_application(state, &req.params),
        "DescribeApplications" => describe_applications(state, &req.params),
        "CreateEnvironment" => create_environment(state, &req.params),
        "TerminateEnvironment" => terminate_environment(state, &req.params),
        "DescribeEnvironments" => describe_environments(state, &req.params),
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
// Operations
// ---------------------------------------------------------------------------

fn create_application(
    state: &ElasticBeanstalkState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let name = params
        .get("ApplicationName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing ApplicationName".into()))?
        .clone();

    if state.applications.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Application already exists: {name}"
        )));
    }

    let arn = format!("arn:aws:elasticbeanstalk:{REGION}:{ACCOUNT_ID}:application/{name}");
    let description = params.get("Description").cloned().unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339();

    let app = BeanstalkApplication {
        name: name.clone(),
        arn,
        description,
        date_created: now.clone(),
        date_updated: now,
    };

    state.applications.insert(name, app);

    Ok(xml_response("CreateApplication", ""))
}

fn delete_application(
    state: &ElasticBeanstalkState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let name = params
        .get("ApplicationName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing ApplicationName".into()))?;

    state
        .applications
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Application not found: {name}")))?;

    // Also remove associated environments
    state
        .environments
        .retain(|_, e| e.application_name != *name);

    Ok(xml_response("DeleteApplication", ""))
}

fn describe_applications(
    state: &ElasticBeanstalkState,
    _params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let mut inner = String::new();
    inner.push_str("<Applications>\n");

    for entry in state.applications.iter() {
        let a = entry.value();
        inner.push_str(&format!(
            r#"  <member>
    <ApplicationName>{}</ApplicationName>
    <ApplicationArn>{}</ApplicationArn>
    <Description>{}</Description>
    <DateCreated>{}</DateCreated>
    <DateUpdated>{}</DateUpdated>
  </member>
"#,
            a.name, a.arn, a.description, a.date_created, a.date_updated
        ));
    }

    inner.push_str("</Applications>");

    Ok(xml_response("DescribeApplications", &inner))
}

fn create_environment(
    state: &ElasticBeanstalkState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let app_name = params
        .get("ApplicationName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing ApplicationName".into()))?
        .clone();

    if !state.applications.contains_key(&app_name) {
        return Err(LawsError::NotFound(format!(
            "Application not found: {app_name}"
        )));
    }

    let env_name = params
        .get("EnvironmentName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing EnvironmentName".into()))?
        .clone();

    if state.environments.contains_key(&env_name) {
        return Err(LawsError::AlreadyExists(format!(
            "Environment already exists: {env_name}"
        )));
    }

    let env_id = format!("e-{}", &uuid::Uuid::new_v4().to_string()[..12]);
    let arn =
        format!("arn:aws:elasticbeanstalk:{REGION}:{ACCOUNT_ID}:environment/{app_name}/{env_name}");
    let solution_stack = params
        .get("SolutionStackName")
        .cloned()
        .unwrap_or_else(|| "64bit Amazon Linux 2 v3.4.0 running Docker".into());

    let cname = format!("{env_name}.{REGION}.elasticbeanstalk.com");

    let env = BeanstalkEnvironment {
        environment_id: env_id,
        environment_name: env_name.clone(),
        application_name: app_name,
        arn,
        status: "Ready".to_string(),
        health: "Green".to_string(),
        solution_stack_name: solution_stack,
        cname,
        date_created: chrono::Utc::now().to_rfc3339(),
    };

    state.environments.insert(env_name, env);

    Ok(xml_response("CreateEnvironment", ""))
}

fn terminate_environment(
    state: &ElasticBeanstalkState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let env_name = params
        .get("EnvironmentName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing EnvironmentName".into()))?;

    let mut env = state
        .environments
        .get_mut(env_name)
        .ok_or_else(|| LawsError::NotFound(format!("Environment not found: {env_name}")))?;

    env.status = "Terminated".to_string();

    Ok(xml_response("TerminateEnvironment", ""))
}

fn describe_environments(
    state: &ElasticBeanstalkState,
    _params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let mut inner = String::new();
    inner.push_str("<Environments>\n");

    for entry in state.environments.iter() {
        let e = entry.value();
        inner.push_str(&format!(
            r#"  <member>
    <EnvironmentId>{}</EnvironmentId>
    <EnvironmentName>{}</EnvironmentName>
    <ApplicationName>{}</ApplicationName>
    <EnvironmentArn>{}</EnvironmentArn>
    <Status>{}</Status>
    <Health>{}</Health>
    <SolutionStackName>{}</SolutionStackName>
    <CNAME>{}</CNAME>
    <DateCreated>{}</DateCreated>
  </member>
"#,
            e.environment_id,
            e.environment_name,
            e.application_name,
            e.arn,
            e.status,
            e.health,
            e.solution_stack_name,
            e.cname,
            e.date_created
        ));
    }

    inner.push_str("</Environments>");

    Ok(xml_response("DescribeEnvironments", &inner))
}
