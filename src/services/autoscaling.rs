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
pub struct AutoScalingGroup {
    pub name: String,
    pub arn: String,
    pub launch_config_name: String,
    pub min_size: u32,
    pub max_size: u32,
    pub desired_capacity: u32,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct LaunchConfiguration {
    pub name: String,
    pub arn: String,
    pub image_id: String,
    pub instance_type: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct AutoScalingState {
    pub groups: DashMap<String, AutoScalingGroup>,
    pub launch_configs: DashMap<String, LaunchConfiguration>,
}

impl Default for AutoScalingState {
    fn default() -> Self {
        Self {
            groups: DashMap::new(),
            launch_configs: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub fn handle_request(
    state: &AutoScalingState,
    headers: &HeaderMap,
    body: &Bytes,
    uri: &Uri,
) -> Response {
    let req = match parse_query_request(uri, headers, body) {
        Ok(r) => r,
        Err(e) => return xml_error_response(&e),
    };

    let result = match req.action.as_str() {
        "CreateAutoScalingGroup" => create_auto_scaling_group(state, &req.params),
        "DeleteAutoScalingGroup" => delete_auto_scaling_group(state, &req.params),
        "DescribeAutoScalingGroups" => describe_auto_scaling_groups(state, &req.params),
        "UpdateAutoScalingGroup" => update_auto_scaling_group(state, &req.params),
        "SetDesiredCapacity" => set_desired_capacity(state, &req.params),
        "CreateLaunchConfiguration" => create_launch_configuration(state, &req.params),
        "DescribeLaunchConfigurations" => describe_launch_configurations(state, &req.params),
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

fn create_auto_scaling_group(
    state: &AutoScalingState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let name = params
        .get("AutoScalingGroupName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing AutoScalingGroupName".into()))?
        .clone();

    if state.groups.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Auto Scaling group already exists: {name}"
        )));
    }

    let arn = format!(
        "arn:aws:autoscaling:{REGION}:{ACCOUNT_ID}:autoScalingGroup:{}:autoScalingGroupName/{name}",
        uuid::Uuid::new_v4()
    );

    let launch_config_name = params
        .get("LaunchConfigurationName")
        .cloned()
        .unwrap_or_default();

    let min_size = params
        .get("MinSize")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let max_size = params
        .get("MaxSize")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    let desired_capacity = params
        .get("DesiredCapacity")
        .and_then(|v| v.parse().ok())
        .unwrap_or(min_size);

    let group = AutoScalingGroup {
        name: name.clone(),
        arn,
        launch_config_name,
        min_size,
        max_size,
        desired_capacity,
        status: "Active".into(),
    };

    state.groups.insert(name, group);

    Ok(xml_response("CreateAutoScalingGroup", ""))
}

fn delete_auto_scaling_group(
    state: &AutoScalingState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let name = params
        .get("AutoScalingGroupName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing AutoScalingGroupName".into()))?;

    state
        .groups
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Auto Scaling group not found: {name}")))?;

    Ok(xml_response("DeleteAutoScalingGroup", ""))
}

fn describe_auto_scaling_groups(
    state: &AutoScalingState,
    _params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let mut inner = String::new();
    inner.push_str("<AutoScalingGroups>\n");

    for entry in state.groups.iter() {
        let g = entry.value();
        inner.push_str(&format!(
            r#"  <member>
    <AutoScalingGroupName>{}</AutoScalingGroupName>
    <AutoScalingGroupARN>{}</AutoScalingGroupARN>
    <LaunchConfigurationName>{}</LaunchConfigurationName>
    <MinSize>{}</MinSize>
    <MaxSize>{}</MaxSize>
    <DesiredCapacity>{}</DesiredCapacity>
    <Status>{}</Status>
  </member>
"#,
            g.name,
            g.arn,
            g.launch_config_name,
            g.min_size,
            g.max_size,
            g.desired_capacity,
            g.status
        ));
    }

    inner.push_str("</AutoScalingGroups>");

    Ok(xml_response("DescribeAutoScalingGroups", &inner))
}

fn update_auto_scaling_group(
    state: &AutoScalingState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let name = params
        .get("AutoScalingGroupName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing AutoScalingGroupName".into()))?;

    let mut group = state
        .groups
        .get_mut(name)
        .ok_or_else(|| LawsError::NotFound(format!("Auto Scaling group not found: {name}")))?;

    if let Some(min) = params.get("MinSize").and_then(|v| v.parse().ok()) {
        group.min_size = min;
    }
    if let Some(max) = params.get("MaxSize").and_then(|v| v.parse().ok()) {
        group.max_size = max;
    }
    if let Some(desired) = params.get("DesiredCapacity").and_then(|v| v.parse().ok()) {
        group.desired_capacity = desired;
    }
    if let Some(lc) = params.get("LaunchConfigurationName") {
        group.launch_config_name = lc.clone();
    }

    Ok(xml_response("UpdateAutoScalingGroup", ""))
}

fn set_desired_capacity(
    state: &AutoScalingState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let name = params
        .get("AutoScalingGroupName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing AutoScalingGroupName".into()))?;

    let desired: u32 = params
        .get("DesiredCapacity")
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| LawsError::InvalidRequest("Missing DesiredCapacity".into()))?;

    let mut group = state
        .groups
        .get_mut(name)
        .ok_or_else(|| LawsError::NotFound(format!("Auto Scaling group not found: {name}")))?;

    group.desired_capacity = desired;

    Ok(xml_response("SetDesiredCapacity", ""))
}

fn create_launch_configuration(
    state: &AutoScalingState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let name = params
        .get("LaunchConfigurationName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing LaunchConfigurationName".into()))?
        .clone();

    if state.launch_configs.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Launch configuration already exists: {name}"
        )));
    }

    let arn = format!(
        "arn:aws:autoscaling:{REGION}:{ACCOUNT_ID}:launchConfiguration:{}:launchConfigurationName/{name}",
        uuid::Uuid::new_v4()
    );

    let image_id = params
        .get("ImageId")
        .cloned()
        .unwrap_or_else(|| "ami-12345678".into());

    let instance_type = params
        .get("InstanceType")
        .cloned()
        .unwrap_or_else(|| "t2.micro".into());

    let lc = LaunchConfiguration {
        name: name.clone(),
        arn,
        image_id,
        instance_type,
    };

    state.launch_configs.insert(name, lc);

    Ok(xml_response("CreateLaunchConfiguration", ""))
}

fn describe_launch_configurations(
    state: &AutoScalingState,
    _params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let mut inner = String::new();
    inner.push_str("<LaunchConfigurations>\n");

    for entry in state.launch_configs.iter() {
        let lc = entry.value();
        inner.push_str(&format!(
            r#"  <member>
    <LaunchConfigurationName>{}</LaunchConfigurationName>
    <LaunchConfigurationARN>{}</LaunchConfigurationARN>
    <ImageId>{}</ImageId>
    <InstanceType>{}</InstanceType>
  </member>
"#,
            lc.name, lc.arn, lc.image_id, lc.instance_type
        ));
    }

    inner.push_str("</LaunchConfigurations>");

    Ok(xml_response("DescribeLaunchConfigurations", &inner))
}
