# Multi-stage build for safety-service-rs with security hardening
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
COPY safety-service-rs/Cargo.toml ./safety-service-rs/

# Create placeholder for dependency caching
RUN mkdir -p safety-service-rs/src && \
    echo 'fn main() { println!("Dependency cache placeholder"); }' > safety-service-rs/src/main.rs && \
    cargo build --release -p safety-service-rs && \
    rm -rf safety-service-rs/src

# Copy actual source code
COPY safety-service-rs/src ./safety-service-rs/src
COPY safety-service-rs/build.rs ./safety-service-rs/build.rs

# Build for release with optimizations and strip debug symbols
WORKDIR /app
RUN cargo build --release -p safety-service-rs && \
    strip /app/target/release/safety-service-rs

# Stage 2: Audit - scan for vulnerabilities
FROM builder AS audit
RUN cargo install cargo-audit
WORKDIR /app
# Run security audit on dependencies
RUN cargo audit

# Stage 3: Runtime - minimal container with enhanced security
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies only
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    wget \
    libssl3 && \
    rm -rf /var/lib/apt/lists/*

# Create unprivileged user
RUN groupadd -r appuser && useradd -r -g appuser -s /bin/false -d /app appuser

# Create app directories and set permissions
RUN mkdir -p /app/data && \
    chown -R appuser:appuser /app

WORKDIR /app

# Copy binary from builder stage
COPY --from=builder --chown=appuser:appuser /app/target/release/safety-service-rs /app/safety-service-rs

# Copy proto files needed at runtime
COPY --from=builder --chown=appuser:appuser /app/.proto ./.proto/

# Create a read-only directory for safety policies
RUN mkdir -p /app/policies && \
    chown -R appuser:appuser /app/policies && \
    chmod -R 550 /app/policies

# Switch to non-root user
USER appuser

# Expose the port
EXPOSE 50055

# Add healthcheck
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --spider -q http://localhost:50055 || exit 1

# Set security flags
ENV DOCKER_CONTENT_TRUST=1

# Run with security options
ENTRYPOINT ["./safety-service-rs"]

# Label the image for better identification and metadata
LABEL org.opencontainers.image.title="Safety Service"
LABEL org.opencontainers.image.description="Phoenix ORCH Safety Service - Content filtering and policy enforcement (Hardened)"
LABEL org.opencontainers.image.vendor="Phoenix ORCH"
LABEL org.opencontainers.image.version="1.0.0"
LABEL org.opencontainers.image.created="2025-12-09"
LABEL org.opencontainers.image.source="https://github.com/phoenixorch"
LABEL org.opencontainers.image.licenses="Proprietary"