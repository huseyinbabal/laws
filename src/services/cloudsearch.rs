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
pub struct Domain {
    pub domain_name: String,
    pub domain_id: String,
    pub arn: String,
    pub search_endpoint: String,
    pub doc_endpoint: String,
    pub created: bool,
    pub deleted: bool,
    pub processing: bool,
    pub requires_index_documents: bool,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CloudSearchState {
    pub domains: DashMap<String, Domain>,
}

impl Default for CloudSearchState {
    fn default() -> Self {
        Self {
            domains: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub fn handle_request(
    state: &CloudSearchState,
    headers: &HeaderMap,
    body: &Bytes,
    uri: &Uri,
) -> Response {
    let req = match parse_query_request(uri, headers, body) {
        Ok(r) => r,
        Err(e) => return xml_error_response(&e),
    };

    let result = match req.action.as_str() {
        "CreateDomain" => create_domain(state, &req.params),
        "DeleteDomain" => delete_domain(state, &req.params),
        "DescribeDomains" => describe_domains(state, &req.params),
        "ListDomainNames" => list_domain_names(state),
        "IndexDocuments" => index_documents(state, &req.params),
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

fn create_domain(
    state: &CloudSearchState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let domain_name = params
        .get("DomainName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing DomainName".into()))?
        .clone();

    if state.domains.contains_key(&domain_name) {
        return Err(LawsError::AlreadyExists(format!(
            "Domain already exists: {domain_name}"
        )));
    }

    let domain_id = uuid::Uuid::new_v4().to_string()[..13].to_string();
    let arn = format!("arn:aws:cloudsearch:{REGION}:{ACCOUNT_ID}:domain/{domain_name}");

    let domain = Domain {
        domain_name: domain_name.clone(),
        domain_id: domain_id.clone(),
        arn: arn.clone(),
        search_endpoint: format!(
            "search-{domain_name}-{domain_id}.{REGION}.cloudsearch.amazonaws.com"
        ),
        doc_endpoint: format!("doc-{domain_name}-{domain_id}.{REGION}.cloudsearch.amazonaws.com"),
        created: true,
        deleted: false,
        processing: false,
        requires_index_documents: false,
    };

    state.domains.insert(domain_name.clone(), domain);

    let inner = format!(
        r#"<CreateDomainResult>
      <DomainStatus>
        <DomainName>{domain_name}</DomainName>
        <DomainId>{domain_id}</DomainId>
        <ARN>{arn}</ARN>
        <Created>true</Created>
        <Deleted>false</Deleted>
        <Processing>false</Processing>
        <RequiresIndexDocuments>false</RequiresIndexDocuments>
      </DomainStatus>
    </CreateDomainResult>"#
    );

    Ok(xml_response("CreateDomain", &inner))
}

fn delete_domain(
    state: &CloudSearchState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let domain_name = params
        .get("DomainName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing DomainName".into()))?;

    let (_, domain) = state
        .domains
        .remove(domain_name)
        .ok_or_else(|| LawsError::NotFound(format!("Domain not found: {domain_name}")))?;

    let inner = format!(
        r#"<DeleteDomainResult>
      <DomainStatus>
        <DomainName>{}</DomainName>
        <DomainId>{}</DomainId>
        <ARN>{}</ARN>
        <Created>true</Created>
        <Deleted>true</Deleted>
        <Processing>false</Processing>
      </DomainStatus>
    </DeleteDomainResult>"#,
        domain.domain_name, domain.domain_id, domain.arn
    );

    Ok(xml_response("DeleteDomain", &inner))
}

fn describe_domains(
    state: &CloudSearchState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    // Collect DomainNames.member.N params
    let mut requested: Vec<String> = Vec::new();
    for i in 1..=10 {
        if let Some(name) = params.get(&format!("DomainNames.member.{i}")) {
            requested.push(name.clone());
        }
    }

    let mut members = String::new();
    for entry in state.domains.iter() {
        let d = entry.value();
        if !requested.is_empty() && !requested.contains(&d.domain_name) {
            continue;
        }
        members.push_str(&format!(
            r#"<member>
          <DomainName>{}</DomainName>
          <DomainId>{}</DomainId>
          <ARN>{}</ARN>
          <Created>{}</Created>
          <Deleted>{}</Deleted>
          <Processing>{}</Processing>
          <RequiresIndexDocuments>{}</RequiresIndexDocuments>
          <SearchService><Endpoint>{}</Endpoint></SearchService>
          <DocService><Endpoint>{}</Endpoint></DocService>
        </member>"#,
            d.domain_name,
            d.domain_id,
            d.arn,
            d.created,
            d.deleted,
            d.processing,
            d.requires_index_documents,
            d.search_endpoint,
            d.doc_endpoint,
        ));
    }

    let inner = format!(
        r#"<DescribeDomainsResult>
      <DomainStatusList>{members}</DomainStatusList>
    </DescribeDomainsResult>"#
    );

    Ok(xml_response("DescribeDomains", &inner))
}

fn list_domain_names(state: &CloudSearchState) -> Result<Response, LawsError> {
    let mut entries = String::new();
    for entry in state.domains.iter() {
        let d = entry.value();
        entries.push_str(&format!(
            "<entry><key>{}</key><value>{}</value></entry>",
            d.domain_name,
            chrono::Utc::now().to_rfc3339(),
        ));
    }

    let inner = format!(
        r#"<ListDomainNamesResult>
      <DomainNames>{entries}</DomainNames>
    </ListDomainNamesResult>"#
    );

    Ok(xml_response("ListDomainNames", &inner))
}

fn index_documents(
    state: &CloudSearchState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let domain_name = params
        .get("DomainName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing DomainName".into()))?;

    let mut domain = state
        .domains
        .get_mut(domain_name)
        .ok_or_else(|| LawsError::NotFound(format!("Domain not found: {domain_name}")))?;

    domain.processing = true;
    domain.requires_index_documents = false;

    let inner = r#"<IndexDocumentsResult>
      <FieldNames>
        <member>title</member>
        <member>content</member>
      </FieldNames>
    </IndexDocumentsResult>"#;

    Ok(xml_response("IndexDocuments", inner))
}
