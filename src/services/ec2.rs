use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, Uri};
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use chrono::Utc;
use dashmap::DashMap;
use rand::Rng;

use crate::error::LawsError;
use crate::protocol::query::{parse_query_request, xml_error_response, xml_response};

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Ec2Instance {
    pub instance_id: String,
    pub image_id: String,
    pub instance_type: String,
    pub state: String,
    pub launch_time: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Ec2State {
    pub instances: Arc<DashMap<String, Ec2Instance>>,
}

impl Default for Ec2State {
    fn default() -> Self {
        Self {
            instances: Arc::new(DashMap::new()),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<Ec2State>) -> Router {
    Router::new()
        .route("/", post(handle_ec2))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn generate_instance_id() -> String {
    let mut rng = rand::thread_rng();
    let val: u64 = rng.gen();
    format!("i-{:016x}", val)
}

fn instance_state_xml(name: &str) -> String {
    let code = match name {
        "running" => 16,
        "stopped" => 80,
        "terminated" => 48,
        "pending" => 0,
        "stopping" => 64,
        "shutting-down" => 32,
        _ => 0,
    };
    format!(
        "<instanceState><code>{code}</code><name>{name}</name></instanceState>"
    )
}

fn collect_indexed_params(params: &HashMap<String, String>, prefix: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut n = 1;
    loop {
        let key = format!("{prefix}.{n}");
        match params.get(&key) {
            Some(v) => {
                values.push(v.clone());
                n += 1;
            }
            None => break,
        }
    }
    values
}

// ---------------------------------------------------------------------------
// Dispatch handler
// ---------------------------------------------------------------------------

pub fn handle_request(
    state: &Ec2State,
    headers: &HeaderMap,
    body: &Bytes,
    uri: &Uri,
) -> Response {
    let req = match parse_query_request(uri, headers, body) {
        Ok(r) => r,
        Err(e) => return xml_error_response(&e),
    };

    let result = match req.action.as_str() {
        "RunInstances" => run_instances(state, &req.params),
        "DescribeInstances" => describe_instances(state),
        "TerminateInstances" => terminate_instances(state, &req.params),
        "StartInstances" => start_instances(state, &req.params),
        "StopInstances" => stop_instances(state, &req.params),
        "RebootInstances" => reboot_instances(state, &req.params),
        "DescribeSecurityGroups" => describe_security_groups(),
        "DescribeVpcs" => describe_vpcs(),
        "DescribeSubnets" => describe_subnets(),
        "DescribeImages" => describe_images(state),
        "DeregisterImage" => deregister_image(&req.params),
        "DescribeVolumes" => describe_volumes(),
        "DeleteVolume" => delete_volume(&req.params),
        "DescribeSnapshots" => describe_snapshots(),
        "DeleteSnapshot" => delete_snapshot(&req.params),
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

async fn handle_ec2(
    State(state): State<Arc<Ec2State>>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_request(&state, &headers, &body, &uri)
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn run_instances(
    state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let image_id = params
        .get("ImageId")
        .ok_or_else(|| LawsError::InvalidRequest("Missing ImageId".into()))?
        .clone();

    let instance_type = params
        .get("InstanceType")
        .cloned()
        .unwrap_or_else(|| "t2.micro".to_string());

    let count: usize = params
        .get("MinCount")
        .or_else(|| params.get("MaxCount"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    let now = Utc::now().to_rfc3339();
    let mut items_xml = String::new();

    for _ in 0..count {
        let instance_id = generate_instance_id();
        let instance = Ec2Instance {
            instance_id: instance_id.clone(),
            image_id: image_id.clone(),
            instance_type: instance_type.clone(),
            state: "running".to_string(),
            launch_time: now.clone(),
        };
        state.instances.insert(instance_id.clone(), instance);

        items_xml.push_str(&format!(
            r#"<item>
  <instanceId>{instance_id}</instanceId>
  <imageId>{image_id}</imageId>
  <instanceType>{instance_type}</instanceType>
  {state}
  <launchTime>{now}</launchTime>
</item>
"#,
            state = instance_state_xml("running"),
        ));
    }

    let inner = format!("<instancesSet>{items_xml}</instancesSet>");
    Ok(xml_response("RunInstances", &inner))
}

fn describe_instances(state: &Ec2State) -> Result<Response, LawsError> {
    let mut instances_xml = String::new();
    for entry in state.instances.iter() {
        let inst = entry.value();
        instances_xml.push_str(&format!(
            r#"<item>
  <instanceId>{}</instanceId>
  <imageId>{}</imageId>
  <instanceType>{}</instanceType>
  {}
  <launchTime>{}</launchTime>
</item>
"#,
            inst.instance_id,
            inst.image_id,
            inst.instance_type,
            instance_state_xml(&inst.state),
            inst.launch_time,
        ));
    }

    let inner = format!(
        "<reservationSet><item><instancesSet>{instances_xml}</instancesSet></item></reservationSet>"
    );
    Ok(xml_response("DescribeInstances", &inner))
}

fn terminate_instances(
    state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let instance_ids = collect_indexed_params(params, "InstanceId");
    if instance_ids.is_empty() {
        return Err(LawsError::InvalidRequest(
            "Missing InstanceId.1".into(),
        ));
    }

    let mut items_xml = String::new();
    for id in &instance_ids {
        let mut inst = state
            .instances
            .get_mut(id)
            .ok_or_else(|| LawsError::NotFound(format!("Instance {id} not found")))?;

        let previous_state = inst.state.clone();
        inst.state = "terminated".to_string();

        items_xml.push_str(&format!(
            r#"<item>
  <instanceId>{id}</instanceId>
  <previousState>{prev}</previousState>
  <currentState>{cur}</currentState>
</item>
"#,
            prev = instance_state_xml(&previous_state),
            cur = instance_state_xml("terminated"),
        ));
    }

    let inner = format!("<instancesSet>{items_xml}</instancesSet>");
    Ok(xml_response("TerminateInstances", &inner))
}

fn change_instance_state(
    state: &Ec2State,
    params: &HashMap<String, String>,
    target_state: &str,
    action: &str,
) -> Result<Response, LawsError> {
    let instance_ids = collect_indexed_params(params, "InstanceId");
    if instance_ids.is_empty() {
        return Err(LawsError::InvalidRequest(
            "Missing InstanceId.1".into(),
        ));
    }

    let mut items_xml = String::new();
    for id in &instance_ids {
        let mut inst = state
            .instances
            .get_mut(id)
            .ok_or_else(|| LawsError::NotFound(format!("Instance {id} not found")))?;

        let previous_state = inst.state.clone();
        inst.state = target_state.to_string();

        items_xml.push_str(&format!(
            r#"<item>
  <instanceId>{id}</instanceId>
  <previousState>{prev}</previousState>
  <currentState>{cur}</currentState>
</item>
"#,
            prev = instance_state_xml(&previous_state),
            cur = instance_state_xml(target_state),
        ));
    }

    let inner = format!("<instancesSet>{items_xml}</instancesSet>");
    Ok(xml_response(action, &inner))
}

fn start_instances(
    state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    change_instance_state(state, params, "running", "StartInstances")
}

fn stop_instances(
    state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    change_instance_state(state, params, "stopped", "StopInstances")
}

fn describe_security_groups() -> Result<Response, LawsError> {
    let inner = "<securityGroupInfo></securityGroupInfo>";
    Ok(xml_response("DescribeSecurityGroups", inner))
}

fn describe_vpcs() -> Result<Response, LawsError> {
    let inner = r#"<vpcSet>
  <item>
    <vpcId>vpc-00000000</vpcId>
    <state>available</state>
    <cidrBlock>172.31.0.0/16</cidrBlock>
    <isDefault>true</isDefault>
  </item>
</vpcSet>"#;
    Ok(xml_response("DescribeVpcs", inner))
}

fn describe_subnets() -> Result<Response, LawsError> {
    let inner = "<subnetSet></subnetSet>";
    Ok(xml_response("DescribeSubnets", inner))
}

fn reboot_instances(
    state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let instance_ids = collect_indexed_params(params, "InstanceId");
    if instance_ids.is_empty() {
        return Err(LawsError::InvalidRequest(
            "Missing InstanceId.1".into(),
        ));
    }
    // Verify all instances exist
    for id in &instance_ids {
        if !state.instances.contains_key(id) {
            return Err(LawsError::NotFound(format!("Instance {id} not found")));
        }
    }
    // RebootInstances returns a simple <return>true</return>
    let inner = "<return>true</return>";
    Ok(xml_response("RebootInstances", inner))
}

fn describe_images(state: &Ec2State) -> Result<Response, LawsError> {
    // Collect unique image IDs from running instances and return them as AMIs
    let mut seen = std::collections::HashSet::new();
    let mut images_xml = String::new();
    for entry in state.instances.iter() {
        let inst = entry.value();
        if seen.insert(inst.image_id.clone()) {
            images_xml.push_str(&format!(
                r#"<item>
  <imageId>{}</imageId>
  <imageState>available</imageState>
  <imageType>machine</imageType>
  <name>{}</name>
  <architecture>x86_64</architecture>
  <rootDeviceType>ebs</rootDeviceType>
  <virtualizationType>hvm</virtualizationType>
  <isPublic>false</isPublic>
</item>
"#,
                inst.image_id, inst.image_id,
            ));
        }
    }
    let inner = format!("<imagesSet>{images_xml}</imagesSet>");
    Ok(xml_response("DescribeImages", &inner))
}

fn deregister_image(params: &HashMap<String, String>) -> Result<Response, LawsError> {
    let _image_id = params
        .get("ImageId")
        .ok_or_else(|| LawsError::InvalidRequest("Missing ImageId".into()))?;
    // Deregister is a no-op on the mock; just acknowledge success
    let inner = "<return>true</return>";
    Ok(xml_response("DeregisterImage", inner))
}

fn describe_volumes() -> Result<Response, LawsError> {
    // Return empty volume set — mock doesn't track volumes separately
    let inner = "<volumeSet></volumeSet>";
    Ok(xml_response("DescribeVolumes", inner))
}

fn delete_volume(params: &HashMap<String, String>) -> Result<Response, LawsError> {
    let _volume_id = params
        .get("VolumeId")
        .ok_or_else(|| LawsError::InvalidRequest("Missing VolumeId".into()))?;
    let inner = "<return>true</return>";
    Ok(xml_response("DeleteVolume", inner))
}

fn describe_snapshots() -> Result<Response, LawsError> {
    // Return empty snapshot set
    let inner = "<snapshotSet></snapshotSet>";
    Ok(xml_response("DescribeSnapshots", inner))
}

fn delete_snapshot(params: &HashMap<String, String>) -> Result<Response, LawsError> {
    let _snapshot_id = params
        .get("SnapshotId")
        .ok_or_else(|| LawsError::InvalidRequest("Missing SnapshotId".into()))?;
    let inner = "<return>true</return>";
    Ok(xml_response("DeleteSnapshot", inner))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_id_format() {
        let id = generate_instance_id();
        assert!(id.starts_with("i-"));
        assert_eq!(id.len(), 2 + 16); // "i-" + 16 hex chars
    }

    #[test]
    fn collect_indexed_params_works() {
        let mut params = HashMap::new();
        params.insert("InstanceId.1".to_string(), "i-aaa".to_string());
        params.insert("InstanceId.2".to_string(), "i-bbb".to_string());
        let ids = collect_indexed_params(&params, "InstanceId");
        assert_eq!(ids, vec!["i-aaa", "i-bbb"]);
    }
}
