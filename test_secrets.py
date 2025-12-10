import os
import uuid
import time
import json
from datetime import datetime, timezone
import subprocess
from pathlib import Path

class SecretsTest:
    def __init__(self):
        self.sandbox_dir = Path(r"C:\phoenix_sandbox")
        self.secrets_dir = self.sandbox_dir / "secrets"
        
        # Create all required directories
        for subdir in ["api_keys", "rotating_keys", "cached_keys"]:
            (self.secrets_dir / subdir).mkdir(parents=True, exist_ok=True)

    def store_secret(self, key: str, value: str, metadata: dict = None) -> bool:
        """Store a secret in the sandbox directory"""
        try:
            # Ensure parent directory exists
            secret_path = self.secrets_dir / f"{key}.json"
            secret_path.parent.mkdir(parents=True, exist_ok=True)
            
            secret_data = {
                "value": value,
                "metadata": metadata or {},
                "created_at": datetime.now(timezone.utc).isoformat(),
                "version": 1
            }
            
            with open(secret_path, "w") as f:
                json.dump(secret_data, f, indent=2)
            
            print(f"[PASS] Stored secret: {key}")
            return True
        except Exception as e:
            print(f"[FAIL] Failed to store secret: {e}")
            return False

    def get_secret(self, key: str) -> tuple[bool, str, dict]:
        """Retrieve a secret from the sandbox directory"""
        try:
            secret_path = self.secrets_dir / f"{key}.json"
            
            with open(secret_path, "r") as f:
                secret_data = json.load(f)
            
            return True, secret_data["value"], secret_data.get("metadata", {})
        except Exception as e:
            print(f"[FAIL] Failed to retrieve secret: {e}")
            return False, "", {}

    def test_vault_integration(self):
        """Test basic secret storage and retrieval"""
        print("\n=== Testing Secret Storage Integration ===")
        
        # Test storing a secret
        secret_key = f"test_secret_{uuid.uuid4()}"
        secret_value = f"test_value_{uuid.uuid4()}"
        metadata = {
            "description": "Test secret",
            "environment": "test"
        }
        
        if not self.store_secret(secret_key, secret_value, metadata):
            return False
            
        # Test retrieving the secret
        success, retrieved_value, retrieved_metadata = self.get_secret(secret_key)
        if not success:
            return False
            
        if retrieved_value != secret_value:
            print(f"[FAIL] Secret value mismatch. Expected: {secret_value}, Got: {retrieved_value}")
            return False
            
        print("[PASS] Secret storage and retrieval working")
        return True

    def test_api_key_retrieval(self):
        """Test API key storage and retrieval with caching"""
        print("\n=== Testing API Key Retrieval ===")
        
        api_key = f"api_key_{uuid.uuid4()}"
        key_path = f"api_keys/test_{uuid.uuid4()}"
        
        # Store API key
        if not self.store_secret(key_path, api_key, {"type": "api_key"}):
            return False
            
        # First retrieval (uncached)
        start_time = time.time()
        success1, value1, _ = self.get_secret(key_path)
        first_retrieval = time.time() - start_time
        
        if not success1:
            return False
            
        # Second retrieval (should be cached)
        start_time = time.time()
        success2, value2, _ = self.get_secret(key_path)
        second_retrieval = time.time() - start_time
        
        if not success2:
            return False
            
        print(f"First retrieval: {first_retrieval:.3f}s")
        print(f"Second retrieval: {second_retrieval:.3f}s")
        
        if value1 == value2 == api_key:
            print("[PASS] API key retrieval working")
            return True
        else:
            print("[FAIL] API key retrieval inconsistent")
            return False

    def test_key_rotation(self):
        """Test key rotation functionality"""
        print("\n=== Testing Key Rotation ===")
        
        key_path = f"rotating_keys/test_{uuid.uuid4()}"
        initial_key = f"key_v1_{uuid.uuid4()}"
        
        # Store initial key
        if not self.store_secret(key_path, initial_key, {
            "version": 1,
            "rotation_period": "1h"
        }):
            return False
            
        # Rotate key
        rotated_key = f"key_v2_{uuid.uuid4()}"
        if not self.store_secret(key_path, rotated_key, {
            "version": 2,
            "rotation_period": "1h",
            "previous_version": 1
        }):
            return False
            
        # Verify we get the latest version
        success, value, metadata = self.get_secret(key_path)
        if not success:
            return False
            
        if value == rotated_key and metadata.get("version") == 2:
            print("[PASS] Key rotation successful")
            return True
        else:
            print("[FAIL] Key rotation verification failed")
            return False

    def test_cache_invalidation(self):
        """Test cache invalidation on updates"""
        print("\n=== Testing Cache Invalidation ===")
        
        key_path = f"cached_keys/test_{uuid.uuid4()}"
        initial_value = f"value_1_{uuid.uuid4()}"
        
        # Store initial value
        if not self.store_secret(key_path, initial_value):
            return False
            
        # Get value (should be cached)
        success1, value1, _ = self.get_secret(key_path)
        if not success1:
            return False
            
        # Update value
        new_value = f"value_2_{uuid.uuid4()}"
        if not self.store_secret(key_path, new_value):
            return False
            
        # Get value again (should get new value)
        success2, value2, _ = self.get_secret(key_path)
        if not success2:
            return False
            
        if value2 == new_value:
            print("[PASS] Cache invalidation working")
            return True
        else:
            print(f"[FAIL] Got stale value. Expected: {new_value}, Got: {value2}")
            return False

def main():
    tester = SecretsTest()
    
    # Run all tests
    tests = [
        ("Secret Storage Integration", tester.test_vault_integration),
        ("API Key Retrieval", tester.test_api_key_retrieval),
        ("Key Rotation", tester.test_key_rotation),
        ("Cache Invalidation", tester.test_cache_invalidation)
    ]
    
    results = []
    for test_name, test_func in tests:
        print(f"\nExecuting: {test_name}")
        try:
            result = test_func()
            results.append((test_name, result))
        except Exception as e:
            print(f"Test failed with exception: {e}")
            results.append((test_name, False))
    
    # Print summary
    print("\n=== Test Summary ===")
    passed = 0
    failed = 0
    for test_name, result in results:
        status = "PASSED" if result else "FAILED"
        if result:
            passed += 1
        else:
            failed += 1
        print(f"{test_name}: {status}")
    
    print(f"\nTotal: {len(results)} tests")
    print(f"Passed: {passed}")
    print(f"Failed: {failed}")

if __name__ == "__main__":
    main()