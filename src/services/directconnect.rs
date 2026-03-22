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
pub struct Connection {
    pub connection_id: String,
    pub connection_name: String,
    pub connection_state: String,
    pub bandwidth: String,
    pub location: String,
}

#[derive(Debug, Clone)]
pub struct VirtualInterface {
    pub virtual_interface_id: String,
    pub connection_id: String,
    pub virtual_interface_name: String,
    pub vlan: i64,
    pub asn: i64,
    pub virtual_interface_state: String,
    pub virtual_interface_type: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct DirectConnectState {
    pub connections: DashMap<String, Connection>,
    pub virtual_interfaces: DashMap<String, VirtualInterface>,
}

impl Default for DirectConnectState {
    fn default() -> Self {
        Self {
            connections: DashMap::new(),
            virtual_interfaces: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &DirectConnectState, target: &str, payload: &Value) -> Response {
    let action = target.strip_prefix("OvertureService.").unwrap_or(target);

    let result = match action {
        "CreateConnection" => create_connection(state, payload),
        "DeleteConnection" => delete_connection(state, payload),
        "DescribeConnections" => describe_connections(state),
        "CreateVirtualInterface" => create_virtual_interface(state, payload),
        "DeleteVirtualInterface" => delete_virtual_interface(state, payload),
        "DescribeVirtualInterfaces" => describe_virtual_interfaces(state),
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

fn connection_to_json(c: &Connection) -> Value {
    json!({
        "connectionId": c.connection_id,
        "connectionName": c.connection_name,
        "connectionState": c.connection_state,
        "bandwidth": c.bandwidth,
        "location": c.location,
        "region": REGION,
        "ownerAccount": ACCOUNT_ID,
    })
}

fn virtual_interface_to_json(vi: &VirtualInterface) -> Value {
    json!({
        "virtualInterfaceId": vi.virtual_interface_id,
        "connectionId": vi.connection_id,
        "virtualInterfaceName": vi.virtual_interface_name,
        "vlan": vi.vlan,
        "asn": vi.asn,
        "virtualInterfaceState": vi.virtual_interface_state,
        "virtualInterfaceType": vi.virtual_interface_type,
        "ownerAccount": ACCOUNT_ID,
        "region": REGION,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_connection(state: &DirectConnectState, payload: &Value) -> Result<Response, LawsError> {
    let connection_name = payload["connectionName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing connectionName".into()))?
        .to_string();

    let bandwidth = payload["bandwidth"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing bandwidth".into()))?
        .to_string();

    let location = payload["location"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing location".into()))?
        .to_string();

    let conn_id = format!("dxcon-{}", &uuid::Uuid::new_v4().to_string()[..8]);

    let connection = Connection {
        connection_id: conn_id.clone(),
        connection_name,
        connection_state: "requested".to_string(),
        bandwidth,
        location,
    };

    let resp = connection_to_json(&connection);
    state.connections.insert(conn_id, connection);
    Ok(json_response(resp))
}

fn delete_connection(state: &DirectConnectState, payload: &Value) -> Result<Response, LawsError> {
    let connection_id = payload["connectionId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing connectionId".into()))?;

    let (_, conn) = state
        .connections
        .remove(connection_id)
        .ok_or_else(|| LawsError::NotFound(format!("Connection not found: {connection_id}")))?;

    state
        .virtual_interfaces
        .retain(|_, vi| vi.connection_id != connection_id);

    let mut resp = connection_to_json(&conn);
    resp["connectionState"] = json!("deleted");
    Ok(json_response(resp))
}

fn describe_connections(state: &DirectConnectState) -> Result<Response, LawsError> {
    let connections: Vec<Value> = state
        .connections
        .iter()
        .map(|entry| connection_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "connections": connections })))
}

fn create_virtual_interface(
    state: &DirectConnectState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let connection_id = payload["connectionId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing connectionId".into()))?
        .to_string();

    if !state.connections.contains_key(&connection_id) {
        return Err(LawsError::NotFound(format!(
            "Connection not found: {connection_id}"
        )));
    }

    let new_vif = payload
        .get("newPublicVirtualInterface")
        .or_else(|| payload.get("newPrivateVirtualInterface"))
        .ok_or_else(|| {
            LawsError::InvalidRequest("Missing virtual interface configuration".into())
        })?;

    let vif_name = new_vif["virtualInterfaceName"]
        .as_str()
        .unwrap_or("vif")
        .to_string();

    let vlan = new_vif["vlan"].as_i64().unwrap_or(0);
    let asn = new_vif["asn"].as_i64().unwrap_or(0);

    let vif_type = if payload.get("newPublicVirtualInterface").is_some() {
        "public"
    } else {
        "private"
    };

    let vif_id = format!("dxvif-{}", &uuid::Uuid::new_v4().to_string()[..8]);

    let vi = VirtualInterface {
        virtual_interface_id: vif_id.clone(),
        connection_id,
        virtual_interface_name: vif_name,
        vlan,
        asn,
        virtual_interface_state: "pending".to_string(),
        virtual_interface_type: vif_type.to_string(),
    };

    let resp = virtual_interface_to_json(&vi);
    state.virtual_interfaces.insert(vif_id, vi);
    Ok(json_response(resp))
}

fn delete_virtual_interface(
    state: &DirectConnectState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let vif_id = payload["virtualInterfaceId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing virtualInterfaceId".into()))?;

    state
        .virtual_interfaces
        .remove(vif_id)
        .ok_or_else(|| LawsError::NotFound(format!("Virtual interface not found: {vif_id}")))?;

    Ok(json_response(json!({ "virtualInterfaceState": "deleted" })))
}

fn describe_virtual_interfaces(state: &DirectConnectState) -> Result<Response, LawsError> {
    let interfaces: Vec<Value> = state
        .virtual_interfaces
        .iter()
        .map(|entry| virtual_interface_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "virtualInterfaces": interfaces })))
}
