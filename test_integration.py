import os
import time
import subprocess
import signal
import psutil
import json
from pathlib import Path
from datetime import datetime, timezone

class IntegrationTest:
    def __init__(self):
        self.sandbox_dir = Path(r"C:\phoenix_sandbox")
        self.executor_path = Path(r"target\release\executor-rs.exe")
        self.executor_process = None
        self.test_data = {}

    def start_executor(self) -> bool:
        """Start the executor service"""
        try:
            print("\nStarting executor service...")
            self.executor_process = subprocess.Popen(
                [str(self.executor_path)],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                creationflags=subprocess.CREATE_NEW_PROCESS_GROUP
            )
            time.sleep(2)  # Give it time to start
            
            if self.executor_process.poll() is None:
                print("[PASS] Executor service started successfully")
                return True
            else:
                print("[FAIL] Executor service failed to start")
                return False
        except Exception as e:
            print(f"[FAIL] Error starting executor: {e}")
            return False

    def stop_executor(self, emergency: bool = False) -> bool:
        """Stop the executor service"""
        if not self.executor_process:
            return True
            
        try:
            if emergency:
                print("\nSimulating emergency shutdown...")
                self.executor_process.kill()
            else:
                print("\nGraceful shutdown...")
                self.executor_process.terminate()
                
            self.executor_process.wait(timeout=5)
            print("[PASS] Executor service stopped")
            return True
        except Exception as e:
            print(f"[FAIL] Error stopping executor: {e}")
            return False
        finally:
            self.executor_process = None

    def setup_test_data(self) -> bool:
        """Set up test data for recovery testing"""
        try:
            # Create test data directory
            test_data_dir = self.sandbox_dir / "test_data"
            test_data_dir.mkdir(parents=True, exist_ok=True)
            
            # Create some test files
            self.test_data = {
                "config.json": {
                    "version": "1.0",
                    "settings": {"enabled": True}
                },
                "data.txt": "Original test data\nLine 2\nLine 3",
                "records/record1.json": {"id": 1, "value": "test1"},
                "records/record2.json": {"id": 2, "value": "test2"}
            }
            
            for path, content in self.test_data.items():
                file_path = test_data_dir / path
                file_path.parent.mkdir(parents=True, exist_ok=True)
                
                with open(file_path, 'w') as f:
                    if isinstance(content, dict):
                        json.dump(content, f, indent=2)
                    else:
                        f.write(content)
            
            print("[PASS] Test data created successfully")
            return True
        except Exception as e:
            print(f"[FAIL] Error setting up test data: {e}")
            return False

    def verify_data_integrity(self) -> bool:
        """Verify test data integrity after recovery"""
        try:
            test_data_dir = self.sandbox_dir / "test_data"
            
            for path, expected_content in self.test_data.items():
                file_path = test_data_dir / path
                
                if not file_path.exists():
                    print(f"[FAIL] Missing file: {path}")
                    return False
                    
                with open(file_path, 'r') as f:
                    if isinstance(expected_content, dict):
                        actual_content = json.load(f)
                        if actual_content != expected_content:
                            print(f"[FAIL] Content mismatch in {path}")
                            return False
                    else:
                        actual_content = f.read()
                        if actual_content != expected_content:
                            print(f"[FAIL] Content mismatch in {path}")
                            return False
            
            print("[PASS] Data integrity verified")
            return True
        except Exception as e:
            print(f"[FAIL] Error verifying data integrity: {e}")
            return False

    def test_emergency_shutdown(self) -> bool:
        """Test emergency shutdown scenario"""
        print("\n=== Testing Emergency Shutdown ===")
        
        if not self.setup_test_data():
            return False
            
        if not self.start_executor():
            return False
            
        # Simulate emergency shutdown
        if not self.stop_executor(emergency=True):
            return False
            
        # Verify data integrity after emergency shutdown
        if not self.verify_data_integrity():
            return False
            
        print("[PASS] Emergency shutdown test completed")
        return True

    def test_service_recovery(self) -> bool:
        """Test Windows Service recovery"""
        print("\n=== Testing Service Recovery ===")
        
        # Start service
        if not self.start_executor():
            return False
            
        # Simulate crash
        print("\nSimulating service crash...")
        if self.executor_process:
            self.executor_process.kill()
            time.sleep(1)
        
        # Verify automatic restart
        try:
            time.sleep(2)  # Wait for potential restart
            
            # Check if a new process started
            for proc in psutil.process_iter(['name']):
                if proc.info['name'] == 'executor-rs.exe':
                    print("[PASS] Service recovered automatically")
                    self.executor_process = proc
                    return True
                    
            print("[FAIL] Service did not recover automatically")
            return False
        except Exception as e:
            print(f"[FAIL] Error checking service recovery: {e}")
            return False
        finally:
            self.stop_executor()

    def test_system_stability(self) -> bool:
        """Test system stability under load"""
        print("\n=== Testing System Stability ===")
        
        if not self.start_executor():
            return False
            
        try:
            # Create some load
            print("\nGenerating system load...")
            load_processes = []
            for _ in range(3):
                p = subprocess.Popen(
                    ["python", "-c", "import time; time.sleep(10)"],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE
                )
                load_processes.append(p)
                
            # Monitor stability
            start_time = time.time()
            stable = True
            while time.time() - start_time < 30:  # Monitor for 30 seconds
                if self.executor_process.poll() is not None:
                    print("[FAIL] Service crashed under load")
                    stable = False
                    break
                time.sleep(1)
                
            if stable:
                print("[PASS] System remained stable under load")
                
            return stable
        except Exception as e:
            print(f"[FAIL] Error during stability test: {e}")
            return False
        finally:
            # Cleanup
            for p in load_processes:
                try:
                    p.kill()
                except:
                    pass
            self.stop_executor()

def main():
    tester = IntegrationTest()
    
    # Run integration tests
    tests = [
        ("Emergency Shutdown", tester.test_emergency_shutdown),
        ("Service Recovery", tester.test_service_recovery),
        ("System Stability", tester.test_system_stability)
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
            
        # Ensure cleanup between tests
        if tester.executor_process:
            tester.stop_executor()
    
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