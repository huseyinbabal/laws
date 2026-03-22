use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, Uri};
use axum::response::Response;
use axum::routing::post;
use axum::Router;

use crate::error::LawsError;
use crate::protocol::query::{parse_query_request, xml_error_response, xml_response};
use crate::storage::mem::MemoryStore;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SnsTopic {
    pub arn: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct SnsSubscription {
    pub arn: String,
    pub topic_arn: String,
    pub protocol: String,
    pub endpoint: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SnsState {
    pub topics: MemoryStore<SnsTopic>,
    pub subscriptions: MemoryStore<SnsSubscription>,
}

impl SnsState {
    pub fn new() -> Self {
        Self {
            topics: MemoryStore::new(),
            subscriptions: MemoryStore::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<SnsState>) -> Router {
    Router::new()
        .route("/", post(handle_sns))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn topic_arn(name: &str) -> String {
    format!("arn:aws:sns:{REGION}:{ACCOUNT_ID}:{name}")
}

fn subscription_arn(topic_name: &str, id: &str) -> String {
    format!("arn:aws:sns:{REGION}:{ACCOUNT_ID}:{topic_name}:{id}")
}

// ---------------------------------------------------------------------------
// Dispatch handler
// ---------------------------------------------------------------------------

pub fn handle_request(
    state: &SnsState,
    headers: &HeaderMap,
    body: &Bytes,
    uri: &Uri,
) -> Response {
    let req = match parse_query_request(uri, headers, body) {
        Ok(r) => r,
        Err(e) => return xml_error_response(&e),
    };

    let result = match req.action.as_str() {
        "CreateTopic" => create_topic(state, &req.params),
        "DeleteTopic" => delete_topic(state, &req.params),
        "ListTopics" => list_topics(state),
        "Subscribe" => subscribe(state, &req.params),
        "Unsubscribe" => unsubscribe(state, &req.params),
        "ListSubscriptions" => list_subscriptions(state),
        "Publish" => publish(&req.params),
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

async fn handle_sns(
    State(state): State<Arc<SnsState>>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_request(&state, &headers, &body, &uri)
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_topic(
    state: &SnsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let name = params
        .get("Name")
        .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?;

    let arn = topic_arn(name);

    // CreateTopic is idempotent in AWS
    if !state.topics.contains(&arn) {
        let topic = SnsTopic {
            arn: arn.clone(),
            name: name.clone(),
        };
        state.topics.insert(arn.clone(), topic);
    }

    let inner = format!("<TopicArn>{}</TopicArn>", quick_xml::escape::escape(&arn));
    Ok(xml_response("CreateTopic", &inner))
}

fn delete_topic(
    state: &SnsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let arn = params
        .get("TopicArn")
        .ok_or_else(|| LawsError::InvalidRequest("Missing TopicArn".into()))?;

    state
        .topics
        .remove(arn)
        .ok_or_else(|| LawsError::NotFound(format!("Topic {arn} not found")))?;

    // Remove associated subscriptions
    let subs_to_remove: Vec<String> = state
        .subscriptions
        .list()
        .into_iter()
        .filter(|(_, sub)| sub.topic_arn == *arn)
        .map(|(key, _)| key)
        .collect();
    for key in subs_to_remove {
        state.subscriptions.remove(&key);
    }

    Ok(xml_response("DeleteTopic", ""))
}

fn list_topics(state: &SnsState) -> Result<Response, LawsError> {
    let topics = state.topics.list_values();
    let mut members_xml = String::new();
    for topic in &topics {
        let arn = quick_xml::escape::escape(&topic.arn);
        members_xml.push_str(&format!(
            "  <member><TopicArn>{arn}</TopicArn></member>\n"
        ));
    }

    let inner = format!("<Topics>\n{members_xml}</Topics>");
    Ok(xml_response("ListTopics", &inner))
}

fn subscribe(
    state: &SnsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let topic_arn = params
        .get("TopicArn")
        .ok_or_else(|| LawsError::InvalidRequest("Missing TopicArn".into()))?;
    let protocol = params
        .get("Protocol")
        .ok_or_else(|| LawsError::InvalidRequest("Missing Protocol".into()))?;
    let endpoint = params
        .get("Endpoint")
        .ok_or_else(|| LawsError::InvalidRequest("Missing Endpoint".into()))?;

    if !state.topics.contains(topic_arn) {
        return Err(LawsError::NotFound(format!(
            "Topic {topic_arn} not found"
        )));
    }

    // Derive topic name from ARN for subscription ARN construction
    let topic_name = topic_arn.rsplit(':').next().unwrap_or("unknown");
    let sub_id = uuid::Uuid::new_v4().to_string();
    let sub_arn = subscription_arn(topic_name, &sub_id);

    let subscription = SnsSubscription {
        arn: sub_arn.clone(),
        topic_arn: topic_arn.clone(),
        protocol: protocol.clone(),
        endpoint: endpoint.clone(),
    };
    state.subscriptions.insert(sub_arn.clone(), subscription);

    let inner = format!(
        "<SubscriptionArn>{}</SubscriptionArn>",
        quick_xml::escape::escape(&sub_arn)
    );
    Ok(xml_response("Subscribe", &inner))
}

fn unsubscribe(
    state: &SnsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let sub_arn = params
        .get("SubscriptionArn")
        .ok_or_else(|| LawsError::InvalidRequest("Missing SubscriptionArn".into()))?;

    state
        .subscriptions
        .remove(sub_arn)
        .ok_or_else(|| LawsError::NotFound(format!("Subscription {sub_arn} not found")))?;

    Ok(xml_response("Unsubscribe", ""))
}

fn list_subscriptions(state: &SnsState) -> Result<Response, LawsError> {
    let subs = state.subscriptions.list_values();
    let mut members_xml = String::new();
    for sub in &subs {
        let arn = quick_xml::escape::escape(&sub.arn);
        let topic_arn = quick_xml::escape::escape(&sub.topic_arn);
        let protocol = quick_xml::escape::escape(&sub.protocol);
        let endpoint = quick_xml::escape::escape(&sub.endpoint);
        members_xml.push_str(&format!(
            r#"  <member>
    <SubscriptionArn>{arn}</SubscriptionArn>
    <TopicArn>{topic_arn}</TopicArn>
    <Protocol>{protocol}</Protocol>
    <Endpoint>{endpoint}</Endpoint>
    <Owner>{ACCOUNT_ID}</Owner>
  </member>
"#
        ));
    }

    let inner = format!("<Subscriptions>\n{members_xml}</Subscriptions>");
    Ok(xml_response("ListSubscriptions", &inner))
}

fn publish(
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let _topic_arn = params
        .get("TopicArn")
        .ok_or_else(|| LawsError::InvalidRequest("Missing TopicArn".into()))?;
    let _message = params
        .get("Message")
        .ok_or_else(|| LawsError::InvalidRequest("Missing Message".into()))?;

    let message_id = uuid::Uuid::new_v4().to_string();

    let inner = format!("<MessageId>{message_id}</MessageId>");
    Ok(xml_response("Publish", &inner))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_arn_format() {
        let arn = topic_arn("my-topic");
        assert_eq!(arn, "arn:aws:sns:us-east-1:000000000000:my-topic");
    }

    #[test]
    fn subscription_arn_format() {
        let arn = subscription_arn("my-topic", "abc-123");
        assert_eq!(
            arn,
            "arn:aws:sns:us-east-1:000000000000:my-topic:abc-123"
        );
    }
}
