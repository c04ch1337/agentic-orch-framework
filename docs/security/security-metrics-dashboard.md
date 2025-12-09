# Security Metrics Dashboard

This document outlines the structure and implementation of the Phoenix Orchestrator security metrics dashboard and reporting framework.

## Overview

The security metrics dashboard provides real-time visibility into the security posture of the Phoenix Orchestrator platform. It aggregates data from multiple security tools and presents key metrics in a digestible format for both technical and non-technical stakeholders.

## Dashboard Components

### 1. Vulnerability Metrics

* **Critical Vulnerabilities**
  * Count of open critical vulnerabilities in dependencies
  * Count of critical container vulnerabilities
  * Count of critical code vulnerabilities
  * Time-to-fix trends for critical vulnerabilities

* **Medium/High Vulnerabilities**
  * Count by severity level
  * Count by component/service
  * Age of open vulnerabilities
  * Mean time to remediation

* **Dependency Health**
  * Percentage of dependencies up to date
  * Count of dependencies with available updates
  * License compliance percentage
  * Dependency freshness score

### 2. Security Testing Coverage

* **SAST Coverage**
  * Percentage of codebase covered by static analysis
  * False positive rate
  * Issue dismissal rate
  * New issues detected per scan

* **Container Security**
  * Percentage of containers scanned
  * Base image update status
  * Security best practice compliance score
  * Average vulnerabilities per container

* **Secret Detection**
  * Secret detection coverage
  * Number of secrets detected
  * Secret rotation status
  * Vault integration health

### 3. Compliance Metrics

* **Control Implementation**
  * Percentage of required controls implemented
  * Failed controls count
  * Control testing frequency
  * Control effectiveness score

* **Policy Compliance**
  * Rate of policy exceptions
  * Policy violations by severity
  * Policy awareness score
  * Automated policy enforcement percentage

### 4. Security Operations

* **Incident Metrics**
  * Open security incidents by severity
  * Mean time to detect (MTTD)
  * Mean time to resolve (MTTR)
  * Incident recurrence rate

* **Response Effectiveness**
  * Average incident response time
  * Percentage of incidents with post-mortems
  * Percentage of implemented recommendations
  * Security team engagement metrics

## Grafana Dashboard Implementation

The security metrics dashboard is implemented in Grafana with data sourced from Prometheus. Below is the configuration for the main security dashboard.

### Dashboard JSON

Create a file at `load-testing/configs/grafana/dashboards/security-metrics-dashboard.json` with the following content:

```json
{
  "annotations": {
    "list": [
      {
        "builtIn": 1,
        "datasource": {
          "type": "grafana",
          "uid": "-- Grafana --"
        },
        "enable": true,
        "hide": true,
        "iconColor": "rgba(0, 211, 255, 1)",
        "name": "Annotations & Alerts",
        "type": "dashboard"
      },
      {
        "datasource": {
          "type": "prometheus",
          "uid": "prometheus"
        },
        "enable": true,
        "iconColor": "red",
        "name": "Security Incidents",
        "target": {
          "expr": "security_incident_created",
          "refId": "A"
        }
      }
    ]
  },
  "editable": true,
  "fiscalYearStartMonth": 0,
  "graphTooltip": 0,
  "id": null,
  "links": [],
  "liveNow": false,
  "panels": [
    {
      "collapsed": false,
      "gridPos": {
        "h": 1,
        "w": 24,
        "x": 0,
        "y": 0
      },
      "id": 1,
      "panels": [],
      "title": "Security Posture Overview",
      "type": "row"
    },
    {
      "datasource": {
        "type": "prometheus",
        "uid": "prometheus"
      },
      "fieldConfig": {
        "defaults": {
          "color": {
            "mode": "thresholds"
          },
          "mappings": [],
          "thresholds": {
            "mode": "absolute",
            "steps": [
              {
                "color": "green",
                "value": null
              },
              {
                "color": "red",
                "value": 1
              }
            ]
          }
        },
        "overrides": []
      },
      "gridPos": {
        "h": 5,
        "w": 6,
        "x": 0,
        "y": 1
      },
      "id": 2,
      "options": {
        "colorMode": "value",
        "graphMode": "area",
        "justifyMode": "auto",
        "orientation": "auto",
        "reduceOptions": {
          "calcs": [
            "lastNotNull"
          ],
          "fields": "",
          "values": false
        },
        "textMode": "auto"
      },
      "pluginVersion": "9.5.2",
      "targets": [
        {
          "datasource": {
            "type": "prometheus",
            "uid": "prometheus"
          },
          "expr": "sum(security_vulnerabilities{severity=\"critical\", status=\"open\"})",
          "refId": "A"
        }
      ],
      "title": "Critical Vulnerabilities",
      "type": "stat"
    },
    {
      "datasource": {
        "type": "prometheus",
        "uid": "prometheus"
      },
      "fieldConfig": {
        "defaults": {
          "color": {
            "mode": "thresholds"
          },
          "mappings": [],
          "thresholds": {
            "mode": "absolute",
            "steps": [
              {
                "color": "green",
                "value": null
              },
              {
                "color": "yellow",
                "value": 5
              },
              {
                "color": "orange",
                "value": 10
              },
              {
                "color": "red",
                "value": 20
              }
            ]
          }
        },
        "overrides": []
      },
      "gridPos": {
        "h": 5,
        "w": 6,
        "x": 6,
        "y": 1
      },
      "id": 3,
      "options": {
        "colorMode": "value",
        "graphMode": "area",
        "justifyMode": "auto",
        "orientation": "auto",
        "reduceOptions": {
          "calcs": [
            "lastNotNull"
          ],
          "fields": "",
          "values": false
        },
        "textMode": "auto"
      },
      "pluginVersion": "9.5.2",
      "targets": [
        {
          "datasource": {
            "type": "prometheus",
            "uid": "prometheus"
          },
          "expr": "sum(security_vulnerabilities{severity=\"high\", status=\"open\"})",
          "refId": "A"
        }
      ],
      "title": "High Vulnerabilities",
      "type": "stat"
    },
    {
      "datasource": {
        "type": "prometheus",
        "uid": "prometheus"
      },
      "fieldConfig": {
        "defaults": {
          "color": {
            "mode": "thresholds"
          },
          "mappings": [],
          "max": 100,
          "min": 0,
          "thresholds": {
            "mode": "absolute",
            "steps": [
              {
                "color": "red",
                "value": null
              },
              {
                "color": "orange",
                "value": 60
              },
              {
                "color": "yellow",
                "value": 80
              },
              {
                "color": "green",
                "value": 90
              }
            ]
          },
          "unit": "percent"
        },
        "overrides": []
      },
      "gridPos": {
        "h": 5,
        "w": 6,
        "x": 12,
        "y": 1
      },
      "id": 4,
      "options": {
        "orientation": "auto",
        "reduceOptions": {
          "calcs": [
            "lastNotNull"
          ],
          "fields": "",
          "values": false
        },
        "showThresholdLabels": false,
        "showThresholdMarkers": true
      },
      "pluginVersion": "9.5.2",
      "targets": [
        {
          "datasource": {
            "type": "prometheus",
            "uid": "prometheus"
          },
          "expr": "security_compliance_score",
          "refId": "A"
        }
      ],
      "title": "Compliance Score",
      "type": "gauge"
    },
    {
      "datasource": {
        "type": "prometheus",
        "uid": "prometheus"
      },
      "fieldConfig": {
        "defaults": {
          "color": {
            "mode": "thresholds"
          },
          "mappings": [],
          "thresholds": {
            "mode": "absolute",
            "steps": [
              {
                "color": "green",
                "value": null
              },
              {
                "color": "red",
                "value": 5
              }
            ]
          }
        },
        "overrides": []
      },
      "gridPos": {
        "h": 5,
        "w": 6,
        "x": 18,
        "y": 1
      },
      "id": 5,
      "options": {
        "colorMode": "value",
        "graphMode": "area",
        "justifyMode": "auto",
        "orientation": "auto",
        "reduceOptions": {
          "calcs": [
            "lastNotNull"
          ],
          "fields": "",
          "values": false
        },
        "textMode": "auto"
      },
      "pluginVersion": "9.5.2",
      "targets": [
        {
          "datasource": {
            "type": "prometheus",
            "uid": "prometheus"
          },
          "expr": "sum(security_incidents_open)",
          "refId": "A"
        }
      ],
      "title": "Open Security Incidents",
      "type": "stat"
    },
    {
      "collapsed": false,
      "gridPos": {
        "h": 1,
        "w": 24,
        "x": 0,
        "y": 6
      },
      "id": 6,
      "panels": [],
      "title": "Vulnerability Trends",
      "type": "row"
    },
    {
      "datasource": {
        "type": "prometheus",
        "uid": "prometheus"
      },
      "fieldConfig": {
        "defaults": {
          "color": {
            "mode": "palette-classic"
          },
          "custom": {
            "axisCenteredZero": false,
            "axisColorMode": "text",
            "axisLabel": "",
            "axisPlacement": "auto",
            "barAlignment": 0,
            "drawStyle": "line",
            "fillOpacity": 10,
            "gradientMode": "none",
            "hideFrom": {
              "legend": false,
              "tooltip": false,
              "viz": false
            },
            "lineInterpolation": "linear",
            "lineWidth": 1,
            "pointSize": 5,
            "scaleDistribution": {
              "type": "linear"
            },
            "showPoints": "never",
            "spanNulls": false,
            "stacking": {
              "group": "A",
              "mode": "none"
            },
            "thresholdsStyle": {
              "mode": "off"
            }
          },
          "mappings": [],
          "thresholds": {
            "mode": "absolute",
            "steps": [
              {
                "color": "green",
                "value": null
              }
            ]
          },
          "unit": "none"
        },
        "overrides": [
          {
            "matcher": {
              "id": "byName",
              "options": "Critical"
            },
            "properties": [
              {
                "id": "color",
                "value": {
                  "fixedColor": "red",
                  "mode": "fixed"
                }
              }
            ]
          },
          {
            "matcher": {
              "id": "byName",
              "options": "High"
            },
            "properties": [
              {
                "id": "color",
                "value": {
                  "fixedColor": "orange",
                  "mode": "fixed"
                }
              }
            ]
          },
          {
            "matcher": {
              "id": "byName",
              "options": "Medium"
            },
            "properties": [
              {
                "id": "color",
                "value": {
                  "fixedColor": "yellow",
                  "mode": "fixed"
                }
              }
            ]
          },
          {
            "matcher": {
              "id": "byName",
              "options": "Low"
            },
            "properties": [
              {
                "id": "color",
                "value": {
                  "fixedColor": "green",
                  "mode": "fixed"
                }
              }
            ]
          }
        ]
      },
      "gridPos": {
        "h": 8,
        "w": 12,
        "x": 0,
        "y": 7
      },
      "id": 7,
      "options": {
        "legend": {
          "calcs": [
            "mean",
            "lastNotNull",
            "max"
          ],
          "displayMode": "table",
          "placement": "bottom",
          "showLegend": true
        },
        "tooltip": {
          "mode": "multi",
          "sort": "none"
        }
      },
      "pluginVersion": "9.5.2",
      "targets": [
        {
          "datasource": {
            "type": "prometheus",
            "uid": "prometheus"
          },
          "expr": "sum(security_vulnerabilities{severity=\"critical\", status=\"open\"}) or vector(0)",
          "legendFormat": "Critical",
          "refId": "A"
        },
        {
          "datasource": {
            "type": "prometheus",
            "uid": "prometheus"
          },
          "expr": "sum(security_vulnerabilities{severity=\"high\", status=\"open\"}) or vector(0)",
          "legendFormat": "High",
          "refId": "B"
        },
        {
          "datasource": {
            "type": "prometheus",
            "uid": "prometheus"
          },
          "expr": "sum(security_vulnerabilities{severity=\"medium\", status=\"open\"}) or vector(0)",
          "legendFormat": "Medium",
          "refId": "C"
        },
        {
          "datasource": {
            "type": "prometheus",
            "uid": "prometheus"
          },
          "expr": "sum(security_vulnerabilities{severity=\"low\", status=\"open\"}) or vector(0)",
          "legendFormat": "Low",
          "refId": "D"
        }
      ],
      "title": "Vulnerability Trends by Severity",
      "type": "timeseries"
    },
    {
      "datasource": {
        "type": "prometheus",
        "uid": "prometheus"
      },
      "fieldConfig": {
        "defaults": {
          "color": {
            "mode": "palette-classic"
          },
          "custom": {
            "axisCenteredZero": false,
            "axisColorMode": "text",
            "axisLabel": "",
            "axisPlacement": "auto",
            "barAlignment": 0,
            "drawStyle": "line",
            "fillOpacity": 10,
            "gradientMode": "none",
            "hideFrom": {
              "legend": false,
              "tooltip": false,
              "viz": false
            },
            "lineInterpolation": "linear",
            "lineWidth": 1,
            "pointSize": 5,
            "scaleDistribution": {
              "type": "linear"
            },
            "showPoints": "never",
            "spanNulls": false,
            "stacking": {
              "group": "A",
              "mode": "none"
            },
            "thresholdsStyle": {
              "mode": "off"
            }
          },
          "mappings": [],
          "thresholds": {
            "mode": "absolute",
            "steps": [
              {
                "color": "green",
                "value": null
              }
            ]
          },
          "unit": "none"
        },
        "overrides": []
      },
      "gridPos": {
        "h": 8,
        "w": 12,
        "x": 12,
        "y": 7
      },
      "id": 8,
      "options": {
        "legend": {
          "calcs": [
            "lastNotNull",
            "max"
          ],
          "displayMode": "table",
          "placement": "bottom",
          "showLegend": true
        },
        "tooltip": {
          "mode": "multi",
          "sort": "none"
        }
      },
      "pluginVersion": "9.5.2",
      "targets": [
        {
          "datasource": {
            "type": "prometheus",
            "uid": "prometheus"
          },
          "expr": "sum(security_vulnerabilities{component=\"dependencies\", status=\"open\"}) or vector(0)",
          "legendFormat": "Dependencies",
          "refId": "A"
        },
        {
          "datasource": {
            "type": "prometheus",
            "uid": "prometheus"
          },
          "expr": "sum(security_vulnerabilities{component=\"containers\", status=\"open\"}) or vector(0)",
          "legendFormat": "Containers",
          "refId": "B"
        },
        {
          "datasource": {
            "type": "prometheus",
            "uid": "prometheus"
          },
          "expr": "sum(security_vulnerabilities{component=\"code\", status=\"open\"}) or vector(0)",
          "legendFormat": "Code",
          "refId": "C"
        },
        {
          "datasource": {
            "type": "prometheus",
            "uid": "prometheus"
          },
          "expr": "sum(security_vulnerabilities{component=\"infrastructure\", status=\"open\"}) or vector(0)",
          "legendFormat": "Infrastructure",
          "refId": "D"
        }
      ],
      "title": "Vulnerability Trends by Component",
      "type": "timeseries"
    }
  ],
  "refresh": "5m",
  "schemaVersion": 38,
  "style": "dark",
  "tags": [
    "security",
    "monitoring",
    "phoenix"
  ],
  "templating": {
    "list": []
  },
  "time": {
    "from": "now-7d",
    "to": "now"
  },
  "timepicker": {},
  "timezone": "",
  "title": "Phoenix Security Metrics Dashboard",
  "uid": "phoenix-security-metrics",
  "version": 1,
  "weekStart": ""
}
```

## Prometheus Metrics Collection

To support the dashboard, we need to define metrics for Prometheus to collect. Add the following to your Prometheus configuration:

### Security Metrics Definitions

Create a new metrics file at `load-testing/configs/prometheus/security-metrics.yml`:

```yaml
- job_name: 'security-metrics'
  scrape_interval: 1h
  static_configs:
    - targets: ['security-metrics-exporter:9100']
  metrics_path: /metrics

- job_name: 'vulnerability-metrics'
  scrape_interval: 6h
  static_configs:
    - targets: ['vulnerability-metrics-exporter:9101']
  metrics_path: /metrics
```

## Reporting Framework

The security metrics dashboard is complemented by a regular reporting framework that provides both automated and human-analyzed security insights.

### Automated Report Generation

Security reports are automatically generated on the following schedule:

| Report Type | Frequency | Distribution | Content |
|-------------|-----------|--------------|---------|
| Daily Security Summary | Daily (8 AM) | Security Team | New vulnerabilities, status changes |
| Weekly Security Report | Monday (9 AM) | Security & Dev Teams | Detailed vulnerability metrics, trends, actions required |
| Monthly Security Review | 1st of Month | Management & Stakeholders | Executive summary, risk assessment, compliance status |
| Quarterly Security Assessment | End of Quarter | Executive Team | Comprehensive review, strategic recommendations |

### Report Format

All security reports follow a consistent format to ensure readability and action-oriented content:

1. **Executive Summary**
   - Overall security status (using traffic light system)
   - Key metrics and changes since last report
   - Critical items requiring immediate attention

2. **Metric Details**
   - Vulnerabilities by severity and component
   - Mean time to remediation trends
   - Policy compliance metrics
   - Dependencies and license status

3. **Risk Analysis**
   - Current risk assessment
   - Changes in risk profile
   - Emerging threats

4. **Action Items**
   - Prioritized list of required actions
   - Owners and due dates
   - Dependencies for completion

5. **Compliance Status**
   - Compliance with required standards
   - Exceptions and compensating controls
   - Audit readiness assessment

### Integration with JIRA

Security findings that require remediation are automatically converted into JIRA tickets via the following process:

1. Findings of Critical or High severity are immediately created as JIRA tickets with Security priority
2. Medium findings are created as normal priority tickets
3. Low findings are batched weekly
4. All tickets include detailed reproduction steps, impact assessment, and recommended fix approaches

## Compliance Framework

The Phoenix Orchestrator platform implements a comprehensive compliance framework based on the following standards:

1. **OWASP Top 10** - Application security risks
2. **NIST Cybersecurity Framework** - Overall security program
3. **CIS Critical Security Controls** - Technical security controls
4. **Industry-specific regulations** as applicable

The compliance framework is supported by:

1. **Control Mapping** - Each security control is mapped to relevant compliance requirements
2. **Evidence Collection** - Automated collection of evidence for compliance verification
3. **Continuous Validation** - Regular testing of controls to ensure effectiveness
4. **Gap Analysis** - Identification of compliance gaps and remediation tracking

## Implementation Roadmap

| Phase | Timeframe | Actions |
|-------|-----------|---------|
| 1 | Week 1 | Deploy security metrics collectors and configure Prometheus |
| 2 | Week 1-2 | Configure Grafana dashboard and alerting rules |
| 3 | Week 2 | Implement automated reporting framework |
| 4 | Week 3 | Integrate with ticketing system for findings |
| 5 | Week 4 | Conduct initial assessment and establish baselines |
| 6 | Week 5+ | Begin regular reporting cycle and continuous improvement |

## Conclusion

The Security Metrics Dashboard and Reporting Framework provides Phoenix Orchestrator with:

1. **Visibility** - Real-time view of security posture
2. **Accountability** - Clear metrics and ownership for security items
3. **Compliance** - Evidence collection and validation for audits
4. **Improvement** - Trend analysis to drive continuous enhancement of security controls

By implementing these measures, Phoenix Orchestrator maintains a strong security posture with measurable results and actionable intelligence.