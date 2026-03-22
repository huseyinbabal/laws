# Stage 1: Build UI
FROM node:22-slim AS ui-builder

WORKDIR /app/ui

# Copy manifests first for better caching
COPY ui/package.json ui/package-lock.json ./

# Install dependencies
RUN npm ci

# Copy UI source
COPY ui/ ./

# Build the UI
RUN npm run build

# Stage 2: Build Rust binary
FROM rust:latest AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy src to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN cargo build --release && rm -rf src

# Copy actual source code
COPY src ./src

# Build the actual binary
RUN touch src/main.rs && cargo build --release

# Stage 3: Runtime (same base as builder for glibc compatibility)
FROM debian:trixie-slim

# Install runtime dependencies for TLS
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3t64 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/laws /usr/local/bin/laws

# Copy UI dist from ui-builder
COPY --from=ui-builder /app/ui/dist /usr/local/share/laws/ui/dist

WORKDIR /usr/local/share/laws

EXPOSE 4566

ENTRYPOINT ["laws"]
