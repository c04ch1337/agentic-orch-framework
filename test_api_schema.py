import requests
import json
import sys
import time

def validate_agi_response_schema(response_data):
    """Validate response against the unified AgiResponse schema"""
    required_fields = {
        "final_answer": str,
        "execution_plan": str,
        "routed_service": str,
        "phoenix_session_id": str,
        "output_artifact_urls": list
    }
    
    validation_errors = []
    
    # Check for required fields and their types
    for field, expected_type in required_fields.items():
        if field not in response_data:
            validation_errors.append(f"Missing required field: {field}")
        elif not isinstance(response_data[field], expected_type):
            validation_errors.append(f"Field {field} has wrong type. Expected {expected_type.__name__}, got {type(response_data[field]).__name__}")
    
    return validation_errors

def test_api_schema():
    # API Gateway endpoint
    base_url = "http://localhost:8000"
    
    # Test request
    test_request = {
        "method": "test_unified_schema",
        "payload": json.dumps({
            "test": "Validating unified AgiResponse schema",
            "require_full_response": True
        }),
        "metadata": {
            "test_id": "schema_validation_001",
            "timestamp": str(int(time.time())),
            "response_format": "agi_response"
        }
    }

    try:
        # 1. First get an auth token
        print("Getting auth token...")
        token_response = requests.get(f"{base_url}/api/v1/token")
        if token_response.status_code != 200:
            print(f"Failed to get auth token: {token_response.text}")
            sys.exit(1)
        
        token = token_response.json()["token"]
        print("Successfully obtained auth token")
        
        # 2. Execute request with auth token
        print("\nExecuting test request...")
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {token}"
        }
        
        response = requests.post(
            f"{base_url}/api/v1/execute",
            json=test_request,
            headers=headers
        )
        
        if response.status_code != 200:
            print(f"Request failed with status {response.status_code}: {response.text}")
            sys.exit(1)
            
        print("Received response from API Gateway")
            
        # 3. Validate response schema
        print("\nValidating response schema...")
        response_data = response.json()
        
        validation_errors = validate_agi_response_schema(response_data)
        
        if validation_errors:
            print("ERROR: Schema validation failed:")
            for error in validation_errors:
                print(f"- {error}")
            print("\nActual response:")
            print(json.dumps(response_data, indent=2))
            sys.exit(1)
            
        print("\nSUCCESS: API Gateway -> Orchestrator communication validated")
        print("Response schema conforms to unified AgiResponse format")
        print("\nResponse data:")
        print(json.dumps(response_data, indent=2))
        
    except requests.exceptions.ConnectionError:
        print("ERROR: Failed to connect to API Gateway. Is the service running?")
        print("Make sure both api-gateway-rs and orchestrator-rs services are running")
        sys.exit(1)
    except Exception as e:
        print(f"Test failed with error: {str(e)}")
        sys.exit(1)

if __name__ == "__main__":
    test_api_schema()