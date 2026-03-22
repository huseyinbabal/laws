#![allow(dead_code)]

mod config;
mod dashboard;
mod error;
mod protocol;
mod services;
mod storage;

use std::sync::Arc;

use axum::extract::State;
use axum::Router;
use clap::Parser;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing::info;

use config::Config;
use dashboard::DashboardState;

#[derive(Clone)]
struct DispatchState {
    // Query protocol services
    sqs: Arc<services::sqs::SqsState>,
    sns: Arc<services::sns::SnsState>,
    iam: Arc<services::iam::IamState>,
    sts: Arc<services::sts::StsState>,
    ec2: Arc<services::ec2::Ec2State>,
    cloudwatch: Arc<services::cloudwatch::CloudWatchState>,
    autoscaling: Arc<services::autoscaling::AutoScalingState>,
    elasticbeanstalk: Arc<services::elasticbeanstalk::ElasticBeanstalkState>,
    cloudsearch: Arc<services::cloudsearch::CloudSearchState>,
    // JSON protocol services (original 50)
    dynamodb: Arc<services::dynamodb::DynamoDbState>,
    cw: Arc<services::cloudwatch_logs::CloudWatchLogsState>,
    sm: Arc<services::secretsmanager::SecretsManagerState>,
    ssm: Arc<services::ssm::SsmState>,
    ecs: Arc<services::ecs::EcsState>,
    sfn: Arc<services::stepfunctions::StepFunctionsState>,
    kinesis: Arc<services::kinesis::KinesisState>,
    eventbridge: Arc<services::eventbridge::EventBridgeState>,
    kms: Arc<services::kms::KmsState>,
    acm: Arc<services::acm::AcmState>,
    rds: Arc<services::rds::RdsState>,
    elasticache: Arc<services::elasticache::ElastiCacheState>,
    redshift: Arc<services::redshift::RedshiftState>,
    cognito: Arc<services::cognito::CognitoState>,
    cloudformation: Arc<services::cloudformation::CloudFormationState>,
    ecr: Arc<services::ecr::EcrState>,
    elb: Arc<services::elb::ElbState>,
    ses: Arc<services::ses::SesState>,
    firehose: Arc<services::firehose::FirehoseState>,
    glue: Arc<services::glue::GlueState>,
    athena: Arc<services::athena::AthenaState>,
    codebuild: Arc<services::codebuild::CodeBuildState>,
    codepipeline: Arc<services::codepipeline::CodePipelineState>,
    waf: Arc<services::waf::WafState>,
    config_service: Arc<services::config_service::ConfigServiceState>,
    organizations: Arc<services::organizations::OrganizationsState>,
    msk: Arc<services::msk::MskState>,
    textract: Arc<services::textract::TextractState>,
    translate: Arc<services::translate::TranslateState>,
    comprehend: Arc<services::comprehend::ComprehendState>,
    rekognition: Arc<services::rekognition::RekognitionState>,
    sagemaker: Arc<services::sagemaker::SageMakerState>,
    cloudtrail: Arc<services::cloudtrail::CloudTrailState>,
    codecommit: Arc<services::codecommit::CodeCommitState>,
    codedeploy: Arc<services::codedeploy::CodeDeployState>,
    documentdb: Arc<services::documentdb::DocumentDbState>,
    dms: Arc<services::dms::DmsState>,
    emr: Arc<services::emr::EmrState>,
    inspector: Arc<services::inspector::InspectorState>,
    lightsail: Arc<services::lightsail::LightsailState>,
    neptune: Arc<services::neptune::NeptuneState>,
    service_catalog: Arc<services::service_catalog::ServiceCatalogState>,
    shield: Arc<services::shield::ShieldState>,
    timestream: Arc<services::timestream::TimestreamState>,
    transfer: Arc<services::transfer::TransferState>,
    workspaces: Arc<services::workspaces::WorkSpacesState>,
    apprunner: Arc<services::apprunner::AppRunnerState>,
    dax: Arc<services::dax::DaxState>,
    fsx: Arc<services::fsx::FsxState>,
    keyspaces: Arc<services::keyspaces::KeyspacesState>,
    kendra: Arc<services::kendra::KendraState>,
    lakeformation: Arc<services::lakeformation::LakeFormationState>,
    memorydb: Arc<services::memorydb::MemoryDbState>,
    cloudmap: Arc<services::cloudmap::CloudMapState>,
    forecast: Arc<services::forecast::ForecastState>,
    personalize: Arc<services::personalize::PersonalizeState>,
    proton: Arc<services::proton::ProtonState>,
    sso: Arc<services::sso::SsoState>,
    ram: Arc<services::ram::RamState>,
    storage_gateway: Arc<services::storage_gateway::StorageGatewayState>,
    // JSON protocol services (batch 6-10)
    accessanalyzer: Arc<services::accessanalyzer::AccessAnalyzerState>,
    acm_pca: Arc<services::acm_pca::AcmPcaState>,
    appflow: Arc<services::appflow::AppFlowState>,
    appstream: Arc<services::appstream::AppStreamState>,
    application_autoscaling: Arc<services::application_autoscaling::ApplicationAutoscalingState>,
    budgets: Arc<services::budgets::BudgetsState>,
    chatbot: Arc<services::chatbot::ChatbotState>,
    cloud9: Arc<services::cloud9::Cloud9State>,
    cloudcontrol: Arc<services::cloudcontrol::CloudControlState>,
    cloudhsm: Arc<services::cloudhsm::CloudHsmState>,
    codeguru: Arc<services::codeguru::CodeGuruState>,
    compute_optimizer: Arc<services::compute_optimizer::ComputeOptimizerState>,
    controltower: Arc<services::controltower::ControlTowerState>,
    costexplorer: Arc<services::costexplorer::CostExplorerState>,
    cur: Arc<services::cur::CurState>,
    datapipeline: Arc<services::datapipeline::DataPipelineState>,
    datasync: Arc<services::datasync::DataSyncState>,
    devicefarm: Arc<services::devicefarm::DeviceFarmState>,
    directconnect: Arc<services::directconnect::DirectConnectState>,
    directory_service: Arc<services::directory_service::DirectoryServiceState>,
    firewall_manager: Arc<services::firewall_manager::FirewallManagerState>,
    frauddetector: Arc<services::frauddetector::FraudDetectorState>,
    gamelift: Arc<services::gamelift::GameLiftState>,
    globalaccelerator: Arc<services::globalaccelerator::GlobalAcceleratorState>,
    health: Arc<services::health::HealthState>,
    healthlake: Arc<services::healthlake::HealthLakeState>,
    identitystore: Arc<services::identitystore::IdentityStoreState>,
    ivs: Arc<services::ivs::IvsState>,
    license_manager: Arc<services::license_manager::LicenseManagerState>,
    mediastore: Arc<services::mediastore::MediaStoreState>,
    network_firewall: Arc<services::network_firewall::NetworkFirewallState>,
    pricing: Arc<services::pricing::PricingState>,
    resiliencehub: Arc<services::resiliencehub::ResilienceHubState>,
    route53domains: Arc<services::route53domains::Route53DomainsState>,
    route53resolver: Arc<services::route53resolver::Route53ResolverState>,
    savingsplans: Arc<services::savingsplans::SavingsPlansState>,
    service_quotas: Arc<services::service_quotas::ServiceQuotasState>,
    snowball: Arc<services::snowball::SnowballState>,
    support: Arc<services::support::SupportState>,
    swf: Arc<services::swf::SwfState>,
    verifiedpermissions: Arc<services::verifiedpermissions::VerifiedPermissionsState>,
    workmail: Arc<services::workmail::WorkMailState>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "laws=info,tower_http=info".into()),
        )
        .init();

    let config = Config::parse();
    let addr = format!("{}:{}", config.host, config.port);

    let dashboard_state = DashboardState::new();
    let app = build_router(&config, dashboard_state.clone());

    info!("laws v{} starting on {}", env!("CARGO_PKG_VERSION"), addr);
    info!("Region: {}, Account: {}", config.region, config.account_id);
    info!("184 AWS services ready");
    info!("Dashboard: http://{}/dashboard", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn build_router(_config: &Config, dashboard_state: DashboardState) -> Router {
    // ── REST-based services (original) ──
    let s3_router = services::s3::router(Arc::new(services::s3::S3State::new()));
    let lambda_router =
        services::lambda::router(Arc::new(services::lambda::LambdaState::default()));
    let apigateway_router =
        services::apigateway::router(Arc::new(services::apigateway::ApiGatewayState::default()));
    let route53_router =
        services::route53::router(Arc::new(services::route53::Route53State::default()));
    let eks_router = services::eks::router(Arc::new(services::eks::EksState::default()));
    let cloudfront_router =
        services::cloudfront::router(Arc::new(services::cloudfront::CloudFrontState::default()));
    let batch_router = services::batch::router(Arc::new(services::batch::BatchState::default()));
    let backup_router =
        services::backup::router(Arc::new(services::backup::BackupState::default()));
    let mq_router = services::mq::router(Arc::new(services::mq::MqState::default()));
    let xray_router = services::xray::router(Arc::new(services::xray::XRayState::default()));
    let appsync_router =
        services::appsync::router(Arc::new(services::appsync::AppSyncState::default()));
    let efs_router = services::efs::router(Arc::new(services::efs::EfsState::default()));
    let guardduty_router =
        services::guardduty::router(Arc::new(services::guardduty::GuardDutyState::default()));
    let iot_router = services::iot::router(Arc::new(services::iot::IotState::default()));
    let macie_router = services::macie::router(Arc::new(services::macie::MacieState::default()));
    let opensearch_router =
        services::opensearch::router(Arc::new(services::opensearch::OpenSearchState::default()));
    let polly_router = services::polly::router(Arc::new(services::polly::PollyState::default()));
    let qldb_router = services::qldb::router(Arc::new(services::qldb::QldbState::default()));
    let mediaconvert_router = services::mediaconvert::router(Arc::new(
        services::mediaconvert::MediaConvertState::default(),
    ));
    let appconfig_router =
        services::appconfig::router(Arc::new(services::appconfig::AppConfigState::default()));
    let detective_router =
        services::detective::router(Arc::new(services::detective::DetectiveState::default()));
    let amplify_router =
        services::amplify::router(Arc::new(services::amplify::AmplifyState::default()));
    let lex_router = services::lex::router(Arc::new(services::lex::LexState::default()));
    let location_router =
        services::location::router(Arc::new(services::location::LocationState::default()));
    let securityhub_router =
        services::securityhub::router(Arc::new(services::securityhub::SecurityHubState::default()));
    let bedrock_router =
        services::bedrock::router(Arc::new(services::bedrock::BedrockState::default()));
    let codeartifact_router = services::codeartifact::router(Arc::new(
        services::codeartifact::CodeArtifactState::default(),
    ));
    let pinpoint_router =
        services::pinpoint::router(Arc::new(services::pinpoint::PinpointState::default()));
    let connect_router =
        services::connect::router(Arc::new(services::connect::ConnectState::default()));
    let glacier_router =
        services::glacier::router(Arc::new(services::glacier::GlacierState::default()));
    let medialive_router =
        services::medialive::router(Arc::new(services::medialive::MediaLiveState::default()));
    let quicksight_router =
        services::quicksight::router(Arc::new(services::quicksight::QuickSightState::default()));

    // ── REST-based services (batch 6-10) ──
    let amp_router = services::amp::router(Arc::new(services::amp::AmpState::default()));
    let apigatewayv2_router = services::apigatewayv2::router(Arc::new(
        services::apigatewayv2::ApiGatewayV2State::default(),
    ));
    let appmesh_router =
        services::appmesh::router(Arc::new(services::appmesh::AppMeshState::default()));
    let auditmanager_router = services::auditmanager::router(Arc::new(
        services::auditmanager::AuditManagerState::default(),
    ));
    let braket_router =
        services::braket::router(Arc::new(services::braket::BraketState::default()));
    let cleanrooms_router =
        services::cleanrooms::router(Arc::new(services::cleanrooms::CleanRoomsState::default()));
    let customer_profiles_router = services::customer_profiles::router(Arc::new(
        services::customer_profiles::CustomerProfilesState::default(),
    ));
    let databrew_router =
        services::databrew::router(Arc::new(services::databrew::DataBrewState::default()));
    let dataexchange_router = services::dataexchange::router(Arc::new(
        services::dataexchange::DataExchangeState::default(),
    ));
    let datazone_router =
        services::datazone::router(Arc::new(services::datazone::DataZoneState::default()));
    let devopsguru_router =
        services::devopsguru::router(Arc::new(services::devopsguru::DevOpsGuruState::default()));
    let dlm_router = services::dlm::router(Arc::new(services::dlm::DlmState::default()));
    let ebs_router = services::ebs::router(Arc::new(services::ebs::EbsState::default()));
    let emr_serverless_router = services::emr_serverless::router(Arc::new(
        services::emr_serverless::EmrServerlessState::default(),
    ));
    let entity_resolution_router = services::entity_resolution::router(Arc::new(
        services::entity_resolution::EntityResolutionState::default(),
    ));
    let eventbridge_scheduler_router = services::eventbridge_scheduler::router(Arc::new(
        services::eventbridge_scheduler::EventBridgeSchedulerState::default(),
    ));
    let finspace_router =
        services::finspace::router(Arc::new(services::finspace::FinSpaceState::default()));
    let fis_router = services::fis::router(Arc::new(services::fis::FisState::default()));
    let greengrass_router =
        services::greengrass::router(Arc::new(services::greengrass::GreengrassState::default()));
    let groundstation_router = services::groundstation::router(Arc::new(
        services::groundstation::GroundStationState::default(),
    ));
    let imagebuilder_router = services::imagebuilder::router(Arc::new(
        services::imagebuilder::ImageBuilderState::default(),
    ));
    let internetmonitor_router = services::internetmonitor::router(Arc::new(
        services::internetmonitor::InternetMonitorState::default(),
    ));
    let mainframe_router =
        services::mainframe::router(Arc::new(services::mainframe::MainframeState::default()));
    let managedblockchain_router = services::managedblockchain::router(Arc::new(
        services::managedblockchain::ManagedBlockchainState::default(),
    ));
    let managed_grafana_router = services::managed_grafana::router(Arc::new(
        services::managed_grafana::ManagedGrafanaState::default(),
    ));
    let mediapackage_router = services::mediapackage::router(Arc::new(
        services::mediapackage::MediaPackageState::default(),
    ));
    let mediatailor_router =
        services::mediatailor::router(Arc::new(services::mediatailor::MediaTailorState::default()));
    let mwaa_router = services::mwaa::router(Arc::new(services::mwaa::MwaaState::default()));
    let networkmanager_router = services::networkmanager::router(Arc::new(
        services::networkmanager::NetworkManagerState::default(),
    ));
    let omics_router = services::omics::router(Arc::new(services::omics::OmicsState::default()));
    let outposts_router =
        services::outposts::router(Arc::new(services::outposts::OutpostsState::default()));
    let pipes_router = services::pipes::router(Arc::new(services::pipes::PipesState::default()));
    let rolesanywhere_router = services::rolesanywhere::router(Arc::new(
        services::rolesanywhere::RolesAnywhereState::default(),
    ));
    let rum_router = services::rum::router(Arc::new(services::rum::RumState::default()));
    let schemas_router =
        services::schemas::router(Arc::new(services::schemas::SchemasState::default()));
    let securitylake_router = services::securitylake::router(Arc::new(
        services::securitylake::SecurityLakeState::default(),
    ));
    let serverlessrepo_router = services::serverlessrepo::router(Arc::new(
        services::serverlessrepo::ServerlessRepoState::default(),
    ));
    let synthetics_router =
        services::synthetics::router(Arc::new(services::synthetics::SyntheticsState::default()));
    let vpc_lattice_router =
        services::vpc_lattice::router(Arc::new(services::vpc_lattice::VpcLatticeState::default()));
    let wellarchitected_router = services::wellarchitected::router(Arc::new(
        services::wellarchitected::WellArchitectedState::default(),
    ));
    let workdocs_router =
        services::workdocs::router(Arc::new(services::workdocs::WorkDocsState::default()));

    // ── Dispatch-based services (JSON + Query protocol) ──
    let dispatch_state = DispatchState {
        // Query protocol
        sqs: Arc::new(services::sqs::SqsState::new()),
        sns: Arc::new(services::sns::SnsState::new()),
        iam: Arc::new(services::iam::IamState::default()),
        sts: Arc::new(services::sts::StsState::default()),
        ec2: Arc::new(services::ec2::Ec2State::default()),
        cloudwatch: Arc::new(services::cloudwatch::CloudWatchState::default()),
        autoscaling: Arc::new(services::autoscaling::AutoScalingState::default()),
        elasticbeanstalk: Arc::new(services::elasticbeanstalk::ElasticBeanstalkState::default()),
        cloudsearch: Arc::new(services::cloudsearch::CloudSearchState::default()),
        // JSON protocol (original)
        dynamodb: Arc::new(services::dynamodb::DynamoDbState::default()),
        cw: Arc::new(services::cloudwatch_logs::CloudWatchLogsState::default()),
        sm: Arc::new(services::secretsmanager::SecretsManagerState::default()),
        ssm: Arc::new(services::ssm::SsmState::default()),
        ecs: Arc::new(services::ecs::EcsState::default()),
        sfn: Arc::new(services::stepfunctions::StepFunctionsState::default()),
        kinesis: Arc::new(services::kinesis::KinesisState::default()),
        eventbridge: Arc::new(services::eventbridge::EventBridgeState::default()),
        kms: Arc::new(services::kms::KmsState::default()),
        acm: Arc::new(services::acm::AcmState::default()),
        rds: Arc::new(services::rds::RdsState::default()),
        elasticache: Arc::new(services::elasticache::ElastiCacheState::default()),
        redshift: Arc::new(services::redshift::RedshiftState::default()),
        cognito: Arc::new(services::cognito::CognitoState::default()),
        cloudformation: Arc::new(services::cloudformation::CloudFormationState::default()),
        ecr: Arc::new(services::ecr::EcrState::default()),
        elb: Arc::new(services::elb::ElbState::default()),
        ses: Arc::new(services::ses::SesState::default()),
        firehose: Arc::new(services::firehose::FirehoseState::default()),
        glue: Arc::new(services::glue::GlueState::default()),
        athena: Arc::new(services::athena::AthenaState::default()),
        codebuild: Arc::new(services::codebuild::CodeBuildState::default()),
        codepipeline: Arc::new(services::codepipeline::CodePipelineState::default()),
        waf: Arc::new(services::waf::WafState::default()),
        config_service: Arc::new(services::config_service::ConfigServiceState::default()),
        organizations: Arc::new(services::organizations::OrganizationsState::default()),
        msk: Arc::new(services::msk::MskState::default()),
        textract: Arc::new(services::textract::TextractState::default()),
        translate: Arc::new(services::translate::TranslateState::default()),
        comprehend: Arc::new(services::comprehend::ComprehendState::default()),
        rekognition: Arc::new(services::rekognition::RekognitionState::default()),
        sagemaker: Arc::new(services::sagemaker::SageMakerState::default()),
        cloudtrail: Arc::new(services::cloudtrail::CloudTrailState::default()),
        codecommit: Arc::new(services::codecommit::CodeCommitState::default()),
        codedeploy: Arc::new(services::codedeploy::CodeDeployState::default()),
        documentdb: Arc::new(services::documentdb::DocumentDbState::default()),
        dms: Arc::new(services::dms::DmsState::default()),
        emr: Arc::new(services::emr::EmrState::default()),
        inspector: Arc::new(services::inspector::InspectorState::default()),
        lightsail: Arc::new(services::lightsail::LightsailState::default()),
        neptune: Arc::new(services::neptune::NeptuneState::default()),
        service_catalog: Arc::new(services::service_catalog::ServiceCatalogState::default()),
        shield: Arc::new(services::shield::ShieldState::default()),
        timestream: Arc::new(services::timestream::TimestreamState::default()),
        transfer: Arc::new(services::transfer::TransferState::default()),
        workspaces: Arc::new(services::workspaces::WorkSpacesState::default()),
        apprunner: Arc::new(services::apprunner::AppRunnerState::default()),
        dax: Arc::new(services::dax::DaxState::default()),
        fsx: Arc::new(services::fsx::FsxState::default()),
        keyspaces: Arc::new(services::keyspaces::KeyspacesState::default()),
        kendra: Arc::new(services::kendra::KendraState::default()),
        lakeformation: Arc::new(services::lakeformation::LakeFormationState::default()),
        memorydb: Arc::new(services::memorydb::MemoryDbState::default()),
        cloudmap: Arc::new(services::cloudmap::CloudMapState::default()),
        forecast: Arc::new(services::forecast::ForecastState::default()),
        personalize: Arc::new(services::personalize::PersonalizeState::default()),
        proton: Arc::new(services::proton::ProtonState::default()),
        sso: Arc::new(services::sso::SsoState::default()),
        ram: Arc::new(services::ram::RamState::default()),
        storage_gateway: Arc::new(services::storage_gateway::StorageGatewayState::default()),
        // JSON protocol (batch 6-10)
        accessanalyzer: Arc::new(services::accessanalyzer::AccessAnalyzerState::default()),
        acm_pca: Arc::new(services::acm_pca::AcmPcaState::default()),
        appflow: Arc::new(services::appflow::AppFlowState::default()),
        appstream: Arc::new(services::appstream::AppStreamState::default()),
        application_autoscaling: Arc::new(
            services::application_autoscaling::ApplicationAutoscalingState::default(),
        ),
        budgets: Arc::new(services::budgets::BudgetsState::default()),
        chatbot: Arc::new(services::chatbot::ChatbotState::default()),
        cloud9: Arc::new(services::cloud9::Cloud9State::default()),
        cloudcontrol: Arc::new(services::cloudcontrol::CloudControlState::default()),
        cloudhsm: Arc::new(services::cloudhsm::CloudHsmState::default()),
        codeguru: Arc::new(services::codeguru::CodeGuruState::default()),
        compute_optimizer: Arc::new(services::compute_optimizer::ComputeOptimizerState::default()),
        controltower: Arc::new(services::controltower::ControlTowerState::default()),
        costexplorer: Arc::new(services::costexplorer::CostExplorerState),
        cur: Arc::new(services::cur::CurState::default()),
        datapipeline: Arc::new(services::datapipeline::DataPipelineState::default()),
        datasync: Arc::new(services::datasync::DataSyncState::default()),
        devicefarm: Arc::new(services::devicefarm::DeviceFarmState::default()),
        directconnect: Arc::new(services::directconnect::DirectConnectState::default()),
        directory_service: Arc::new(services::directory_service::DirectoryServiceState::default()),
        firewall_manager: Arc::new(services::firewall_manager::FirewallManagerState::default()),
        frauddetector: Arc::new(services::frauddetector::FraudDetectorState::default()),
        gamelift: Arc::new(services::gamelift::GameLiftState::default()),
        globalaccelerator: Arc::new(services::globalaccelerator::GlobalAcceleratorState::default()),
        health: Arc::new(services::health::HealthState::default()),
        healthlake: Arc::new(services::healthlake::HealthLakeState::default()),
        identitystore: Arc::new(services::identitystore::IdentityStoreState::default()),
        ivs: Arc::new(services::ivs::IvsState::default()),
        license_manager: Arc::new(services::license_manager::LicenseManagerState::default()),
        mediastore: Arc::new(services::mediastore::MediaStoreState::default()),
        network_firewall: Arc::new(services::network_firewall::NetworkFirewallState::default()),
        pricing: Arc::new(services::pricing::PricingState),
        resiliencehub: Arc::new(services::resiliencehub::ResilienceHubState::default()),
        route53domains: Arc::new(services::route53domains::Route53DomainsState::default()),
        route53resolver: Arc::new(services::route53resolver::Route53ResolverState::default()),
        savingsplans: Arc::new(services::savingsplans::SavingsPlansState::default()),
        service_quotas: Arc::new(services::service_quotas::ServiceQuotasState::default()),
        snowball: Arc::new(services::snowball::SnowballState::default()),
        support: Arc::new(services::support::SupportState::default()),
        swf: Arc::new(services::swf::SwfState::default()),
        verifiedpermissions: Arc::new(
            services::verifiedpermissions::VerifiedPermissionsState::default(),
        ),
        workmail: Arc::new(services::workmail::WorkMailState::default()),
    };

    let dispatch_router: Router<()> = Router::<DispatchState>::new()
        .route("/", axum::routing::post(dispatch_handler))
        .fallback(dispatch_handler)
        .with_state(dispatch_state);

    Router::new()
        // Original REST routers
        .merge(lambda_router)
        .merge(apigateway_router)
        .merge(route53_router)
        .merge(eks_router)
        .merge(cloudfront_router)
        .merge(batch_router)
        .merge(backup_router)
        .merge(mq_router)
        .merge(xray_router)
        .merge(appsync_router)
        .merge(efs_router)
        .merge(guardduty_router)
        .nest("/iot", iot_router)
        .merge(macie_router)
        .merge(opensearch_router)
        .merge(polly_router)
        .merge(qldb_router)
        .merge(mediaconvert_router)
        .nest("/appconfig", appconfig_router)
        .merge(detective_router)
        .merge(amplify_router)
        .merge(lex_router)
        .merge(location_router)
        .merge(securityhub_router)
        .merge(bedrock_router)
        .merge(codeartifact_router)
        .merge(pinpoint_router)
        .merge(connect_router)
        .merge(glacier_router)
        .merge(medialive_router)
        .merge(quicksight_router)
        // Batch 6-10 REST routers
        .nest("/amp", amp_router)
        .merge(apigatewayv2_router)
        .merge(appmesh_router)
        .merge(auditmanager_router)
        .merge(braket_router)
        .merge(cleanrooms_router)
        .merge(customer_profiles_router)
        .merge(databrew_router)
        .merge(dataexchange_router)
        .merge(datazone_router)
        .nest("/devopsguru", devopsguru_router)
        .nest("/dlm", dlm_router)
        .merge(ebs_router)
        .nest("/emr-serverless", emr_serverless_router)
        .merge(entity_resolution_router)
        .merge(eventbridge_scheduler_router)
        .merge(finspace_router)
        .merge(fis_router)
        .merge(greengrass_router)
        .merge(groundstation_router)
        .merge(imagebuilder_router)
        .merge(internetmonitor_router)
        .nest("/m2", mainframe_router)
        .merge(managedblockchain_router)
        .nest("/grafana", managed_grafana_router)
        .nest("/mediapackage", mediapackage_router)
        .nest("/mediatailor", mediatailor_router)
        .nest("/mwaa", mwaa_router)
        .merge(networkmanager_router)
        .merge(omics_router)
        .merge(outposts_router)
        .merge(pipes_router)
        .merge(rolesanywhere_router)
        .merge(rum_router)
        .merge(schemas_router)
        .merge(securitylake_router)
        .nest("/serverlessrepo", serverlessrepo_router)
        .merge(synthetics_router)
        .merge(vpc_lattice_router)
        .merge(wellarchitected_router)
        .merge(workdocs_router)
        // Dashboard
        .merge(dashboard::router(dashboard_state.clone()))
        .route_service("/dashboard", ServeFile::new("ui/dist/index.html"))
        .route_service("/dashboard/", ServeFile::new("ui/dist/index.html"))
        .nest_service("/dashboard/assets", ServeDir::new("ui/dist/assets"))
        // Dispatch fallback + S3 catch-all
        .merge(dispatch_router)
        .merge(s3_router)
        .layer(axum::middleware::from_fn_with_state(
            dashboard_state,
            dashboard::request_logger,
        ))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

#[axum::debug_handler(state = DispatchState)]
async fn dispatch_handler(
    State(ds): State<DispatchState>,
    headers: axum::http::HeaderMap,
    uri: axum::http::Uri,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    // JSON protocol: dispatch by X-Amz-Target header
    if let Some(target) = headers.get("x-amz-target").and_then(|v| v.to_str().ok()) {
        let body_str = String::from_utf8_lossy(&body).to_string();
        let payload: serde_json::Value =
            serde_json::from_str(&body_str).unwrap_or(serde_json::Value::Null);

        // SQS (awsquery → awsjson migration in newer SDKs)
        if target.starts_with("AmazonSQS") {
            return services::sqs::handle_json_request(&ds.sqs, target, &payload);
        }

        // DynamoDB (special: takes &[u8])
        if target.starts_with("DynamoDB") {
            return services::dynamodb::handle_request(&ds.dynamodb, target, &body);
        }
        // Kinesis (special: takes &[u8])
        if target.starts_with("Kinesis_") {
            return services::kinesis::handle_request(&ds.kinesis, target, &body);
        }
        // EventBridge (special: takes &[u8])
        if target.starts_with("AWSEvents") {
            return services::eventbridge::handle_request(&ds.eventbridge, target, &body);
        }

        // All other JSON protocol services (takes &Value)
        // Order matters: more specific prefixes first (e.g., AmazonRDSv19_DocDB before AmazonRDSv19)
        if target.starts_with("Logs_") {
            return services::cloudwatch_logs::handle_request(&ds.cw, target, &payload).await;
        } else if target.starts_with("secretsmanager") {
            return services::secretsmanager::handle_request(&ds.sm, target, &payload).await;
        } else if target.starts_with("AmazonSSM") {
            return services::ssm::handle_request(&ds.ssm, target, &payload).await;
        } else if target.starts_with("AmazonEC2ContainerServiceV20141113") {
            return services::ecs::handle_request(&ds.ecs, target, &payload).await;
        } else if target.starts_with("AWSStepFunctions") {
            return services::stepfunctions::handle_request(&ds.sfn, target, &payload).await;
        } else if target.starts_with("TrentService") {
            return services::kms::handle_request(&ds.kms, target, &payload).await;
        } else if target.starts_with("CertificateManager") {
            return services::acm::handle_request(&ds.acm, target, &payload).await;
        } else if target.starts_with("AmazonRDSv19_DocDB") {
            return services::documentdb::handle_request(&ds.documentdb, target, &payload).await;
        } else if target.starts_with("AmazonRDSv19") {
            return services::rds::handle_request(&ds.rds, target, &payload).await;
        } else if target.starts_with("AmazonElastiCacheV9") {
            return services::elasticache::handle_request(&ds.elasticache, target, &payload).await;
        } else if target.starts_with("RedshiftServiceVersion20121201") {
            return services::redshift::handle_request(&ds.redshift, target, &payload).await;
        } else if target.starts_with("AWSCognitoIdentityProviderService") {
            return services::cognito::handle_request(&ds.cognito, target, &payload).await;
        } else if target.starts_with("CloudFormation_20100515") {
            return services::cloudformation::handle_request(&ds.cloudformation, target, &payload)
                .await;
        } else if target.starts_with("AmazonEC2ContainerRegistry") {
            return services::ecr::handle_request(&ds.ecr, target, &payload).await;
        } else if target.starts_with("ElasticLoadBalancingV2") {
            return services::elb::handle_request(&ds.elb, target, &payload).await;
        } else if target.starts_with("SimpleEmailServiceV2") {
            return services::ses::handle_request(&ds.ses, target, &payload).await;
        } else if target.starts_with("Firehose_20150804") {
            return services::firehose::handle_request(&ds.firehose, target, &payload).await;
        } else if target.starts_with("AWSGlue") {
            return services::glue::handle_request(&ds.glue, target, &payload).await;
        } else if target.starts_with("AmazonAthena") {
            return services::athena::handle_request(&ds.athena, target, &payload).await;
        } else if target.starts_with("CodeBuild_20161006") {
            return services::codebuild::handle_request(&ds.codebuild, target, &payload).await;
        } else if target.starts_with("CodePipeline_20150709") {
            return services::codepipeline::handle_request(&ds.codepipeline, target, &payload)
                .await;
        } else if target.starts_with("AWSWAF_20190729") {
            return services::waf::handle_request(&ds.waf, target, &payload).await;
        } else if target.starts_with("StarlingDoveService") {
            return services::config_service::handle_request(&ds.config_service, target, &payload)
                .await;
        } else if target.starts_with("AWSOrganizationsV20161128") {
            return services::organizations::handle_request(&ds.organizations, target, &payload)
                .await;
        } else if target.starts_with("Kafka") {
            return services::msk::handle_request(&ds.msk, target, &payload).await;
        } else if target.starts_with("Textract") {
            return services::textract::handle_request(&ds.textract, target, &payload).await;
        } else if target.starts_with("AWSShineFrontendService_20170701") {
            return services::translate::handle_request(&ds.translate, target, &payload).await;
        } else if target.starts_with("Comprehend_20171127") {
            return services::comprehend::handle_request(&ds.comprehend, target, &payload).await;
        } else if target.starts_with("RekognitionService") {
            return services::rekognition::handle_request(&ds.rekognition, target, &payload).await;
        } else if target.starts_with("SageMaker") {
            return services::sagemaker::handle_request(&ds.sagemaker, target, &payload).await;
        } else if target.starts_with("CloudTrail_20131101")
            || target.contains("CloudTrail_20131101")
        {
            return services::cloudtrail::handle_request(&ds.cloudtrail, target, &payload).await;
        } else if target.starts_with("CodeCommit_20150413") {
            return services::codecommit::handle_request(&ds.codecommit, target, &payload).await;
        } else if target.starts_with("CodeDeploy_20141006") {
            return services::codedeploy::handle_request(&ds.codedeploy, target, &payload).await;
        } else if target.starts_with("AmazonDMSv20160101") {
            return services::dms::handle_request(&ds.dms, target, &payload).await;
        } else if target.starts_with("ElasticMapReduce") {
            return services::emr::handle_request(&ds.emr, target, &payload).await;
        } else if target.starts_with("InspectorService") {
            return services::inspector::handle_request(&ds.inspector, target, &payload).await;
        } else if target.starts_with("Lightsail_20161128") {
            return services::lightsail::handle_request(&ds.lightsail, target, &payload).await;
        } else if target.starts_with("AmazonNeptuneV20171115") {
            return services::neptune::handle_request(&ds.neptune, target, &payload).await;
        } else if target.starts_with("AWS242ServiceCatalogService") {
            return services::service_catalog::handle_request(
                &ds.service_catalog,
                target,
                &payload,
            )
            .await;
        } else if target.starts_with("AWSShield_20160616") {
            return services::shield::handle_request(&ds.shield, target, &payload).await;
        } else if target.starts_with("Timestream_20181101") {
            return services::timestream::handle_request(&ds.timestream, target, &payload).await;
        } else if target.starts_with("TransferService") {
            return services::transfer::handle_request(&ds.transfer, target, &payload).await;
        } else if target.starts_with("WorkspacesService") {
            return services::workspaces::handle_request(&ds.workspaces, target, &payload).await;
        } else if target.starts_with("AppRunner") {
            return services::apprunner::handle_request(&ds.apprunner, target, &payload).await;
        } else if target.starts_with("AmazonDAXV3") {
            return services::dax::handle_request(&ds.dax, target, &payload).await;
        } else if target.starts_with("AWSSimbaAPIService_v20180301") {
            return services::fsx::handle_request(&ds.fsx, target, &payload).await;
        } else if target.starts_with("KeyspacesService") {
            return services::keyspaces::handle_request(&ds.keyspaces, target, &payload).await;
        } else if target.starts_with("AWSKendraFrontendService") {
            return services::kendra::handle_request(&ds.kendra, target, &payload).await;
        } else if target.starts_with("AWSLakeFormation") {
            return services::lakeformation::handle_request(&ds.lakeformation, target, &payload)
                .await;
        } else if target.starts_with("AmazonMemoryDB") {
            return services::memorydb::handle_request(&ds.memorydb, target, &payload).await;
        } else if target.starts_with("Route53AutoNaming_v20170314") {
            return services::cloudmap::handle_request(&ds.cloudmap, target, &payload).await;
        } else if target.starts_with("AmazonForecast") {
            return services::forecast::handle_request(&ds.forecast, target, &payload).await;
        } else if target.starts_with("AmazonPersonalize") {
            return services::personalize::handle_request(&ds.personalize, target, &payload).await;
        } else if target.starts_with("AwsProton20200720") {
            return services::proton::handle_request(&ds.proton, target, &payload).await;
        } else if target.starts_with("SWBExternalService") {
            return services::sso::handle_request(&ds.sso, target, &payload).await;
        } else if target.starts_with("AmazonResourceSharing") {
            return services::ram::handle_request(&ds.ram, target, &payload).await;
        } else if target.starts_with("StorageGateway_20130630") {
            return services::storage_gateway::handle_request(
                &ds.storage_gateway,
                target,
                &payload,
            )
            .await;
        // Batch 6-10 JSON services
        } else if target.starts_with("AccessAnalyzer") {
            return services::accessanalyzer::handle_request(&ds.accessanalyzer, target, &payload)
                .await;
        } else if target.starts_with("ACMPrivateCA") {
            return services::acm_pca::handle_request(&ds.acm_pca, target, &payload).await;
        } else if target.starts_with("SandstoneConfigurationServiceLambda") {
            return services::appflow::handle_request(&ds.appflow, target, &payload).await;
        } else if target.starts_with("PhotonAdminProxyService") {
            return services::appstream::handle_request(&ds.appstream, target, &payload).await;
        } else if target.starts_with("AnyScaleFrontendService") {
            return services::application_autoscaling::handle_request(
                &ds.application_autoscaling,
                target,
                &payload,
            )
            .await;
        } else if target.starts_with("AWSBudgetServiceGateway") {
            return services::budgets::handle_request(&ds.budgets, target, &payload).await;
        } else if target.starts_with("WheatleyOrchestration_20171011") {
            return services::chatbot::handle_request(&ds.chatbot, target, &payload).await;
        } else if target.starts_with("AWSCloud9WorkspaceManagementService") {
            return services::cloud9::handle_request(&ds.cloud9, target, &payload).await;
        } else if target.starts_with("CloudApiService") {
            return services::cloudcontrol::handle_request(&ds.cloudcontrol, target, &payload)
                .await;
        } else if target.starts_with("BaldrApiService") {
            return services::cloudhsm::handle_request(&ds.cloudhsm, target, &payload).await;
        } else if target.starts_with("CodeGuruProfilerService") {
            return services::codeguru::handle_request(&ds.codeguru, target, &payload).await;
        } else if target.starts_with("ComputeOptimizerService") {
            return services::compute_optimizer::handle_request(
                &ds.compute_optimizer,
                target,
                &payload,
            )
            .await;
        } else if target.starts_with("ControltowerService") {
            return services::controltower::handle_request(&ds.controltower, target, &payload)
                .await;
        } else if target.starts_with("AWSInsightsIndexService") {
            return services::costexplorer::handle_request(&ds.costexplorer, target, &payload)
                .await;
        } else if target.starts_with("AWSOrigamiServiceGatewayService") {
            return services::cur::handle_request(&ds.cur, target, &payload).await;
        } else if target.starts_with("DataPipeline") {
            return services::datapipeline::handle_request(&ds.datapipeline, target, &payload)
                .await;
        } else if target.starts_with("FmrsService") {
            return services::datasync::handle_request(&ds.datasync, target, &payload).await;
        } else if target.starts_with("DeviceFarm_20150623") {
            return services::devicefarm::handle_request(&ds.devicefarm, target, &payload).await;
        } else if target.starts_with("OvertureService") {
            return services::directconnect::handle_request(&ds.directconnect, target, &payload)
                .await;
        } else if target.starts_with("DirectoryService_20150416") {
            return services::directory_service::handle_request(
                &ds.directory_service,
                target,
                &payload,
            )
            .await;
        } else if target.starts_with("AWSFMS_20180101") {
            return services::firewall_manager::handle_request(
                &ds.firewall_manager,
                target,
                &payload,
            )
            .await;
        } else if target.starts_with("AWSHawksNestServiceFacade") {
            return services::frauddetector::handle_request(&ds.frauddetector, target, &payload)
                .await;
        } else if target.starts_with("GameLift") {
            return services::gamelift::handle_request(&ds.gamelift, target, &payload).await;
        } else if target.starts_with("GlobalAccelerator_V20180706") {
            return services::globalaccelerator::handle_request(
                &ds.globalaccelerator,
                target,
                &payload,
            )
            .await;
        } else if target.starts_with("AWSHealth_20160804") {
            return services::health::handle_request(&ds.health, target, &payload).await;
        } else if target.starts_with("HealthLake") {
            return services::healthlake::handle_request(&ds.healthlake, target, &payload).await;
        } else if target.starts_with("AWSIdentityStore") {
            return services::identitystore::handle_request(&ds.identitystore, target, &payload)
                .await;
        } else if target.starts_with("AmazonInteractiveVideoService") {
            return services::ivs::handle_request(&ds.ivs, target, &payload).await;
        } else if target.starts_with("AWSLicenseManager") {
            return services::license_manager::handle_request(
                &ds.license_manager,
                target,
                &payload,
            )
            .await;
        } else if target.starts_with("MediaStore_20170901") {
            return services::mediastore::handle_request(&ds.mediastore, target, &payload).await;
        } else if target.starts_with("NetworkFirewall_20201112") {
            return services::network_firewall::handle_request(
                &ds.network_firewall,
                target,
                &payload,
            )
            .await;
        } else if target.starts_with("AWSPriceListService") {
            return services::pricing::handle_request(&ds.pricing, target, &payload).await;
        } else if target.starts_with("AwsResilienceHub") {
            return services::resiliencehub::handle_request(&ds.resiliencehub, target, &payload)
                .await;
        } else if target.starts_with("Route53Domains_v20140515") {
            return services::route53domains::handle_request(&ds.route53domains, target, &payload)
                .await;
        } else if target.starts_with("Route53Resolver") {
            return services::route53resolver::handle_request(
                &ds.route53resolver,
                target,
                &payload,
            )
            .await;
        } else if target.starts_with("AWSSavingsPlan") {
            return services::savingsplans::handle_request(&ds.savingsplans, target, &payload)
                .await;
        } else if target.starts_with("ServiceQuotasV20190624") {
            return services::service_quotas::handle_request(&ds.service_quotas, target, &payload)
                .await;
        } else if target.starts_with("AWSIESnowballJobManagementService") {
            return services::snowball::handle_request(&ds.snowball, target, &payload).await;
        } else if target.starts_with("AWSSupport_20130415") {
            return services::support::handle_request(&ds.support, target, &payload).await;
        } else if target.starts_with("SimpleWorkflowService") {
            return services::swf::handle_request(&ds.swf, target, &payload).await;
        } else if target.starts_with("VerifiedPermissions") {
            return services::verifiedpermissions::handle_request(
                &ds.verifiedpermissions,
                target,
                &payload,
            )
            .await;
        } else if target.starts_with("WorkMailService") {
            return services::workmail::handle_request(&ds.workmail, target, &payload).await;
        }
    }

    // Query protocol: dispatch by Action parameter
    // Also extract action from X-Amz-Target header (e.g. "AmazonSQS.ListQueues" → "ListQueues")
    // for services that migrated from query protocol to JSON protocol in newer AWS SDKs.
    let target_action = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        .and_then(|t| t.split('.').next_back())
        .map(|a| a.to_string());
    if let Some(action) = extract_action(&body, &uri).or(target_action) {
        let sqs_actions = [
            "CreateQueue",
            "DeleteQueue",
            "ListQueues",
            "SendMessage",
            "ReceiveMessage",
            "DeleteMessage",
            "GetQueueUrl",
            "PurgeQueue",
        ];
        let sns_actions = [
            "CreateTopic",
            "DeleteTopic",
            "ListTopics",
            "Subscribe",
            "Unsubscribe",
            "ListSubscriptions",
            "Publish",
        ];
        let iam_actions = [
            "CreateUser",
            "DeleteUser",
            "ListUsers",
            "GetUser",
            "CreateRole",
            "DeleteRole",
            "GetRole",
            "ListRoles",
            "ListAttachedRolePolicies",
            "CreatePolicy",
            "DeletePolicy",
            "ListPolicies",
            "AttachRolePolicy",
            "DetachRolePolicy",
            "ListAttachedUserPolicies",
            "ListGroupsForUser",
            "ListAccessKeys",
            "ListGroups",
            "GetGroup",
        ];
        let sts_actions = ["GetCallerIdentity", "AssumeRole"];
        let ec2_actions = [
            "RunInstances",
            "DescribeInstances",
            "TerminateInstances",
            "StartInstances",
            "StopInstances",
            "RebootInstances",
            "DescribeSecurityGroups",
            "DescribeVpcs",
            "DescribeSubnets",
            "DescribeImages",
            "DeregisterImage",
            "DescribeVolumes",
            "DeleteVolume",
            "DescribeSnapshots",
            "DeleteSnapshot",
        ];
        let cw_metric_actions = [
            "PutMetricData",
            "ListMetrics",
            "GetMetricData",
            "PutMetricAlarm",
            "DescribeAlarms",
            "DeleteAlarms",
        ];
        let autoscaling_actions = [
            "CreateAutoScalingGroup",
            "DeleteAutoScalingGroup",
            "DescribeAutoScalingGroups",
            "UpdateAutoScalingGroup",
            "SetDesiredCapacity",
            "CreateLaunchConfiguration",
            "DescribeLaunchConfigurations",
        ];
        let elasticbeanstalk_actions = [
            "CreateApplication",
            "DeleteApplication",
            "DescribeApplications",
            "CreateEnvironment",
            "TerminateEnvironment",
            "DescribeEnvironments",
        ];
        let cloudsearch_actions = [
            "CreateDomain",
            "DeleteDomain",
            "DescribeDomains",
            "ListDomainNames",
            "IndexDocuments",
        ];
        let rds_actions = [
            "CreateDBInstance",
            "DeleteDBInstance",
            "DescribeDBInstances",
            "StartDBInstance",
            "StopDBInstance",
            "RebootDBInstance",
            "DescribeDBSnapshots",
            "DeleteDBSnapshot",
        ];
        let elb_actions = [
            "CreateLoadBalancer",
            "DeleteLoadBalancer",
            "DescribeLoadBalancers",
            "CreateTargetGroup",
            "DeleteTargetGroup",
            "DescribeTargetGroups",
            "RegisterTargets",
            "DeregisterTargets",
            "CreateListener",
            "DescribeListeners",
            "DeleteListener",
            "DescribeRules",
            "DeleteRule",
            "DescribeTargetHealth",
        ];
        let cloudformation_actions = [
            "CreateStack",
            "DeleteStack",
            "DescribeStacks",
            "ListStacks",
            "UpdateStack",
            "DescribeStackResources",
        ];
        let elasticache_actions = [
            "CreateCacheCluster",
            "DeleteCacheCluster",
            "DescribeCacheClusters",
            "CreateReplicationGroup",
            "DeleteReplicationGroup",
            "DescribeReplicationGroups",
        ];

        if sqs_actions.contains(&action.as_str()) {
            return services::sqs::handle_request(&ds.sqs, &headers, &body, &uri);
        } else if sns_actions.contains(&action.as_str()) {
            return services::sns::handle_request(&ds.sns, &headers, &body, &uri);
        } else if iam_actions.contains(&action.as_str()) {
            return services::iam::handle_request(&ds.iam, &headers, &body, &uri);
        } else if sts_actions.contains(&action.as_str()) {
            return services::sts::handle_request(&ds.sts, &body);
        } else if ec2_actions.contains(&action.as_str()) {
            return services::ec2::handle_request(&ds.ec2, &headers, &body, &uri);
        } else if cw_metric_actions.contains(&action.as_str()) {
            return services::cloudwatch::handle_request(&ds.cloudwatch, &headers, &body, &uri);
        } else if autoscaling_actions.contains(&action.as_str()) {
            return services::autoscaling::handle_request(&ds.autoscaling, &headers, &body, &uri);
        } else if elasticbeanstalk_actions.contains(&action.as_str()) {
            return services::elasticbeanstalk::handle_request(
                &ds.elasticbeanstalk,
                &headers,
                &body,
                &uri,
            );
        } else if cloudsearch_actions.contains(&action.as_str()) {
            return services::cloudsearch::handle_request(&ds.cloudsearch, &headers, &body, &uri);
        } else if rds_actions.contains(&action.as_str()) {
            return services::rds::handle_query_request(&ds.rds, &headers, &body, &uri);
        } else if elb_actions.contains(&action.as_str()) {
            return services::elb::handle_query_request(&ds.elb, &headers, &body, &uri);
        } else if cloudformation_actions.contains(&action.as_str()) {
            return services::cloudformation::handle_query_request(
                &ds.cloudformation,
                &headers,
                &body,
                &uri,
            );
        } else if elasticache_actions.contains(&action.as_str()) {
            return services::elasticache::handle_query_request(
                &ds.elasticache,
                &headers,
                &body,
                &uri,
            );
        }
    }

    (
        axum::http::StatusCode::NOT_FOUND,
        "Unknown service or action",
    )
        .into_response()
}

fn extract_action(body: &[u8], uri: &axum::http::Uri) -> Option<String> {
    let body_str = std::str::from_utf8(body).unwrap_or("");
    for pair in form_urlencoded::parse(body_str.as_bytes()) {
        if pair.0 == "Action" {
            return Some(pair.1.to_string());
        }
    }
    if let Some(query) = uri.query() {
        for pair in form_urlencoded::parse(query.as_bytes()) {
            if pair.0 == "Action" {
                return Some(pair.1.to_string());
            }
        }
    }
    None
}
