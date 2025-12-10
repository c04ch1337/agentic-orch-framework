import grpc
import sys
import os
import time

# Generate gRPC stubs
os.system("python -m grpc_tools.protoc -I.proto --python_out=. --grpc_python_out=. .proto/agi_core.proto")

# Import generated stubs
import agi_core_pb2
import agi_core_pb2_grpc

def run_test(name, script_content):
    print(f"\n=== Running {name} ===")
    print(f"Script content:\n{script_content}")
    
    # Create temp script file
    script_path = f"test_{int(time.time())}.py"
    with open(script_path, 'w') as f:
        f.write(script_content)
    
    try:
        # Create gRPC channel
        channel = grpc.insecure_channel('localhost:50055')
        stub = agi_core_pb2_grpc.ExecutorServiceStub(channel)
        
        # Create command request
        request = agi_core_pb2.CommandRequest(
            command="python",
            args=[script_path],
            env={}
        )
        
        # Execute command
        start_time = time.time()
        response = stub.execute_command(request)
        duration = time.time() - start_time
        
        print(f"Duration: {duration:.1f}s")
        print(f"Exit Code: {response.exit_code}")
        print("Stdout:", response.stdout)
        print("Stderr:", response.stderr)
        
        return response.exit_code, response.stdout, response.stderr
        
    except Exception as e:
        print(f"Test error: {e}")
        return -1, "", str(e)
    finally:
        try:
            os.remove(script_path)
        except:
            pass

def main():
    print("Starting Emergency Resilience Tests...")
    print("Testing executor-rs watchdog functionality")
    
    # Test 1: CPU Limit Breach
    cpu_script = """
import multiprocessing
import time

def cpu_load():
    x = 0
    while True:
        x += 1

if __name__ == '__main__':
    print("Starting CPU stress test...")
    procs = []
    for _ in range(multiprocessing.cpu_count()):
        p = multiprocessing.Process(target=cpu_load)
        p.start()
        procs.append(p)
        print(f"Started worker: {p.pid}")
    
    while True:
        time.sleep(1)
        print("Still running...")
"""
    cpu_test = run_test("CPU Limit Test", cpu_script)
    
    time.sleep(2)
    
    # Test 2: Memory Limit Breach
    memory_script = """
import array
import time

print("Starting memory stress test...")
chunks = []
chunk_size = 50 * 1024 * 1024  # 50MB chunks

try:
    for i in range(20):  # Try to allocate 1GB
        chunks.append(array.array('B', [1] * chunk_size))
        print(f"Allocated {(i+1)*50}MB")
        time.sleep(0.1)
except MemoryError as e:
    print(f"Memory allocation failed: {e}")

print("Holding memory...")
time.sleep(10)
"""
    memory_test = run_test("Memory Limit Test", memory_script)
    
    time.sleep(2)
    
    # Test 3: Execution Timeout
    timeout_script = """
import time

print("Starting timeout test...")
for i in range(20):
    print(f"Elapsed: {i}s")
    time.sleep(1)
"""
    timeout_test = run_test("Timeout Test", timeout_script)
    
    # Print summary
    print("\n=== Test Summary ===")
    print("1. CPU Limit Test:", "PASSED" if cpu_test[0] == 888 else "FAILED")
    print("2. Memory Limit Test:", "PASSED" if memory_test[0] == 888 else "FAILED")
    print("3. Timeout Test:", "PASSED" if timeout_test[0] == 888 else "FAILED")

if __name__ == "__main__":
    main()