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
pub struct GlueDatabase {
    pub name: String,
    pub catalog_id: String,
    pub create_time: String,
}

#[derive(Debug, Clone)]
pub struct GlueTable {
    pub name: String,
    pub database_name: String,
    pub storage_descriptor: Value,
    pub create_time: String,
}

#[derive(Debug, Clone)]
pub struct GlueCrawler {
    pub name: String,
    pub role: String,
    pub database_name: String,
    pub targets: Value,
    pub state: String,
    pub arn: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct GlueState {
    pub databases: DashMap<String, GlueDatabase>,
    pub tables: DashMap<String, GlueTable>,
    pub crawlers: DashMap<String, GlueCrawler>,
}

impl Default for GlueState {
    fn default() -> Self {
        Self {
            databases: DashMap::new(),
            tables: DashMap::new(),
            crawlers: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &GlueState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target.strip_prefix("AWSGlue.").unwrap_or(target);

    let result = match action {
        "CreateDatabase" => create_database(state, payload).await,
        "DeleteDatabase" => delete_database(state, payload).await,
        "GetDatabase" => get_database(state, payload).await,
        "GetDatabases" => get_databases(state).await,
        "CreateTable" => create_table(state, payload).await,
        "DeleteTable" => delete_table(state, payload).await,
        "GetTable" => get_table(state, payload).await,
        "GetTables" => get_tables(state, payload).await,
        "CreateCrawler" => create_crawler(state, payload).await,
        "StartCrawler" => start_crawler(state, payload).await,
        "GetCrawler" => get_crawler(state, payload).await,
        "GetCrawlers" => get_crawlers(state).await,
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

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

async fn create_database(
    state: &GlueState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let db_input = &payload["DatabaseInput"];
    let name = db_input["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DatabaseInput.Name is required".to_string()))?
        .to_string();

    if state.databases.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Database already exists: {}",
            name
        )));
    }

    let now = chrono::Utc::now().to_rfc3339();

    let db = GlueDatabase {
        name: name.clone(),
        catalog_id: ACCOUNT_ID.to_string(),
        create_time: now,
    };

    state.databases.insert(name, db);

    Ok(json_response(json!({})))
}

async fn delete_database(
    state: &GlueState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;

    state
        .databases
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Database not found: {}", name)))?;

    // Remove associated tables
    state.tables.retain(|_, t| t.database_name != name);

    Ok(json_response(json!({})))
}

async fn get_database(
    state: &GlueState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;

    let db = state
        .databases
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("Database not found: {}", name)))?;

    Ok(json_response(json!({
        "Database": {
            "Name": db.name,
            "CatalogId": db.catalog_id,
            "CreateTime": db.create_time,
        }
    })))
}

async fn get_databases(state: &GlueState) -> Result<Response, LawsError> {
    let databases: Vec<Value> = state
        .databases
        .iter()
        .map(|entry| {
            let db = entry.value();
            json!({
                "Name": db.name,
                "CatalogId": db.catalog_id,
                "CreateTime": db.create_time,
            })
        })
        .collect();

    Ok(json_response(json!({
        "DatabaseList": databases,
    })))
}

async fn create_table(
    state: &GlueState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let database_name = payload["DatabaseName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DatabaseName is required".to_string()))?
        .to_string();

    if !state.databases.contains_key(&database_name) {
        return Err(LawsError::NotFound(format!(
            "Database not found: {}",
            database_name
        )));
    }

    let table_input = &payload["TableInput"];
    let name = table_input["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("TableInput.Name is required".to_string()))?
        .to_string();

    let key = format!("{}:{}", database_name, name);
    if state.tables.contains_key(&key) {
        return Err(LawsError::AlreadyExists(format!(
            "Table already exists: {}",
            name
        )));
    }

    let storage_descriptor = table_input["StorageDescriptor"].clone();
    let now = chrono::Utc::now().to_rfc3339();

    let table = GlueTable {
        name: name.clone(),
        database_name: database_name.clone(),
        storage_descriptor,
        create_time: now,
    };

    state.tables.insert(key, table);

    Ok(json_response(json!({})))
}

async fn delete_table(
    state: &GlueState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let database_name = payload["DatabaseName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DatabaseName is required".to_string()))?;
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;

    let key = format!("{}:{}", database_name, name);
    state
        .tables
        .remove(&key)
        .ok_or_else(|| LawsError::NotFound(format!("Table not found: {}", name)))?;

    Ok(json_response(json!({})))
}

async fn get_table(state: &GlueState, payload: &serde_json::Value) -> Result<Response, LawsError> {
    let database_name = payload["DatabaseName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DatabaseName is required".to_string()))?;
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;

    let key = format!("{}:{}", database_name, name);
    let table = state
        .tables
        .get(&key)
        .ok_or_else(|| LawsError::NotFound(format!("Table not found: {}", name)))?;

    Ok(json_response(json!({
        "Table": {
            "Name": table.name,
            "DatabaseName": table.database_name,
            "StorageDescriptor": table.storage_descriptor,
            "CreateTime": table.create_time,
        }
    })))
}

async fn get_tables(state: &GlueState, payload: &serde_json::Value) -> Result<Response, LawsError> {
    let database_name = payload["DatabaseName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DatabaseName is required".to_string()))?;

    let tables: Vec<Value> = state
        .tables
        .iter()
        .filter(|entry| entry.value().database_name == database_name)
        .map(|entry| {
            let t = entry.value();
            json!({
                "Name": t.name,
                "DatabaseName": t.database_name,
                "StorageDescriptor": t.storage_descriptor,
                "CreateTime": t.create_time,
            })
        })
        .collect();

    Ok(json_response(json!({
        "TableList": tables,
    })))
}

async fn create_crawler(
    state: &GlueState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    if state.crawlers.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Crawler already exists: {}",
            name
        )));
    }

    let role = payload["Role"].as_str().unwrap_or("").to_string();
    let database_name = payload["DatabaseName"].as_str().unwrap_or("").to_string();
    let targets = payload["Targets"].clone();
    let arn = format!("arn:aws:glue:{}:{}:crawler/{}", REGION, ACCOUNT_ID, name);

    let crawler = GlueCrawler {
        name: name.clone(),
        role,
        database_name,
        targets,
        state: "READY".to_string(),
        arn,
    };

    state.crawlers.insert(name, crawler);

    Ok(json_response(json!({})))
}

async fn start_crawler(
    state: &GlueState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;

    let mut crawler = state
        .crawlers
        .get_mut(name)
        .ok_or_else(|| LawsError::NotFound(format!("Crawler not found: {}", name)))?;

    crawler.state = "RUNNING".to_string();

    Ok(json_response(json!({})))
}

async fn get_crawler(
    state: &GlueState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;

    let crawler = state
        .crawlers
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("Crawler not found: {}", name)))?;

    Ok(json_response(json!({
        "Crawler": {
            "Name": crawler.name,
            "Role": crawler.role,
            "DatabaseName": crawler.database_name,
            "Targets": crawler.targets,
            "State": crawler.state,
            "CrawlerArn": crawler.arn,
        }
    })))
}

async fn get_crawlers(state: &GlueState) -> Result<Response, LawsError> {
    let crawlers: Vec<Value> = state
        .crawlers
        .iter()
        .map(|entry| {
            let c = entry.value();
            json!({
                "Name": c.name,
                "Role": c.role,
                "DatabaseName": c.database_name,
                "Targets": c.targets,
                "State": c.state,
                "CrawlerArn": c.arn,
            })
        })
        .collect();

    Ok(json_response(json!({
        "Crawlers": crawlers,
    })))
}
