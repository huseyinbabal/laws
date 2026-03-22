use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::post;
use chrono::Utc;
use dashmap::DashMap;
use serde_json::{json, Map, Value};

use crate::error::LawsError;
use crate::protocol::json::{json_error_response, json_response, parse_target};

// ---------------------------------------------------------------------------
// State & data model
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct DynamoTable {
    pub table_name: String,
    pub key_schema: Vec<KeySchemaElement>,
    pub attribute_definitions: Vec<AttributeDefinition>,
    pub items: HashMap<String, Value>,
    pub creation_timestamp: f64,
    pub status: String,
}

#[derive(Clone)]
pub struct KeySchemaElement {
    pub attribute_name: String,
    pub key_type: String,
}

#[derive(Clone)]
pub struct AttributeDefinition {
    pub attribute_name: String,
    pub attribute_type: String,
}

pub struct DynamoDbState {
    pub tables: DashMap<String, DynamoTable>,
}

impl Default for DynamoDbState {
    fn default() -> Self {
        Self {
            tables: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<DynamoDbState>) -> axum::Router {
    axum::Router::new()
        .route("/", post(handle))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Main dispatch handler
// ---------------------------------------------------------------------------

pub fn handle_request(state: &DynamoDbState, target: &str, body: &[u8]) -> Response {
    let action = target.split('.').last().unwrap_or("");

    let body: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(e) => {
            return json_error_response(&LawsError::InvalidRequest(format!(
                "invalid JSON body: {e}"
            )))
        }
    };

    let result = match action {
        "CreateTable" => create_table(state, &body),
        "DeleteTable" => delete_table(state, &body),
        "DescribeTable" => describe_table(state, &body),
        "ListTables" => list_tables(state),
        "PutItem" => put_item(state, &body),
        "GetItem" => get_item(state, &body),
        "DeleteItem" => delete_item(state, &body),
        "Query" => query_items(state, &body),
        "Scan" => scan_items(state, &body),
        other => Err(LawsError::InvalidRequest(format!(
            "unknown action: {other}"
        ))),
    };

    match result {
        Ok(v) => json_response(v),
        Err(e) => json_error_response(&e),
    }
}

async fn handle(
    State(state): State<Arc<DynamoDbState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target = match parse_target(&headers) {
        Ok(t) => t,
        Err(e) => return json_error_response(&e),
    };

    handle_request(&state, &target.action, &body)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_str<'a>(body: &'a Value, field: &str) -> Result<&'a str, LawsError> {
    body.get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest(format!("missing required field: {field}")))
}

fn parse_key_schema(body: &Value) -> Result<Vec<KeySchemaElement>, LawsError> {
    let arr = body
        .get("KeySchema")
        .and_then(|v| v.as_array())
        .ok_or_else(|| LawsError::InvalidRequest("missing KeySchema".into()))?;

    arr.iter()
        .map(|elem| {
            let name = elem
                .get("AttributeName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    LawsError::InvalidRequest("KeySchema element missing AttributeName".into())
                })?;
            let key_type = elem
                .get("KeyType")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    LawsError::InvalidRequest("KeySchema element missing KeyType".into())
                })?;
            Ok(KeySchemaElement {
                attribute_name: name.to_owned(),
                key_type: key_type.to_owned(),
            })
        })
        .collect()
}

fn parse_attribute_definitions(body: &Value) -> Result<Vec<AttributeDefinition>, LawsError> {
    let arr = body
        .get("AttributeDefinitions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| LawsError::InvalidRequest("missing AttributeDefinitions".into()))?;

    arr.iter()
        .map(|elem| {
            let name = elem
                .get("AttributeName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    LawsError::InvalidRequest("AttributeDefinition missing AttributeName".into())
                })?;
            let attr_type = elem
                .get("AttributeType")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    LawsError::InvalidRequest("AttributeDefinition missing AttributeType".into())
                })?;
            Ok(AttributeDefinition {
                attribute_name: name.to_owned(),
                attribute_type: attr_type.to_owned(),
            })
        })
        .collect()
}

fn table_description(table: &DynamoTable) -> Value {
    let key_schema: Vec<Value> = table
        .key_schema
        .iter()
        .map(|k| {
            json!({
                "AttributeName": k.attribute_name,
                "KeyType": k.key_type,
            })
        })
        .collect();

    let attr_defs: Vec<Value> = table
        .attribute_definitions
        .iter()
        .map(|a| {
            json!({
                "AttributeName": a.attribute_name,
                "AttributeType": a.attribute_type,
            })
        })
        .collect();

    json!({
        "TableName": table.table_name,
        "TableStatus": table.status,
        "KeySchema": key_schema,
        "AttributeDefinitions": attr_defs,
        "CreationDateTime": table.creation_timestamp,
        "ItemCount": table.items.len(),
        "TableArn": format!("arn:aws:dynamodb:us-east-1:000000000000:table/{}", table.table_name),
    })
}

/// Build a composite primary key string from the item and the table's key schema.
fn extract_primary_key(
    key_schema: &[KeySchemaElement],
    item: &Map<String, Value>,
) -> Result<String, LawsError> {
    let mut parts: Vec<String> = Vec::new();
    for ks in key_schema {
        let attr_val = item.get(&ks.attribute_name).ok_or_else(|| {
            LawsError::InvalidRequest(format!("item missing key attribute: {}", ks.attribute_name))
        })?;
        parts.push(serde_json::to_string(attr_val).unwrap());
    }
    Ok(parts.join("##"))
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_table(state: &DynamoDbState, body: &Value) -> Result<Value, LawsError> {
    let table_name = require_str(body, "TableName")?.to_owned();
    let key_schema = parse_key_schema(body)?;
    let attribute_definitions = parse_attribute_definitions(body)?;

    if state.tables.contains_key(&table_name) {
        return Err(LawsError::AlreadyExists(format!(
            "table already exists: {table_name}"
        )));
    }

    let table = DynamoTable {
        table_name: table_name.clone(),
        key_schema,
        attribute_definitions,
        items: HashMap::new(),
        creation_timestamp: Utc::now().timestamp() as f64,
        status: "ACTIVE".into(),
    };

    let desc = table_description(&table);
    state.tables.insert(table_name, table);

    Ok(json!({ "TableDescription": desc }))
}

fn delete_table(state: &DynamoDbState, body: &Value) -> Result<Value, LawsError> {
    let table_name = require_str(body, "TableName")?;
    let (_, table) = state
        .tables
        .remove(table_name)
        .ok_or_else(|| LawsError::NotFound(format!("table not found: {table_name}")))?;

    Ok(json!({ "TableDescription": table_description(&table) }))
}

fn describe_table(state: &DynamoDbState, body: &Value) -> Result<Value, LawsError> {
    let table_name = require_str(body, "TableName")?;
    let table = state
        .tables
        .get(table_name)
        .ok_or_else(|| LawsError::NotFound(format!("table not found: {table_name}")))?;

    Ok(json!({ "Table": table_description(&table) }))
}

fn list_tables(state: &DynamoDbState) -> Result<Value, LawsError> {
    let names: Vec<String> = state
        .tables
        .iter()
        .map(|entry| entry.key().clone())
        .collect();
    Ok(json!({ "TableNames": names }))
}

fn put_item(state: &DynamoDbState, body: &Value) -> Result<Value, LawsError> {
    let table_name = require_str(body, "TableName")?;
    let item = body
        .get("Item")
        .and_then(|v| v.as_object())
        .ok_or_else(|| LawsError::InvalidRequest("missing Item".into()))?;

    let mut table = state
        .tables
        .get_mut(table_name)
        .ok_or_else(|| LawsError::NotFound(format!("table not found: {table_name}")))?;

    let pk = extract_primary_key(&table.key_schema, item)?;
    table.items.insert(pk, Value::Object(item.clone()));

    Ok(json!({}))
}

fn get_item(state: &DynamoDbState, body: &Value) -> Result<Value, LawsError> {
    let table_name = require_str(body, "TableName")?;
    let key = body
        .get("Key")
        .and_then(|v| v.as_object())
        .ok_or_else(|| LawsError::InvalidRequest("missing Key".into()))?;

    let table = state
        .tables
        .get(table_name)
        .ok_or_else(|| LawsError::NotFound(format!("table not found: {table_name}")))?;

    let pk = extract_primary_key(&table.key_schema, key)?;
    match table.items.get(&pk) {
        Some(item) => Ok(json!({ "Item": item })),
        None => Ok(json!({})),
    }
}

fn delete_item(state: &DynamoDbState, body: &Value) -> Result<Value, LawsError> {
    let table_name = require_str(body, "TableName")?;
    let key = body
        .get("Key")
        .and_then(|v| v.as_object())
        .ok_or_else(|| LawsError::InvalidRequest("missing Key".into()))?;

    let mut table = state
        .tables
        .get_mut(table_name)
        .ok_or_else(|| LawsError::NotFound(format!("table not found: {table_name}")))?;

    let pk = extract_primary_key(&table.key_schema, key)?;
    table.items.remove(&pk);

    Ok(json!({}))
}

fn query_items(state: &DynamoDbState, body: &Value) -> Result<Value, LawsError> {
    let table_name = require_str(body, "TableName")?;

    let table = state
        .tables
        .get(table_name)
        .ok_or_else(|| LawsError::NotFound(format!("table not found: {table_name}")))?;

    // Find the partition key name (HASH key).
    let partition_key_name = table
        .key_schema
        .iter()
        .find(|k| k.key_type == "HASH")
        .map(|k| k.attribute_name.clone())
        .ok_or_else(|| LawsError::Internal("table missing HASH key".into()))?;

    // Extract the desired partition key value from ExpressionAttributeValues
    // by looking at KeyConditionExpression.  We do a simplified parse:
    // expect something like "pk_name = :val" and grab :val from
    // ExpressionAttributeValues.
    let partition_value = extract_partition_value(body, &partition_key_name)?;

    let matching: Vec<&Value> = table
        .items
        .values()
        .filter(|item| {
            item.get(&partition_key_name)
                .map(|v| v == &partition_value)
                .unwrap_or(false)
        })
        .collect();

    let count = matching.len();
    Ok(json!({
        "Items": matching,
        "Count": count,
        "ScannedCount": count,
    }))
}

fn extract_partition_value(body: &Value, partition_key_name: &str) -> Result<Value, LawsError> {
    // Try ExpressionAttributeValues approach first.
    if let Some(expr) = body.get("KeyConditionExpression").and_then(|v| v.as_str()) {
        if let Some(eav) = body
            .get("ExpressionAttributeValues")
            .and_then(|v| v.as_object())
        {
            // Parse expressions like "pk = :pk" or "pk = :val"
            for part in expr.split(" AND ") {
                let part = part.trim();
                if let Some((left, right)) = part.split_once('=') {
                    let left = left.trim();
                    let right = right.trim();
                    if left == partition_key_name {
                        if let Some(val) = eav.get(right) {
                            return Ok(val.clone());
                        }
                    }
                }
            }
        }
    }

    // Fallback: try to get the value from a simple KeyConditions map (legacy).
    if let Some(kc) = body.get("KeyConditions").and_then(|v| v.as_object()) {
        if let Some(cond) = kc.get(partition_key_name).and_then(|v| v.as_object()) {
            if let Some(vals) = cond.get("AttributeValueList").and_then(|v| v.as_array()) {
                if let Some(first) = vals.first() {
                    return Ok(first.clone());
                }
            }
        }
    }

    Err(LawsError::InvalidRequest(
        "could not determine partition key value from query expression".into(),
    ))
}

fn scan_items(state: &DynamoDbState, body: &Value) -> Result<Value, LawsError> {
    let table_name = require_str(body, "TableName")?;

    let table = state
        .tables
        .get(table_name)
        .ok_or_else(|| LawsError::NotFound(format!("table not found: {table_name}")))?;

    let items: Vec<&Value> = table.items.values().collect();
    let count = items.len();
    Ok(json!({
        "Items": items,
        "Count": count,
        "ScannedCount": count,
    }))
}
