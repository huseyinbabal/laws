use std::collections::HashMap;
use std::convert::Infallible;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use chrono::Utc;
use http_body_util::BodyExt;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

// ---------------------------------------------------------------------------
// Event type
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct ApiRequestEvent {
    pub id: String,
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub service: String,
    pub action: String,
    pub status_code: u16,
    pub duration_ms: u64,
    pub request_headers: HashMap<String, String>,
    pub request_body: Option<String>,
    pub response_headers: HashMap<String, String>,
    pub response_body: Option<String>,
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct DashboardState {
    pub tx: broadcast::Sender<ApiRequestEvent>,
}

impl DashboardState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self { tx }
    }
}

// ---------------------------------------------------------------------------
// SSE route
// ---------------------------------------------------------------------------

pub fn router(state: DashboardState) -> Router {
    Router::new()
        .route("/api/dashboard/events", get(sse_handler))
        .with_state(state)
}

async fn sse_handler(
    State(state): State<DashboardState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|r| r.ok())
        .map(|event| {
            Ok(Event::default()
                .event("request")
                .data(serde_json::to_string(&event).unwrap_or_default()))
        });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    )
}

// ---------------------------------------------------------------------------
// Request logging middleware
// ---------------------------------------------------------------------------

pub async fn request_logger(
    State(dashboard): State<DashboardState>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();

    // Skip dashboard/static paths (but NOT dispatch requests like POST /)
    if path.starts_with("/api/dashboard")
        || path.starts_with("/dashboard")
        || path.starts_with("/assets")
    {
        return next.run(req).await;
    }

    // Skip only GET / (the S3 bucket-list / health check), not POST /
    if path == "/" && req.method() == axum::http::Method::GET {
        return next.run(req).await;
    }

    let method = req.method().to_string();
    let (service, action) = extract_service_and_action(&req);

    // Capture request headers
    let request_headers: HashMap<String, String> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
        .collect();

    // Buffer request body so we can read it and still forward it
    let (parts, body) = req.into_parts();
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => axum::body::Bytes::new(),
    };

    let request_body = if body_bytes.is_empty() {
        None
    } else {
        let raw = String::from_utf8_lossy(&body_bytes).to_string();
        // Try to pretty-print JSON, otherwise keep raw
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            Some(serde_json::to_string_pretty(&v).unwrap_or(raw))
        } else {
            Some(raw)
        }
    };

    // Re-assemble request with the buffered body
    let req = Request::from_parts(parts, Body::from(body_bytes));

    let start = Instant::now();
    let response = next.run(req).await;
    let duration = start.elapsed();

    // Capture response headers and body
    let resp_status = response.status().as_u16();
    let response_headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
        .collect();

    let (resp_parts, resp_body) = response.into_parts();
    let resp_body_bytes = match resp_body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => axum::body::Bytes::new(),
    };

    let response_body = if resp_body_bytes.is_empty() {
        None
    } else {
        let raw = String::from_utf8_lossy(&resp_body_bytes).to_string();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            Some(serde_json::to_string_pretty(&v).unwrap_or(raw))
        } else {
            Some(raw)
        }
    };

    let event = ApiRequestEvent {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: Utc::now().to_rfc3339(),
        method,
        path,
        service,
        action,
        status_code: resp_status,
        duration_ms: duration.as_millis() as u64,
        request_headers,
        request_body,
        response_headers,
        response_body,
    };

    let _ = dashboard.tx.send(event);

    // Re-assemble the response
    Response::from_parts(resp_parts, Body::from(resp_body_bytes))
}

// ---------------------------------------------------------------------------
// Service/action extraction
// ---------------------------------------------------------------------------

fn extract_service_and_action(req: &Request) -> (String, String) {
    // Try X-Amz-Target header (JSON protocol)
    if let Some(target) = req
        .headers()
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
    {
        return parse_amz_target(target);
    }

    // Try Action query/form param from URI
    if let Some(query) = req.uri().query() {
        for pair in form_urlencoded::parse(query.as_bytes()) {
            if pair.0 == "Action" {
                let action = pair.1.to_string();
                let service = guess_service_from_action(&action);
                return (service, action);
            }
        }
    }

    // Fall back to path-based heuristics for REST services
    let path = req.uri().path();
    guess_service_from_path(path)
}

fn parse_amz_target(target: &str) -> (String, String) {
    // Format: "ServicePrefix.ActionName" e.g. "DynamoDB_20120810.PutItem"
    if let Some(dot_pos) = target.rfind('.') {
        let prefix = &target[..dot_pos];
        let action = &target[dot_pos + 1..];
        let service = match prefix {
            p if p.starts_with("AmazonSQS") => "SQS",
            p if p.starts_with("DynamoDB") => "DynamoDB",
            p if p.starts_with("Logs_") => "CloudWatch Logs",
            p if p.starts_with("AmazonEC2ContainerServiceV") => "ECS",
            p if p.starts_with("AWSStepFunctions") => "Step Functions",
            p if p.starts_with("Kinesis_") => "Kinesis",
            p if p.starts_with("AWSEvents") => "EventBridge",
            p if p.starts_with("TrentService") => "KMS",
            p if p.starts_with("CertificateManager") => "ACM",
            p if p.starts_with("AmazonRDSv19_DocDB") => "DocumentDB",
            p if p.starts_with("AmazonRDSv19") => "RDS",
            p if p.starts_with("AmazonElastiCacheV9") => "ElastiCache",
            p if p.starts_with("RedshiftServiceVersion") => "Redshift",
            p if p.starts_with("AWSCognitoIdentityProviderService") => "Cognito",
            p if p.starts_with("CloudFormation_") => "CloudFormation",
            p if p.starts_with("AmazonEC2ContainerRegistry") => "ECR",
            p if p.starts_with("ElasticLoadBalancingV2") => "ELB",
            p if p.starts_with("SimpleEmailServiceV2") => "SES",
            p if p.starts_with("Firehose_") => "Firehose",
            p if p.starts_with("AWSGlue") => "Glue",
            p if p.starts_with("AmazonAthena") => "Athena",
            p if p.starts_with("CodeBuild_") => "CodeBuild",
            p if p.starts_with("CodePipeline_") => "CodePipeline",
            p if p.starts_with("AWSWAF_") => "WAF",
            p if p.starts_with("StarlingDoveService") => "Config",
            p if p.starts_with("AWSOrganizations") => "Organizations",
            p if p.starts_with("Kafka") => "MSK",
            p if p.starts_with("Textract") => "Textract",
            p if p.starts_with("AWSShineFrontendService") => "Translate",
            p if p.starts_with("Comprehend_") => "Comprehend",
            p if p.starts_with("RekognitionService") => "Rekognition",
            p if p.starts_with("SageMaker") => "SageMaker",
            p if p.starts_with("secretsmanager") => "Secrets Manager",
            p if p.starts_with("AmazonSSM") => "SSM",
            p if p.starts_with("AccessAnalyzer") => "Access Analyzer",
            p if p.starts_with("ACMPrivateCA") => "ACM PCA",
            p if p.starts_with("SandstoneConfiguration") => "AppFlow",
            p if p.starts_with("PhotonAdminProxyService") => "AppStream",
            p if p.starts_with("AnyScaleFrontendService") => "Application Auto Scaling",
            p if p.starts_with("AWSBudgetServiceGateway") => "Budgets",
            p if p.starts_with("WheatleyOrchestration") => "Chatbot",
            p if p.starts_with("AWSCloud9") => "Cloud9",
            p if p.starts_with("CloudApiService") => "CloudControl",
            p if p.starts_with("BaldrApiService") => "CloudHSM",
            p if p.starts_with("CodeGuruProfilerService") => "CodeGuru",
            p if p.starts_with("ComputeOptimizerService") => "Compute Optimizer",
            p if p.starts_with("ControltowerService") => "Control Tower",
            p if p.starts_with("AWSInsightsIndexService") => "Cost Explorer",
            p if p.starts_with("AWSOrigamiServiceGatewayService") => "Cost & Usage Reports",
            p if p.starts_with("DataPipeline") => "Data Pipeline",
            p if p.starts_with("FmrsService") => "DataSync",
            p if p.starts_with("DeviceFarm_") => "Device Farm",
            p if p.starts_with("OvertureService") => "Direct Connect",
            p if p.starts_with("DirectoryService_") => "Directory Service",
            p if p.starts_with("AWSFMS_") => "Firewall Manager",
            p if p.starts_with("AWSHawksNest") => "Fraud Detector",
            p if p.starts_with("GameLift") => "GameLift",
            p if p.starts_with("GlobalAccelerator_") => "Global Accelerator",
            p if p.starts_with("AWSHealth_") => "Health",
            p if p.starts_with("HealthLake") => "HealthLake",
            p if p.starts_with("AWSIdentityStore") => "Identity Store",
            p if p.starts_with("AmazonInteractiveVideoService") => "IVS",
            p if p.starts_with("AWSLicenseManager") => "License Manager",
            p if p.starts_with("MediaStore_") => "MediaStore",
            p if p.starts_with("NetworkFirewall_") => "Network Firewall",
            p if p.starts_with("AWSPriceListService") => "Pricing",
            p if p.starts_with("AwsResilienceHub") => "Resilience Hub",
            p if p.starts_with("Route53Domains_") => "Route 53 Domains",
            p if p.starts_with("Route53Resolver") => "Route 53 Resolver",
            p if p.starts_with("AWSSavingsPlan") => "Savings Plans",
            p if p.starts_with("ServiceQuotas") => "Service Quotas",
            p if p.starts_with("AWSIESnowball") => "Snowball",
            p if p.starts_with("AWSSupport_") => "Support",
            p if p.starts_with("SimpleWorkflowService") => "SWF",
            p if p.starts_with("VerifiedPermissions") => "Verified Permissions",
            p if p.starts_with("WorkMailService") => "WorkMail",
            p if p.starts_with("WorkspacesService") => "WorkSpaces",
            p if p.starts_with("AppRunner") => "App Runner",
            p if p.starts_with("AmazonDAXV3") => "DAX",
            p if p.starts_with("AWSSimbaAPIService") => "FSx",
            p if p.starts_with("KeyspacesService") => "Keyspaces",
            p if p.starts_with("AWSKendraFrontendService") => "Kendra",
            p if p.starts_with("AWSLakeFormation") => "Lake Formation",
            p if p.starts_with("AmazonMemoryDB") => "MemoryDB",
            p if p.starts_with("Route53AutoNaming") => "Cloud Map",
            p if p.starts_with("AmazonForecast") => "Forecast",
            p if p.starts_with("AmazonPersonalize") => "Personalize",
            p if p.starts_with("AwsProton") => "Proton",
            p if p.starts_with("SWBExternalService") => "SSO",
            p if p.starts_with("AmazonResourceSharing") => "RAM",
            p if p.starts_with("StorageGateway_") => "Storage Gateway",
            p if p.starts_with("CloudTrail_") => "CloudTrail",
            p if p.starts_with("CodeCommit_") => "CodeCommit",
            p if p.starts_with("CodeDeploy_") => "CodeDeploy",
            p if p.starts_with("AmazonDMSv") => "DMS",
            p if p.starts_with("ElasticMapReduce") => "EMR",
            p if p.starts_with("InspectorService") => "Inspector",
            p if p.starts_with("Lightsail_") => "Lightsail",
            p if p.starts_with("AmazonNeptune") => "Neptune",
            p if p.starts_with("AWS242ServiceCatalog") => "Service Catalog",
            p if p.starts_with("AWSShield_") => "Shield",
            p if p.starts_with("Timestream_") => "Timestream",
            p if p.starts_with("TransferService") => "Transfer Family",
            _ => prefix,
        };
        return (service.to_string(), action.to_string());
    }
    (target.to_string(), String::new())
}

fn guess_service_from_action(action: &str) -> String {
    match action {
        a if a.contains("Queue")
            || a == "SendMessage"
            || a == "ReceiveMessage"
            || a == "DeleteMessage" =>
        {
            "SQS".to_string()
        }
        a if a.contains("Topic") || a == "Subscribe" || a == "Unsubscribe" || a == "Publish" => {
            "SNS".to_string()
        }
        a if a.contains("User") || a.contains("Role") || a.contains("Policy") => "IAM".to_string(),
        a if a == "GetCallerIdentity" || a == "AssumeRole" => "STS".to_string(),
        a if a.contains("Instances")
            || a.contains("SecurityGroup")
            || a.contains("Vpc")
            || a.contains("Subnet") =>
        {
            "EC2".to_string()
        }
        a if a.contains("Metric") || a.contains("Alarm") => "CloudWatch".to_string(),
        a if a.contains("AutoScalingGroup")
            || a.contains("LaunchConfiguration")
            || a == "SetDesiredCapacity" =>
        {
            "Auto Scaling".to_string()
        }
        a if a.contains("Application") || a.contains("Environment") => {
            "Elastic Beanstalk".to_string()
        }
        a if a.contains("Domain") => "CloudSearch".to_string(),
        _ => "Unknown".to_string(),
    }
}

fn guess_service_from_path(path: &str) -> (String, String) {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let (service, _action) = match segments.first().copied() {
        Some("2015-03-31") => ("Lambda", path),
        Some("restapis") => ("API Gateway", path),
        Some("v2") if path.contains("/apis") => ("API Gateway V2", path),
        Some("2013-04-01") => ("Route 53", path),
        Some("clusters") if path.contains("/eks") || !path.contains("/ecs") => ("EKS", path),
        Some("2020-05-31") => ("CloudFront", path),
        Some("v1")
            if path.contains("/batch") || path.contains("/compute") || path.contains("/job") =>
        {
            ("Batch", path)
        }
        Some("backup") => ("Backup", path),
        Some("v1") if path.contains("/broker") => ("MQ", path),
        Some("Traces") | Some("TraceIds") | Some("Groups") | Some("GetGroups") => ("X-Ray", path),
        Some("v1") if path.contains("/apis") && path.contains("graphql") => ("AppSync", path),
        Some("2015-02-01") => ("EFS", path),
        Some("detector") => ("GuardDuty", path),
        Some("things") | Some("policies") if path.contains("/iot") || segments.len() <= 3 => {
            ("IoT", path)
        }
        Some("2021-01-01") => ("OpenSearch", path),
        Some("v1") if path.contains("/lexicons") || path.contains("/speech") => ("Polly", path),
        Some("ledgers") => ("QLDB", path),
        Some("2017-08-29") => ("MediaConvert", path),
        Some("applications") => ("AppConfig", path),
        Some("graph") => ("Detective", path),
        Some("apps") if path.contains("/amplify") || segments.len() <= 2 => ("Amplify", path),
        Some("bots") => ("Lex", path),
        Some("maps") | Some("geofences") | Some("trackers") => ("Location", path),
        Some("productSubscriptions") | Some("findings") => ("Security Hub", path),
        Some("foundation-models") | Some("model-customization") | Some("custom-models") => {
            ("Bedrock", path)
        }
        Some("v1") if path.contains("/domain") || path.contains("/repository") => {
            ("CodeArtifact", path)
        }
        Some("v1") if path.contains("/apps") => ("Pinpoint", path),
        Some("connect") => ("Connect", path),
        Some("vaults") if path.contains("glacier") || path.contains("archives") => {
            ("Glacier", path)
        }
        Some("prod") if path.contains("/channels") || path.contains("/inputs") => {
            ("MediaLive", path)
        }
        Some("accounts") => ("QuickSight", path),
        Some("workspaces") if path.contains("/prometheus") || path.contains("/amp") => {
            ("AMP", path)
        }
        Some("v20190125") => ("App Mesh", path),
        Some("assessments") => ("Audit Manager", path),
        Some("quantum-task") | Some("device") => ("Braket", path),
        Some("collaborations") => ("Clean Rooms", path),
        Some("domains") if path.contains("/profiles") => ("Customer Profiles", path),
        Some("projects") if path.contains("/databrew") => ("DataBrew", path),
        Some("v1") if path.contains("/data-sets") => ("Data Exchange", path),
        Some("v2") if path.contains("/domains") => ("DataZone", path),
        Some("channels") if path.contains("/mediapackage") => ("MediaPackage", path),
        Some("channels") if path.contains("/mediatailor") => ("MediaTailor", path),
        Some("insights") | Some("channels") => ("DevOps Guru", path),
        Some("policies") if path.contains("/dlm") => ("DLM", path),
        Some("snapshots") => ("EBS", path),
        Some("experimentTemplates") | Some("experiments") => ("FIS", path),
        Some("greengrass") => ("Greengrass", path),
        Some("satellite") | Some("config") => ("Ground Station", path),
        Some("images") | Some("components") if path.contains("/imagebuilder") => {
            ("Image Builder", path)
        }
        Some("v20210603") => ("Internet Monitor", path),
        Some("networks") => ("Managed Blockchain", path),
        Some("playbackConfiguration") => ("MediaTailor", path),
        Some("environments") => ("MWAA", path),
        Some("global-networks") => ("Network Manager", path),
        Some("sequenceStore") | Some("workflow") => ("Omics", path),
        Some("outposts") | Some("sites") => ("Outposts", path),
        Some("v1") if path.contains("/pipes") => ("EventBridge Pipes", path),
        Some("v1") if path.contains("/registries") => ("Schemas", path),
        Some("v1") if path.contains("/datalake") => ("Security Lake", path),
        Some("trustanchors") | Some("profiles") => ("Roles Anywhere", path),
        Some("appmonitors") => ("RUM", path),
        Some("schedules") | Some("schedule-groups") => ("EventBridge Scheduler", path),
        Some("canary") | Some("canaries") => ("Synthetics", path),
        Some("services") if path.contains("/lattice") || path.contains("/targetgroups") => {
            ("VPC Lattice", path)
        }
        Some("workloads") => ("Well-Architected", path),
        Some("api") if path.contains("/v1") => ("WorkDocs", path),
        _ => ("Unknown", path),
    };

    let action_str = segments.last().copied().unwrap_or("").to_string();
    (service.to_string(), action_str)
}
