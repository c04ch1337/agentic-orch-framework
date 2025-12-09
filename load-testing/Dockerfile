FROM loadimpact/k6:latest

# Install necessary tools
RUN apk add --no-cache \
    bash \
    curl \
    jq \
    ca-certificates

# Set up working directory
WORKDIR /app

# Copy scripts and scenarios
COPY ./scripts/ /app/scripts/
COPY ./scenarios/ /app/scenarios/
COPY ./configs/ /app/configs/

# Copy entrypoint script
COPY ./entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

# Set default environment variables
ENV K6_OUT=influxdb=http://influxdb:8086/k6
ENV K6_PROMETHEUS_RW_SERVER_URL=http://prometheus:9090/api/v1/write
ENV DEFAULT_SCENARIO=baseline
ENV VUS=10
ENV DURATION=30s

# Set entrypoint
ENTRYPOINT ["/app/entrypoint.sh"]