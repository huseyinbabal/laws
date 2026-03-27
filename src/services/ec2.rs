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

use crate::error::LawsError;
use crate::protocol::query::{parse_query_request, xml_error_response, xml_response_ec2};

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
    pub reservation_id: String,
    pub private_ip_address: String,
    pub private_dns_name: String,
    pub public_dns_name: String,
    pub vpc_id: String,
    pub subnet_id: String,
    pub key_name: String,
    pub ami_launch_index: u32,
    pub architecture: String,
    pub root_device_type: String,
    pub root_device_name: String,
    pub virtualization_type: String,
    pub hypervisor: String,
    pub ebs_optimized: bool,
    pub source_dest_check: bool,
    pub security_group_id: String,
    pub security_group_name: String,
}

#[derive(Clone, Debug)]
pub struct Ec2Reservation {
    pub reservation_id: String,
    pub owner_id: String,
    pub instance_ids: Vec<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Ec2State {
    pub instances: Arc<DashMap<String, Ec2Instance>>,
    pub reservations: Arc<DashMap<String, Ec2Reservation>>,
    pub owner_id: String,
    pub region: String,
}

impl Default for Ec2State {
    fn default() -> Self {
        Self {
            instances: Arc::new(DashMap::new()),
            reservations: Arc::new(DashMap::new()),
            owner_id: "000000000000".to_string(),
            region: "us-east-1".to_string(),
        }
    }
}

impl Ec2State {
    pub fn new(owner_id: String, region: String) -> Self {
        Self {
            instances: Arc::new(DashMap::new()),
            reservations: Arc::new(DashMap::new()),
            owner_id,
            region,
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<Ec2State>) -> Router {
    Router::new().route("/", post(handle_ec2)).with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn generate_instance_id() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    let val: u64 = rng.random();
    format!("i-{:016x}", val)
}

fn generate_reservation_id() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    let val: u64 = rng.random();
    format!("r-{:016x}", val)
}

fn generate_sg_id() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    let val: u64 = rng.random();
    format!("sg-{:016x}", val)
}

fn generate_private_ip() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    let b: u8 = rng.random();
    let c: u8 = rng.random();
    format!("172.31.{b}.{c}")
}

fn state_code(name: &str) -> u16 {
    match name {
        "pending" => 0,
        "running" => 16,
        "shutting-down" => 32,
        "terminated" => 48,
        "stopping" => 64,
        "stopped" => 80,
        _ => 0,
    }
}

/// Renders `<instanceState>` for DescribeInstances / RunInstances instance items.
fn instance_state_xml(name: &str) -> String {
    let code = state_code(name);
    format!("<instanceState><code>{code}</code><name>{name}</name></instanceState>")
}

/// Renders `<code>` + `<name>` without an outer wrapper, for use inside
/// `<currentState>` / `<previousState>` in InstanceStateChange responses.
fn state_change_xml(name: &str) -> String {
    let code = state_code(name);
    format!("<code>{code}</code><name>{name}</name>")
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

/// Renders a full `<item>` XML block for a single EC2 instance, matching the
/// AWS DescribeInstances / RunInstances Instance shape.
fn instance_to_xml(inst: &Ec2Instance, owner_id: &str) -> String {
    format!(
        r#"<item>
  <instanceId>{instance_id}</instanceId>
  <imageId>{image_id}</imageId>
  {state}
  <privateDnsName>{private_dns_name}</privateDnsName>
  <dnsName>{public_dns_name}</dnsName>
  <reason/>
  <keyName>{key_name}</keyName>
  <amiLaunchIndex>{ami_launch_index}</amiLaunchIndex>
  <productCodes/>
  <instanceType>{instance_type}</instanceType>
  <launchTime>{launch_time}</launchTime>
  <placement>
    <availabilityZone>us-east-1a</availabilityZone>
    <groupName/>
    <tenancy>default</tenancy>
  </placement>
  <monitoring>
    <state>disabled</state>
  </monitoring>
  <subnetId>{subnet_id}</subnetId>
  <vpcId>{vpc_id}</vpcId>
  <privateIpAddress>{private_ip}</privateIpAddress>
  <sourceDestCheck>{source_dest_check}</sourceDestCheck>
  <groupSet>
    <item>
      <groupId>{sg_id}</groupId>
      <groupName>{sg_name}</groupName>
    </item>
  </groupSet>
  <architecture>{architecture}</architecture>
  <rootDeviceType>{root_device_type}</rootDeviceType>
  <rootDeviceName>{root_device_name}</rootDeviceName>
  <blockDeviceMapping/>
  <virtualizationType>{virt_type}</virtualizationType>
  <hypervisor>{hypervisor}</hypervisor>
  <networkInterfaceSet/>
  <ebsOptimized>{ebs_optimized}</ebsOptimized>
  <ownerId>{owner_id}</ownerId>
  <tagSet/>
</item>"#,
        instance_id = inst.instance_id,
        image_id = inst.image_id,
        state = instance_state_xml(&inst.state),
        private_dns_name = inst.private_dns_name,
        public_dns_name = inst.public_dns_name,
        key_name = inst.key_name,
        ami_launch_index = inst.ami_launch_index,
        instance_type = inst.instance_type,
        launch_time = inst.launch_time,
        subnet_id = inst.subnet_id,
        vpc_id = inst.vpc_id,
        private_ip = inst.private_ip_address,
        source_dest_check = inst.source_dest_check,
        sg_id = inst.security_group_id,
        sg_name = inst.security_group_name,
        architecture = inst.architecture,
        root_device_type = inst.root_device_type,
        root_device_name = inst.root_device_name,
        virt_type = inst.virtualization_type,
        hypervisor = inst.hypervisor,
        ebs_optimized = inst.ebs_optimized,
        owner_id = owner_id,
    )
}

// ---------------------------------------------------------------------------
// Dispatch handler
// ---------------------------------------------------------------------------

pub fn handle_request(state: &Ec2State, headers: &HeaderMap, body: &Bytes, uri: &Uri) -> Response {
    let req = match parse_query_request(uri, headers, body) {
        Ok(r) => r,
        Err(e) => return xml_error_response(&e),
    };

    let result = match req.action.as_str() {
        "RunInstances" => run_instances(state, &req.params),
        "DescribeInstances" => describe_instances(state, &req.params),
        "TerminateInstances" => terminate_instances(state, &req.params),
        "StartInstances" => start_instances(state, &req.params),
        "StopInstances" => stop_instances(state, &req.params),
        "RebootInstances" => reboot_instances(state, &req.params),
        "DescribeSecurityGroups" => describe_security_groups(state, &req.params),
        "DescribeVpcs" => describe_vpcs(state, &req.params),
        "DescribeSubnets" => describe_subnets(state, &req.params),
        "DescribeImages" => describe_images(state, &req.params),
        "DeregisterImage" => deregister_image(&req.params),
        "DescribeVolumes" => describe_volumes(state, &req.params),
        "DeleteVolume" => delete_volume(&req.params),
        "DescribeSnapshots" => describe_snapshots(state, &req.params),
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
// Pagination helper
// ---------------------------------------------------------------------------

/// Parse MaxResults and NextToken from request parameters. Returns
/// `(start_index, max_results)`. The NextToken is simply the stringified
/// start index (good enough for an in-memory mock).
fn parse_pagination(params: &HashMap<String, String>, default_max: usize) -> (usize, usize) {
    let start: usize = params
        .get("NextToken")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let max: usize = params
        .get("MaxResults")
        .and_then(|v| v.parse().ok())
        .unwrap_or(default_max);
    (start, max)
}

fn next_token_xml(start: usize, count: usize, total: usize) -> String {
    if start + count < total {
        format!("<nextToken>{}</nextToken>", start + count)
    } else {
        String::new()
    }
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

    let key_name = params.get("KeyName").cloned().unwrap_or_default();

    let count: usize = params
        .get("MinCount")
        .or_else(|| params.get("MaxCount"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let reservation_id = generate_reservation_id();
    let sg_id = generate_sg_id();
    let sg_name = "default".to_string();
    let vpc_id = "vpc-00000000".to_string();
    let subnet_id = "subnet-00000000".to_string();

    let mut instance_ids = Vec::with_capacity(count);
    let mut items_xml = String::new();

    for idx in 0..count {
        let instance_id = generate_instance_id();
        let private_ip = generate_private_ip();
        let private_dns = format!(
            "ip-{}.{}.compute.internal",
            private_ip.replace('.', "-"),
            state.region
        );

        let instance = Ec2Instance {
            instance_id: instance_id.clone(),
            image_id: image_id.clone(),
            instance_type: instance_type.clone(),
            state: "pending".to_string(),
            launch_time: now.clone(),
            reservation_id: reservation_id.clone(),
            private_ip_address: private_ip,
            private_dns_name: private_dns,
            public_dns_name: String::new(),
            vpc_id: vpc_id.clone(),
            subnet_id: subnet_id.clone(),
            key_name: key_name.clone(),
            ami_launch_index: idx as u32,
            architecture: "x86_64".to_string(),
            root_device_type: "ebs".to_string(),
            root_device_name: "/dev/sda1".to_string(),
            virtualization_type: "hvm".to_string(),
            hypervisor: "xen".to_string(),
            ebs_optimized: false,
            source_dest_check: true,
            security_group_id: sg_id.clone(),
            security_group_name: sg_name.clone(),
        };

        items_xml.push_str(&instance_to_xml(&instance, &state.owner_id));
        instance_ids.push(instance_id.clone());
        state.instances.insert(instance_id, instance);
    }

    let reservation = Ec2Reservation {
        reservation_id: reservation_id.clone(),
        owner_id: state.owner_id.clone(),
        instance_ids,
    };
    state
        .reservations
        .insert(reservation_id.clone(), reservation);

    let inner = format!(
        r#"<reservationId>{reservation_id}</reservationId>
    <ownerId>{owner_id}</ownerId>
    <groupSet/>
    <instancesSet>{items_xml}</instancesSet>"#,
        reservation_id = reservation_id,
        owner_id = state.owner_id,
    );
    Ok(xml_response_ec2("RunInstances", &inner))
}

fn describe_instances(
    state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    // Collect filter instance IDs if provided
    let filter_ids = collect_indexed_params(params, "InstanceId");

    // Group instances by reservation
    let mut reservation_map: HashMap<String, Vec<String>> = HashMap::new();
    for entry in state.instances.iter() {
        let inst = entry.value();
        if !filter_ids.is_empty() && !filter_ids.contains(&inst.instance_id) {
            continue;
        }
        reservation_map
            .entry(inst.reservation_id.clone())
            .or_default()
            .push(inst.instance_id.clone());
    }

    // Collect all reservations into a Vec for pagination
    let mut all_reservations: Vec<(String, Vec<String>)> = reservation_map.into_iter().collect();
    all_reservations.sort_by(|a, b| a.0.cmp(&b.0));

    let total = all_reservations.len();
    let (start, max) = parse_pagination(params, 1000);
    let page = &all_reservations[start..std::cmp::min(start + max, total)];

    let mut reservations_xml = String::new();
    for (res_id, inst_ids) in page {
        let owner_id = state
            .reservations
            .get(res_id)
            .map(|r| r.owner_id.clone())
            .unwrap_or_else(|| state.owner_id.clone());

        let mut instances_xml = String::new();
        for iid in inst_ids {
            if let Some(inst) = state.instances.get(iid) {
                instances_xml.push_str(&instance_to_xml(&inst, &owner_id));
            }
        }

        reservations_xml.push_str(&format!(
            r#"<item>
  <reservationId>{res_id}</reservationId>
  <ownerId>{owner_id}</ownerId>
  <groupSet/>
  <instancesSet>{instances_xml}</instancesSet>
</item>"#
        ));
    }

    let token = next_token_xml(start, page.len(), total);
    let inner = format!("<reservationSet>{reservations_xml}</reservationSet>{token}");
    Ok(xml_response_ec2("DescribeInstances", &inner))
}

fn terminate_instances(
    state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let instance_ids = collect_indexed_params(params, "InstanceId");
    if instance_ids.is_empty() {
        return Err(LawsError::InvalidRequest("Missing InstanceId.1".into()));
    }

    let mut items_xml = String::new();
    for id in &instance_ids {
        let mut inst = state
            .instances
            .get_mut(id)
            .ok_or_else(|| LawsError::NotFound(format!("Instance {id} not found")))?;

        let previous_state = inst.state.clone();
        inst.state = "shutting-down".to_string();

        items_xml.push_str(&format!(
            r#"<item>
  <instanceId>{id}</instanceId>
  <currentState>{current}</currentState>
  <previousState>{previous}</previousState>
</item>"#,
            current = state_change_xml("shutting-down"),
            previous = state_change_xml(&previous_state),
        ));
    }

    let inner = format!("<instancesSet>{items_xml}</instancesSet>");
    Ok(xml_response_ec2("TerminateInstances", &inner))
}

fn change_instance_state(
    state: &Ec2State,
    params: &HashMap<String, String>,
    target_state: &str,
    action: &str,
) -> Result<Response, LawsError> {
    let instance_ids = collect_indexed_params(params, "InstanceId");
    if instance_ids.is_empty() {
        return Err(LawsError::InvalidRequest("Missing InstanceId.1".into()));
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
  <currentState>{current}</currentState>
  <previousState>{previous}</previousState>
</item>"#,
            current = state_change_xml(target_state),
            previous = state_change_xml(&previous_state),
        ));
    }

    let inner = format!("<instancesSet>{items_xml}</instancesSet>");
    Ok(xml_response_ec2(action, &inner))
}

fn start_instances(
    state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    change_instance_state(state, params, "pending", "StartInstances")
}

fn stop_instances(
    state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    change_instance_state(state, params, "stopping", "StopInstances")
}

fn describe_security_groups(
    state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let (start, max) = parse_pagination(params, 1000);

    // Return at least a default security group for the default VPC
    let items = [format!(
        r#"<item>
  <ownerId>{owner_id}</ownerId>
  <groupId>sg-00000000</groupId>
  <groupName>default</groupName>
  <groupDescription>default VPC security group</groupDescription>
  <vpcId>vpc-00000000</vpcId>
  <ipPermissions>
    <item>
      <ipProtocol>-1</ipProtocol>
      <groups>
        <item>
          <userId>{owner_id}</userId>
          <groupId>sg-00000000</groupId>
        </item>
      </groups>
      <ipRanges/>
      <ipv6Ranges/>
      <prefixListIds/>
    </item>
  </ipPermissions>
  <ipPermissionsEgress>
    <item>
      <ipProtocol>-1</ipProtocol>
      <groups/>
      <ipRanges>
        <item>
          <cidrIp>0.0.0.0/0</cidrIp>
        </item>
      </ipRanges>
      <ipv6Ranges/>
      <prefixListIds/>
    </item>
  </ipPermissionsEgress>
  <tagSet/>
</item>"#,
        owner_id = state.owner_id,
    )];

    let total = items.len();
    let page = &items[start..std::cmp::min(start + max, total)];
    let sg_xml: String = page.iter().cloned().collect();
    let token = next_token_xml(start, page.len(), total);

    let inner = format!("<securityGroupInfo>{sg_xml}</securityGroupInfo>{token}");
    Ok(xml_response_ec2("DescribeSecurityGroups", &inner))
}

fn describe_vpcs(
    state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let (start, max) = parse_pagination(params, 1000);

    let items = [format!(
        r#"<item>
  <vpcId>vpc-00000000</vpcId>
  <ownerId>{owner_id}</ownerId>
  <state>available</state>
  <cidrBlock>172.31.0.0/16</cidrBlock>
  <cidrBlockAssociationSet>
    <item>
      <cidrBlock>172.31.0.0/16</cidrBlock>
      <associationId>vpc-cidr-assoc-00000000</associationId>
      <cidrBlockState>
        <state>associated</state>
      </cidrBlockState>
    </item>
  </cidrBlockAssociationSet>
  <dhcpOptionsId>dopt-00000000</dhcpOptionsId>
  <instanceTenancy>default</instanceTenancy>
  <isDefault>true</isDefault>
  <tagSet/>
</item>"#,
        owner_id = state.owner_id,
    )];

    let total = items.len();
    let page = &items[start..std::cmp::min(start + max, total)];
    let vpc_xml: String = page.iter().cloned().collect();
    let token = next_token_xml(start, page.len(), total);

    let inner = format!("<vpcSet>{vpc_xml}</vpcSet>{token}");
    Ok(xml_response_ec2("DescribeVpcs", &inner))
}

fn describe_subnets(
    _state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let (_start, _max) = parse_pagination(params, 1000);
    let inner = "<subnetSet/><nextToken/>";
    Ok(xml_response_ec2("DescribeSubnets", inner))
}

fn reboot_instances(
    state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let instance_ids = collect_indexed_params(params, "InstanceId");
    if instance_ids.is_empty() {
        return Err(LawsError::InvalidRequest("Missing InstanceId.1".into()));
    }
    // Verify all instances exist
    for id in &instance_ids {
        if !state.instances.contains_key(id) {
            return Err(LawsError::NotFound(format!("Instance {id} not found")));
        }
    }
    // RebootInstances returns a simple <return>true</return>
    let inner = "<return>true</return>";
    Ok(xml_response_ec2("RebootInstances", inner))
}

fn describe_images(
    state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let (start, max) = parse_pagination(params, 1000);
    let filter_ids = collect_indexed_params(params, "ImageId");

    // Collect unique image IDs from running instances and return them as AMIs
    let mut seen = std::collections::HashSet::new();
    let mut all_images: Vec<String> = Vec::new();

    for entry in state.instances.iter() {
        let inst = entry.value();
        if seen.insert(inst.image_id.clone()) {
            if !filter_ids.is_empty() && !filter_ids.contains(&inst.image_id) {
                continue;
            }
            all_images.push(format!(
                r#"<item>
  <imageId>{image_id}</imageId>
  <imageLocation>{owner_id}/{image_id}</imageLocation>
  <imageState>available</imageState>
  <imageOwnerId>{owner_id}</imageOwnerId>
  <isPublic>false</isPublic>
  <architecture>x86_64</architecture>
  <imageType>machine</imageType>
  <name>{image_id}</name>
  <description/>
  <rootDeviceType>ebs</rootDeviceType>
  <rootDeviceName>/dev/sda1</rootDeviceName>
  <blockDeviceMapping>
    <item>
      <deviceName>/dev/sda1</deviceName>
      <ebs>
        <snapshotId>snap-00000000</snapshotId>
        <volumeSize>8</volumeSize>
        <deleteOnTermination>true</deleteOnTermination>
        <volumeType>gp2</volumeType>
      </ebs>
    </item>
  </blockDeviceMapping>
  <virtualizationType>hvm</virtualizationType>
  <hypervisor>xen</hypervisor>
  <enaSupport>true</enaSupport>
  <sriovNetSupport>simple</sriovNetSupport>
  <creationDate>{launch_time}</creationDate>
  <tagSet/>
</item>"#,
                image_id = inst.image_id,
                owner_id = state.owner_id,
                launch_time = inst.launch_time,
            ));
        }
    }

    let total = all_images.len();
    let end = std::cmp::min(start + max, total);
    let page = if start < total {
        &all_images[start..end]
    } else {
        &[]
    };
    let images_xml: String = page.iter().cloned().collect();
    let token = next_token_xml(start, page.len(), total);

    let inner = format!("<imagesSet>{images_xml}</imagesSet>{token}");
    Ok(xml_response_ec2("DescribeImages", &inner))
}

fn deregister_image(params: &HashMap<String, String>) -> Result<Response, LawsError> {
    let _image_id = params
        .get("ImageId")
        .ok_or_else(|| LawsError::InvalidRequest("Missing ImageId".into()))?;
    // Deregister is a no-op on the mock; just acknowledge success
    let inner = "<return>true</return>";
    Ok(xml_response_ec2("DeregisterImage", inner))
}

fn describe_volumes(
    _state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let (_start, _max) = parse_pagination(params, 1000);
    let inner = "<volumeSet/><nextToken/>";
    Ok(xml_response_ec2("DescribeVolumes", inner))
}

fn delete_volume(params: &HashMap<String, String>) -> Result<Response, LawsError> {
    let _volume_id = params
        .get("VolumeId")
        .ok_or_else(|| LawsError::InvalidRequest("Missing VolumeId".into()))?;
    let inner = "<return>true</return>";
    Ok(xml_response_ec2("DeleteVolume", inner))
}

fn describe_snapshots(
    _state: &Ec2State,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let (_start, _max) = parse_pagination(params, 1000);
    let inner = "<snapshotSet/><nextToken/>";
    Ok(xml_response_ec2("DescribeSnapshots", inner))
}

fn delete_snapshot(params: &HashMap<String, String>) -> Result<Response, LawsError> {
    let _snapshot_id = params
        .get("SnapshotId")
        .ok_or_else(|| LawsError::InvalidRequest("Missing SnapshotId".into()))?;
    let inner = "<return>true</return>";
    Ok(xml_response_ec2("DeleteSnapshot", inner))
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
    fn reservation_id_format() {
        let id = generate_reservation_id();
        assert!(id.starts_with("r-"));
        assert_eq!(id.len(), 2 + 16);
    }

    #[test]
    fn collect_indexed_params_works() {
        let mut params = HashMap::new();
        params.insert("InstanceId.1".to_string(), "i-aaa".to_string());
        params.insert("InstanceId.2".to_string(), "i-bbb".to_string());
        let ids = collect_indexed_params(&params, "InstanceId");
        assert_eq!(ids, vec!["i-aaa", "i-bbb"]);
    }

    #[test]
    fn state_change_xml_format() {
        let xml = state_change_xml("running");
        assert_eq!(xml, "<code>16</code><name>running</name>");
        // Must NOT contain <instanceState> wrapper
        assert!(!xml.contains("instanceState"));
    }

    #[test]
    fn instance_state_xml_format() {
        let xml = instance_state_xml("running");
        assert!(xml.contains("<instanceState>"));
        assert!(xml.contains("<code>16</code>"));
        assert!(xml.contains("<name>running</name>"));
    }
}
