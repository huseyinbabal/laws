// AWS service definitions with icon URLs from https://icon.icepanel.io/AWS
// Each service maps to the display name used in the laws backend dashboard

export interface AwsService {
  id: string            // kebab-case identifier for routing
  name: string          // display name matching backend service name
  category: string      // AWS category grouping
  iconUrl: string       // SVG icon URL
  description: string   // short description
}

const icon = (category: string, name: string) =>
  `https://icon.icepanel.io/AWS/svg/${category}/${name}.svg`

export const AWS_SERVICES: AwsService[] = [
  // Compute
  { id: "ec2", name: "EC2", category: "Compute", iconUrl: icon("Compute", "EC2"), description: "Virtual servers in the cloud" },
  { id: "lambda", name: "Lambda", category: "Compute", iconUrl: icon("Compute", "Lambda"), description: "Run code without thinking about servers" },
  { id: "ecs", name: "ECS", category: "Containers", iconUrl: icon("Containers", "Elastic-Container-Service"), description: "Run and manage Docker containers" },
  { id: "eks", name: "EKS", category: "Containers", iconUrl: icon("Containers", "Elastic-Kubernetes-Service"), description: "Managed Kubernetes service" },
  { id: "ecr", name: "ECR", category: "Containers", iconUrl: icon("Containers", "Elastic-Container-Registry"), description: "Container image registry" },
  { id: "batch", name: "Batch", category: "Compute", iconUrl: icon("Compute", "Batch"), description: "Batch computing at any scale" },
  { id: "lightsail", name: "Lightsail", category: "Compute", iconUrl: icon("Compute", "Lightsail"), description: "Easy-to-use virtual private servers" },
  { id: "app-runner", name: "App Runner", category: "Compute", iconUrl: icon("Compute", "App-Runner"), description: "Build and run containerized apps" },
  { id: "elastic-beanstalk", name: "Elastic Beanstalk", category: "Compute", iconUrl: icon("Compute", "Elastic-Beanstalk"), description: "Run and manage web apps" },
  { id: "autoscaling", name: "Auto Scaling", category: "Compute", iconUrl: icon("Compute", "EC2-Auto-Scaling"), description: "Scale compute capacity automatically" },

  // Storage
  { id: "s3", name: "S3", category: "Storage", iconUrl: icon("Storage", "Simple-Storage-Service"), description: "Scalable object storage" },
  { id: "ebs", name: "EBS", category: "Storage", iconUrl: icon("Storage", "Elastic-Block-Store"), description: "Block storage for EC2" },
  { id: "efs", name: "EFS", category: "Storage", iconUrl: icon("Storage", "EFS"), description: "Managed file storage for EC2" },
  { id: "glacier", name: "Glacier", category: "Storage", iconUrl: icon("Storage", "Simple-Storage-Service"), description: "Archive storage in the cloud" },
  { id: "fsx", name: "FSx", category: "Storage", iconUrl: icon("Storage", "FSx"), description: "Fully managed file systems" },
  { id: "storage-gateway", name: "Storage Gateway", category: "Storage", iconUrl: icon("Storage", "Storage-Gateway"), description: "Hybrid cloud storage" },
  { id: "backup", name: "Backup", category: "Storage", iconUrl: icon("Storage", "Backup"), description: "Centralized backup service" },

  // Database
  { id: "dynamodb", name: "DynamoDB", category: "Database", iconUrl: icon("Database", "DynamoDB"), description: "Managed NoSQL database" },
  { id: "rds", name: "RDS", category: "Database", iconUrl: icon("Database", "RDS"), description: "Managed relational database" },
  { id: "elasticache", name: "ElastiCache", category: "Database", iconUrl: icon("Database", "ElastiCache"), description: "In-memory caching service" },
  { id: "redshift", name: "Redshift", category: "Database", iconUrl: icon("Analytics", "Redshift"), description: "Data warehousing" },
  { id: "neptune", name: "Neptune", category: "Database", iconUrl: icon("Database", "Neptune"), description: "Managed graph database" },
  { id: "documentdb", name: "DocumentDB", category: "Database", iconUrl: icon("Database", "DocumentDB"), description: "MongoDB-compatible document database" },
  { id: "dax", name: "DAX", category: "Database", iconUrl: icon("Database", "DynamoDB"), description: "DynamoDB Accelerator" },
  { id: "keyspaces", name: "Keyspaces", category: "Database", iconUrl: icon("Database", "Keyspaces"), description: "Managed Apache Cassandra" },
  { id: "memorydb", name: "MemoryDB", category: "Database", iconUrl: icon("Database", "MemoryDB-for-Redis"), description: "Redis-compatible in-memory database" },
  { id: "qldb", name: "QLDB", category: "Database", iconUrl: icon("Blockchain", "Quantum-Ledger-Database"), description: "Managed ledger database" },
  { id: "timestream", name: "Timestream", category: "Database", iconUrl: icon("Database", "Timestream"), description: "Time series database" },

  // Networking & Content Delivery
  { id: "cloudfront", name: "CloudFront", category: "Networking & Content Delivery", iconUrl: icon("Networking-Content-Delivery", "CloudFront"), description: "Global content delivery network" },
  { id: "route53", name: "Route 53", category: "Networking & Content Delivery", iconUrl: icon("Networking-Content-Delivery", "Route-53"), description: "Scalable DNS and domain registration" },
  { id: "route53-domains", name: "Route 53 Domains", category: "Networking & Content Delivery", iconUrl: icon("Networking-Content-Delivery", "Route-53"), description: "Domain name registration" },
  { id: "route53-resolver", name: "Route 53 Resolver", category: "Networking & Content Delivery", iconUrl: icon("Networking-Content-Delivery", "Route-53"), description: "DNS query resolution" },
  { id: "elb", name: "ELB", category: "Networking & Content Delivery", iconUrl: icon("Networking-Content-Delivery", "Elastic-Load-Balancing"), description: "Load balancing" },
  { id: "apigateway", name: "API Gateway", category: "Networking & Content Delivery", iconUrl: icon("App-Integration", "API-Gateway"), description: "Build, deploy, and manage APIs" },
  { id: "apigatewayv2", name: "API Gateway V2", category: "Networking & Content Delivery", iconUrl: icon("App-Integration", "API-Gateway"), description: "WebSocket and HTTP APIs" },
  { id: "vpc-lattice", name: "VPC Lattice", category: "Networking & Content Delivery", iconUrl: icon("Networking-Content-Delivery", "Virtual-Private-Cloud"), description: "Service-to-service connectivity" },
  { id: "direct-connect", name: "Direct Connect", category: "Networking & Content Delivery", iconUrl: icon("Networking-Content-Delivery", "Direct-Connect"), description: "Dedicated network connection to AWS" },
  { id: "global-accelerator", name: "Global Accelerator", category: "Networking & Content Delivery", iconUrl: icon("Networking-Content-Delivery", "Global-Accelerator"), description: "Improve global app availability" },
  { id: "cloudmap", name: "Cloud Map", category: "Networking & Content Delivery", iconUrl: icon("Networking-Content-Delivery", "Cloud-Map"), description: "Service discovery for cloud resources" },
  { id: "app-mesh", name: "App Mesh", category: "Networking & Content Delivery", iconUrl: icon("Networking-Content-Delivery", "App-Mesh"), description: "Application-level networking" },
  { id: "network-firewall", name: "Network Firewall", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Network-Firewall"), description: "Network security across VPCs" },

  // Security, Identity & Compliance
  { id: "iam", name: "IAM", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Identity-and-Access-Management"), description: "Manage access to AWS services" },
  { id: "sts", name: "STS", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Identity-and-Access-Management"), description: "Temporary security credentials" },
  { id: "cognito", name: "Cognito", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Cognito"), description: "User sign-up and authentication" },
  { id: "kms", name: "KMS", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Key-Management-Service"), description: "Create and manage encryption keys" },
  { id: "secretsmanager", name: "Secrets Manager", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Secrets-Manager"), description: "Manage secrets securely" },
  { id: "acm", name: "ACM", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Certificate-Manager"), description: "Provision and manage SSL/TLS certificates" },
  { id: "acm-pca", name: "ACM PCA", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Certificate-Manager"), description: "Private certificate authority" },
  { id: "waf", name: "WAF", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "WAF"), description: "Web application firewall" },
  { id: "shield", name: "Shield", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Shield"), description: "DDoS protection" },
  { id: "guardduty", name: "GuardDuty", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "GuardDuty"), description: "Intelligent threat detection" },
  { id: "inspector", name: "Inspector", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Inspector"), description: "Automated security assessments" },
  { id: "security-hub", name: "Security Hub", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Security-Hub"), description: "Unified security and compliance center" },
  { id: "access-analyzer", name: "Access Analyzer", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Identity-and-Access-Management"), description: "Analyze resource access" },
  { id: "detective", name: "Detective", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Detective"), description: "Investigate security findings" },
  { id: "firewall-manager", name: "Firewall Manager", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Firewall-Manager"), description: "Central firewall rule management" },
  { id: "macie", name: "Macie", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Macie"), description: "Discover and protect sensitive data" },
  { id: "sso", name: "SSO", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "IAM-Identity-Center"), description: "Single sign-on access" },
  { id: "verified-permissions", name: "Verified Permissions", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Verified-Permissions"), description: "Fine-grained authorization" },
  { id: "cloudhsm", name: "CloudHSM", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "CloudHSM"), description: "Hardware-based key storage" },
  { id: "directory-service", name: "Directory Service", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Directory-Service"), description: "Managed Microsoft Active Directory" },
  { id: "identity-store", name: "Identity Store", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "IAM-Identity-Center"), description: "Identity store for IAM Identity Center" },
  { id: "ram", name: "RAM", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Resource-Access-Manager"), description: "Share AWS resources" },
  { id: "security-lake", name: "Security Lake", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Security-Lake"), description: "Centralized security data lake" },
  { id: "rolesanywhere", name: "Roles Anywhere", category: "Security, Identity & Compliance", iconUrl: icon("Security-Identity-Compliance", "Identity-and-Access-Management"), description: "IAM roles for workloads outside AWS" },
  { id: "fraud-detector", name: "Fraud Detector", category: "Security, Identity & Compliance", iconUrl: icon("Machine-Learning", "Fraud-Detector"), description: "Detect online fraud" },
  { id: "license-manager", name: "License Manager", category: "Security, Identity & Compliance", iconUrl: icon("Management-Governance", "License-Manager"), description: "Manage software licenses" },

  // Application Integration
  { id: "sqs", name: "SQS", category: "Application Integration", iconUrl: icon("App-Integration", "Simple-Queue-Service"), description: "Managed message queuing" },
  { id: "sns", name: "SNS", category: "Application Integration", iconUrl: icon("App-Integration", "Simple-Notification-Service"), description: "Pub/sub messaging and notifications" },
  { id: "eventbridge", name: "EventBridge", category: "Application Integration", iconUrl: icon("App-Integration", "EventBridge"), description: "Serverless event bus" },
  { id: "eventbridge-scheduler", name: "EventBridge Scheduler", category: "Application Integration", iconUrl: icon("App-Integration", "EventBridge"), description: "Schedule one-time and recurring tasks" },
  { id: "eventbridge-pipes", name: "EventBridge Pipes", category: "Application Integration", iconUrl: icon("App-Integration", "EventBridge"), description: "Point-to-point integrations" },
  { id: "step-functions", name: "Step Functions", category: "Application Integration", iconUrl: icon("App-Integration", "Step-Functions"), description: "Coordinate distributed applications" },
  { id: "mq", name: "MQ", category: "Application Integration", iconUrl: icon("App-Integration", "MQ"), description: "Managed message broker" },
  { id: "appsync", name: "AppSync", category: "Application Integration", iconUrl: icon("App-Integration", "AppSync"), description: "GraphQL APIs at scale" },
  { id: "swf", name: "SWF", category: "Application Integration", iconUrl: icon("App-Integration", "Step-Functions"), description: "Simple workflow service" },
  { id: "schemas", name: "Schemas", category: "Application Integration", iconUrl: icon("App-Integration", "EventBridge"), description: "EventBridge Schema Registry" },

  // Analytics
  { id: "athena", name: "Athena", category: "Analytics", iconUrl: icon("Analytics", "Athena"), description: "Interactive query service" },
  { id: "kinesis", name: "Kinesis", category: "Analytics", iconUrl: icon("Analytics", "Kinesis"), description: "Real-time data streaming" },
  { id: "firehose", name: "Firehose", category: "Analytics", iconUrl: icon("Analytics", "Kinesis-Firehose"), description: "Load streaming data" },
  { id: "glue", name: "Glue", category: "Analytics", iconUrl: icon("Analytics", "Glue"), description: "ETL and data integration" },
  { id: "emr", name: "EMR", category: "Analytics", iconUrl: icon("Analytics", "EMR"), description: "Big data platform" },
  { id: "opensearch", name: "OpenSearch", category: "Analytics", iconUrl: icon("Analytics", "OpenSearch-Service"), description: "Search and analytics engine" },
  { id: "msk", name: "MSK", category: "Analytics", iconUrl: icon("Analytics", "Managed-Streaming-for-Apache-Kafka"), description: "Managed Streaming for Apache Kafka" },
  { id: "cloudsearch", name: "CloudSearch", category: "Analytics", iconUrl: icon("Analytics", "CloudSearch"), description: "Managed search service" },
  { id: "lake-formation", name: "Lake Formation", category: "Analytics", iconUrl: icon("Analytics", "Lake-Formation"), description: "Build secure data lakes" },
  { id: "data-pipeline", name: "Data Pipeline", category: "Analytics", iconUrl: icon("Analytics", "Data-Pipeline"), description: "Orchestrate data-driven workflows" },
  { id: "data-exchange", name: "Data Exchange", category: "Analytics", iconUrl: icon("Analytics", "Data-Exchange"), description: "Find and subscribe to third-party data" },
  { id: "quicksight", name: "QuickSight", category: "Analytics", iconUrl: icon("Analytics", "QuickSight"), description: "Business intelligence service" },

  // Developer Tools
  { id: "codebuild", name: "CodeBuild", category: "Developer Tools", iconUrl: icon("Developer-Tools", "CodeBuild"), description: "Build and test code" },
  { id: "codepipeline", name: "CodePipeline", category: "Developer Tools", iconUrl: icon("Developer-Tools", "CodePipeline"), description: "Continuous delivery service" },
  { id: "codecommit", name: "CodeCommit", category: "Developer Tools", iconUrl: icon("Developer-Tools", "CodeCommit"), description: "Source control service" },
  { id: "codedeploy", name: "CodeDeploy", category: "Developer Tools", iconUrl: icon("Developer-Tools", "CodeDeploy"), description: "Automate code deployments" },
  { id: "codeartifact", name: "CodeArtifact", category: "Developer Tools", iconUrl: icon("Developer-Tools", "CodeArtifact"), description: "Artifact management" },
  { id: "codeguru", name: "CodeGuru", category: "Developer Tools", iconUrl: icon("Machine-Learning", "CodeGuru"), description: "Code reviews and performance" },
  { id: "cloud9", name: "Cloud9", category: "Developer Tools", iconUrl: icon("Developer-Tools", "Cloud9"), description: "Cloud IDE" },
  { id: "device-farm", name: "Device Farm", category: "Developer Tools", iconUrl: icon("Front-End-Web-Mobile", "Device-Farm"), description: "Test apps on real devices" },
  { id: "xray", name: "X-Ray", category: "Developer Tools", iconUrl: icon("Developer-Tools", "X-Ray"), description: "Analyze and debug applications" },

  // Management & Governance
  { id: "cloudformation", name: "CloudFormation", category: "Management & Governance", iconUrl: icon("Management-Governance", "CloudFormation"), description: "Infrastructure as code" },
  { id: "cloudwatch", name: "CloudWatch", category: "Management & Governance", iconUrl: icon("Management-Governance", "CloudWatch"), description: "Monitoring and observability" },
  { id: "cloudwatch-logs", name: "CloudWatch Logs", category: "Management & Governance", iconUrl: icon("Management-Governance", "CloudWatch"), description: "Log management" },
  { id: "cloudtrail", name: "CloudTrail", category: "Management & Governance", iconUrl: icon("Management-Governance", "CloudTrail"), description: "Track user activity and API usage" },
  { id: "config", name: "Config", category: "Management & Governance", iconUrl: icon("Management-Governance", "Config"), description: "Track resource configurations" },
  { id: "ssm", name: "SSM", category: "Management & Governance", iconUrl: icon("Management-Governance", "Systems-Manager"), description: "Operational hub for AWS" },
  { id: "organizations", name: "Organizations", category: "Management & Governance", iconUrl: icon("Management-Governance", "Organizations"), description: "Central governance and management" },
  { id: "control-tower", name: "Control Tower", category: "Management & Governance", iconUrl: icon("Management-Governance", "Control-Tower"), description: "Govern multi-account environment" },
  { id: "cloudcontrol", name: "CloudControl", category: "Management & Governance", iconUrl: icon("Management-Governance", "CloudFormation"), description: "Cloud Control API" },
  { id: "service-catalog", name: "Service Catalog", category: "Management & Governance", iconUrl: icon("Management-Governance", "Service-Catalog"), description: "Create and manage catalogs" },
  { id: "service-quotas", name: "Service Quotas", category: "Management & Governance", iconUrl: icon("Management-Governance", "Service-Catalog"), description: "View and manage quotas" },
  { id: "compute-optimizer", name: "Compute Optimizer", category: "Management & Governance", iconUrl: icon("Compute", "Compute-Optimizer"), description: "Optimize compute resources" },
  { id: "health", name: "Health", category: "Management & Governance", iconUrl: icon("Management-Governance", "Personal-Health-Dashboard"), description: "AWS service health dashboard" },
  { id: "wellarchitected", name: "Well-Architected", category: "Management & Governance", iconUrl: icon("Management-Governance", "Well-Architected-Tool"), description: "Review architecture best practices" },
  { id: "appconfig", name: "AppConfig", category: "Management & Governance", iconUrl: icon("Management-Governance", "AppConfig"), description: "Application configuration management" },
  { id: "audit-manager", name: "Audit Manager", category: "Management & Governance", iconUrl: icon("Security-Identity-Compliance", "Audit-Manager"), description: "Continuously audit AWS usage" },
  { id: "resilience-hub", name: "Resilience Hub", category: "Management & Governance", iconUrl: icon("Management-Governance", "Resilience-Hub"), description: "Prepare and protect applications" },
  { id: "proton", name: "Proton", category: "Management & Governance", iconUrl: icon("Management-Governance", "Proton"), description: "Automate management for container and serverless" },

  // Machine Learning
  { id: "sagemaker", name: "SageMaker", category: "Machine Learning", iconUrl: icon("Machine-Learning", "SageMaker"), description: "Build, train, and deploy ML models" },
  { id: "bedrock", name: "Bedrock", category: "Machine Learning", iconUrl: icon("Machine-Learning", "SageMaker"), description: "Build with foundation models" },
  { id: "comprehend", name: "Comprehend", category: "Machine Learning", iconUrl: icon("Machine-Learning", "Comprehend"), description: "NLP service" },
  { id: "rekognition", name: "Rekognition", category: "Machine Learning", iconUrl: icon("Machine-Learning", "Rekognition"), description: "Image and video analysis" },
  { id: "textract", name: "Textract", category: "Machine Learning", iconUrl: icon("Machine-Learning", "Textract"), description: "Extract text and data from documents" },
  { id: "translate", name: "Translate", category: "Machine Learning", iconUrl: icon("Machine-Learning", "Translate"), description: "Natural language translation" },
  { id: "polly", name: "Polly", category: "Machine Learning", iconUrl: icon("Machine-Learning", "Polly"), description: "Text-to-speech service" },
  { id: "lex", name: "Lex", category: "Machine Learning", iconUrl: icon("Machine-Learning", "Lex"), description: "Build conversational interfaces" },
  { id: "forecast", name: "Forecast", category: "Machine Learning", iconUrl: icon("Machine-Learning", "Forecast"), description: "Time-series forecasting" },
  { id: "personalize", name: "Personalize", category: "Machine Learning", iconUrl: icon("Machine-Learning", "Personalize"), description: "Real-time personalization" },
  { id: "kendra", name: "Kendra", category: "Machine Learning", iconUrl: icon("Machine-Learning", "Kendra"), description: "Intelligent search service" },
  { id: "healthlake", name: "HealthLake", category: "Machine Learning", iconUrl: icon("Machine-Learning", "HealthLake"), description: "Store and analyze health data" },

  // Migration & Transfer
  { id: "dms", name: "DMS", category: "Migration & Transfer", iconUrl: icon("Database", "Database-Migration-Service"), description: "Database migration service" },
  { id: "datasync", name: "DataSync", category: "Migration & Transfer", iconUrl: icon("Migration-Transfer", "DataSync"), description: "Data transfer service" },
  { id: "transfer-family", name: "Transfer Family", category: "Migration & Transfer", iconUrl: icon("Migration-Transfer", "Transfer-Family"), description: "SFTP, FTPS, and FTP service" },
  { id: "snowball", name: "Snowball", category: "Migration & Transfer", iconUrl: icon("Storage", "Snowball"), description: "Physical data transport" },

  // Customer Engagement
  { id: "ses", name: "SES", category: "Customer Engagement", iconUrl: icon("Business-Applications", "Simple-Email-Service"), description: "Email sending and receiving" },
  { id: "connect", name: "Connect", category: "Customer Engagement", iconUrl: icon("Business-Applications", "Connect"), description: "Cloud contact center" },
  { id: "pinpoint", name: "Pinpoint", category: "Customer Engagement", iconUrl: icon("Business-Applications", "Pinpoint"), description: "Multichannel marketing" },

  // Media Services
  { id: "mediaconvert", name: "MediaConvert", category: "Media Services", iconUrl: icon("Media-Services", "Elemental-MediaConvert"), description: "File-based video transcoding" },
  { id: "medialive", name: "MediaLive", category: "Media Services", iconUrl: icon("Media-Services", "Elemental-MediaLive"), description: "Live video processing" },
  { id: "mediapackage", name: "MediaPackage", category: "Media Services", iconUrl: icon("Media-Services", "Elemental-MediaPackage"), description: "Video origination and packaging" },
  { id: "mediatailor", name: "MediaTailor", category: "Media Services", iconUrl: icon("Media-Services", "Elemental-MediaTailor"), description: "Personalized ad insertion" },
  { id: "mediastore", name: "MediaStore", category: "Media Services", iconUrl: icon("Media-Services", "Elemental-MediaStore"), description: "Media-optimized storage" },
  { id: "ivs", name: "IVS", category: "Media Services", iconUrl: icon("Media-Services", "Interactive-Video-Service"), description: "Live interactive video" },

  // Internet of Things
  { id: "iot", name: "IoT", category: "Internet of Things", iconUrl: icon("Internet-of-Things", "IoT-Core"), description: "Connect devices to the cloud" },
  { id: "greengrass", name: "Greengrass", category: "Internet of Things", iconUrl: icon("Internet-of-Things", "IoT-Greengrass"), description: "Local compute and messaging for devices" },

  // End User Computing
  { id: "workspaces", name: "WorkSpaces", category: "End User Computing", iconUrl: icon("End-User-Computing", "WorkSpaces-Family"), description: "Virtual desktops in the cloud" },
  { id: "appstream", name: "AppStream", category: "End User Computing", iconUrl: icon("End-User-Computing", "AppStream"), description: "Application streaming" },
  { id: "workdocs", name: "WorkDocs", category: "End User Computing", iconUrl: icon("Business-Applications", "WorkDocs"), description: "Enterprise storage and sharing" },
  { id: "workmail", name: "WorkMail", category: "End User Computing", iconUrl: icon("Business-Applications", "WorkMail"), description: "Managed email and calendaring" },

  // Cloud Financial Management
  { id: "cost-explorer", name: "Cost Explorer", category: "Cloud Financial Management", iconUrl: icon("Cloud-Financial-Management", "Cost-Explorer"), description: "Analyze AWS costs" },
  { id: "budgets", name: "Budgets", category: "Cloud Financial Management", iconUrl: icon("Cloud-Financial-Management", "Budgets"), description: "Set custom cost and usage budgets" },
  { id: "cost-usage-reports", name: "Cost & Usage Reports", category: "Cloud Financial Management", iconUrl: icon("Cloud-Financial-Management", "Cost-and-Usage-Report"), description: "Detailed cost and usage reports" },
  { id: "savings-plans", name: "Savings Plans", category: "Cloud Financial Management", iconUrl: icon("Cloud-Financial-Management", "Savings-Plans"), description: "Flexible pricing model" },

  // Other
  { id: "amplify", name: "Amplify", category: "Front-End Web & Mobile", iconUrl: icon("Front-End-Web-Mobile", "Amplify"), description: "Full-stack app development" },
  { id: "location", name: "Location", category: "Front-End Web & Mobile", iconUrl: icon("Front-End-Web-Mobile", "Location-Service"), description: "Location-based services" },
  { id: "appflow", name: "AppFlow", category: "Application Integration", iconUrl: icon("App-Integration", "AppFlow"), description: "SaaS data integration" },
  { id: "application-autoscaling", name: "Application Auto Scaling", category: "Compute", iconUrl: icon("Compute", "EC2-Auto-Scaling"), description: "Scaling for AWS resources" },
  { id: "chatbot", name: "Chatbot", category: "Management & Governance", iconUrl: icon("Management-Governance", "Chatbot"), description: "ChatOps for AWS" },
  { id: "databrew", name: "DataBrew", category: "Analytics", iconUrl: icon("Analytics", "Glue-DataBrew"), description: "Visual data preparation" },
  { id: "datazone", name: "DataZone", category: "Analytics", iconUrl: icon("Analytics", "DataZone"), description: "Data management service" },
  { id: "devops-guru", name: "DevOps Guru", category: "Machine Learning", iconUrl: icon("Machine-Learning", "DevOps-Guru"), description: "ML-powered operations" },
  { id: "dlm", name: "DLM", category: "Storage", iconUrl: icon("Storage", "Elastic-Block-Store"), description: "Data Lifecycle Manager" },
  { id: "emr-serverless", name: "EMR Serverless", category: "Analytics", iconUrl: icon("Analytics", "EMR"), description: "Serverless big data" },
  { id: "entity-resolution", name: "Entity Resolution", category: "Analytics", iconUrl: icon("Analytics", "Glue"), description: "Match and link records" },
  { id: "fis", name: "FIS", category: "Management & Governance", iconUrl: icon("Management-Governance", "Fault-Injection-Simulator"), description: "Fault Injection Simulator" },
  { id: "gamelift", name: "GameLift", category: "Games", iconUrl: icon("Games", "GameLift"), description: "Game server hosting" },
  { id: "braket", name: "Braket", category: "Quantum Technologies", iconUrl: icon("Quantum-Technologies", "Braket"), description: "Quantum computing service" },
  { id: "clean-rooms", name: "Clean Rooms", category: "Analytics", iconUrl: icon("Analytics", "Clean-Rooms"), description: "Collaborate on data without sharing" },
  { id: "customer-profiles", name: "Customer Profiles", category: "Customer Engagement", iconUrl: icon("Business-Applications", "Connect"), description: "Unified customer profiles" },
  { id: "finspace", name: "FinSpace", category: "Analytics", iconUrl: icon("Analytics", "FinSpace"), description: "Financial data analytics" },
  { id: "ground-station", name: "Ground Station", category: "Satellite", iconUrl: icon("Satellite", "Ground-Station"), description: "Control satellite communications" },
  { id: "image-builder", name: "Image Builder", category: "Compute", iconUrl: icon("Compute", "EC2-Image-Builder"), description: "Build and manage images" },
  { id: "internet-monitor", name: "Internet Monitor", category: "Management & Governance", iconUrl: icon("Management-Governance", "CloudWatch"), description: "Internet availability monitoring" },
  { id: "mainframe", name: "Mainframe Modernization", category: "Migration & Transfer", iconUrl: icon("Migration-Transfer", "Mainframe-Modernization"), description: "Mainframe modernization" },
  { id: "managed-blockchain", name: "Managed Blockchain", category: "Blockchain", iconUrl: icon("Blockchain", "Managed-Blockchain"), description: "Create and manage blockchains" },
  { id: "managed-grafana", name: "Managed Grafana", category: "Management & Governance", iconUrl: icon("Management-Governance", "Managed-Grafana"), description: "Managed Grafana dashboards" },
  { id: "mwaa", name: "MWAA", category: "Application Integration", iconUrl: icon("App-Integration", "Managed-Workflows-for-Apache-Airflow"), description: "Managed Apache Airflow" },
  { id: "network-manager", name: "Network Manager", category: "Networking & Content Delivery", iconUrl: icon("Networking-Content-Delivery", "Cloud-WAN"), description: "Network management" },
  { id: "omics", name: "Omics", category: "Analytics", iconUrl: icon("Machine-Learning", "Omics"), description: "Genomics and biological data" },
  { id: "outposts", name: "Outposts", category: "Compute", iconUrl: icon("Compute", "Outposts-family"), description: "Run AWS on-premises" },
  { id: "pricing", name: "Pricing", category: "Cloud Financial Management", iconUrl: icon("Cloud-Financial-Management", "Cost-Explorer"), description: "AWS Price List" },
  { id: "rum", name: "RUM", category: "Management & Governance", iconUrl: icon("Management-Governance", "CloudWatch"), description: "Real User Monitoring" },
  { id: "serverless-repo", name: "Serverless Repo", category: "Compute", iconUrl: icon("Compute", "Serverless-Application-Repository"), description: "Serverless Application Repository" },
  { id: "support", name: "Support", category: "Management & Governance", iconUrl: icon("Management-Governance", "Trusted-Advisor"), description: "AWS Support" },
  { id: "synthetics", name: "Synthetics", category: "Management & Governance", iconUrl: icon("Management-Governance", "CloudWatch"), description: "Canary tests for endpoints" },
]

// Build lookup maps
export const SERVICE_BY_ID = new Map(AWS_SERVICES.map(s => [s.id, s]))
export const SERVICE_BY_NAME = new Map(AWS_SERVICES.map(s => [s.name, s]))

// Get all unique categories
export const SERVICE_CATEGORIES = [...new Set(AWS_SERVICES.map(s => s.category))].sort()

// Get icon URL for a service name (used in dashboard)
export function getServiceIcon(serviceName: string): string | undefined {
  return SERVICE_BY_NAME.get(serviceName)?.iconUrl
}
