use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::Json;
use dashmap::DashMap;
use serde_json::{json, Value};

use crate::error::LawsError;
use crate::protocol::rest_json;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Image {
    pub arn: String,
    pub name: String,
    pub version: String,
    pub platform: String,
    pub state: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Component {
    pub arn: String,
    pub name: String,
    pub version: String,
    pub platform: String,
    pub component_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ImagePipeline {
    pub arn: String,
    pub name: String,
    pub image_recipe_arn: String,
    pub status: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ImageBuilderState {
    pub images: DashMap<String, Image>,
    pub components: DashMap<String, Component>,
    pub pipelines: DashMap<String, ImagePipeline>,
}

impl Default for ImageBuilderState {
    fn default() -> Self {
        Self {
            images: DashMap::new(),
            components: DashMap::new(),
            pipelines: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<ImageBuilderState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/images",
            axum::routing::post(create_image).get(list_images),
        )
        .route(
            "/images/{arn}",
            axum::routing::get(get_image).delete(delete_image),
        )
        .route(
            "/components",
            axum::routing::post(create_component).get(list_components),
        )
        .route(
            "/imagePipelines",
            axum::routing::post(create_image_pipeline),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_image(
    State(state): State<Arc<ImageBuilderState>>,
    Json(payload): Json<Value>,
) -> Response {
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();

    let version = payload
        .get("semanticVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_owned();

    let platform = payload
        .get("platform")
        .and_then(|v| v.as_str())
        .unwrap_or("Linux")
        .to_owned();

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:imagebuilder:{REGION}:{ACCOUNT_ID}:image/{name}/{version}/{id}");
    let now = chrono::Utc::now().to_rfc3339();

    let image = Image {
        arn: arn.clone(),
        name: name.clone(),
        version: version.clone(),
        platform: platform.clone(),
        state: "BUILDING".into(),
        created_at: now.clone(),
    };

    state.images.insert(arn.clone(), image);

    rest_json::created(json!({
        "requestId": id,
        "imageBuildVersionArn": arn
    }))
}

async fn get_image(
    State(state): State<Arc<ImageBuilderState>>,
    Path(arn): Path<String>,
) -> Response {
    match state.images.get(&arn) {
        Some(img) => rest_json::ok(json!({
            "image": {
                "arn": img.arn,
                "name": img.name,
                "version": img.version,
                "platform": img.platform,
                "state": { "status": img.state },
                "dateCreated": img.created_at
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!("Image not found: {arn}"))),
    }
}

async fn list_images(State(state): State<Arc<ImageBuilderState>>) -> Response {
    let images: Vec<Value> = state
        .images
        .iter()
        .map(|e| {
            let img = e.value();
            json!({
                "arn": img.arn,
                "name": img.name,
                "version": img.version,
                "platform": img.platform,
                "state": { "status": img.state },
                "dateCreated": img.created_at
            })
        })
        .collect();

    rest_json::ok(json!({
        "imageVersionList": images
    }))
}

async fn delete_image(
    State(state): State<Arc<ImageBuilderState>>,
    Path(arn): Path<String>,
) -> Response {
    match state.images.remove(&arn) {
        Some(_) => rest_json::ok(json!({
            "imageBuildVersionArn": arn
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!("Image not found: {arn}"))),
    }
}

async fn create_component(
    State(state): State<Arc<ImageBuilderState>>,
    Json(payload): Json<Value>,
) -> Response {
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();

    let version = payload
        .get("semanticVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_owned();

    let platform = payload
        .get("platform")
        .and_then(|v| v.as_str())
        .unwrap_or("Linux")
        .to_owned();

    let component_type = payload
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("BUILD")
        .to_owned();

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:imagebuilder:{REGION}:{ACCOUNT_ID}:component/{name}/{version}/{id}");
    let now = chrono::Utc::now().to_rfc3339();

    let component = Component {
        arn: arn.clone(),
        name: name.clone(),
        version,
        platform,
        component_type,
        created_at: now,
    };

    state.components.insert(arn.clone(), component);

    rest_json::created(json!({
        "requestId": id,
        "componentBuildVersionArn": arn
    }))
}

async fn list_components(State(state): State<Arc<ImageBuilderState>>) -> Response {
    let components: Vec<Value> = state
        .components
        .iter()
        .map(|e| {
            let c = e.value();
            json!({
                "arn": c.arn,
                "name": c.name,
                "version": c.version,
                "platform": c.platform,
                "type": c.component_type,
                "dateCreated": c.created_at
            })
        })
        .collect();

    rest_json::ok(json!({
        "componentVersionList": components
    }))
}

async fn create_image_pipeline(
    State(state): State<Arc<ImageBuilderState>>,
    Json(payload): Json<Value>,
) -> Response {
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();

    let image_recipe_arn = payload
        .get("imageRecipeArn")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:imagebuilder:{REGION}:{ACCOUNT_ID}:image-pipeline/{name}");
    let now = chrono::Utc::now().to_rfc3339();

    let pipeline = ImagePipeline {
        arn: arn.clone(),
        name,
        image_recipe_arn,
        status: "ENABLED".into(),
        created_at: now,
    };

    state.pipelines.insert(arn.clone(), pipeline);

    rest_json::created(json!({
        "requestId": id,
        "imagePipelineArn": arn
    }))
}
