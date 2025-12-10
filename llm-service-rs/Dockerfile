# Multi-stage build for llm-service-rs with security hardening
# Stage 1: Builder - compile the application
FROM rust:1.75-slim AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    protobuf-compiler \
    ca-certificates \
    pkg-config \
    libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Create app directories
WORKDIR /app

# Copy only manifests first to leverage Docker caching
COPY Cargo.toml Cargo.lock ./
COPY .proto ./.proto/
COPY llm-service-rs/Cargo.toml ./llm-service-rs/

# Create placeholder for dependency caching
RUN mkdir -p llm-service-rs/src && \
    echo 'fn main() { println!("Dependency cache placeholder"); }' > llm-service-rs/src/main.rs && \
    cargo build --release -p llm-service-rs && \
    rm -rf llm-service-rs/src

# Copy actual source code
COPY llm-service-rs/src ./llm-service-rs/src
COPY llm-service-rs/build.rs ./llm-service-rs/build.rs

# Build for release with optimizations
WORKDIR /app
RUN cargo build --release -p llm-service-rs

# Stage 2: Audit - scan for vulnerabilities
FROM builder AS audit
RUN cargo install cargo-audit
WORKDIR /app
# Run security audit on dependencies
RUN cargo audit

# Stage 3: Runtime - minimal distroless container
FROM gcr.io/distroless/cc AS runtime

# Create unprivileged user
# Note: In distroless, we need to use numeric user IDs as there's no useradd
# Using 1000:1000 which is a standard unprivileged user:group ID
USER 1000:1000

# Create app directories (these will be created in the builder and copied over)
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder --chown=1000:1000 /app/target/release/llm-service-rs /app/llm-service-rs

# Copy proto files needed at runtime
COPY --from=builder --chown=1000:1000 /app/.proto ./.proto/

# Expose the port
EXPOSE 50053

# Set security flags
ENV DOCKER_CONTENT_TRUST=1

# Run with security options
ENTRYPOINT ["/app/llm-service-rs"]

# Label the image for better identification and metadata
LABEL org.opencontainers.image.title="LLM Service"
LABEL org.opencontainers.image.description="Phoenix ORCH LLM Service - Secure API Interface (Hardened)"
LABEL org.opencontainers.image.vendor="Phoenix ORCH"
LABEL org.opencontainers.image.version="1.0.0"
LABEL org.opencontainers.image.created="2025-12-09"
LABEL org.opencontainers.image.source="https://github.com/phoenixorch"
LABEL org.opencontainers.image.licenses="Proprietary"