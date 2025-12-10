"""
Test client for PHOENIX ORCH Executor-RS Service
Tests Windows native unsandboxed execution functionality
"""

import grpc
import sys
import time
from concurrent import futures

# Since we don't have the Python gRPC stubs generated, we'll test with grpcurl instead
# This script documents the test cases to be executed

test_cases = [
    {
        "id": "TEST-01",
        "name": "Basic Echo Command",
        "command": "echo",
        "args": ["Hello from Phoenix Orch"],
        "expected": "Output containing 'Hello from Phoenix Orch'"
    },
    {
        "id": "TEST-02", 
        "name": "Directory Listing",
        "command": "dir",
        "args": [],
        "expected": "Directory listing of sandbox"
    },
    {
        "id": "TEST-03",
        "name": "Disallowed Command",
        "command": "netstat",
        "args": ["-an"],
        "expected": "Command rejected - not in allowlist"
    },
    {
        "id": "TEST-04",
        "name": "Python Execution",
        "command": "python",
        "args": ["-c", "print('Phoenix Orch Test')"],
        "expected": "Python output or error if not installed"
    },
    {
        "id": "TEST-05",
        "name": "Long Running Process (Timeout Test)",
        "command": "cmd",
        "args": ["/c", "timeout", "/t", "40"],
        "expected": "Process terminated after 30 seconds"
    },
    {
        "id": "TEST-06",
        "name": "File Write in Sandbox",
        "command": "cmd",
        "args": ["/c", "echo test > C:\\phoenix_sandbox\\test.txt"],
        "expected": "File created in sandbox"
    },
    {
        "id": "TEST-07",
        "name": "File Write Outside Sandbox",
        "command": "cmd",
        "args": ["/c", "echo test > C:\\Windows\\Temp\\test.txt"],
        "expected": "Access denied or error"
    }
]

def print_test_plan():
    print("=" * 60)
    print("PHOENIX ORCH EXECUTOR-RS TEST PLAN")
    print("=" * 60)
    print("\nService Status:")
    print("  - URL: localhost:50055")
    print("  - Protocol: gRPC")
    print("  - Sandbox: C:\\phoenix_sandbox")
    print("\nTest Cases to Execute:")
    print("-" * 60)
    
    for test in test_cases:
        print(f"\n{test['id']}: {test['name']}")
        print(f"  Command: {test['command']} {' '.join(test['args'])}")
        print(f"  Expected: {test['expected']}")
    
    print("\n" + "=" * 60)
    print("\nTo execute tests, use grpcurl or implement gRPC client")
    print("Example grpcurl command:")
    print('  grpcurl -plaintext -d \'{"command":"echo","args":["test"]}\' \\')
    print('    localhost:50055 agi_core.ExecutorService/ExecuteCommand')

if __name__ == "__main__":
    print_test_plan()