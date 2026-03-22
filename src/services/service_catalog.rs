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
pub struct Portfolio {
    pub id: String,
    pub arn: String,
    pub display_name: String,
    pub description: String,
    pub provider_name: String,
    pub created_time: String,
}

#[derive(Debug, Clone)]
pub struct Product {
    pub id: String,
    pub arn: String,
    pub name: String,
    pub owner: String,
    pub product_type: String,
    pub created_time: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ServiceCatalogState {
    pub portfolios: DashMap<String, Portfolio>,
    pub products: DashMap<String, Product>,
}

impl Default for ServiceCatalogState {
    fn default() -> Self {
        Self {
            portfolios: DashMap::new(),
            products: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &ServiceCatalogState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("AWS242ServiceCatalogService.")
        .unwrap_or(target);

    let result = match action {
        "CreatePortfolio" => create_portfolio(state, payload),
        "DeletePortfolio" => delete_portfolio(state, payload),
        "ListPortfolios" => list_portfolios(state),
        "CreateProduct" => create_product(state, payload),
        "DeleteProduct" => delete_product(state, payload),
        "SearchProducts" => search_products(state),
        "AssociateProductWithPortfolio" => associate_product_with_portfolio(state, payload),
        other => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            other
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

fn create_portfolio(
    state: &ServiceCatalogState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let display_name = payload["DisplayName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DisplayName".into()))?
        .to_string();

    let description = payload["Description"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let provider_name = payload["ProviderName"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let id = format!("port-{}", &uuid::Uuid::new_v4().to_string()[..12]);
    let arn = format!(
        "arn:aws:catalog:{REGION}:{ACCOUNT_ID}:portfolio/{id}"
    );
    let now = chrono::Utc::now().to_rfc3339();

    let portfolio = Portfolio {
        id: id.clone(),
        arn: arn.clone(),
        display_name: display_name.clone(),
        description: description.clone(),
        provider_name: provider_name.clone(),
        created_time: now.clone(),
    };

    state.portfolios.insert(id.clone(), portfolio);

    Ok(json_response(json!({
        "PortfolioDetail": {
            "Id": id,
            "ARN": arn,
            "DisplayName": display_name,
            "Description": description,
            "ProviderName": provider_name,
            "CreatedTime": now,
        }
    })))
}

fn delete_portfolio(
    state: &ServiceCatalogState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let id = payload["Id"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Id".into()))?;

    state
        .portfolios
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("Portfolio '{}' not found", id)))?;

    Ok(json_response(json!({})))
}

fn list_portfolios(
    state: &ServiceCatalogState,
) -> Result<Response, LawsError> {
    let portfolios: Vec<Value> = state
        .portfolios
        .iter()
        .map(|entry| {
            let p = entry.value();
            json!({
                "Id": p.id,
                "ARN": p.arn,
                "DisplayName": p.display_name,
                "Description": p.description,
                "ProviderName": p.provider_name,
                "CreatedTime": p.created_time,
            })
        })
        .collect();

    Ok(json_response(json!({
        "PortfolioDetails": portfolios
    })))
}

fn create_product(
    state: &ServiceCatalogState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
        .to_string();

    let owner = payload["Owner"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let product_type = payload["ProductType"]
        .as_str()
        .unwrap_or("CLOUD_FORMATION_TEMPLATE")
        .to_string();

    let id = format!("prod-{}", &uuid::Uuid::new_v4().to_string()[..12]);
    let arn = format!(
        "arn:aws:catalog:{REGION}:{ACCOUNT_ID}:product/{id}"
    );
    let now = chrono::Utc::now().to_rfc3339();

    let product = Product {
        id: id.clone(),
        arn: arn.clone(),
        name: name.clone(),
        owner: owner.clone(),
        product_type: product_type.clone(),
        created_time: now.clone(),
    };

    state.products.insert(id.clone(), product);

    Ok(json_response(json!({
        "ProductViewDetail": {
            "ProductViewSummary": {
                "Id": id,
                "Name": name,
                "Owner": owner,
                "Type": product_type,
            },
            "ProductARN": arn,
            "CreatedTime": now,
        }
    })))
}

fn delete_product(
    state: &ServiceCatalogState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let id = payload["Id"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Id".into()))?;

    state
        .products
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("Product '{}' not found", id)))?;

    Ok(json_response(json!({})))
}

fn search_products(
    state: &ServiceCatalogState,
) -> Result<Response, LawsError> {
    let products: Vec<Value> = state
        .products
        .iter()
        .map(|entry| {
            let p = entry.value();
            json!({
                "ProductViewSummary": {
                    "Id": p.id,
                    "Name": p.name,
                    "Owner": p.owner,
                    "Type": p.product_type,
                    "ProductId": p.id,
                }
            })
        })
        .collect();

    Ok(json_response(json!({
        "ProductViewSummaries": products
    })))
}

fn associate_product_with_portfolio(
    _state: &ServiceCatalogState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let _product_id = payload["ProductId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ProductId".into()))?;

    let _portfolio_id = payload["PortfolioId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing PortfolioId".into()))?;

    Ok(json_response(json!({})))
}
