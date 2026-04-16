#![allow(dead_code)]

mod config;
mod dashboard;
mod error;
mod persistence;
mod protocol;
mod services;
mod storage;

use std::sync::Arc;

use axum::extract::State;
use axum::Router;
use clap::Parser;
use tower_http::cors::CorsLayer;
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
    // REST-based services shared for the dashboard resources API
    s3: Arc<services::s3::S3State>,
    lambda: Arc<services::lambda::LambdaState>,
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

    // Handle --persist / --reset
    let db: Option<Arc<persistence::SqliteStore>> = if config.persist {
        let db_path = config.resolve_db_path();
        if config.reset {
            if let Err(e) = persistence::SqliteStore::reset(&db_path) {
                tracing::error!("Failed to reset database: {e}");
            } else {
                info!("Database reset: {db_path}");
            }
        }
        match persistence::SqliteStore::open(&db_path) {
            Ok(store) => {
                info!("Persistence enabled: {db_path}");
                Some(Arc::new(store))
            }
            Err(e) => {
                tracing::error!("Failed to open database: {e} — running in-memory only");
                None
            }
        }
    } else {
        None
    };

    let dashboard_state = DashboardState::new();
    let app = build_router(&config, dashboard_state.clone(), db.clone());

    info!("laws v{} starting on {}", env!("CARGO_PKG_VERSION"), addr);
    info!("Region: {}, Account: {}", config.region, config.account_id);
    info!("184 AWS services ready");
    info!("Dashboard: http://{}/dashboard", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn build_router(
    _config: &Config,
    dashboard_state: DashboardState,
    _db: Option<Arc<persistence::SqliteStore>>,
) -> Router {
    // ── REST-based services (original) ──
    let s3_state = Arc::new(services::s3::S3State::new(&_db));
    let lambda_state = Arc::new(services::lambda::LambdaState::new(&_db));
    let s3_router = services::s3::router(s3_state.clone());
    let lambda_router = services::lambda::router(lambda_state.clone());
    let apigateway_router =
        services::apigateway::router(Arc::new(services::apigateway::ApiGatewayState::new(&_db)));
    let route53_router =
        services::route53::router(Arc::new(services::route53::Route53State::default()));
    let eks_router = services::eks::router(Arc::new(services::eks::EksState::new(&_db)));
    let cloudfront_router =
        services::cloudfront::router(Arc::new(services::cloudfront::CloudFrontState::new(&_db)));
    let batch_router = services::batch::router(Arc::new(services::batch::BatchState::new(&_db)));
    let backup_router =
        services::backup::router(Arc::new(services::backup::BackupState::new(&_db)));
    let mq_router = services::mq::router(Arc::new(services::mq::MqState::new(&_db)));
    let xray_router = services::xray::router(Arc::new(services::xray::XRayState::new(&_db)));
    let appsync_router =
        services::appsync::router(Arc::new(services::appsync::AppSyncState::new(&_db)));
    let efs_router = services::efs::router(Arc::new(services::efs::EfsState::new(&_db)));
    let guardduty_router =
        services::guardduty::router(Arc::new(services::guardduty::GuardDutyState::new(&_db)));
    let iot_router = services::iot::router(Arc::new(services::iot::IotState::new(&_db)));
    let macie_router = services::macie::router(Arc::new(services::macie::MacieState::new(&_db)));
    let opensearch_router =
        services::opensearch::router(Arc::new(services::opensearch::OpenSearchState::new(&_db)));
    let polly_router = services::polly::router(Arc::new(services::polly::PollyState::new(&_db)));
    let qldb_router = services::qldb::router(Arc::new(services::qldb::QldbState::new(&_db)));
    let mediaconvert_router = services::mediaconvert::router(Arc::new(
        services::mediaconvert::MediaConvertState::new(&_db),
    ));
    let appconfig_router =
        services::appconfig::router(Arc::new(services::appconfig::AppConfigState::new(&_db)));
    let detective_router =
        services::detective::router(Arc::new(services::detective::DetectiveState::new(&_db)));
    let amplify_router =
        services::amplify::router(Arc::new(services::amplify::AmplifyState::new(&_db)));
    let lex_router = services::lex::router(Arc::new(services::lex::LexState::new(&_db)));
    let location_router =
        services::location::router(Arc::new(services::location::LocationState::new(&_db)));
    let securityhub_router =
        services::securityhub::router(Arc::new(services::securityhub::SecurityHubState::new(&_db)));
    let bedrock_router =
        services::bedrock::router(Arc::new(services::bedrock::BedrockState::new(&_db)));
    let codeartifact_router = services::codeartifact::router(Arc::new(
        services::codeartifact::CodeArtifactState::new(&_db),
    ));
    let pinpoint_router =
        services::pinpoint::router(Arc::new(services::pinpoint::PinpointState::new(&_db)));
    let connect_router =
        services::connect::router(Arc::new(services::connect::ConnectState::new(&_db)));
    let glacier_router =
        services::glacier::router(Arc::new(services::glacier::GlacierState::new(&_db)));
    let medialive_router =
        services::medialive::router(Arc::new(services::medialive::MediaLiveState::new(&_db)));
    let quicksight_router =
        services::quicksight::router(Arc::new(services::quicksight::QuickSightState::new(&_db)));

    // ── REST-based services (batch 6-10) ──
    let amp_router = services::amp::router(Arc::new(services::amp::AmpState::new(&_db)));
    let apigatewayv2_router = services::apigatewayv2::router(Arc::new(
        services::apigatewayv2::ApiGatewayV2State::default(),
    ));
    let appmesh_router =
        services::appmesh::router(Arc::new(services::appmesh::AppMeshState::new(&_db)));
    let auditmanager_router = services::auditmanager::router(Arc::new(
        services::auditmanager::AuditManagerState::new(&_db),
    ));
    let braket_router =
        services::braket::router(Arc::new(services::braket::BraketState::new(&_db)));
    let cleanrooms_router =
        services::cleanrooms::router(Arc::new(services::cleanrooms::CleanRoomsState::new(&_db)));
    let customer_profiles_router = services::customer_profiles::router(Arc::new(
        services::customer_profiles::CustomerProfilesState::new(&_db),
    ));
    let databrew_router =
        services::databrew::router(Arc::new(services::databrew::DataBrewState::new(&_db)));
    let dataexchange_router = services::dataexchange::router(Arc::new(
        services::dataexchange::DataExchangeState::new(&_db),
    ));
    let datazone_router =
        services::datazone::router(Arc::new(services::datazone::DataZoneState::new(&_db)));
    let devopsguru_router =
        services::devopsguru::router(Arc::new(services::devopsguru::DevOpsGuruState::new(&_db)));
    let dlm_router = services::dlm::router(Arc::new(services::dlm::DlmState::new(&_db)));
    let ebs_router = services::ebs::router(Arc::new(services::ebs::EbsState::new(&_db)));
    let emr_serverless_router = services::emr_serverless::router(Arc::new(
        services::emr_serverless::EmrServerlessState::new(&_db),
    ));
    let entity_resolution_router = services::entity_resolution::router(Arc::new(
        services::entity_resolution::EntityResolutionState::new(&_db),
    ));
    let eventbridge_scheduler_router = services::eventbridge_scheduler::router(Arc::new(
        services::eventbridge_scheduler::EventBridgeSchedulerState::new(&_db),
    ));
    let finspace_router =
        services::finspace::router(Arc::new(services::finspace::FinSpaceState::new(&_db)));
    let fis_router = services::fis::router(Arc::new(services::fis::FisState::new(&_db)));
    let greengrass_router =
        services::greengrass::router(Arc::new(services::greengrass::GreengrassState::new(&_db)));
    let groundstation_router = services::groundstation::router(Arc::new(
        services::groundstation::GroundStationState::new(&_db),
    ));
    let imagebuilder_router = services::imagebuilder::router(Arc::new(
        services::imagebuilder::ImageBuilderState::new(&_db),
    ));
    let internetmonitor_router = services::internetmonitor::router(Arc::new(
        services::internetmonitor::InternetMonitorState::new(&_db),
    ));
    let mainframe_router =
        services::mainframe::router(Arc::new(services::mainframe::MainframeState::new(&_db)));
    let managedblockchain_router = services::managedblockchain::router(Arc::new(
        services::managedblockchain::ManagedBlockchainState::new(&_db),
    ));
    let managed_grafana_router = services::managed_grafana::router(Arc::new(
        services::managed_grafana::ManagedGrafanaState::new(&_db),
    ));
    let mediapackage_router = services::mediapackage::router(Arc::new(
        services::mediapackage::MediaPackageState::new(&_db),
    ));
    let mediatailor_router =
        services::mediatailor::router(Arc::new(services::mediatailor::MediaTailorState::new(&_db)));
    let mwaa_router = services::mwaa::router(Arc::new(services::mwaa::MwaaState::new(&_db)));
    let networkmanager_router = services::networkmanager::router(Arc::new(
        services::networkmanager::NetworkManagerState::new(&_db),
    ));
    let omics_router = services::omics::router(Arc::new(services::omics::OmicsState::new(&_db)));
    let outposts_router =
        services::outposts::router(Arc::new(services::outposts::OutpostsState::new(&_db)));
    let pipes_router = services::pipes::router(Arc::new(services::pipes::PipesState::new(&_db)));
    let rolesanywhere_router = services::rolesanywhere::router(Arc::new(
        services::rolesanywhere::RolesAnywhereState::new(&_db),
    ));
    let rum_router = services::rum::router(Arc::new(services::rum::RumState::new(&_db)));
    let schemas_router =
        services::schemas::router(Arc::new(services::schemas::SchemasState::new(&_db)));
    let securitylake_router = services::securitylake::router(Arc::new(
        services::securitylake::SecurityLakeState::new(&_db),
    ));
    let serverlessrepo_router = services::serverlessrepo::router(Arc::new(
        services::serverlessrepo::ServerlessRepoState::new(&_db),
    ));
    let synthetics_router =
        services::synthetics::router(Arc::new(services::synthetics::SyntheticsState::new(&_db)));
    let vpc_lattice_router =
        services::vpc_lattice::router(Arc::new(services::vpc_lattice::VpcLatticeState::new(&_db)));
    let wellarchitected_router = services::wellarchitected::router(Arc::new(
        services::wellarchitected::WellArchitectedState::new(&_db),
    ));
    let workdocs_router =
        services::workdocs::router(Arc::new(services::workdocs::WorkDocsState::new(&_db)));

    // ── Dispatch-based services (JSON + Query protocol) ──
    let dispatch_state = DispatchState {
        // Query protocol
        sqs: Arc::new(services::sqs::SqsState::new(&_db)),
        sns: Arc::new(services::sns::SnsState::new(&_db)),
        iam: Arc::new(services::iam::IamState::new(&_db)),
        sts: Arc::new(services::sts::StsState::default()),
        ec2: Arc::new(services::ec2::Ec2State::new(
            _config.account_id.clone(),
            _config.region.clone(),
            &_db,
        )),
        cloudwatch: Arc::new(services::cloudwatch::CloudWatchState::new(&_db)),
        autoscaling: Arc::new(services::autoscaling::AutoScalingState::new(&_db)),
        elasticbeanstalk: Arc::new(services::elasticbeanstalk::ElasticBeanstalkState::new(&_db)),
        cloudsearch: Arc::new(services::cloudsearch::CloudSearchState::new(&_db)),
        // JSON protocol (original)
        dynamodb: Arc::new(services::dynamodb::DynamoDbState::new(&_db)),
        cw: Arc::new(services::cloudwatch_logs::CloudWatchLogsState::new(&_db)),
        sm: Arc::new(services::secretsmanager::SecretsManagerState::new(&_db)),
        ssm: Arc::new(services::ssm::SsmState::new(&_db)),
        ecs: Arc::new(services::ecs::EcsState::new(&_db)),
        sfn: Arc::new(services::stepfunctions::StepFunctionsState::new(&_db)),
        kinesis: Arc::new(services::kinesis::KinesisState::new(&_db)),
        eventbridge: Arc::new(services::eventbridge::EventBridgeState::new(&_db)),
        kms: Arc::new(services::kms::KmsState::new(&_db)),
        acm: Arc::new(services::acm::AcmState::new(&_db)),
        rds: Arc::new(services::rds::RdsState::new(&_db)),
        elasticache: Arc::new(services::elasticache::ElastiCacheState::new(&_db)),
        redshift: Arc::new(services::redshift::RedshiftState::new(&_db)),
        cognito: Arc::new(services::cognito::CognitoState::new(&_db)),
        cloudformation: Arc::new(services::cloudformation::CloudFormationState::new(&_db)),
        ecr: Arc::new(services::ecr::EcrState::new(&_db)),
        elb: Arc::new(services::elb::ElbState::new(&_db)),
        ses: Arc::new(services::ses::SesState::new(&_db)),
        firehose: Arc::new(services::firehose::FirehoseState::new(&_db)),
        glue: Arc::new(services::glue::GlueState::new(&_db)),
        athena: Arc::new(services::athena::AthenaState::new(&_db)),
        codebuild: Arc::new(services::codebuild::CodeBuildState::new(&_db)),
        codepipeline: Arc::new(services::codepipeline::CodePipelineState::new(&_db)),
        waf: Arc::new(services::waf::WafState::new(&_db)),
        config_service: Arc::new(services::config_service::ConfigServiceState::new(&_db)),
        organizations: Arc::new(services::organizations::OrganizationsState::new(&_db)),
        msk: Arc::new(services::msk::MskState::new(&_db)),
        textract: Arc::new(services::textract::TextractState::new(&_db)),
        translate: Arc::new(services::translate::TranslateState::new(&_db)),
        comprehend: Arc::new(services::comprehend::ComprehendState::new(&_db)),
        rekognition: Arc::new(services::rekognition::RekognitionState::new(&_db)),
        sagemaker: Arc::new(services::sagemaker::SageMakerState::new(&_db)),
        cloudtrail: Arc::new(services::cloudtrail::CloudTrailState::new(&_db)),
        codecommit: Arc::new(services::codecommit::CodeCommitState::new(&_db)),
        codedeploy: Arc::new(services::codedeploy::CodeDeployState::new(&_db)),
        documentdb: Arc::new(services::documentdb::DocumentDbState::new(&_db)),
        dms: Arc::new(services::dms::DmsState::new(&_db)),
        emr: Arc::new(services::emr::EmrState::new(&_db)),
        inspector: Arc::new(services::inspector::InspectorState::new(&_db)),
        lightsail: Arc::new(services::lightsail::LightsailState::new(&_db)),
        neptune: Arc::new(services::neptune::NeptuneState::new(&_db)),
        service_catalog: Arc::new(services::service_catalog::ServiceCatalogState::new(&_db)),
        shield: Arc::new(services::shield::ShieldState::new(&_db)),
        timestream: Arc::new(services::timestream::TimestreamState::new(&_db)),
        transfer: Arc::new(services::transfer::TransferState::new(&_db)),
        workspaces: Arc::new(services::workspaces::WorkSpacesState::new(&_db)),
        apprunner: Arc::new(services::apprunner::AppRunnerState::new(&_db)),
        dax: Arc::new(services::dax::DaxState::new(&_db)),
        fsx: Arc::new(services::fsx::FsxState::new(&_db)),
        keyspaces: Arc::new(services::keyspaces::KeyspacesState::new(&_db)),
        kendra: Arc::new(services::kendra::KendraState::new(&_db)),
        lakeformation: Arc::new(services::lakeformation::LakeFormationState::new(&_db)),
        memorydb: Arc::new(services::memorydb::MemoryDbState::new(&_db)),
        cloudmap: Arc::new(services::cloudmap::CloudMapState::new(&_db)),
        forecast: Arc::new(services::forecast::ForecastState::new(&_db)),
        personalize: Arc::new(services::personalize::PersonalizeState::new(&_db)),
        proton: Arc::new(services::proton::ProtonState::new(&_db)),
        sso: Arc::new(services::sso::SsoState::new(&_db)),
        ram: Arc::new(services::ram::RamState::new(&_db)),
        storage_gateway: Arc::new(services::storage_gateway::StorageGatewayState::new(&_db)),
        // JSON protocol (batch 6-10)
        accessanalyzer: Arc::new(services::accessanalyzer::AccessAnalyzerState::new(&_db)),
        acm_pca: Arc::new(services::acm_pca::AcmPcaState::new(&_db)),
        appflow: Arc::new(services::appflow::AppFlowState::new(&_db)),
        appstream: Arc::new(services::appstream::AppStreamState::new(&_db)),
        application_autoscaling: Arc::new(
            services::application_autoscaling::ApplicationAutoscalingState::new(&_db),
        ),
        budgets: Arc::new(services::budgets::BudgetsState::new(&_db)),
        chatbot: Arc::new(services::chatbot::ChatbotState::new(&_db)),
        cloud9: Arc::new(services::cloud9::Cloud9State::default()),
        cloudcontrol: Arc::new(services::cloudcontrol::CloudControlState::new(&_db)),
        cloudhsm: Arc::new(services::cloudhsm::CloudHsmState::new(&_db)),
        codeguru: Arc::new(services::codeguru::CodeGuruState::new(&_db)),
        compute_optimizer: Arc::new(services::compute_optimizer::ComputeOptimizerState::new(
            &_db,
        )),
        controltower: Arc::new(services::controltower::ControlTowerState::new(&_db)),
        costexplorer: Arc::new(services::costexplorer::CostExplorerState),
        cur: Arc::new(services::cur::CurState::new(&_db)),
        datapipeline: Arc::new(services::datapipeline::DataPipelineState::new(&_db)),
        datasync: Arc::new(services::datasync::DataSyncState::new(&_db)),
        devicefarm: Arc::new(services::devicefarm::DeviceFarmState::new(&_db)),
        directconnect: Arc::new(services::directconnect::DirectConnectState::new(&_db)),
        directory_service: Arc::new(services::directory_service::DirectoryServiceState::new(
            &_db,
        )),
        firewall_manager: Arc::new(services::firewall_manager::FirewallManagerState::new(&_db)),
        frauddetector: Arc::new(services::frauddetector::FraudDetectorState::new(&_db)),
        gamelift: Arc::new(services::gamelift::GameLiftState::new(&_db)),
        globalaccelerator: Arc::new(services::globalaccelerator::GlobalAcceleratorState::new(
            &_db,
        )),
        health: Arc::new(services::health::HealthState::new(&_db)),
        healthlake: Arc::new(services::healthlake::HealthLakeState::new(&_db)),
        identitystore: Arc::new(services::identitystore::IdentityStoreState::new(&_db)),
        ivs: Arc::new(services::ivs::IvsState::new(&_db)),
        license_manager: Arc::new(services::license_manager::LicenseManagerState::new(&_db)),
        mediastore: Arc::new(services::mediastore::MediaStoreState::new(&_db)),
        network_firewall: Arc::new(services::network_firewall::NetworkFirewallState::new(&_db)),
        pricing: Arc::new(services::pricing::PricingState),
        resiliencehub: Arc::new(services::resiliencehub::ResilienceHubState::new(&_db)),
        route53domains: Arc::new(services::route53domains::Route53DomainsState::default()),
        route53resolver: Arc::new(services::route53resolver::Route53ResolverState::default()),
        savingsplans: Arc::new(services::savingsplans::SavingsPlansState::new(&_db)),
        service_quotas: Arc::new(services::service_quotas::ServiceQuotasState::new(&_db)),
        snowball: Arc::new(services::snowball::SnowballState::new(&_db)),
        support: Arc::new(services::support::SupportState::new(&_db)),
        swf: Arc::new(services::swf::SwfState::new(&_db)),
        verifiedpermissions: Arc::new(
            services::verifiedpermissions::VerifiedPermissionsState::new(&_db),
        ),
        workmail: Arc::new(services::workmail::WorkMailState::new(&_db)),
        s3: s3_state,
        lambda: lambda_state,
    };

    let dispatch_router: Router<()> = Router::<DispatchState>::new()
        .route("/", axum::routing::post(dispatch_handler))
        .route(
            "/api/dashboard/resources/{service}",
            axum::routing::get(resources_handler),
        )
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
        .nest("/dashboard", {
            static_serve::embed_assets!(
                "ui/dist",
                strip_html_ext = true, // don't require explicit "index.html"
                cache_busted_paths = ["assets"]
            );
            static_router()
                // Don't use below fallbacks
                .fallback(|| async { (http::StatusCode::NOT_FOUND, "Dashboard asset not found") })
        })
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

// ---------------------------------------------------------------------------
// Dashboard resources API
// ---------------------------------------------------------------------------

async fn resources_handler(
    State(ds): State<DispatchState>,
    axum::extract::Path(service): axum::extract::Path<String>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    use serde_json::json;

    let resources: serde_json::Value = match service.to_lowercase().as_str() {
        "ec2" => {
            let instances: Vec<serde_json::Value> = ds
                .ec2
                .instances
                .iter()
                .map(|entry| {
                    let i = entry.value();
                    json!({
                        "instanceId": i.instance_id,
                        "state": i.state,
                        "instanceType": i.instance_type,
                        "imageId": i.image_id,
                        "launchTime": i.launch_time,
                        "vpcId": i.vpc_id,
                        "subnetId": i.subnet_id,
                        "privateIpAddress": i.private_ip_address,
                    })
                })
                .collect();
            json!({ "service": "EC2", "resourceType": "Instances", "resources": instances })
        }
        "s3" => {
            let buckets: Vec<serde_json::Value> = ds
                .s3
                .buckets
                .list()
                .iter()
                .map(|(_key, b)| {
                    json!({
                        "name": b.name,
                        "creationDate": b.creation_date,
                    })
                })
                .collect();
            json!({ "service": "S3", "resourceType": "Buckets", "resources": buckets })
        }
        "sqs" => {
            let queues: Vec<serde_json::Value> = ds
                .sqs
                .queues
                .iter()
                .map(|entry| {
                    let q = entry.value();
                    json!({
                        "queueName": q.name,
                        "queueUrl": q.url,
                        "messageCount": q.messages.len(),
                    })
                })
                .collect();
            json!({ "service": "SQS", "resourceType": "Queues", "resources": queues })
        }
        "dynamodb" => {
            let tables: Vec<serde_json::Value> = ds
                .dynamodb
                .tables
                .iter()
                .map(|entry| {
                    let t = entry.value();
                    json!({
                        "tableName": t.table_name,
                        "status": t.status,
                        "itemCount": t.items.len(),
                    })
                })
                .collect();
            json!({ "service": "DynamoDB", "resourceType": "Tables", "resources": tables })
        }
        "lambda" => {
            let functions: Vec<serde_json::Value> = ds
                .lambda
                .functions
                .list()
                .iter()
                .map(|(_key, f)| {
                    json!({
                        "functionName": f.function_name,
                        "runtime": f.runtime,
                        "handler": f.handler,
                        "lastModified": f.last_modified,
                    })
                })
                .collect();
            json!({ "service": "Lambda", "resourceType": "Functions", "resources": functions })
        }
        _ => {
            json!({ "service": service, "resourceType": "Unknown", "resources": [] })
        }
    };

    (
        axum::http::StatusCode::OK,
        [("content-type", "application/json")],
        serde_json::to_string(&resources).unwrap_or_default(),
    )
        .into_response()
}
