# Contributing to laws

Thank you for your interest in contributing to laws! This document provides guidelines and information for contributors.

## Before You Start

**Important:** Before adding a new AWS service or major feature, please start a discussion in our [GitHub Discussions](https://github.com/huseyinbabal/laws/discussions) board. This helps us:

- Avoid duplicate work
- Discuss the best approach
- Ensure the feature aligns with project goals
- Get community feedback

## How to Contribute

1. **Fork the repository**
2. **Create your feature branch** (`git checkout -b feature/amazing-feature`)
3. **Commit your changes** (`git commit -m 'Add some amazing feature'`)
4. **Push to the branch** (`git push origin feature/amazing-feature`)
5. **Open a Pull Request**

## Development Setup

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/laws.git
cd laws

# Build the project. If Node.js is installed, build.rs rebuilds the dashboard
# UI (ui/ -> ui/dist) and embeds it; otherwise the committed ui/dist is used.
# When you change the UI, commit the regenerated ui/dist with your change.
cargo build

# Run in development mode
cargo run

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run linter
cargo clippy -- -D warnings
```

## Architecture

laws uses a protocol-driven architecture with in-memory storage for emulating AWS services locally.

```
src/
├── main.rs                 # Router setup, service dispatch
├── config.rs               # CLI configuration (clap)
├── dashboard.rs            # SSE endpoint for real-time dashboard
├── error.rs                # Error types (LawsError)
├── storage.rs              # In-memory storage (DashMap-backed)
├── protocol/               # AWS API protocol handlers
│   ├── mod.rs              # Shared utilities (request_id, status codes)
│   ├── query.rs            # Query protocol (EC2, IAM, STS)
│   ├── json.rs             # JSON-RPC protocol (DynamoDB, ECS)
│   ├── rest_json.rs        # REST-JSON protocol (Lambda, EKS)
│   └── rest_xml.rs         # REST-XML protocol (S3, CloudFront)
└── services/               # One module per AWS service (184 total)
    ├── mod.rs
    ├── s3.rs
    ├── dynamodb.rs
    ├── lambda.rs
    └── ...

ui/                         # Vue 3 dashboard
├── src/
│   ├── main.ts             # Vue app + router setup
│   ├── App.vue             # Root component with SSE provider
│   ├── components/         # Vue components (Layout, ServiceGrid, LiveLogs, etc.)
│   ├── composables/        # useSSE, usePinnedServices
│   ├── data/               # AWS service definitions with icons
│   └── types/              # TypeScript interfaces
├── package.json
└── vite.config.ts
```

## Adding a New AWS Service

### 1. Start a Discussion

Before writing any code, [open a discussion](https://github.com/huseyinbabal/laws/discussions/new?category=ideas) to propose the new service. Include:

- Which AWS service you want to add
- Which operations you plan to support
- Why this service would be valuable

### 2. Create the Service Module

Create `src/services/myservice.rs`:

```rust
use std::sync::Arc;

use axum::Router;
use dashmap::DashMap;

pub struct MyServiceState {
    pub items: DashMap<String, MyItem>,
}

pub fn router(state: Arc<MyServiceState>) -> Router {
    // Define routes based on the service's AWS API protocol
    todo!()
}
```

### 3. Register the Service

Add the module to `src/services/mod.rs`:

```rust
pub mod myservice;
```

Wire it up in `src/main.rs` by adding the state initialization and router.

### 4. Add to Dashboard

Add the service entry to `ui/src/data/aws-services.ts` with the correct category and icon URL.

### 5. Test Your Changes

```bash
# Build and run
cargo build && cargo run

# Test with AWS CLI
aws --endpoint-url http://localhost:4566 myservice list-items

# Run linter
cargo clippy -- -D warnings
cargo fmt --check

# Type-check the UI on its own (cargo build already runs the UI build)
cd ui && npm run build
```

## Protocol Reference

| Protocol | Style | Examples |
|----------|-------|---------|
| Query | Form-encoded params + XML response | EC2, IAM, STS, SQS, SNS |
| JSON | JSON body + `X-Amz-Target` header | DynamoDB, ECS, Kinesis |
| REST-JSON | REST paths + JSON body | Lambda, EKS, API Gateway |
| REST-XML | REST paths + XML body | S3, CloudFront, Route 53 |

## Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Pass all clippy lints (`cargo clippy -- -D warnings`)
- Write descriptive commit messages
- Add comments for complex logic

## Pull Request Guidelines

- Keep PRs focused on a single feature or fix
- Update documentation if needed
- Ensure `cargo clippy -- -D warnings` passes
- Ensure `npm run build` passes in the `ui/` directory
- Reference any related issues or discussions

## Questions?

If you have questions, feel free to:

- Open a [Discussion](https://github.com/huseyinbabal/laws/discussions)
- Check existing issues and PRs

Thank you for contributing!
