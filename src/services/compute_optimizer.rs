use axum::response::{IntoResponse, Response};
use http::StatusCode;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::LawsError;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ComputeOptimizerState {
    pub enrolled: AtomicBool,
}

impl Default for ComputeOptimizerState {
    fn default() -> Self {
        Self {
            enrolled: AtomicBool::new(false),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &ComputeOptimizerState,
    target: &str,
    _payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("ComputeOptimizerService.")
        .unwrap_or(target);

    let result = match action {
        "GetAutoScalingGroupRecommendations" => get_asg_recommendations(),
        "GetEC2InstanceRecommendations" => get_ec2_recommendations(),
        "GetEBSVolumeRecommendations" => get_ebs_recommendations(),
        "GetLambdaFunctionRecommendations" => get_lambda_recommendations(),
        "GetEnrollmentStatus" => get_enrollment_status(state),
        "UpdateEnrollmentStatus" => update_enrollment_status(state, _payload),
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
        [("Content-Type", "application/x-amz-json-1.0")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn get_asg_recommendations() -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "autoScalingGroupRecommendations": [
            {
                "accountId": ACCOUNT_ID,
                "autoScalingGroupArn": format!("arn:aws:autoscaling:{REGION}:{ACCOUNT_ID}:autoScalingGroup:example-id:autoScalingGroupName/example-asg"),
                "autoScalingGroupName": "example-asg",
                "currentConfiguration": {
                    "desiredCapacity": 2,
                    "instanceType": "m5.large",
                    "maxSize": 4,
                    "minSize": 1,
                },
                "finding": "OVER_PROVISIONED",
                "recommendationOptions": [
                    {
                        "configuration": {
                            "desiredCapacity": 2,
                            "instanceType": "t3.large",
                            "maxSize": 4,
                            "minSize": 1,
                        },
                        "performanceRisk": 2.0,
                        "rank": 1,
                        "projectedUtilizationMetrics": [
                            { "name": "Cpu", "statistic": "Maximum", "value": 45.0 },
                        ],
                    }
                ],
                "utilizationMetrics": [
                    { "name": "Cpu", "statistic": "Maximum", "value": 25.0 },
                    { "name": "Memory", "statistic": "Maximum", "value": 30.0 },
                ],
            }
        ],
        "errors": [],
    })))
}

fn get_ec2_recommendations() -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "instanceRecommendations": [
            {
                "accountId": ACCOUNT_ID,
                "instanceArn": format!("arn:aws:ec2:{REGION}:{ACCOUNT_ID}:instance/i-0123456789abcdef0"),
                "instanceName": "example-instance",
                "currentInstanceType": "m5.xlarge",
                "finding": "OVER_PROVISIONED",
                "findingReasonCodes": ["CPUOverprovisioned", "MemoryOverprovisioned"],
                "recommendationOptions": [
                    {
                        "instanceType": "m5.large",
                        "performanceRisk": 1.5,
                        "rank": 1,
                        "projectedUtilizationMetrics": [
                            { "name": "Cpu", "statistic": "Maximum", "value": 50.0 },
                            { "name": "Memory", "statistic": "Maximum", "value": 55.0 },
                        ],
                    },
                    {
                        "instanceType": "t3.xlarge",
                        "performanceRisk": 2.0,
                        "rank": 2,
                        "projectedUtilizationMetrics": [
                            { "name": "Cpu", "statistic": "Maximum", "value": 40.0 },
                        ],
                    },
                ],
                "utilizationMetrics": [
                    { "name": "Cpu", "statistic": "Maximum", "value": 20.0 },
                    { "name": "Memory", "statistic": "Maximum", "value": 25.0 },
                ],
            }
        ],
        "errors": [],
    })))
}

fn get_ebs_recommendations() -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "volumeRecommendations": [
            {
                "accountId": ACCOUNT_ID,
                "volumeArn": format!("arn:aws:ec2:{REGION}:{ACCOUNT_ID}:volume/vol-0123456789abcdef0"),
                "currentConfiguration": {
                    "volumeType": "gp2",
                    "volumeSize": 100,
                    "volumeBaselineIOPS": 300,
                    "volumeBaselineThroughput": 125,
                },
                "finding": "NOT_OPTIMIZED",
                "recommendationOptions": [
                    {
                        "configuration": {
                            "volumeType": "gp3",
                            "volumeSize": 100,
                            "volumeBaselineIOPS": 3000,
                            "volumeBaselineThroughput": 125,
                        },
                        "performanceRisk": 1.0,
                        "rank": 1,
                    }
                ],
                "utilizationMetrics": [
                    { "name": "VolumeReadOpsPerSecond", "statistic": "Maximum", "value": 150.0 },
                    { "name": "VolumeWriteOpsPerSecond", "statistic": "Maximum", "value": 100.0 },
                ],
            }
        ],
        "errors": [],
    })))
}

fn get_lambda_recommendations() -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "functionRecommendations": [
            {
                "accountId": ACCOUNT_ID,
                "functionArn": format!("arn:aws:lambda:{REGION}:{ACCOUNT_ID}:function:example-function"),
                "functionVersion": "$LATEST",
                "currentMemorySize": 512,
                "finding": "OVER_PROVISIONED",
                "findingReasonCodes": ["MemoryOverprovisioned"],
                "memorySizeRecommendationOptions": [
                    {
                        "memorySize": 256,
                        "rank": 1,
                        "projectedUtilizationMetrics": [
                            { "name": "Duration", "statistic": "Expected", "value": 150.0 },
                        ],
                    }
                ],
                "utilizationMetrics": [
                    { "name": "Duration", "statistic": "Average", "value": 100.0 },
                    { "name": "Memory", "statistic": "Maximum", "value": 128.0 },
                ],
                "numberOfInvocations": 10000,
            }
        ],
        "errors": [],
    })))
}

fn get_enrollment_status(state: &ComputeOptimizerState) -> Result<Response, LawsError> {
    let enrolled = state.enrolled.load(Ordering::Relaxed);
    let status = if enrolled { "Active" } else { "Inactive" };

    Ok(json_response(json!({
        "status": status,
        "memberAccountsEnrolled": false,
    })))
}

fn update_enrollment_status(
    state: &ComputeOptimizerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let status = payload["status"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("status is required".to_string()))?;

    let enrolled = status == "Active";
    state.enrolled.store(enrolled, Ordering::Relaxed);

    Ok(json_response(json!({
        "status": status,
    })))
}
