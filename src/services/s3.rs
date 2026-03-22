use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::Router;
use chrono::Utc;
use md5::{Digest, Md5};
use serde::Deserialize;

use crate::error::LawsError;
use crate::storage::mem::MemoryStore;

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct S3Bucket {
    pub name: String,
    pub creation_date: String,
}

#[derive(Clone, Debug)]
pub struct S3Object {
    pub key: String,
    pub body: Vec<u8>,
    pub content_type: String,
    pub etag: String,
    pub last_modified: String,
    pub size: usize,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct S3State {
    pub buckets: MemoryStore<S3Bucket>,
    pub objects: MemoryStore<S3Object>,
}

impl S3State {
    pub fn new() -> Self {
        Self {
            buckets: MemoryStore::new(),
            objects: MemoryStore::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<S3State>) -> Router {
    Router::new()
        .route("/", get(list_buckets))
        .route(
            "/{bucket}",
            put(create_bucket).delete(delete_bucket).get(list_objects),
        )
        .route(
            "/{bucket}/{*key}",
            put(put_object).get(get_object).delete(delete_object),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn object_store_key(bucket: &str, key: &str) -> String {
    format!("{bucket}/{key}")
}

fn compute_etag(data: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(data);
    let hash = hasher.finalize();
    format!("\"{}\"", hex::encode(hash))
}

fn xml_response(status: StatusCode, body: String) -> Response {
    (
        status,
        [("content-type", "application/xml; charset=utf-8")],
        body,
    )
        .into_response()
}

fn xml_error(err: &LawsError) -> Response {
    let code = err.error_code();
    let err_msg = err.to_string();
    let message = quick_xml::escape::escape(&err_msg);
    let request_id = uuid::Uuid::new_v4();
    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
  <Code>{code}</Code>
  <Message>{message}</Message>
  <RequestId>{request_id}</RequestId>
</Error>"#
    );
    let status = match err {
        LawsError::NotFound(_) => StatusCode::NOT_FOUND,
        LawsError::AlreadyExists(_) => StatusCode::CONFLICT,
        LawsError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
        LawsError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    xml_response(status, body)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_bucket(
    State(state): State<Arc<S3State>>,
    Path(bucket): Path<String>,
) -> Response {
    if state.buckets.contains(&bucket) {
        return xml_error(&LawsError::AlreadyExists(format!("Bucket {bucket} already exists")));
    }

    let b = S3Bucket {
        name: bucket.clone(),
        creation_date: Utc::now().to_rfc3339(),
    };
    state.buckets.insert(bucket, b);

    xml_response(
        StatusCode::OK,
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<CreateBucketConfiguration>
  <LocationConstraint>us-east-1</LocationConstraint>
</CreateBucketConfiguration>"#
        ),
    )
}

async fn delete_bucket(
    State(state): State<Arc<S3State>>,
    Path(bucket): Path<String>,
) -> Response {
    if !state.buckets.contains(&bucket) {
        return xml_error(&LawsError::NotFound(format!("Bucket {bucket} not found")));
    }

    // Remove all objects in the bucket
    let prefix = format!("{bucket}/");
    let keys_to_remove: Vec<String> = state
        .objects
        .list()
        .into_iter()
        .filter(|(k, _)| k.starts_with(&prefix))
        .map(|(k, _)| k)
        .collect();
    for key in keys_to_remove {
        state.objects.remove(&key);
    }

    state.buckets.remove(&bucket);
    (StatusCode::NO_CONTENT, "").into_response()
}

async fn list_buckets(State(state): State<Arc<S3State>>) -> Response {
    let buckets = state.buckets.list_values();
    let mut bucket_xml = String::new();
    for b in &buckets {
        let name = quick_xml::escape::escape(&b.name);
        bucket_xml.push_str(&format!(
            "    <Bucket><Name>{name}</Name><CreationDate>{}</CreationDate></Bucket>\n",
            b.creation_date
        ));
    }

    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListAllMyBucketsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Owner>
    <ID>000000000000</ID>
    <DisplayName>laws</DisplayName>
  </Owner>
  <Buckets>
{bucket_xml}  </Buckets>
</ListAllMyBucketsResult>"#
    );

    xml_response(StatusCode::OK, body)
}

#[derive(Deserialize, Default)]
pub struct ListObjectsParams {
    #[serde(rename = "list-type")]
    pub list_type: Option<String>,
    pub prefix: Option<String>,
    pub delimiter: Option<String>,
    #[serde(rename = "max-keys")]
    pub max_keys: Option<usize>,
}

async fn list_objects(
    State(state): State<Arc<S3State>>,
    Path(bucket): Path<String>,
    Query(params): Query<ListObjectsParams>,
) -> Response {
    if !state.buckets.contains(&bucket) {
        return xml_error(&LawsError::NotFound(format!("Bucket {bucket} not found")));
    }

    let store_prefix = format!("{bucket}/");
    let user_prefix = params.prefix.unwrap_or_default();
    let full_prefix = format!("{store_prefix}{user_prefix}");
    let max_keys = params.max_keys.unwrap_or(1000);

    let objects: Vec<S3Object> = state
        .objects
        .list()
        .into_iter()
        .filter(|(k, _)| k.starts_with(&full_prefix))
        .take(max_keys)
        .map(|(_, v)| v)
        .collect();

    let is_v2 = params.list_type.as_deref() == Some("2");

    let mut contents_xml = String::new();
    for obj in &objects {
        let key = quick_xml::escape::escape(&obj.key);
        contents_xml.push_str(&format!(
            r#"    <Contents>
      <Key>{key}</Key>
      <LastModified>{}</LastModified>
      <ETag>{}</ETag>
      <Size>{}</Size>
      <StorageClass>STANDARD</StorageClass>
    </Contents>
"#,
            obj.last_modified, obj.etag, obj.size
        ));
    }

    let escaped_bucket = quick_xml::escape::escape(&bucket);
    let escaped_prefix = quick_xml::escape::escape(&user_prefix);

    let body = if is_v2 {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Name>{escaped_bucket}</Name>
  <Prefix>{escaped_prefix}</Prefix>
  <KeyCount>{}</KeyCount>
  <MaxKeys>{max_keys}</MaxKeys>
  <IsTruncated>false</IsTruncated>
{contents_xml}</ListBucketResult>"#,
            objects.len()
        )
    } else {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Name>{escaped_bucket}</Name>
  <Prefix>{escaped_prefix}</Prefix>
  <MaxKeys>{max_keys}</MaxKeys>
  <IsTruncated>false</IsTruncated>
{contents_xml}</ListBucketResult>"#
        )
    };

    xml_response(StatusCode::OK, body)
}

async fn put_object(
    State(state): State<Arc<S3State>>,
    Path((bucket, key)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !state.buckets.contains(&bucket) {
        return xml_error(&LawsError::NotFound(format!("Bucket {bucket} not found")));
    }

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let data = body.to_vec();
    let etag = compute_etag(&data);
    let size = data.len();

    let obj = S3Object {
        key: key.clone(),
        body: data,
        content_type,
        etag: etag.clone(),
        last_modified: Utc::now().to_rfc3339(),
        size,
    };

    let store_key = object_store_key(&bucket, &key);
    state.objects.insert(store_key, obj);

    (StatusCode::OK, [("ETag", etag)], "").into_response()
}

async fn get_object(
    State(state): State<Arc<S3State>>,
    Path((bucket, key)): Path<(String, String)>,
) -> Response {
    let store_key = object_store_key(&bucket, &key);
    match state.objects.get(&store_key) {
        Some(obj) => (
            StatusCode::OK,
            [
                ("content-type", obj.content_type.as_str()),
                ("etag", obj.etag.as_str()),
                ("last-modified", obj.last_modified.as_str()),
                ("content-length", &obj.size.to_string()),
            ],
            obj.body,
        )
            .into_response(),
        None => xml_error(&LawsError::NotFound(format!(
            "Object {key} not found in bucket {bucket}"
        ))),
    }
}

async fn delete_object(
    State(state): State<Arc<S3State>>,
    Path((bucket, key)): Path<(String, String)>,
) -> Response {
    let store_key = object_store_key(&bucket, &key);
    state.objects.remove(&store_key);
    // S3 returns 204 even if the object didn't exist
    (StatusCode::NO_CONTENT, "").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etag_computation() {
        let etag = compute_etag(b"hello");
        // MD5 of "hello" = 5d41402abc4b2a76b9719d911017c592
        assert_eq!(etag, "\"5d41402abc4b2a76b9719d911017c592\"");
    }
}
