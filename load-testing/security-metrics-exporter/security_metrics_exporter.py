#!/usr/bin/env python3
# Security Metrics Exporter for Phoenix Orchestrator
# This script collects and exports security metrics for Prometheus

import os
import time
import json
import random
import logging
from datetime import datetime
from prometheus_client import start_http_server, Gauge, Counter, Summary, Enum, Info

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('security-metrics-exporter')

# Define metrics
class SecurityMetrics:
    def __init__(self):
        # Vulnerability metrics
        self.vulnerabilities = Gauge(
            'security_vulnerabilities', 
            'Number of security vulnerabilities',
            ['severity', 'component', 'status']
        )
        
        # Compliance metrics
        self.compliance_score = Gauge(
            'security_compliance_score',
            'Overall security compliance score (0-100)'
        )
        
        self.compliance_controls = Gauge(
            'security_compliance_controls',
            'Number of compliance controls',
            ['status']  # implemented, failed, pending
        )
        
        # Incident metrics
        self.incidents_open = Gauge(
            'security_incidents_open',
            'Number of open security incidents',
            ['severity']
        )
        
        self.incident_created = Counter(
            'security_incident_created',
            'Security incident created',
            ['severity', 'type']
        )
        
        self.incident_resolved = Counter(
            'security_incident_resolved',
            'Security incident resolved',
            ['severity', 'type']
        )
        
        # Response time metrics
        self.response_time = Summary(
            'security_incident_response_time',
            'Time to respond to security incidents in seconds',
            ['severity']
        )
        
        # Dependency metrics
        self.dependency_status = Gauge(
            'security_dependencies',
            'Dependency status metrics',
            ['status']  # up_to_date, outdated, vulnerable
        )
        
        self.dependency_license = Gauge(
            'security_dependency_licenses',
            'Dependency license compliance',
            ['status']  # compliant, non_compliant
        )
        
        # SAST metrics
        self.sast_coverage = Gauge(
            'security_sast_coverage',
            'Percentage of code covered by SAST'
        )
        
        self.sast_findings = Gauge(
            'security_sast_findings',
            'Number of SAST findings',
            ['severity']
        )
        
        # Container metrics
        self.container_vulnerabilities = Gauge(
            'security_container_vulnerabilities',
            'Number of container vulnerabilities',
            ['severity', 'service', 'fixable']
        )
        
        self.container_compliance = Gauge(
            'security_container_compliance',
            'Container security best practices compliance',
            ['service']
        )
        
    def simulate_data(self):
        """Simulate security metrics data for testing purposes"""
        # Clear previous metrics to prevent stale data
        self.vulnerabilities._metrics.clear()
        self.incidents_open._metrics.clear()
        self.container_vulnerabilities._metrics.clear()
        
        # Simulate vulnerability metrics
        severity_levels = ['critical', 'high', 'medium', 'low']
        components = ['code', 'dependencies', 'containers', 'infrastructure']
        statuses = ['open', 'in_progress', 'resolved']
        
        # Realistic distribution - more low and medium than high and critical
        for severity in severity_levels:
            for component in components:
                for status in statuses:
                    # Generate somewhat realistic values based on severity and status
                    if severity == 'critical' and status == 'open':
                        value = random.randint(0, 2)  # Few critical open issues
                    elif severity == 'high' and status == 'open':
                        value = random.randint(1, 5)  # Some high open issues
                    elif severity == 'medium' and status == 'open':
                        value = random.randint(3, 15)  # More medium open issues
                    elif severity == 'low' and status == 'open':
                        value = random.randint(5, 20)  # Many low open issues
                    elif status == 'in_progress':
                        value = random.randint(1, 10)  # Some in progress
                    elif status == 'resolved':
                        value = random.randint(10, 50)  # Many resolved
                    
                    self.vulnerabilities.labels(severity, component, status).set(value)
        
        # Compliance score - should be between 0 and 100
        self.compliance_score.set(random.uniform(75.0, 95.0))
        
        # Compliance controls
        self.compliance_controls.labels('implemented').set(random.randint(80, 120))
        self.compliance_controls.labels('failed').set(random.randint(0, 10))
        self.compliance_controls.labels('pending').set(random.randint(5, 20))
        
        # Open incidents
        self.incidents_open.labels('critical').set(random.randint(0, 1))
        self.incidents_open.labels('high').set(random.randint(0, 3))
        self.incidents_open.labels('medium').set(random.randint(1, 5))
        self.incidents_open.labels('low').set(random.randint(2, 10))
        
        # Dependency status
        self.dependency_status.labels('up_to_date').set(random.randint(80, 150))
        self.dependency_status.labels('outdated').set(random.randint(10, 30))
        self.dependency_status.labels('vulnerable').set(random.randint(0, 8))
        
        # License compliance
        self.dependency_license.labels('compliant').set(random.randint(90, 180))
        self.dependency_license.labels('non_compliant').set(random.randint(0, 5))
        
        # SAST coverage
        self.sast_coverage.set(random.uniform(80.0, 95.0))
        
        # SAST findings
        self.sast_findings.labels('critical').set(random.randint(0, 2))
        self.sast_findings.labels('high').set(random.randint(1, 7))
        self.sast_findings.labels('medium').set(random.randint(5, 15))
        self.sast_findings.labels('low').set(random.randint(10, 25))
        
        # Container vulnerabilities
        services = ['api-gateway-rs', 'auth-service-rs', 'data-router-rs', 'llm-service-rs', 
                   'orchestrator-service-rs', 'tools-service-rs']
        
        for service in services:
            # Compliance score per service (0-100)
            self.container_compliance.labels(service).set(random.uniform(80.0, 98.0))
            
            for severity in severity_levels:
                for fixable in ['true', 'false']:
                    if severity == 'critical':
                        value = random.randint(0, 1)
                    elif severity == 'high':
                        value = random.randint(0, 3)
                    elif severity == 'medium':
                        value = random.randint(2, 8)
                    else:  # low
                        value = random.randint(3, 15)
                        
                    self.container_vulnerabilities.labels(
                        severity, service, fixable
                    ).set(value)
        
        # Occasionally simulate a new security incident
        if random.random() < 0.05:  # 5% chance per cycle
            incident_types = ['vulnerability', 'access', 'malware', 'configuration']
            severity = random.choice(['low', 'medium', 'high', 'critical'])
            incident_type = random.choice(incident_types)
            
            self.incident_created.labels(severity, incident_type).inc()
            logger.info(f"Simulated new {severity} {incident_type} security incident")
            
        # Occasionally simulate resolving an incident
        if random.random() < 0.08:  # 8% chance per cycle
            incident_types = ['vulnerability', 'access', 'malware', 'configuration']
            severity = random.choice(['low', 'medium', 'high', 'critical'])
            incident_type = random.choice(incident_types)
            
            self.incident_resolved.labels(severity, incident_type).inc()
            # Simulate response time
            if severity == 'critical':
                response_time = random.uniform(300, 3600)  # 5min - 1hr
            elif severity == 'high':
                response_time = random.uniform(3600, 14400)  # 1-4hrs
            elif severity == 'medium':
                response_time = random.uniform(14400, 86400)  # 4-24hrs
            else:
                response_time = random.uniform(86400, 259200)  # 1-3 days
                
            self.response_time.labels(severity).observe(response_time)
            logger.info(f"Simulated resolved {severity} {incident_type} security incident")


def main():
    # Start up the server to expose metrics
    port = int(os.environ.get('METRICS_PORT', 9100))
    start_http_server(port)
    logger.info(f"Security metrics server started on port {port}")
    
    # Initialize metrics
    metrics = SecurityMetrics()
    
    # Generate initial data
    metrics.simulate_data()
    
    # Update metrics every minute
    while True:
        metrics.simulate_data()
        logger.info("Security metrics updated")
        time.sleep(60)


if __name__ == '__main__':
    main()