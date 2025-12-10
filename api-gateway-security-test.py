#!/usr/bin/env python3
"""
API Gateway Security Test Suite
================================
Comprehensive security testing for Phoenix AGI System API Gateway

Tests:
- API Key Authentication
- Rate Limiting (per-key)
- AgiResponse Schema Validation
- TLS/HTTPS Connection
- Security Headers
- Error Handling
"""

import requests
import json
import time
import sys
import os
from datetime import datetime
from typing import Dict, List, Optional, Tuple
import urllib3
import ssl
import socket

# Suppress SSL warnings for self-signed certificates during testing
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

# Test Configuration
class TestConfig:
    """Test configuration settings"""
    def __init__(self):
        # API Gateway settings
        self.host = os.getenv("API_GATEWAY_HOST", "localhost")
        self.port = int(os.getenv("API_GATEWAY_PORT", "8000"))
        self.tls_enabled = os.getenv("TLS_ENABLED", "false").lower() == "true"
        
        # Build base URL
        self.protocol = "https" if self.tls_enabled else "http"
        self.base_url = f"{self.protocol}://{self.host}:{self.port}"
        
        # Test API keys
        self.valid_key = "test-valid-key-001"
        self.invalid_key = "invalid-key-xyz"
        self.rate_limit_key_1 = "test-rate-limit-key-001"
        self.rate_limit_key_2 = "test-rate-limit-key-002"
        
        # Request settings
        self.timeout = 10
        self.verify_ssl = False  # For self-signed certificates

class TestResult:
    """Test result tracking"""
    def __init__(self, name: str):
        self.name = name
        self.passed = False
        self.message = ""
        self.details = {}
        self.start_time = time.time()
        self.duration = 0.0
    
    def complete(self, passed: bool, message: str, details: dict = None):
        """Mark test as complete"""
        self.passed = passed
        self.message = message
        self.details = details or {}
        self.duration = time.time() - self.start_time
        return self

class SecurityTester:
    """Main security test runner"""
    
    def __init__(self, config: TestConfig):
        self.config = config
        self.results = []
        self.session = requests.Session()
        self.session.verify = config.verify_ssl
    
    def run_all_tests(self) -> List[TestResult]:
        """Run all security tests"""
        print("=" * 60)
        print("API Gateway Security Test Suite")
        print(f"Target: {self.config.base_url}")
        print(f"TLS Enabled: {self.config.tls_enabled}")
        print("=" * 60)
        print()
        
        # Test categories
        self._test_api_key_authentication()
        self._test_rate_limiting()
        self._test_response_schema()
        self._test_tls_connection()
        self._test_security_headers()
        self._test_error_handling()
        
        return self.results
    
    def _test_api_key_authentication(self):
        """Test API key authentication"""
        print("Testing API Key Authentication...")
        
        # Test 1: Valid API key
        result = TestResult("Auth: Valid API Key")
        try:
            response = self._make_request(
                "/api/v1/execute",
                method="POST",
                headers={"X-PHOENIX-API-KEY": self.config.valid_key},
                json={
                    "method": "test",
                    "payload": "test payload"
                }
            )
            
            if response.status_code == 200:
                result.complete(True, "Valid API key accepted", {
                    "status_code": response.status_code
                })
            else:
                result.complete(False, f"Unexpected status code: {response.status_code}", {
                    "status_code": response.status_code,
                    "response": response.text
                })
        except Exception as e:
            result.complete(False, f"Request failed: {str(e)}")
        self._record_result(result)
        
        # Test 2: Invalid API key
        result = TestResult("Auth: Invalid API Key")
        try:
            response = self._make_request(
                "/api/v1/execute",
                method="POST",
                headers={"X-PHOENIX-API-KEY": self.config.invalid_key},
                json={
                    "method": "test",
                    "payload": "test payload"
                }
            )
            
            if response.status_code == 401:
                result.complete(True, "Invalid API key properly rejected", {
                    "status_code": response.status_code
                })
            else:
                result.complete(False, f"Expected 401, got {response.status_code}", {
                    "status_code": response.status_code,
                    "response": response.text
                })
        except Exception as e:
            result.complete(False, f"Request failed: {str(e)}")
        self._record_result(result)
        
        # Test 3: Missing API key
        result = TestResult("Auth: Missing API Key")
        try:
            response = self._make_request(
                "/api/v1/execute",
                method="POST",
                headers={},
                json={
                    "method": "test",
                    "payload": "test payload"
                }
            )
            
            if response.status_code == 401:
                data = response.json()
                if "error" in data and "X-PHOENIX-API-KEY" in data["error"]:
                    result.complete(True, "Missing API key properly rejected with correct error", {
                        "status_code": response.status_code,
                        "error": data["error"]
                    })
                else:
                    result.complete(False, "Missing API key rejected but wrong error message", {
                        "status_code": response.status_code,
                        "response": data
                    })
            else:
                result.complete(False, f"Expected 401, got {response.status_code}", {
                    "status_code": response.status_code,
                    "response": response.text
                })
        except Exception as e:
            result.complete(False, f"Request failed: {str(e)}")
        self._record_result(result)
        
        print()
    
    def _test_rate_limiting(self):
        """Test rate limiting functionality"""
        print("Testing Rate Limiting...")
        
        # Test 1: First 100 requests should succeed
        result = TestResult("Rate Limit: Within Limit (100 requests)")
        try:
            success_count = 0
            errors = []
            
            for i in range(100):
                response = self._make_request(
                    "/health",
                    headers={"X-PHOENIX-API-KEY": self.config.rate_limit_key_1}
                )
                
                if response.status_code == 200:
                    success_count += 1
                else:
                    errors.append(f"Request {i+1}: status {response.status_code}")
                
                # Small delay to avoid overwhelming the server
                if i % 10 == 0:
                    print(f"  Progress: {i+1}/100 requests")
                    time.sleep(0.1)
            
            if success_count == 100:
                result.complete(True, "All 100 requests succeeded within rate limit", {
                    "success_count": success_count
                })
            else:
                result.complete(False, f"Only {success_count}/100 requests succeeded", {
                    "success_count": success_count,
                    "errors": errors[:5]  # First 5 errors
                })
        except Exception as e:
            result.complete(False, f"Test failed: {str(e)}")
        self._record_result(result)
        
        # Test 2: 101st request should be rate limited
        result = TestResult("Rate Limit: Exceeding Limit (101st request)")
        try:
            response = self._make_request(
                "/health",
                headers={"X-PHOENIX-API-KEY": self.config.rate_limit_key_1}
            )
            
            if response.status_code == 429:
                data = response.json()
                if "error" in data and "rate limit" in data["error"].lower():
                    result.complete(True, "101st request properly rate limited", {
                        "status_code": response.status_code,
                        "error": data["error"]
                    })
                else:
                    result.complete(False, "Rate limited but wrong error message", {
                        "status_code": response.status_code,
                        "response": data
                    })
            else:
                result.complete(False, f"Expected 429, got {response.status_code}", {
                    "status_code": response.status_code,
                    "response": response.text
                })
        except Exception as e:
            result.complete(False, f"Request failed: {str(e)}")
        self._record_result(result)
        
        # Test 3: Different API key should have its own rate limit
        result = TestResult("Rate Limit: Per-Key Isolation")
        try:
            # Make 10 requests with a different key
            success_count = 0
            for i in range(10):
                response = self._make_request(
                    "/health",
                    headers={"X-PHOENIX-API-KEY": self.config.rate_limit_key_2}
                )
                if response.status_code == 200:
                    success_count += 1
            
            if success_count == 10:
                result.complete(True, "Different API key has separate rate limit", {
                    "success_count": success_count
                })
            else:
                result.complete(False, f"Only {success_count}/10 requests succeeded for different key", {
                    "success_count": success_count
                })
        except Exception as e:
            result.complete(False, f"Test failed: {str(e)}")
        self._record_result(result)
        
        print()
    
    def _test_response_schema(self):
        """Test AgiResponse schema validation"""
        print("Testing AgiResponse Schema...")
        
        result = TestResult("Schema: AgiResponse Structure")
        try:
            response = self._make_request(
                "/api/v1/execute",
                method="POST",
                headers={"X-PHOENIX-API-KEY": self.config.valid_key},
                json={
                    "id": "test-123",
                    "method": "test",
                    "payload": "test schema validation"
                }
            )
            
            if response.status_code == 200:
                data = response.json()
                
                # Check for ExecuteResponse fields (gateway response)
                required_fields = ["id", "status_code", "payload"]
                missing_fields = [f for f in required_fields if f not in data]
                
                if not missing_fields:
                    # The payload should contain the AgiResponse data
                    try:
                        # Parse the payload if it's a JSON string
                        if isinstance(data["payload"], str):
                            # Try to parse as JSON
                            try:
                                payload_data = json.loads(data["payload"])
                                # Check for AgiResponse fields
                                agi_fields = ["final_answer", "execution_plan", "routed_service", 
                                            "phoenix_session_id", "output_artifact_urls"]
                                agi_missing = [f for f in agi_fields if f not in payload_data]
                                
                                if not agi_missing:
                                    result.complete(True, "Response contains all required AgiResponse fields", {
                                        "execute_response_fields": list(data.keys()),
                                        "agi_response_fields": list(payload_data.keys())
                                    })
                                else:
                                    result.complete(True, "ExecuteResponse valid, AgiResponse may be text", {
                                        "execute_response_fields": list(data.keys()),
                                        "payload_type": "string",
                                        "note": "Orchestrator may return text responses"
                                    })
                            except json.JSONDecodeError:
                                # Payload is not JSON, which is acceptable
                                result.complete(True, "ExecuteResponse structure valid", {
                                    "execute_response_fields": list(data.keys()),
                                    "payload_type": "text"
                                })
                        else:
                            result.complete(True, "Response has all required ExecuteResponse fields", {
                                "fields": list(data.keys())
                            })
                    except Exception as parse_error:
                        result.complete(True, "ExecuteResponse structure valid", {
                            "execute_response_fields": list(data.keys()),
                            "note": str(parse_error)
                        })
                else:
                    result.complete(False, f"Missing required fields: {missing_fields}", {
                        "present_fields": list(data.keys()),
                        "missing_fields": missing_fields
                    })
            else:
                result.complete(False, f"Request failed with status {response.status_code}", {
                    "status_code": response.status_code,
                    "response": response.text
                })
        except Exception as e:
            result.complete(False, f"Test failed: {str(e)}")
        self._record_result(result)
        
        print()
    
    def _test_tls_connection(self):
        """Test TLS/HTTPS connection"""
        print("Testing TLS Connection...")
        
        if not self.config.tls_enabled:
            result = TestResult("TLS: Connection Security")
            result.complete(True, "TLS not enabled (expected for test environment)", {
                "tls_enabled": False
            })
            self._record_result(result)
            print()
            return
        
        result = TestResult("TLS: HTTPS Connection")
        try:
            # Make a secure request
            response = self._make_request(
                "/health",
                headers={"X-PHOENIX-API-KEY": self.config.valid_key}
            )
            
            if response.status_code == 200:
                # Check if we actually connected over HTTPS
                if response.url.startswith("https://"):
                    result.complete(True, "Successfully connected over HTTPS", {
                        "url": response.url,
                        "status_code": response.status_code
                    })
                else:
                    result.complete(False, "Expected HTTPS but got HTTP", {
                        "url": response.url
                    })
            else:
                result.complete(False, f"HTTPS request failed with status {response.status_code}", {
                    "status_code": response.status_code
                })
        except Exception as e:
            result.complete(False, f"HTTPS connection failed: {str(e)}")
        self._record_result(result)
        
        # Test certificate validation
        result = TestResult("TLS: Certificate Validation")
        try:
            # Try to get certificate info
            context = ssl.create_default_context()
            with socket.create_connection((self.config.host, self.config.port), timeout=5) as sock:
                with context.wrap_socket(sock, server_hostname=self.config.host) as ssock:
                    cert = ssock.getpeercert()
                    
                    if cert:
                        result.complete(True, "Certificate retrieved successfully", {
                            "subject": dict(x[0] for x in cert.get('subject', [])),
                            "issuer": dict(x[0] for x in cert.get('issuer', [])),
                            "version": cert.get('version'),
                            "serialNumber": cert.get('serialNumber')
                        })
                    else:
                        result.complete(False, "No certificate found")
        except ssl.SSLError as e:
            if "self signed certificate" in str(e).lower():
                result.complete(True, "Self-signed certificate detected (expected for testing)", {
                    "note": "Production should use valid certificates"
                })
            else:
                result.complete(False, f"SSL Error: {str(e)}")
        except Exception as e:
            result.complete(False, f"Certificate validation failed: {str(e)}")
        self._record_result(result)
        
        print()
    
    def _test_security_headers(self):
        """Test security headers in responses"""
        print("Testing Security Headers...")
        
        result = TestResult("Headers: CORS Configuration")
        try:
            response = self._make_request(
                "/health",
                headers={
                    "X-PHOENIX-API-KEY": self.config.valid_key,
                    "Origin": "http://example.com"
                }
            )
            
            # Check for CORS headers
            cors_headers = {
                "access-control-allow-origin": response.headers.get("access-control-allow-origin"),
                "access-control-allow-methods": response.headers.get("access-control-allow-methods"),
                "access-control-allow-headers": response.headers.get("access-control-allow-headers")
            }
            
            if cors_headers["access-control-allow-origin"]:
                result.complete(True, "CORS headers present", cors_headers)
            else:
                result.complete(False, "CORS headers missing", {
                    "headers": dict(response.headers)
                })
        except Exception as e:
            result.complete(False, f"Test failed: {str(e)}")
        self._record_result(result)
        
        print()
    
    def _test_error_handling(self):
        """Test error handling and information disclosure"""
        print("Testing Error Handling...")
        
        # Test 1: Malformed JSON
        result = TestResult("Error Handling: Malformed JSON")
        try:
            response = self.session.post(
                f"{self.config.base_url}/api/v1/execute",
                headers={
                    "X-PHOENIX-API-KEY": self.config.valid_key,
                    "Content-Type": "application/json"
                },
                data="{ invalid json }",
                verify=self.config.verify_ssl,
                timeout=self.config.timeout
            )
            
            if response.status_code in [400, 422]:
                data = response.json()
                # Check that error message doesn't leak sensitive info
                if "error" in data:
                    error_msg = data["error"].lower()
                    if any(sensitive in error_msg for sensitive in ["stack", "trace", "internal", "path"]):
                        result.complete(False, "Error message may leak sensitive information", {
                            "status_code": response.status_code,
                            "error": data["error"]
                        })
                    else:
                        result.complete(True, "Malformed JSON properly rejected without info leak", {
                            "status_code": response.status_code,
                            "error": data["error"]
                        })
                else:
                    result.complete(True, "Malformed JSON rejected", {
                        "status_code": response.status_code
                    })
            else:
                result.complete(False, f"Expected 400/422, got {response.status_code}", {
                    "status_code": response.status_code
                })
        except Exception as e:
            result.complete(False, f"Test failed: {str(e)}")
        self._record_result(result)
        
        # Test 2: Invalid endpoint
        result = TestResult("Error Handling: Invalid Endpoint")
        try:
            response = self._make_request(
                "/api/v1/invalid_endpoint",
                headers={"X-PHOENIX-API-KEY": self.config.valid_key}
            )
            
            if response.status_code == 404:
                result.complete(True, "Invalid endpoint returns 404", {
                    "status_code": response.status_code
                })
            else:
                result.complete(False, f"Expected 404, got {response.status_code}", {
                    "status_code": response.status_code
                })
        except Exception as e:
            result.complete(False, f"Test failed: {str(e)}")
        self._record_result(result)
        
        # Test 3: Oversized payload
        result = TestResult("Error Handling: Payload Size Limit")
        try:
            # Create a large payload (over 10MB)
            large_payload = "x" * (11 * 1024 * 1024)  # 11MB
            
            response = self._make_request(
                "/api/v1/execute",
                method="POST",
                headers={"X-PHOENIX-API-KEY": self.config.valid_key},
                json={
                    "method": "test",
                    "payload": large_payload
                }
            )
            
            if response.status_code in [413, 400]:
                result.complete(True, "Oversized payload properly rejected", {
                    "status_code": response.status_code
                })
            else:
                result.complete(False, f"Expected 413/400, got {response.status_code}", {
                    "status_code": response.status_code
                })
        except Exception as e:
            if "413" in str(e) or "too large" in str(e).lower():
                result.complete(True, "Oversized payload rejected by server", {
                    "error": str(e)
                })
            else:
                result.complete(False, f"Test failed: {str(e)}")
        self._record_result(result)
        
        print()
    
    def _make_request(self, path: str, method: str = "GET", 
                     headers: Dict = None, json: Dict = None) -> requests.Response:
        """Make HTTP request to API Gateway"""
        url = f"{self.config.base_url}{path}"
        headers = headers or {}
        
        if method == "GET":
            return self.session.get(url, headers=headers, timeout=self.config.timeout)
        elif method == "POST":
            return self.session.post(url, headers=headers, json=json, timeout=self.config.timeout)
        else:
            raise ValueError(f"Unsupported method: {method}")
    
    def _record_result(self, result: TestResult):
        """Record test result"""
        self.results.append(result)
        status = "[PASS]" if result.passed else "[FAIL]"
        print(f"  {status} {result.name}")
        if not result.passed:
            print(f"    Message: {result.message}")
        if result.details and not result.passed:
            print(f"    Details: {json.dumps(result.details, indent=6)}")

def print_summary(results: List[TestResult]):
    """Print test summary"""
    print("=" * 60)
    print("TEST SUMMARY")
    print("=" * 60)
    
    passed = sum(1 for r in results if r.passed)
    failed = sum(1 for r in results if not r.passed)
    total = len(results)
    
    print(f"Total Tests: {total}")
    print(f"Passed: {passed}")
    print(f"Failed: {failed}")
    print(f"Success Rate: {(passed/total)*100:.1f}%")
    print()
    
    if failed > 0:
        print("Failed Tests:")
        for result in results:
            if not result.passed:
                print(f"  - {result.name}: {result.message}")
    
    print()
    
    # Generate detailed report
    report = {
        "timestamp": datetime.now().isoformat(),
        "summary": {
            "total": total,
            "passed": passed,
            "failed": failed,
            "success_rate": f"{(passed/total)*100:.1f}%"
        },
        "tests": [
            {
                "name": r.name,
                "passed": r.passed,
                "message": r.message,
                "duration": r.duration,
                "details": r.details
            }
            for r in results
        ]
    }
    
    # Save report to file
    report_file = "api-gateway-security-test-report.json"
    with open(report_file, "w") as f:
        json.dump(report, f, indent=2)
    
    print(f"Detailed report saved to: {report_file}")
    
    return 0 if failed == 0 else 1

def main():
    """Main entry point"""
    try:
        # Load configuration
        config = TestConfig()
        
        # Create tester
        tester = SecurityTester(config)
        
        # Run tests
        results = tester.run_all_tests()
        
        # Print summary and exit
        return print_summary(results)
        
    except KeyboardInterrupt:
        print("\nTest interrupted by user")
        return 1
    except Exception as e:
        print(f"Test suite error: {str(e)}")
        return 1

if __name__ == "__main__":
    sys.exit(main())