# laws

**L**ocal **AWS** — A lightweight AWS service emulator written in Rust.

laws emulates AWS services locally for development and testing. No Docker required, no Python runtime — just a single binary.

### Demo

[![I Built a Local AWS Emulator in Rust with 184 Services and a Real-Time Dashboard](https://img.youtube.com/vi/rWvwflxIy3I/maxresdefault.jpg)](https://www.youtube.com/watch?v=rWvwflxIy3I)

<details>
<summary>Supported Services (184)</summary>

| # | Service | Protocol | Key Operations |
|---|---------|----------|----------------|
| 1 | Access Analyzer | JSON | CreateAnalyzer, ListAnalyzers, GetAnalyzer, ListFindings |
| 2 | ACM | JSON | RequestCertificate, DescribeCertificate, ListCertificates, DeleteCertificate |
| 3 | ACM PCA | JSON | CreateCertificateAuthority, IssueCertificate, GetCertificate |
| 4 | Amplify | REST-JSON | CreateApp, ListApps, GetApp, DeleteApp, CreateBranch |
| 5 | API Gateway | REST-JSON | CreateRestApi, GetRestApis, CreateResource, PutMethod |
| 6 | API Gateway V2 | REST-JSON | CreateApi, GetApis, GetApi, DeleteApi, CreateRoute |
| 7 | App Mesh | REST-JSON | CreateMesh, ListMeshes, CreateVirtualNode, CreateVirtualService |
| 8 | AppConfig | REST-JSON | CreateApplication, GetApplication, CreateEnvironment |
| 9 | AppFlow | JSON | CreateFlow, DeleteFlow, DescribeFlow, ListFlows, StartFlow |
| 10 | Application Auto Scaling | JSON | RegisterScalableTarget, PutScalingPolicy, DescribeScalableTargets |
| 11 | App Runner | JSON | CreateService, DeleteService, DescribeService, ListServices |
| 12 | AppStream | JSON | CreateFleet, DeleteFleet, DescribeFleets, CreateStack |
| 13 | AppSync | REST-JSON | CreateGraphqlApi, ListGraphqlApis, DeleteGraphqlApi |
| 14 | Athena | JSON | CreateWorkGroup, StartQueryExecution, GetQueryExecution |
| 15 | Audit Manager | REST-JSON | CreateAssessment, ListAssessments, CreateControl |
| 16 | Auto Scaling | Query | CreateAutoScalingGroup, DescribeAutoScalingGroups, SetDesiredCapacity |
| 17 | Backup | REST-JSON | CreateBackupVault, CreateBackupPlan, ListBackupVaults |
| 18 | Batch | REST-JSON | CreateComputeEnvironment, CreateJobQueue, SubmitJob |
| 19 | Bedrock | REST-JSON | ListFoundationModels, CreateModelCustomizationJob |
| 20 | Braket | REST-JSON | CreateQuantumTask, GetQuantumTask, SearchDevices |
| 21 | Budgets | JSON | CreateBudget, DeleteBudget, DescribeBudgets |
| 22 | Chatbot | JSON | CreateSlackChannelConfiguration, ListConfigurations |
| 23 | Clean Rooms | REST-JSON | CreateCollaboration, ListCollaborations, CreateMembership |
| 24 | Cloud9 | JSON | CreateEnvironmentEC2, DescribeEnvironments, ListEnvironments |
| 25 | CloudControl | JSON | CreateResource, GetResource, ListResources, UpdateResource |
| 26 | CloudFormation | JSON | CreateStack, DeleteStack, DescribeStacks, UpdateStack |
| 27 | CloudFront | REST-XML | CreateDistribution, GetDistribution, ListDistributions |
| 28 | CloudHSM | JSON | CreateCluster, DeleteCluster, DescribeClusters, CreateHsm |
| 29 | CloudMap | JSON | CreatePrivateDnsNamespace, CreateService, RegisterInstance |
| 30 | CloudSearch | Query | CreateDomain, DeleteDomain, DescribeDomains, ListDomainNames |
| 31 | CloudTrail | JSON | CreateTrail, DeleteTrail, DescribeTrails, StartLogging |
| 32 | CloudWatch Logs | JSON | CreateLogGroup, CreateLogStream, PutLogEvents, GetLogEvents |
| 33 | CloudWatch Metrics | Query | PutMetricData, GetMetricData, ListMetrics, PutMetricAlarm |
| 34 | CodeArtifact | REST-JSON | CreateDomain, CreateRepository, ListDomains |
| 35 | CodeBuild | JSON | CreateProject, StartBuild, BatchGetBuilds, ListProjects |
| 36 | CodeCommit | JSON | CreateRepository, GetRepository, ListRepositories |
| 37 | CodeDeploy | JSON | CreateApplication, CreateDeploymentGroup, CreateDeployment |
| 38 | CodeGuru | JSON | CreateProfilingGroup, ListProfilingGroups, GetRecommendations |
| 39 | CodePipeline | JSON | CreatePipeline, GetPipeline, StartPipelineExecution |
| 40 | Cognito | JSON | CreateUserPool, CreateUserPoolClient, SignUp, InitiateAuth |
| 41 | Comprehend | JSON | DetectSentiment, DetectEntities, DetectKeyPhrases |
| 42 | Compute Optimizer | JSON | GetEC2InstanceRecommendations, GetEnrollmentStatus |
| 43 | Config | JSON | PutConfigRule, DescribeConfigRules, PutConfigurationRecorder |
| 44 | Connect | REST-JSON | CreateInstance, ListInstances, CreateContactFlow |
| 45 | Control Tower | JSON | CreateLandingZone, EnableControl, ListEnabledControls |
| 46 | Cost Explorer | JSON | GetCostAndUsage, GetCostForecast, GetDimensionValues |
| 47 | Cost & Usage Reports | JSON | PutReportDefinition, DescribeReportDefinitions |
| 48 | Customer Profiles | REST-JSON | CreateDomain, ListDomains, PutProfileObject |
| 49 | DataBrew | REST-JSON | CreateProject, ListProjects, CreateRecipe, CreateDataset |
| 50 | Data Exchange | REST-JSON | CreateDataSet, ListDataSets, CreateRevision |
| 51 | Data Pipeline | JSON | CreatePipeline, DescribePipelines, ActivatePipeline |
| 52 | DataSync | JSON | CreateTask, ListTasks, CreateLocationS3, StartTaskExecution |
| 53 | DataZone | REST-JSON | CreateDomain, ListDomains, CreateProject |
| 54 | DAX | JSON | CreateCluster, DeleteCluster, DescribeClusters |
| 55 | Detective | REST-JSON | CreateGraph, ListGraphs, DeleteGraph, CreateMembers |
| 56 | Device Farm | JSON | CreateProject, ListProjects, CreateUpload, ScheduleRun |
| 57 | DevOps Guru | REST-JSON | AddNotificationChannel, ListInsights, SearchInsights |
| 58 | Direct Connect | JSON | CreateConnection, DescribeConnections, CreateVirtualInterface |
| 59 | Directory Service | JSON | CreateDirectory, DescribeDirectories, CreateMicrosoftAD |
| 60 | DLM | REST-JSON | CreateLifecyclePolicy, GetLifecyclePolicies |
| 61 | DMS | JSON | CreateReplicationInstance, CreateEndpoint, CreateReplicationTask |
| 62 | DocumentDB | JSON | CreateDBCluster, DescribeDBClusters, CreateDBInstance |
| 63 | DynamoDB | JSON | CreateTable, PutItem, GetItem, DeleteItem, Query, Scan |
| 64 | EBS | REST-JSON | StartSnapshot, PutSnapshotBlock, GetSnapshotBlock |
| 65 | EC2 | Query | RunInstances, DescribeInstances, TerminateInstances |
| 66 | ECR | JSON | CreateRepository, DescribeRepositories, PutImage |
| 67 | ECS | JSON | CreateCluster, RegisterTaskDefinition, RunTask |
| 68 | EFS | REST-JSON | CreateFileSystem, DescribeFileSystems, DeleteFileSystem |
| 69 | EKS | REST-JSON | CreateCluster, DescribeCluster, ListClusters |
| 70 | Elastic Beanstalk | Query | CreateApplication, CreateEnvironment, DescribeEnvironments |
| 71 | ElastiCache | JSON | CreateCacheCluster, DescribeCacheClusters |
| 72 | ELB | JSON | CreateLoadBalancer, DescribeLoadBalancers, CreateTargetGroup |
| 73 | EMR | JSON | RunJobFlow, TerminateJobFlows, ListClusters |
| 74 | EMR Serverless | REST-JSON | CreateApplication, ListApplications, StartJobRun |
| 75 | Entity Resolution | REST-JSON | CreateMatchingWorkflow, CreateSchemaMapping |
| 76 | EventBridge | JSON | CreateEventBus, PutRule, PutTargets, PutEvents |
| 77 | EventBridge Pipes | REST-JSON | CreatePipe, ListPipes, DescribePipe, StartPipe |
| 78 | EventBridge Scheduler | REST-JSON | CreateSchedule, ListSchedules, CreateScheduleGroup |
| 79 | FinSpace | REST-JSON | CreateEnvironment, ListEnvironments, CreateKxDatabase |
| 80 | Firehose | JSON | CreateDeliveryStream, PutRecord, ListDeliveryStreams |
| 81 | Firewall Manager | JSON | PutPolicy, GetPolicy, ListPolicies |
| 82 | FIS | REST-JSON | CreateExperimentTemplate, StartExperiment, ListExperiments |
| 83 | Forecast | JSON | CreateDataset, CreatePredictor, CreateForecast |
| 84 | Fraud Detector | JSON | GetDetectors, CreateModel, GetEventPrediction |
| 85 | FSx | JSON | CreateFileSystem, DescribeFileSystems, CreateBackup |
| 86 | GameLift | JSON | CreateFleet, ListFleets, CreateGameSessionQueue |
| 87 | Glacier | REST-JSON | CreateVault, ListVaults, DescribeVault, UploadArchive |
| 88 | Global Accelerator | JSON | CreateAccelerator, ListAccelerators, CreateListener |
| 89 | Glue | JSON | CreateDatabase, CreateTable, GetTables, CreateCrawler |
| 90 | Grafana (Managed) | REST-JSON | CreateWorkspace, ListWorkspaces, DescribeWorkspace |
| 91 | Greengrass | REST-JSON | CreateComponentVersion, ListComponents, ListCoreDevices |
| 92 | Ground Station | REST-JSON | ListSatellites, CreateConfig, ListConfigs |
| 93 | GuardDuty | REST-JSON | CreateDetector, ListDetectors, GetDetector |
| 94 | Health | JSON | DescribeEvents, DescribeEventDetails, DescribeAffectedEntities |
| 95 | HealthLake | JSON | CreateFHIRDatastore, ListFHIRDatastores, StartFHIRImportJob |
| 96 | IAM | Query | CreateUser, CreateRole, CreatePolicy, AttachRolePolicy |
| 97 | Identity Store | JSON | CreateUser, DescribeUser, ListUsers, CreateGroup |
| 98 | Image Builder | REST-JSON | CreateImage, ListImages, CreateComponent, CreateImagePipeline |
| 99 | Inspector | JSON | CreateAssessmentTemplate, ListFindings |
| 100 | Internet Monitor | REST-JSON | CreateMonitor, ListMonitors, GetMonitor |
| 101 | IoT Core | REST-JSON | CreateThing, ListThings, CreatePolicy, DeleteThing |
| 102 | IVS | JSON | CreateChannel, ListChannels, CreateStreamKey |
| 103 | Kendra | JSON | CreateIndex, ListIndices, CreateDataSource, Query |
| 104 | Keyspaces | JSON | CreateKeyspace, CreateTable, ListKeyspaces |
| 105 | Kinesis | JSON | CreateStream, DeleteStream, PutRecord, GetRecords |
| 106 | KMS | JSON | CreateKey, Encrypt, Decrypt, GenerateDataKey |
| 107 | Lake Formation | JSON | RegisterResource, GrantPermissions, ListPermissions |
| 108 | Lambda | REST-JSON | CreateFunction, Invoke, ListFunctions, DeleteFunction |
| 109 | Lex | REST-JSON | PutBot, GetBots, GetBot, DeleteBot, PostText |
| 110 | License Manager | JSON | CreateLicense, ListReceivedLicenses, CreateLicenseConfiguration |
| 111 | Lightsail | JSON | CreateInstances, GetInstances, DeleteInstance |
| 112 | Location Service | REST-JSON | CreateMap, ListMaps, CreateGeofenceCollection |
| 113 | Macie | REST-JSON | CreateClassificationJob, ListClassificationJobs |
| 114 | Mainframe (M2) | REST-JSON | CreateApplication, ListApplications, CreateEnvironment |
| 115 | Managed Blockchain | REST-JSON | CreateNetwork, ListNetworks, CreateNode |
| 116 | MediaConvert | REST-JSON | CreateQueue, CreateJob, ListJobs |
| 117 | MediaLive | REST-JSON | CreateChannel, ListChannels, CreateInput |
| 118 | MediaPackage | REST-JSON | CreateChannel, ListChannels, CreateOriginEndpoint |
| 119 | MediaStore | JSON | CreateContainer, ListContainers, PutContainerPolicy |
| 120 | MediaTailor | REST-JSON | PutPlaybackConfiguration, ListPlaybackConfigurations |
| 121 | MemoryDB | JSON | CreateCluster, DescribeClusters, CreateSnapshot |
| 122 | MQ | REST-JSON | CreateBroker, DescribeBroker, ListBrokers |
| 123 | MSK | JSON | CreateCluster, ListClusters, DescribeCluster |
| 124 | MWAA | REST-JSON | CreateEnvironment, ListEnvironments, GetEnvironment |
| 125 | Neptune | JSON | CreateDBCluster, DescribeDBClusters, CreateDBInstance |
| 126 | Network Firewall | JSON | CreateFirewall, ListFirewalls, CreateFirewallPolicy |
| 127 | Network Manager | REST-JSON | CreateGlobalNetwork, CreateSite, CreateDevice |
| 128 | Omics | REST-JSON | CreateSequenceStore, CreateWorkflow, ListWorkflows |
| 129 | OpenSearch | REST-JSON | CreateDomain, ListDomainNames, DescribeDomain |
| 130 | Organizations | JSON | CreateOrganization, CreateAccount, ListAccounts |
| 131 | Outposts | REST-JSON | CreateOutpost, ListOutposts, CreateSite |
| 132 | Personalize | JSON | CreateDataset, CreateSolution, CreateCampaign |
| 133 | Pinpoint | REST-JSON | CreateApp, GetApps, CreateCampaign |
| 134 | Polly | REST-JSON | PutLexicon, ListLexicons, SynthesizeSpeech |
| 135 | Pricing | JSON | DescribeServices, GetProducts, ListPriceLists |
| 136 | Proton | JSON | CreateEnvironmentTemplate, CreateService |
| 137 | QLDB | REST-JSON | CreateLedger, ListLedgers, DescribeLedger |
| 138 | QuickSight | REST-JSON | CreateDataSet, ListDataSets, CreateDashboard |
| 139 | RAM | JSON | CreateResourceShare, GetResourceShares |
| 140 | RDS | JSON | CreateDBInstance, DescribeDBInstances, CreateDBCluster |
| 141 | Redshift | JSON | CreateCluster, DescribeClusters, PauseCluster |
| 142 | Rekognition | JSON | CreateCollection, DetectLabels, IndexFaces |
| 143 | Resilience Hub | JSON | CreateApp, ListApps, StartAppAssessment |
| 144 | Roles Anywhere | REST-JSON | CreateTrustAnchor, ListTrustAnchors, CreateProfile |
| 145 | Route 53 | REST-XML | CreateHostedZone, ListHostedZones, ChangeResourceRecordSets |
| 146 | Route 53 Domains | JSON | RegisterDomain, ListDomains, CheckDomainAvailability |
| 147 | Route 53 Resolver | JSON | CreateResolverEndpoint, ListResolverEndpoints, CreateResolverRule |
| 148 | RUM | REST-JSON | CreateAppMonitor, ListAppMonitors, PutRumEvents |
| 149 | S3 | REST-XML | CreateBucket, PutObject, GetObject, DeleteObject, ListBuckets |
| 150 | SageMaker | JSON | CreateNotebookInstance, CreateEndpoint, CreateTrainingJob |
| 151 | Savings Plans | JSON | CreateSavingsPlan, DescribeSavingsPlans |
| 152 | Schemas | REST-JSON | CreateRegistry, ListRegistries, CreateSchema |
| 153 | Secrets Manager | JSON | CreateSecret, GetSecretValue, UpdateSecret, DeleteSecret |
| 154 | Security Hub | REST-JSON | EnableSecurityHub, BatchImportFindings, GetFindings |
| 155 | Security Lake | REST-JSON | CreateDataLake, CreateSubscriber, ListSubscribers |
| 156 | Serverless Repo | REST-JSON | CreateApplication, ListApplications |
| 157 | Service Catalog | JSON | CreatePortfolio, CreateProduct, SearchProducts |
| 158 | Service Quotas | JSON | ListServiceQuotas, RequestServiceQuotaIncrease |
| 159 | SES | JSON | CreateEmailIdentity, SendEmail, ListEmailIdentities |
| 160 | Shield | JSON | CreateProtection, ListProtections, CreateSubscription |
| 161 | Snowball | JSON | CreateJob, DescribeJob, ListJobs, CreateCluster |
| 162 | SNS | Query | CreateTopic, Subscribe, Publish, Unsubscribe |
| 163 | SQS | Query | CreateQueue, SendMessage, ReceiveMessage, DeleteMessage |
| 164 | SSM Parameter Store | JSON | PutParameter, GetParameter, GetParametersByPath |
| 165 | SSO (IAM Identity Center) | JSON | CreatePermissionSet, CreateAccountAssignment |
| 166 | Step Functions | JSON | CreateStateMachine, StartExecution, DescribeExecution |
| 167 | Storage Gateway | JSON | ActivateGateway, ListGateways |
| 168 | STS | Query | GetCallerIdentity, AssumeRole |
| 169 | Support | JSON | CreateCase, DescribeCases, DescribeTrustedAdvisorChecks |
| 170 | SWF | JSON | RegisterDomain, RegisterWorkflowType, StartWorkflowExecution |
| 171 | Synthetics | REST-JSON | CreateCanary, DescribeCanaries, StartCanary |
| 172 | Textract | JSON | StartDocumentAnalysis, DetectDocumentText |
| 173 | Timestream | JSON | CreateDatabase, CreateTable, ListDatabases |
| 174 | Transfer Family | JSON | CreateServer, ListServers, CreateUser |
| 175 | Translate | JSON | TranslateText, ListTerminologies, ImportTerminology |
| 176 | Verified Permissions | JSON | CreatePolicyStore, CreatePolicy, IsAuthorized |
| 177 | VPC Lattice | REST-JSON | CreateService, ListServices, CreateTargetGroup |
| 178 | WAF v2 | JSON | CreateWebACL, ListWebACLs, CreateRuleGroup |
| 179 | Well-Architected | REST-JSON | CreateWorkload, ListWorkloads, ListLensReviews |
| 180 | WorkDocs | REST-JSON | CreateFolder, DescribeFolderContents, InitiateDocumentVersionUpload |
| 181 | WorkMail | JSON | CreateOrganization, ListOrganizations, CreateUser |
| 182 | WorkSpaces | JSON | CreateWorkspaces, DescribeWorkspaces, TerminateWorkspaces |
| 183 | X-Ray | REST-JSON | PutTraceSegments, GetTraceSummaries, BatchGetTraces |
| 184 | EventBridge Scheduler | REST-JSON | CreateSchedule, ListSchedules, CreateScheduleGroup |

</details>

## Installation

### Homebrew (macOS/Linux)

```bash
brew install huseyinbabal/tap/laws
```

### Scoop (Windows)

```powershell
scoop bucket add huseyinbabal https://github.com/huseyinbabal/scoop-bucket
scoop install laws
```

### Download Pre-built Binaries

Download the latest release from the [Releases page](https://github.com/huseyinbabal/laws/releases/latest).

| Platform | Architecture | Download |
|----------|--------------|----------|
| **macOS** | Apple Silicon (M1/M2/M3) | `laws-aarch64-apple-darwin.tar.gz` |
| **macOS** | Intel | `laws-x86_64-apple-darwin.tar.gz` |
| **Linux** | x86_64 (musl) | `laws-x86_64-unknown-linux-musl.tar.gz` |
| **Linux** | ARM64 (musl) | `laws-aarch64-unknown-linux-musl.tar.gz` |
| **Windows** | x86_64 | `laws-x86_64-pc-windows-msvc.zip` |

#### Quick Install (macOS/Linux)

```bash
# macOS Apple Silicon
curl -sL https://github.com/huseyinbabal/laws/releases/latest/download/laws-aarch64-apple-darwin.tar.gz | tar xz
sudo mv laws /usr/local/bin/

# macOS Intel
curl -sL https://github.com/huseyinbabal/laws/releases/latest/download/laws-x86_64-apple-darwin.tar.gz | tar xz
sudo mv laws /usr/local/bin/

# Linux x86_64
curl -sL https://github.com/huseyinbabal/laws/releases/latest/download/laws-x86_64-unknown-linux-musl.tar.gz | tar xz
sudo mv laws /usr/local/bin/

# Linux ARM64
curl -sL https://github.com/huseyinbabal/laws/releases/latest/download/laws-aarch64-unknown-linux-musl.tar.gz | tar xz
sudo mv laws /usr/local/bin/
```

#### Windows

1. Download `laws-x86_64-pc-windows-msvc.zip` from the [Releases page](https://github.com/huseyinbabal/laws/releases/latest)
2. Extract the zip file
3. Add the extracted folder to your PATH, or move `laws.exe` to a directory in your PATH

### Using Cargo

```bash
cargo install laws
```

### Using Docker

```bash
# Run laws
docker run --rm -p 4566:4566 ghcr.io/huseyinbabal/laws

# Run on a custom port
docker run --rm -p 8080:8080 ghcr.io/huseyinbabal/laws --port 8080

# Build locally
docker build -t laws .
docker run --rm -p 4566:4566 laws
```

### From Source

```bash
git clone https://github.com/huseyinbabal/laws.git
cd laws
cargo build --release
./target/release/laws
```

## Quick Start

```bash
# Run with defaults (port 4566)
laws

# Custom port
laws --port 8080

# With debug logging
RUST_LOG=debug laws
```

## Usage with AWS CLI

```bash
# Configure endpoint
export AWS_ENDPOINT_URL=http://localhost:4566

# Or use --endpoint-url flag
aws --endpoint-url http://localhost:4566 s3 mb s3://my-bucket
aws --endpoint-url http://localhost:4566 sqs create-queue --queue-name my-queue
aws --endpoint-url http://localhost:4566 dynamodb create-table --table-name users \
    --attribute-definitions AttributeName=id,AttributeType=S \
    --key-schema AttributeName=id,KeyType=HASH
```

## Usage with AWS SDKs

Point your SDK's endpoint URL to `http://localhost:4566`. Any credentials will be accepted.

## Architecture

laws uses a data-driven architecture inspired by [taws](https://github.com/huseyinbabal/taws):

- **Protocol handlers** for all 4 AWS API styles (Query, JSON, REST-JSON, REST-XML)
- **In-memory storage** with `DashMap` for lock-free concurrent access
- **Single binary** — no containers, no runtime dependencies
- **Fast startup** — ready in milliseconds

## License

MIT
