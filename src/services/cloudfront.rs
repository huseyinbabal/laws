use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use dashmap::DashMap;
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;
use crate::protocol::rest_xml;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Distribution {
    pub id: String,
    pub arn: String,
    pub domain_name: String,
    pub status: String,
    pub origins: Vec<String>,
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CloudFrontState {
    pub distributions: DashMap<String, Distribution>,
}

impl Default for CloudFrontState {
    fn default() -> Self {
        Self {
            distributions: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<CloudFrontState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/2020-05-31/distribution",
            post(create_distribution).get(list_distributions),
        )
        .route(
            "/2020-05-31/distribution/{id}",
            get(get_distribution).delete(delete_distribution),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn random_id() -> String {
    use rand::Rng;
    use rand::distributions::Alphanumeric;
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(14)
        .map(|c| char::from(c).to_ascii_uppercase())
        .collect::<String>()
}

fn distribution_to_xml(d: &Distribution) -> String {
    let origins_xml: String = d
        .origins
        .iter()
        .map(|o| format!("      <Origin><DomainName>{o}</DomainName></Origin>\n"))
        .collect();

    format!(
        r#"  <Distribution>
    <Id>{}</Id>
    <ARN>{}</ARN>
    <DomainName>{}</DomainName>
    <Status>{}</Status>
    <Enabled>{}</Enabled>
    <Origins>
{origins_xml}    </Origins>
  </Distribution>"#,
        d.id, d.arn, d.domain_name, d.status, d.enabled
    )
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_distribution(
    State(state): State<Arc<CloudFrontState>>,
    body: Bytes,
) -> Response {
    match do_create_distribution(&state, &body) {
        Ok(resp) => resp,
        Err(e) => rest_xml::error_response(&e),
    }
}

fn do_create_distribution(
    state: &CloudFrontState,
    body: &[u8],
) -> Result<Response, LawsError> {
    let id = random_id();
    let arn = format!("arn:aws:cloudfront::{ACCOUNT_ID}:distribution/{id}");
    let domain_name = format!("{id}.cloudfront.net");

    // Try to parse origins from XML body; fall back to empty
    let body_str = std::str::from_utf8(body).unwrap_or("");
    let mut origins = Vec::new();

    // Simple extraction of DomainName values from the request body
    let mut remaining = body_str;
    while let Some(start) = remaining.find("<DomainName>") {
        let after = &remaining[start + 12..];
        if let Some(end) = after.find("</DomainName>") {
            origins.push(after[..end].to_string());
            remaining = &after[end..];
        } else {
            break;
        }
    }

    let dist = Distribution {
        id: id.clone(),
        arn,
        domain_name,
        status: "Deployed".into(),
        origins,
        enabled: true,
    };

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
{}"#,
        distribution_to_xml(&dist)
    );

    state.distributions.insert(id, dist);

    Ok((
        StatusCode::CREATED,
        [("content-type", "application/xml")],
        xml,
    )
        .into_response())
}

async fn list_distributions(State(state): State<Arc<CloudFrontState>>) -> Response {
    let mut items_xml = String::new();
    for entry in state.distributions.iter() {
        items_xml.push_str(&distribution_to_xml(entry.value()));
        items_xml.push('\n');
    }

    let quantity = state.distributions.len();

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<DistributionList>
  <Quantity>{quantity}</Quantity>
  <Items>
{items_xml}  </Items>
</DistributionList>"#
    );

    (StatusCode::OK, [("content-type", "application/xml")], xml).into_response()
}

async fn get_distribution(
    State(state): State<Arc<CloudFrontState>>,
    Path(id): Path<String>,
) -> Response {
    match state.distributions.get(&id) {
        Some(dist) => {
            let xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
{}"#,
                distribution_to_xml(&dist)
            );
            (StatusCode::OK, [("content-type", "application/xml")], xml).into_response()
        }
        None => rest_xml::error_response(&LawsError::NotFound(format!(
            "distribution not found: {id}"
        ))),
    }
}

async fn delete_distribution(
    State(state): State<Arc<CloudFrontState>>,
    Path(id): Path<String>,
) -> Response {
    match state.distributions.remove(&id) {
        Some(_) => StatusCode::NO_CONTENT.into_response(),
        None => rest_xml::error_response(&LawsError::NotFound(format!(
            "distribution not found: {id}"
        ))),
    }
}
