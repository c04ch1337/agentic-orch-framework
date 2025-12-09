FROM python:3.11-slim

WORKDIR /app

# Install dependencies
RUN pip install --no-cache-dir prometheus-client==0.17.1 requests==2.31.0

# Copy the exporter script
COPY security_metrics_exporter.py .

# Make the script executable
RUN chmod +x security_metrics_exporter.py

# Set environment variables
ENV PYTHONUNBUFFERED=1
ENV METRICS_PORT=9100

# Expose the metrics port
EXPOSE 9100

# Run the exporter
CMD ["python3", "security_metrics_exporter.py"]