import os
import time
import json
import shutil
import threading
import queue
import random
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed

SANDBOX_DIR = r"C:\phoenix_sandbox"
TEST_DATA_DIR = os.path.join(SANDBOX_DIR, "test_data")

def setup_test_data():
    """Create initial test data structure"""
    print("\nSetting up test data...")
    
    # Create test directory if it doesn't exist
    os.makedirs(TEST_DATA_DIR, exist_ok=True)
    
    # Create initial test files
    test_files = {
        "config.json": {
            "version": "1.0",
            "settings": {
                "enabled": True,
                "max_items": 100
            }
        },
        "data.txt": "Original content\nLine 2\nLine 3",
        "records/": {
            "record1.json": {"id": 1, "value": "test1"},
            "record2.json": {"id": 2, "value": "test2"}
        }
    }
    
    for path, content in test_files.items():
        full_path = os.path.join(TEST_DATA_DIR, path)
        if path.endswith('/'):
            os.makedirs(full_path, exist_ok=True)
            for subpath, subcontent in content.items():
                with open(os.path.join(full_path, subpath), 'w') as f:
                    if isinstance(subcontent, dict):
                        json.dump(subcontent, f, indent=2)
                    else:
                        f.write(subcontent)
        else:
            with open(full_path, 'w') as f:
                if isinstance(content, dict):
                    json.dump(content, f, indent=2)
                else:
                    f.write(content)
    
    print("Test data setup complete.")
    return test_files

def verify_data_integrity(test_files):
    """Verify that all test files exist and have correct content"""
    print("\nVerifying data integrity...")
    
    for path, expected_content in test_files.items():
        full_path = os.path.join(TEST_DATA_DIR, path)
        if path.endswith('/'):
            if not os.path.isdir(full_path):
                print(f"ERROR: Directory {full_path} does not exist!")
                return False
            for subpath, subcontent in expected_content.items():
                subfile_path = os.path.join(full_path, subpath)
                if not os.path.isfile(subfile_path):
                    print(f"ERROR: File {subfile_path} does not exist!")
                    return False
                with open(subfile_path, 'r') as f:
                    actual_content = f.read()
                    if isinstance(subcontent, dict):
                        try:
                            actual_json = json.loads(actual_content)
                            if actual_json != subcontent:
                                print(f"ERROR: Content mismatch in {subfile_path}")
                                return False
                        except json.JSONDecodeError:
                            print(f"ERROR: Invalid JSON in {subfile_path}")
                            return False
                    elif actual_content != subcontent:
                        print(f"ERROR: Content mismatch in {subfile_path}")
                        return False
        else:
            if not os.path.isfile(full_path):
                print(f"ERROR: File {full_path} does not exist!")
                return False
            with open(full_path, 'r') as f:
                actual_content = f.read()
                if isinstance(expected_content, dict):
                    try:
                        actual_json = json.loads(actual_content)
                        if actual_json != expected_content:
                            print(f"ERROR: Content mismatch in {full_path}")
                            return False
                    except json.JSONDecodeError:
                        print(f"ERROR: Invalid JSON in {full_path}")
                        return False
                elif actual_content != expected_content:
                    print(f"ERROR: Content mismatch in {full_path}")
                    return False
    
    print("Data integrity verification passed.")
    return True

def test_atomic_operations():
    """Test atomic file operations with rollback"""
    print("\n=== Testing Atomic Operations ===")
    
    # Setup initial data
    test_files = setup_test_data()
    
    # Create backup/snapshot
    snapshot_dir = os.path.join(SANDBOX_DIR, f"snapshot_{int(time.time())}")
    print(f"\nCreating snapshot in {snapshot_dir}")
    shutil.copytree(TEST_DATA_DIR, snapshot_dir)
    
    try:
        # Attempt multiple file modifications
        print("\nAttempting file modifications...")
        
        # 1. Modify config.json
        config_path = os.path.join(TEST_DATA_DIR, "config.json")
        with open(config_path, 'r') as f:
            config = json.load(f)
        config["settings"]["max_items"] = 200
        with open(config_path, 'w') as f:
            json.dump(config, f, indent=2)
        
        # 2. Append to data.txt
        data_path = os.path.join(TEST_DATA_DIR, "data.txt")
        with open(data_path, 'a') as f:
            f.write("\nNew line added")
        
        # 3. Add new record
        new_record_path = os.path.join(TEST_DATA_DIR, "records", "record3.json")
        with open(new_record_path, 'w') as f:
            json.dump({"id": 3, "value": "test3"}, f, indent=2)
        
        # Simulate failure
        print("\nSimulating failure during operations...")
        raise Exception("Simulated failure")
        
    except Exception as e:
        print(f"\nError occurred: {e}")
        print("Rolling back changes...")
        
        # Perform rollback
        shutil.rmtree(TEST_DATA_DIR)
        shutil.copytree(snapshot_dir, TEST_DATA_DIR)
        
        # Verify rollback
        if verify_data_integrity(test_files):
            print("Rollback successful - data restored to original state")
        else:
            print("ERROR: Rollback failed - data integrity check failed")
    
    finally:
        # Cleanup
        if os.path.exists(snapshot_dir):
            shutil.rmtree(snapshot_dir)

def worker_task(worker_id, record_queue, error_queue):
    """Worker task for concurrent access testing"""
    try:
        while True:
            try:
                record_id = record_queue.get_nowait()
            except queue.Empty:
                break
                
            record_path = os.path.join(TEST_DATA_DIR, "records", f"record{record_id}.json")
            
            # Read record
            try:
                with open(record_path, 'r') as f:
                    record = json.load(f)
            except FileNotFoundError:
                record = {"id": record_id, "value": f"test{record_id}", "updates": 0}
            
            # Modify record
            record["updates"] = record.get("updates", 0) + 1
            record["last_worker"] = worker_id
            
            # Simulate some work
            time.sleep(random.uniform(0.1, 0.3))
            
            # Write back
            with open(record_path, 'w') as f:
                json.dump(record, f, indent=2)
                
            record_queue.task_done()
            
    except Exception as e:
        error_queue.put(f"Worker {worker_id} error: {str(e)}")

def test_concurrent_access():
    """Test concurrent access scenarios"""
    print("\n=== Testing Concurrent Access ===")
    
    # Setup test data
    setup_test_data()
    
    # Create a queue of record IDs to process
    record_queue = queue.Queue()
    error_queue = queue.Queue()
    
    # Add records to queue
    for i in range(1, 101):  # Process 100 records
        record_queue.put(i)
    
    # Create worker threads
    num_workers = 4
    threads = []
    
    print(f"\nStarting {num_workers} workers for concurrent access test...")
    
    for i in range(num_workers):
        thread = threading.Thread(
            target=worker_task,
            args=(i, record_queue, error_queue)
        )
        thread.start()
        threads.append(thread)
    
    # Wait for all threads to complete
    for thread in threads:
        thread.join()
    
    # Check for errors
    if not error_queue.empty():
        print("\nErrors occurred during concurrent access:")
        while not error_queue.empty():
            print(error_queue.get())
    else:
        print("\nConcurrent access test completed successfully")
    
    # Verify final state
    print("\nVerifying record states after concurrent access...")
    records_dir = os.path.join(TEST_DATA_DIR, "records")
    total_updates = 0
    
    for filename in os.listdir(records_dir):
        if filename.endswith('.json'):
            with open(os.path.join(records_dir, filename), 'r') as f:
                record = json.load(f)
                total_updates += record.get("updates", 0)
                print(f"Record {record['id']}: {record.get('updates', 0)} updates by worker {record.get('last_worker', 'unknown')}")
    
    print(f"\nTotal updates across all records: {total_updates}")

def main():
    print("Starting Data Integrity Rollback Tests...")
    
    # Test atomic operations with rollback
    test_atomic_operations()
    
    # Test concurrent access
    test_concurrent_access()

if __name__ == "__main__":
    main()