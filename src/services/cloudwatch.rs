use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, Uri};
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use chrono::Utc;
use dashmap::DashMap;

use crate::error::LawsError;
use crate::protocol::query::{parse_query_request, xml_error_response, xml_response};

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct MetricDataPoint {
    pub metric_name: String,
    pub namespace: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: String,
}

#[derive(Clone, Debug)]
pub struct CloudWatchAlarm {
    pub alarm_name: String,
    pub metric_name: String,
    pub namespace: String,
    pub comparison_operator: String,
    pub threshold: f64,
    pub period: u32,
    pub evaluation_periods: u32,
    pub statistic: String,
    pub state_value: String,
    pub arn: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct CloudWatchState {
    /// Keyed by "{namespace}/{metric_name}"
    pub metrics: Arc<DashMap<String, Vec<MetricDataPoint>>>,
    /// Keyed by alarm name
    pub alarms: Arc<DashMap<String, CloudWatchAlarm>>,
}

impl Default for CloudWatchState {
    fn default() -> Self {
        Self {
            metrics: Arc::new(DashMap::new()),
            alarms: Arc::new(DashMap::new()),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<CloudWatchState>) -> Router {
    Router::new()
        .route("/", post(handle_cloudwatch))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn metric_key(namespace: &str, metric_name: &str) -> String {
    format!("{namespace}/{metric_name}")
}

fn alarm_arn(alarm_name: &str) -> String {
    format!("arn:aws:cloudwatch:us-east-1:000000000000:alarm:{alarm_name}")
}

fn collect_indexed_params(params: &HashMap<String, String>, prefix: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut n = 1;
    loop {
        let key = format!("{prefix}.{n}");
        match params.get(&key) {
            Some(v) => {
                values.push(v.clone());
                n += 1;
            }
            None => break,
        }
    }
    values
}

// ---------------------------------------------------------------------------
// Dispatch handler
// ---------------------------------------------------------------------------

pub fn handle_request(
    state: &CloudWatchState,
    headers: &HeaderMap,
    body: &Bytes,
    uri: &Uri,
) -> Response {
    let req = match parse_query_request(uri, headers, body) {
        Ok(r) => r,
        Err(e) => return xml_error_response(&e),
    };

    let result = match req.action.as_str() {
        "PutMetricData" => put_metric_data(state, &req.params),
        "ListMetrics" => list_metrics(state, &req.params),
        "GetMetricData" => get_metric_data(state, &req.params),
        "PutMetricAlarm" => put_metric_alarm(state, &req.params),
        "DescribeAlarms" => describe_alarms(state),
        "DeleteAlarms" => delete_alarms(state, &req.params),
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

async fn handle_cloudwatch(
    State(state): State<Arc<CloudWatchState>>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_request(&state, &headers, &body, &uri)
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn put_metric_data(
    state: &CloudWatchState,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let namespace = params
        .get("Namespace")
        .ok_or_else(|| LawsError::InvalidRequest("Missing Namespace".into()))?
        .clone();

    // Collect metric data members (MetricData.member.N.*)
    let mut n = 1;
    loop {
        let prefix = format!("MetricData.member.{n}");
        let metric_name_key = format!("{prefix}.MetricName");
        let metric_name = match params.get(&metric_name_key) {
            Some(v) => v.clone(),
            None => break,
        };

        let value: f64 = params
            .get(&format!("{prefix}.Value"))
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0);

        let unit = params
            .get(&format!("{prefix}.Unit"))
            .cloned()
            .unwrap_or_else(|| "None".to_string());

        let timestamp = params
            .get(&format!("{prefix}.Timestamp"))
            .cloned()
            .unwrap_or_else(|| Utc::now().to_rfc3339());

        let data_point = MetricDataPoint {
            metric_name: metric_name.clone(),
            namespace: namespace.clone(),
            value,
            unit,
            timestamp,
        };

        let key = metric_key(&namespace, &metric_name);
        state
            .metrics
            .entry(key)
            .or_insert_with(Vec::new)
            .push(data_point);

        n += 1;
    }

    Ok(xml_response("PutMetricData", ""))
}

fn list_metrics(
    state: &CloudWatchState,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let namespace_filter = params.get("Namespace");

    let mut members_xml = String::new();
    for entry in state.metrics.iter() {
        let points = entry.value();
        if let Some(first) = points.first() {
            if let Some(ns_filter) = namespace_filter {
                if &first.namespace != ns_filter {
                    continue;
                }
            }
            members_xml.push_str(&format!(
                r#"<member>
  <Namespace>{}</Namespace>
  <MetricName>{}</MetricName>
  <Dimensions></Dimensions>
</member>
"#,
                quick_xml::escape::escape(&first.namespace),
                quick_xml::escape::escape(&first.metric_name),
            ));
        }
    }

    let inner = format!("<Metrics>{members_xml}</Metrics>");
    Ok(xml_response("ListMetrics", &inner))
}

fn get_metric_data(
    state: &CloudWatchState,
    _params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let mut members_xml = String::new();
    for entry in state.metrics.iter() {
        let points = entry.value();
        if let Some(first) = points.first() {
            let mut values_xml = String::new();
            let mut timestamps_xml = String::new();
            for pt in points.iter() {
                values_xml.push_str(&format!("<member>{}</member>", pt.value));
                timestamps_xml.push_str(&format!(
                    "<member>{}</member>",
                    quick_xml::escape::escape(&pt.timestamp)
                ));
            }

            members_xml.push_str(&format!(
                r#"<member>
  <Id>{}</Id>
  <Label>{}</Label>
  <Values>{values_xml}</Values>
  <Timestamps>{timestamps_xml}</Timestamps>
</member>
"#,
                quick_xml::escape::escape(&first.metric_name),
                quick_xml::escape::escape(&first.metric_name),
            ));
        }
    }

    let inner = format!("<MetricDataResults>{members_xml}</MetricDataResults>");
    Ok(xml_response("GetMetricData", &inner))
}

fn put_metric_alarm(
    state: &CloudWatchState,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let alarm_name = params
        .get("AlarmName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing AlarmName".into()))?
        .clone();

    let metric_name = params
        .get("MetricName")
        .ok_or_else(|| LawsError::InvalidRequest("Missing MetricName".into()))?
        .clone();

    let namespace = params
        .get("Namespace")
        .ok_or_else(|| LawsError::InvalidRequest("Missing Namespace".into()))?
        .clone();

    let comparison_operator = params
        .get("ComparisonOperator")
        .cloned()
        .unwrap_or_else(|| "GreaterThanThreshold".to_string());

    let threshold: f64 = params
        .get("Threshold")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);

    let period: u32 = params
        .get("Period")
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);

    let evaluation_periods: u32 = params
        .get("EvaluationPeriods")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    let statistic = params
        .get("Statistic")
        .cloned()
        .unwrap_or_else(|| "Average".to_string());

    let arn = alarm_arn(&alarm_name);

    let alarm = CloudWatchAlarm {
        alarm_name: alarm_name.clone(),
        metric_name,
        namespace,
        comparison_operator,
        threshold,
        period,
        evaluation_periods,
        statistic,
        state_value: "OK".to_string(),
        arn,
    };

    state.alarms.insert(alarm_name, alarm);
    Ok(xml_response("PutMetricAlarm", ""))
}

fn describe_alarms(state: &CloudWatchState) -> Result<Response, LawsError> {
    let mut members_xml = String::new();
    for entry in state.alarms.iter() {
        let a = entry.value();
        members_xml.push_str(&format!(
            r#"<member>
  <AlarmName>{}</AlarmName>
  <AlarmArn>{}</AlarmArn>
  <MetricName>{}</MetricName>
  <Namespace>{}</Namespace>
  <ComparisonOperator>{}</ComparisonOperator>
  <Threshold>{}</Threshold>
  <Period>{}</Period>
  <EvaluationPeriods>{}</EvaluationPeriods>
  <Statistic>{}</Statistic>
  <StateValue>{}</StateValue>
</member>
"#,
            quick_xml::escape::escape(&a.alarm_name),
            quick_xml::escape::escape(&a.arn),
            quick_xml::escape::escape(&a.metric_name),
            quick_xml::escape::escape(&a.namespace),
            quick_xml::escape::escape(&a.comparison_operator),
            a.threshold,
            a.period,
            a.evaluation_periods,
            quick_xml::escape::escape(&a.statistic),
            quick_xml::escape::escape(&a.state_value),
        ));
    }

    let inner = format!("<MetricAlarms>{members_xml}</MetricAlarms>");
    Ok(xml_response("DescribeAlarms", &inner))
}

fn delete_alarms(
    state: &CloudWatchState,
    params: &HashMap<String, String>,
) -> Result<Response, LawsError> {
    let alarm_names = collect_indexed_params(params, "AlarmNames.member");
    if alarm_names.is_empty() {
        return Err(LawsError::InvalidRequest(
            "Missing AlarmNames.member.1".into(),
        ));
    }

    for name in &alarm_names {
        state.alarms.remove(name);
    }

    Ok(xml_response("DeleteAlarms", ""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alarm_arn_format() {
        let arn = alarm_arn("cpu-high");
        assert_eq!(
            arn,
            "arn:aws:cloudwatch:us-east-1:000000000000:alarm:cpu-high"
        );
    }

    #[test]
    fn metric_key_format() {
        let key = metric_key("AWS/EC2", "CPUUtilization");
        assert_eq!(key, "AWS/EC2/CPUUtilization");
    }
}
