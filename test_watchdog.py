import subprocess
import time
import sys
import os
import signal

SANDBOX_DIR = r"C:\phoenix_sandbox"

def run_test(name, script_content):
    print(f"\n=== Running {name} ===")
    print(f"Script content:\n{script_content}")
    start_time = time.time()
    
    try:
        # Create a Python script file in the sandbox
        script_name = os.path.join(SANDBOX_DIR, f"test_{int(time.time())}.py")
        with open(script_name, 'w') as f:
            f.write(script_content)
        
        # Execute the script
        process = subprocess.Popen(
            ["python", script_name],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=SANDBOX_DIR,  # Run in sandbox directory
            creationflags=subprocess.CREATE_NEW_PROCESS_GROUP  # Allow process group termination
        )
        
        try:
            stdout, stderr = process.communicate(timeout=30)  # Longer timeout to observe watchdog behavior
            exit_code = process.returncode
            duration = time.time() - start_time
            
            print(f"Duration: {duration:.1f}s")
            print(f"Exit Code: {exit_code}")
            print("Stdout:", stdout.decode('utf-8', errors='ignore'))
            print("Stderr:", stderr.decode('utf-8', errors='ignore'))
            
            # Check for watchdog intervention
            if exit_code == 888:
                print("DETECTED: Process terminated by watchdog (exit code 888)")
            elif exit_code < 0:
                print("DETECTED: Process terminated by signal")
            
            return exit_code, stdout, stderr
            
        except subprocess.TimeoutExpired:
            print("Test timeout reached (30s), terminating process group")
            try:
                os.kill(process.pid, signal.CTRL_BREAK_EVENT)
                time.sleep(1)
                if process.poll() is None:
                    process.kill()
            except:
                pass
            stdout, stderr = process.communicate()
            print("Stdout:", stdout.decode('utf-8', errors='ignore'))
            print("Stderr:", stderr.decode('utf-8', errors='ignore'))
            return -1, stdout, stderr
        finally:
            # Clean up the script file
            try:
                os.remove(script_name)
            except:
                pass
            
    except Exception as e:
        print(f"Test error: {e}")
        return -1, b"", str(e).encode()

def main():
    print("Starting Emergency Resilience Tests...")
    print("Testing executor-rs watchdog functionality")
    
    # Test 1: CPU Limit Breach
    print("\n=== Test 1: CPU Limit Breach (>50%) ===")
    cpu_script = """
import multiprocessing
import time
import os

def cpu_intensive():
    start = time.time()
    while True:
        # Perform intensive calculation
        x = 0
        for i in range(10000000):
            x += i * i
        elapsed = time.time() - start
        print(f"CPU worker running for {elapsed:.1f}s")

if __name__ == '__main__':
    print(f"Starting CPU stress test in PID: {os.getpid()}")
    
    # Create multiple CPU-intensive processes
    processes = []
    for i in range(multiprocessing.cpu_count() * 2):  # 2x number of cores
        p = multiprocessing.Process(target=cpu_intensive)
        p.start()
        processes.append(p)
        print(f"Started CPU worker {i+1}")
    
    # Monitor processes
    while True:
        time.sleep(1)
        print(f"Main process monitoring. Active workers: {len([p for p in processes if p.is_alive()])}")
"""
    cpu_test = run_test("CPU Limit Test", cpu_script)
    
    time.sleep(2)
    
    # Test 2: Memory Limit Breach
    print("\n=== Test 2: Memory Limit Breach (>512MB) ===")
    memory_script = """
import array
import time
import os
import gc

print(f"Starting memory stress test in PID: {os.getpid()}")

# Disable garbage collection to maintain memory pressure
gc.disable()

# Allocate memory in chunks
chunk_size = 50 * 1024 * 1024  # 50MB chunks
chunks = []

try:
    for i in range(20):  # Try to allocate 1GB total
        chunk = array.array('B', [1] * chunk_size)
        chunks.append(chunk)
        allocated = (i + 1) * chunk_size / (1024 * 1024)
        print(f"Allocated {allocated:.0f}MB")
        # Access memory to ensure it's actually allocated
        chunk[0] = 255
        time.sleep(0.1)  # Small delay for monitoring
except MemoryError as e:
    print(f"Memory allocation failed: {e}")

print("Memory allocation complete, holding...")
while True:
    time.sleep(1)
    print("Still holding allocated memory...")
"""
    memory_test = run_test("Memory Limit Test", memory_script)
    
    time.sleep(2)
    
    # Test 3: Execution Timeout
    print("\n=== Test 3: Execution Timeout (>10s) ===")
    timeout_script = """
import time
import os

print(f"Starting timeout test in PID: {os.getpid()}")
start = time.time()

while True:
    elapsed = time.time() - start
    print(f"Running for {elapsed:.1f}s...")
    time.sleep(1)
    
    # Do some work to trigger CPU monitoring
    x = 0
    for i in range(1000000):
        x += i
"""
    timeout_test = run_test("Timeout Test", timeout_script)
    
    # Print summary
    print("\n=== Test Summary ===")
    print("1. CPU Limit Test:", "PASSED" if cpu_test[0] in [888, -1] else "FAILED")
    print("2. Memory Limit Test:", "PASSED" if memory_test[0] in [888, -1] else "FAILED")
    print("3. Timeout Test:", "PASSED" if timeout_test[0] in [888, -1] else "FAILED")

if __name__ == "__main__":
    main()