import requests
import time
import sys
import asyncio
import aiohttp
import json
from datetime import datetime

def check_service_availability(base_url):
    """Check if the API Gateway service is running"""
    try:
        response = requests.get(f"{base_url}/health", timeout=5)
        return True
    except requests.exceptions.ConnectionError:
        print("\nERROR: API Gateway service is not running!")
        print("Please ensure the following:")
        print("1. API Gateway service is started")
        print("2. Service is running on http://localhost:8000")
        print("3. No firewall is blocking the connection")
        return False
    except Exception as e:
        print(f"\nERROR: Unexpected error checking service: {str(e)}")
        return False

async def test_api_security():
    base_url = "http://localhost:8000"
    valid_api_key = "phoenix-default-key-2024"
    invalid_api_key = "invalid-key"
    
    # First check if service is available
    if not check_service_availability(base_url):
        print("\nPlease start the API Gateway service with:")
        print("cargo run --bin api-gateway-rs")
        sys.exit(1)
    
    print("\n=== Testing API Key Authentication ===")
    
    # Test 1: Missing API Key
    print("\nTest 1: Missing API Key")
    try:
        response = requests.post(
            f"{base_url}/api/v1/execute",
            json={"method": "test", "payload": "test"},
            headers={"Content-Type": "application/json"},
            timeout=5
        )
        if response.status_code != 401:
            print(f"ERROR: Expected 401 for missing API key, got {response.status_code}")
            sys.exit(1)
        print("✓ Missing API key returns 401")
    except Exception as e:
        print(f"ERROR: Request failed - {str(e)}")
        sys.exit(1)

    # Test 2: Invalid API Key
    print("\nTest 2: Invalid API Key")
    try:
        response = requests.post(
            f"{base_url}/api/v1/execute",
            json={"method": "test", "payload": "test"},
            headers={
                "Content-Type": "application/json",
                "X-PHOENIX-API-KEY": invalid_api_key
            },
            timeout=5
        )
        if response.status_code != 401:
            print(f"ERROR: Expected 401 for invalid API key, got {response.status_code}")
            sys.exit(1)
        print("✓ Invalid API key returns 401")
    except Exception as e:
        print(f"ERROR: Request failed - {str(e)}")
        sys.exit(1)

    # Test 3: Valid API Key
    print("\nTest 3: Valid API Key")
    try:
        response = requests.post(
            f"{base_url}/api/v1/execute",
            json={"method": "test", "payload": "test"},
            headers={
                "Content-Type": "application/json",
                "X-PHOENIX-API-KEY": valid_api_key
            },
            timeout=5
        )
        if response.status_code == 401:
            print(f"ERROR: Valid API key returned 401")
            sys.exit(1)
        print("✓ Valid API key accepted")
    except Exception as e:
        print(f"ERROR: Request failed - {str(e)}")
        sys.exit(1)

    print("\n=== Testing Rate Limiting ===")
    print("\nTest 4: Rate Limit (100 requests per minute)")
    
    # Prepare session for async requests
    async with aiohttp.ClientSession() as session:
        # Create 101 requests (exceeding the 100/minute limit)
        tasks = []
        for i in range(101):
            tasks.append(asyncio.create_task(make_request(session, base_url, valid_api_key, i)))
        
        try:
            # Wait for all requests to complete
            responses = await asyncio.gather(*tasks)
            
            # Count responses by status code
            status_counts = {}
            for status in [r[0] for r in responses]:
                status_counts[status] = status_counts.get(status, 0) + 1
            
            # Verify rate limiting
            if 429 not in status_counts:
                print("ERROR: Rate limiting not triggered (expected at least one 429 response)")
                sys.exit(1)
            
            print(f"✓ Rate limiting active: {status_counts}")
            print(f"✓ Successfully received {status_counts.get(429, 0)} rate limit responses")
        except Exception as e:
            print(f"ERROR: Rate limit test failed - {str(e)}")
            sys.exit(1)

async def make_request(session, base_url, api_key, i):
    try:
        start_time = time.time()
        async with session.post(
            f"{base_url}/api/v1/execute",
            json={"method": "test", "payload": f"test_{i}"},
            headers={
                "Content-Type": "application/json",
                "X-PHOENIX-API-KEY": api_key
            },
            timeout=aiohttp.ClientTimeout(total=5)
        ) as response:
            duration = time.time() - start_time
            return response.status, duration
    except Exception as e:
        print(f"Request {i} failed: {str(e)}")
        return 0, 0

if __name__ == "__main__":
    print("Starting API Security Tests...")
    print(f"Timestamp: {datetime.now().isoformat()}")
    
    try:
        asyncio.run(test_api_security())
        print("\nAll security tests completed!")
    except KeyboardInterrupt:
        print("\nTests interrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"\nTests failed with error: {str(e)}")
        sys.exit(1)