use axum::response::{IntoResponse, Response};
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

#[allow(dead_code)]
const ACCOUNT_ID: &str = "000000000000";
#[allow(dead_code)]
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct PricingState;

impl Default for PricingState {
    fn default() -> Self {
        Self
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(_state: &PricingState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("AWSPriceListService.")
        .unwrap_or(target);

    let result = match action {
        "DescribeServices" => describe_services(payload),
        "GetAttributeValues" => get_attribute_values(payload),
        "GetProducts" => get_products(payload),
        "ListPriceLists" => list_price_lists(payload),
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

fn describe_services(payload: &Value) -> Result<Response, LawsError> {
    let service_code = payload["ServiceCode"].as_str().unwrap_or("");

    let services = if service_code.is_empty() {
        vec![
            json!({
                "ServiceCode": "AmazonEC2",
                "AttributeNames": ["instanceType", "location", "operatingSystem", "tenancy"]
            }),
            json!({
                "ServiceCode": "AmazonS3",
                "AttributeNames": ["storageClass", "location", "volumeType"]
            }),
            json!({
                "ServiceCode": "AmazonRDS",
                "AttributeNames": ["instanceType", "databaseEngine", "location"]
            }),
            json!({
                "ServiceCode": "AWSLambda",
                "AttributeNames": ["memorySize", "location"]
            }),
        ]
    } else {
        vec![json!({
            "ServiceCode": service_code,
            "AttributeNames": ["instanceType", "location", "operatingSystem"]
        })]
    };

    Ok(json_response(json!({
        "Services": services,
        "FormatVersion": "aws_v1",
    })))
}

fn get_attribute_values(payload: &Value) -> Result<Response, LawsError> {
    let _service_code = payload["ServiceCode"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ServiceCode".into()))?;

    let attribute_name = payload["AttributeName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing AttributeName".into()))?;

    let values = match attribute_name {
        "instanceType" => vec![
            json!({"Value": "t2.micro"}),
            json!({"Value": "t2.small"}),
            json!({"Value": "t3.medium"}),
            json!({"Value": "m5.large"}),
            json!({"Value": "m5.xlarge"}),
        ],
        "location" => vec![
            json!({"Value": "US East (N. Virginia)"}),
            json!({"Value": "US West (Oregon)"}),
            json!({"Value": "EU (Ireland)"}),
        ],
        "operatingSystem" => vec![
            json!({"Value": "Linux"}),
            json!({"Value": "Windows"}),
            json!({"Value": "RHEL"}),
        ],
        _ => vec![json!({"Value": "default"})],
    };

    Ok(json_response(json!({
        "AttributeValues": values,
    })))
}

fn get_products(payload: &Value) -> Result<Response, LawsError> {
    let service_code = payload["ServiceCode"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ServiceCode".into()))?;

    let product = json!({
        "product": {
            "productFamily": "Compute Instance",
            "attributes": {
                "servicecode": service_code,
                "location": "US East (N. Virginia)",
                "instanceType": "t2.micro",
                "operatingSystem": "Linux",
            },
            "sku": "MOCK000000SKU"
        },
        "serviceCode": service_code,
        "terms": {
            "OnDemand": {
                "MOCK000000SKU.JRTCKXETXF": {
                    "priceDimensions": {
                        "MOCK000000SKU.JRTCKXETXF.6YS6EN2CT7": {
                            "unit": "Hrs",
                            "pricePerUnit": {
                                "USD": "0.0116"
                            },
                            "description": "Mock pricing"
                        }
                    }
                }
            }
        }
    });

    Ok(json_response(json!({
        "PriceList": [serde_json::to_string(&product).unwrap()],
        "FormatVersion": "aws_v1",
    })))
}

fn list_price_lists(_payload: &Value) -> Result<Response, LawsError> {
    let price_lists = vec![
        json!({
            "PriceListArn": "arn:aws:pricing:::price-list/000000000001",
            "RegionCode": "us-east-1",
            "CurrencyCode": "USD",
            "FileFormats": ["json", "csv"],
        }),
        json!({
            "PriceListArn": "arn:aws:pricing:::price-list/000000000002",
            "RegionCode": "us-west-2",
            "CurrencyCode": "USD",
            "FileFormats": ["json", "csv"],
        }),
    ];

    Ok(json_response(json!({
        "PriceLists": price_lists,
    })))
}
