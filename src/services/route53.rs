use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::error::LawsError;
use crate::protocol::rest_xml;

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize, Deserialize)]
pub struct HostedZone {
    pub id: String,
    pub name: String,
    pub caller_reference: String,
    pub record_set_count: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ResourceRecordSet {
    pub name: String,
    pub type_: String,
    pub ttl: u32,
    pub records: Vec<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct Route53State {
    pub zones: DashMap<String, HostedZone>,
    pub record_sets: DashMap<String, Vec<ResourceRecordSet>>,
}

impl Default for Route53State {
    fn default() -> Self {
        Self {
            zones: DashMap::new(),
            record_sets: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<Route53State>) -> axum::Router {
    axum::Router::new()
        .route(
            "/2013-04-01/hostedzone",
            axum::routing::post(create_hosted_zone).get(list_hosted_zones),
        )
        .route(
            "/2013-04-01/hostedzone/{zone_id}",
            get(get_hosted_zone).delete(delete_hosted_zone),
        )
        .route(
            "/2013-04-01/hostedzone/{zone_id}/rrset",
            axum::routing::post(change_resource_record_sets).get(list_resource_record_sets),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn random_zone_id() -> String {
    use rand::RngExt;
    let suffix: String = rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(13)
        .map(|c| char::from(c).to_ascii_uppercase())
        .map(char::from)
        .collect();
    format!("Z{suffix}")
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml.find(&close)?;
    Some(xml[start..end].to_string())
}

/// Extract all occurrences of a tag from XML.
fn extract_all_xml_tags(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let mut results = Vec::new();
    let mut search_from = 0;
    while let Some(start_pos) = xml[search_from..].find(&open) {
        let abs_start = search_from + start_pos + open.len();
        if let Some(end_pos) = xml[abs_start..].find(&close) {
            results.push(xml[abs_start..abs_start + end_pos].to_string());
            search_from = abs_start + end_pos + close.len();
        } else {
            break;
        }
    }
    results
}

fn xml_response(status: StatusCode, body: String) -> Response {
    (status, [("content-type", "application/xml")], body).into_response()
}

fn hosted_zone_xml(zone: &HostedZone) -> String {
    format!(
        r#"<HostedZone>
    <Id>/hostedzone/{id}</Id>
    <Name>{name}</Name>
    <CallerReference>{caller_ref}</CallerReference>
    <Config><PrivateZone>false</PrivateZone></Config>
    <ResourceRecordSetCount>{count}</ResourceRecordSetCount>
  </HostedZone>"#,
        id = zone.id,
        name = zone.name,
        caller_ref = zone.caller_reference,
        count = zone.record_set_count,
    )
}

fn record_set_xml(rrs: &ResourceRecordSet) -> String {
    let records_xml: String = rrs
        .records
        .iter()
        .map(|v| format!("<ResourceRecord><Value>{v}</Value></ResourceRecord>"))
        .collect::<Vec<_>>()
        .join("\n      ");
    format!(
        r#"<ResourceRecordSet>
      <Name>{name}</Name>
      <Type>{type_}</Type>
      <TTL>{ttl}</TTL>
      <ResourceRecords>
      {records}
      </ResourceRecords>
    </ResourceRecordSet>"#,
        name = rrs.name,
        type_ = rrs.type_,
        ttl = rrs.ttl,
        records = records_xml,
    )
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_hosted_zone(State(state): State<Arc<Route53State>>, body: Bytes) -> Response {
    let xml = match std::str::from_utf8(&body) {
        Ok(s) => s.to_string(),
        Err(_) => {
            return rest_xml::error_response(&LawsError::InvalidRequest(
                "Invalid UTF-8 in request body".into(),
            ));
        }
    };

    let name = match extract_xml_tag(&xml, "Name") {
        Some(n) => n,
        None => {
            return rest_xml::error_response(&LawsError::InvalidRequest(
                "Missing Name element".into(),
            ));
        }
    };

    let caller_reference = match extract_xml_tag(&xml, "CallerReference") {
        Some(r) => r,
        None => {
            return rest_xml::error_response(&LawsError::InvalidRequest(
                "Missing CallerReference element".into(),
            ));
        }
    };

    // Check for duplicate caller reference.
    let duplicate = state
        .zones
        .iter()
        .any(|z| z.caller_reference == caller_reference);
    if duplicate {
        return rest_xml::error_response(&LawsError::AlreadyExists(format!(
            "HostedZone with CallerReference {} already exists",
            caller_reference
        )));
    }

    let zone_id = random_zone_id();
    let zone = HostedZone {
        id: zone_id.clone(),
        name: name.clone(),
        caller_reference: caller_reference.clone(),
        record_set_count: 0,
    };

    state.zones.insert(zone_id.clone(), zone.clone());
    state.record_sets.insert(zone_id.clone(), Vec::new());

    let change_id = uuid::Uuid::new_v4();
    let submitted_at = Utc::now().to_rfc3339();

    let response_body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CreateHostedZoneResponse xmlns="https://route53.amazonaws.com/doc/2013-04-01/">
  {hosted_zone}
  <ChangeInfo>
    <Id>/change/{change_id}</Id>
    <Status>INSYNC</Status>
    <SubmittedAt>{submitted_at}</SubmittedAt>
  </ChangeInfo>
</CreateHostedZoneResponse>"#,
        hosted_zone = hosted_zone_xml(&zone),
    );

    xml_response(StatusCode::CREATED, response_body)
}

async fn list_hosted_zones(State(state): State<Arc<Route53State>>) -> Response {
    let zones_xml: String = state
        .zones
        .iter()
        .map(|entry| hosted_zone_xml(entry.value()))
        .collect::<Vec<_>>()
        .join("\n  ");

    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListHostedZonesResponse xmlns="https://route53.amazonaws.com/doc/2013-04-01/">
  <HostedZones>
  {zones_xml}
  </HostedZones>
  <IsTruncated>false</IsTruncated>
  <MaxItems>100</MaxItems>
</ListHostedZonesResponse>"#,
    );

    xml_response(StatusCode::OK, body)
}

async fn get_hosted_zone(
    State(state): State<Arc<Route53State>>,
    Path(zone_id): Path<String>,
) -> Response {
    match state.zones.get(&zone_id) {
        Some(zone) => {
            let body = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<GetHostedZoneResponse xmlns="https://route53.amazonaws.com/doc/2013-04-01/">
  {hosted_zone}
</GetHostedZoneResponse>"#,
                hosted_zone = hosted_zone_xml(&zone),
            );
            xml_response(StatusCode::OK, body)
        }
        None => rest_xml::error_response(&LawsError::NotFound(format!(
            "HostedZone not found: {zone_id}"
        ))),
    }
}

async fn delete_hosted_zone(
    State(state): State<Arc<Route53State>>,
    Path(zone_id): Path<String>,
) -> Response {
    match state.zones.remove(&zone_id) {
        Some(_) => {
            state.record_sets.remove(&zone_id);

            let change_id = uuid::Uuid::new_v4();
            let submitted_at = Utc::now().to_rfc3339();

            let body = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<DeleteHostedZoneResponse xmlns="https://route53.amazonaws.com/doc/2013-04-01/">
  <ChangeInfo>
    <Id>/change/{change_id}</Id>
    <Status>INSYNC</Status>
    <SubmittedAt>{submitted_at}</SubmittedAt>
  </ChangeInfo>
</DeleteHostedZoneResponse>"#,
            );
            xml_response(StatusCode::OK, body)
        }
        None => rest_xml::error_response(&LawsError::NotFound(format!(
            "HostedZone not found: {zone_id}"
        ))),
    }
}

async fn change_resource_record_sets(
    State(state): State<Arc<Route53State>>,
    Path(zone_id): Path<String>,
    body: Bytes,
) -> Response {
    if !state.zones.contains_key(&zone_id) {
        return rest_xml::error_response(&LawsError::NotFound(format!(
            "HostedZone not found: {zone_id}"
        )));
    }

    let xml = match std::str::from_utf8(&body) {
        Ok(s) => s.to_string(),
        Err(_) => {
            return rest_xml::error_response(&LawsError::InvalidRequest(
                "Invalid UTF-8 in request body".into(),
            ));
        }
    };

    // Extract all <Change> blocks.
    let changes = extract_all_xml_tags(&xml, "Change");

    for change_xml in &changes {
        let action = match extract_xml_tag(change_xml, "Action") {
            Some(a) => a.to_uppercase(),
            None => continue,
        };

        let rrs_xml = match extract_xml_tag(change_xml, "ResourceRecordSet") {
            Some(r) => r,
            None => continue,
        };

        let name = match extract_xml_tag(&rrs_xml, "Name") {
            Some(n) => n,
            None => continue,
        };

        let type_ = match extract_xml_tag(&rrs_xml, "Type") {
            Some(t) => t,
            None => continue,
        };

        let ttl = extract_xml_tag(&rrs_xml, "TTL")
            .and_then(|t| t.parse::<u32>().ok())
            .unwrap_or(300);

        let records: Vec<String> = extract_all_xml_tags(&rrs_xml, "Value");

        let rrs = ResourceRecordSet {
            name: name.clone(),
            type_: type_.clone(),
            ttl,
            records,
        };

        let mut record_sets = state
            .record_sets
            .entry(zone_id.clone())
            .or_insert_with(Vec::new);

        match action.as_str() {
            "CREATE" => {
                record_sets.push(rrs);
            }
            "UPSERT" => {
                record_sets.retain(|r| !(r.name == name && r.type_ == type_));
                record_sets.push(rrs);
            }
            "DELETE" => {
                record_sets.retain(|r| !(r.name == name && r.type_ == type_));
            }
            _ => {
                return rest_xml::error_response(&LawsError::InvalidRequest(format!(
                    "Invalid action: {action}"
                )));
            }
        }

        // Update record set count on the zone.
        if let Some(mut zone) = state.zones.get_mut(&zone_id) {
            zone.record_set_count = record_sets.len() as u32;
        }
    }

    let change_id = uuid::Uuid::new_v4();
    let submitted_at = Utc::now().to_rfc3339();

    let response_body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ChangeResourceRecordSetsResponse xmlns="https://route53.amazonaws.com/doc/2013-04-01/">
  <ChangeInfo>
    <Id>/change/{change_id}</Id>
    <Status>INSYNC</Status>
    <SubmittedAt>{submitted_at}</SubmittedAt>
  </ChangeInfo>
</ChangeResourceRecordSetsResponse>"#,
    );

    xml_response(StatusCode::OK, response_body)
}

async fn list_resource_record_sets(
    State(state): State<Arc<Route53State>>,
    Path(zone_id): Path<String>,
) -> Response {
    if !state.zones.contains_key(&zone_id) {
        return rest_xml::error_response(&LawsError::NotFound(format!(
            "HostedZone not found: {zone_id}"
        )));
    }

    let records_xml = match state.record_sets.get(&zone_id) {
        Some(records) => records
            .iter()
            .map(record_set_xml)
            .collect::<Vec<_>>()
            .join("\n    "),
        None => String::new(),
    };

    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListResourceRecordSetsResponse xmlns="https://route53.amazonaws.com/doc/2013-04-01/">
  <ResourceRecordSets>
    {records_xml}
  </ResourceRecordSets>
  <IsTruncated>false</IsTruncated>
  <MaxItems>100</MaxItems>
</ListResourceRecordSetsResponse>"#,
    );

    xml_response(StatusCode::OK, body)
}
