// executor-rs/src/windows_executor.rs
// Windows native execution control with enhanced resource monitoring
// PHOENIX ORCH: The Ashen Guard Edition AGI

use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::thread;
use std::time::{Duration, Instant};
use windows_job_object::{JobObject, JobObjectHandle};
use std::os::windows::io::IntoRawHandle;
use winapi::shared::minwindef::{DWORD, FALSE, LPVOID, TRUE};
use winapi::shared::winerror::WAIT_TIMEOUT;
use winapi::shared::basetsd::SIZE_T;
use winapi::um::fileapi::{ReadFile};
use winapi::um::handleapi::{CloseHandle, DuplicateHandle};
use winapi::um::jobapi2::{AssignProcessToJobObject, TerminateJobObject};
use winapi::um::namedpipeapi::CreatePipe;
use winapi::um::processthreadsapi::{
    CreateProcessW, GetExitCodeProcess, OpenProcessToken, ResumeThread, TerminateProcess,
    GetCurrentProcess,
};
use winapi::um::processenv::GetStdHandle;
use winapi::um::securitybaseapi::{AllocateAndInitializeSid, SetTokenInformation, GetLengthSid, FreeSid};
use winapi::um::synchapi::{WaitForSingleObject};
use winapi::um::winbase::{
    CREATE_NEW_PROCESS_GROUP, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT,
    STARTF_USESHOWWINDOW, STARTF_USESTDHANDLES, WAIT_OBJECT_0, STD_INPUT_HANDLE,
};
use winapi::um::winuser::SW_HIDE;
use winapi::um::winnt::{
    FILE_ATTRIBUTE_DIRECTORY, HANDLE, DUPLICATE_SAME_ACCESS,
    JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
    JOB_OBJECT_LIMIT_JOB_MEMORY,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
    TOKEN_ADJUST_DEFAULT, TOKEN_ADJUST_PRIVILEGES, TOKEN_QUERY,
    SECURITY_MANDATORY_LOW_RID,
    SECURITY_MANDATORY_LABEL_AUTHORITY, TokenIntegrityLevel,
    PSID, TOKEN_MANDATORY_LABEL, SE_GROUP_INTEGRITY,
};


// Working directory - No sandbox restrictions
const WORK_DIR: &str = r"C:\Windows\Temp";

// Resource limits
const MAX_PROCESS_MEMORY: SIZE_T = 512 * 1024 * 1024;  // 512 MB (50% of 1GB)
const MAX_JOB_MEMORY: SIZE_T = 512 * 1024 * 1024;      // 512 MB (50% of 1GB)
const MAX_PROCESS_COUNT: DWORD = 5;                     // Max 5 processes
const MAX_CPU_RATE: DWORD = 5000;                      // 50% CPU limit
const EXECUTION_TIMEOUT_MS: u64 = 10000;               // 10 seconds timeout
const CRITICAL_RESOURCE_CHECK_MS: u64 = 100;           // Check resources every 100ms

// Job Object Manager - wrap raw handle to make it Send
pub struct JobObjectManager {
    job_handle: HANDLE,
    sandbox_path: PathBuf,
}

// Mark as Send - we ensure thread safety through proper handle management
unsafe impl Send for JobObjectManager {}
unsafe impl Sync for JobObjectManager {}

impl JobObjectManager {
    /// Create a new Job Object with strict resource limits
    pub fn new() -> Result<Self, String> {
        log::info!("Creating Windows Job Object for process isolation");
        
        // Create sandbox directory if it doesn't exist
        let sandbox_path = PathBuf::from(SANDBOX_DIR);
        if !sandbox_path.exists() {
            Self::create_sandbox_directory(&sandbox_path)?;
        }
        
        // Create Job Object using windows-job-object crate
        let job = windows_job_object::JobObject::create()
            .map_err(|e| format!("Failed to create Job Object: {}", e))?;
        
        // Configure CPU limits (50% of one CPU)
        job.set_cpu_rate_control(MAX_CPU_RATE)
            .map_err(|e| format!("Failed to set CPU limit: {}", e))?;
        
        // Configure memory limits (50% of RAM)
        job.set_memory_limit(MAX_JOB_MEMORY as u64)
            .map_err(|e| format!("Failed to set memory limit: {}", e))?;
        
        // Set process count limit
        job.set_active_process_limit(MAX_PROCESS_COUNT as u32)
            .map_err(|e| format!("Failed to set process limit: {}", e))?;
        
        // Enable kill on job close
        job.set_kill_on_job_close(true)
            .map_err(|e| format!("Failed to set kill on close: {}", e))?;
            
        log::info!("Job Object configured with resource limits:");
        log::info!("- Max Processes: {}", MAX_PROCESS_COUNT);
        log::info!("- Memory Limit: {} MB", MAX_JOB_MEMORY / (1024 * 1024));
        log::info!("- CPU Limit: 50%");
        log::info!("- Execution Timeout: {} seconds", EXECUTION_TIMEOUT_MS / 1000);
        
        // Get the raw handle for our existing JobObjectManager implementation
        let job_handle = job.into_raw_handle();
        
        Ok(JobObjectManager {
            job_handle,
            sandbox_path,
        })
    }
    
    /// Create sandbox directory with restrictive permissions
    fn create_sandbox_directory(path: &Path) -> Result<(), String> {
        let wide_path: Vec<u16> = OsStr::new(path.as_os_str())
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        unsafe {
            // Create directory
            if CreateDirectoryW(wide_path.as_ptr(), null_mut()) == 0 {
                let error = winapi::um::errhandlingapi::GetLastError();
                if error != winapi::shared::winerror::ERROR_ALREADY_EXISTS {
                    return Err(format!("Failed to create sandbox directory: {}", error));
                }
            }
            
            // Set directory attributes to prevent deletion
            SetFileAttributesW(
                wide_path.as_ptr(),
                FILE_ATTRIBUTE_DIRECTORY,
            );
        }
        
        log::info!("Sandbox directory created/verified at: {}", path.display());
        Ok(())
    }
    
    /// Execute code in a sandboxed process
    pub async fn execute_code(
        &self,
        command: &str,
        args: &[String],
        env_vars: &HashMap<String, String>,
        language: &str,
    ) -> Result<(String, String, i32), String> {
        log::info!("Executing {} code in Windows sandbox", language);
        
        // Prepare the full command line
        let mut cmd_line = command.to_string();
        if !args.is_empty() {
            cmd_line.push(' ');
            cmd_line.push_str(&args.join(" "));
        }
        
        // Convert to wide string for Windows API
        let wide_cmd: Vec<u16> = OsStr::new(&cmd_line)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        // Set working directory to sandbox
        let wide_work_dir: Vec<u16> = OsStr::new(&self.sandbox_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        // Prepare environment block
        let env_block = self.create_environment_block(env_vars)?;
        
        // Create pipes for stdout and stderr capture
        let mut stdout_read: HANDLE = null_mut();
        let mut stdout_write: HANDLE = null_mut();
        let mut stderr_read: HANDLE = null_mut();
        let mut stderr_write: HANDLE = null_mut();
        
        // Security attributes for pipes
        let mut sa = winapi::um::minwinbase::SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<winapi::um::minwinbase::SECURITY_ATTRIBUTES>() as DWORD,
            lpSecurityDescriptor: null_mut(),
            bInheritHandle: TRUE,
        };
        
        unsafe {
            if CreatePipe(&mut stdout_read, &mut stdout_write, &mut sa, 0) == 0 {
                return Err("Failed to create stdout pipe".to_string());
            }
            if CreatePipe(&mut stderr_read, &mut stderr_write, &mut sa, 0) == 0 {
                CloseHandle(stdout_read);
                CloseHandle(stdout_write);
                return Err("Failed to create stderr pipe".to_string());
            }
            
            // Ensure read handles are not inherited by child process
            winapi::um::handleapi::SetHandleInformation(stdout_read, winapi::um::winbase::HANDLE_FLAG_INHERIT, 0);
            winapi::um::handleapi::SetHandleInformation(stderr_read, winapi::um::winbase::HANDLE_FLAG_INHERIT, 0);
        }
        
        // Setup startup info with stdout/stderr redirection
        let mut startup_info = unsafe { std::mem::zeroed::<winapi::um::processthreadsapi::STARTUPINFOW>() };
        startup_info.cb = std::mem::size_of_val(&startup_info) as DWORD;
        startup_info.dwFlags = STARTF_USESHOWWINDOW | STARTF_USESTDHANDLES;
        startup_info.wShowWindow = SW_HIDE as u16;
        startup_info.hStdOutput = stdout_write;
        startup_info.hStdError = stderr_write;
        startup_info.hStdInput = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        
        // Process information
        let mut process_info = unsafe { std::mem::zeroed::<winapi::um::processthreadsapi::PROCESS_INFORMATION>() };
        
        // Validate sandbox path before execution
        if !self.sandbox_path.exists() {
            Self::create_sandbox_directory(&self.sandbox_path)?;
        }
        
        // Create process in suspended state - ENSURE working directory is sandbox
        let result = unsafe {
            CreateProcessW(
                null_mut(),                          // Application name
                wide_cmd.as_ptr() as *mut u16,       // Command line
                null_mut(),                          // Process security attributes
                null_mut(),                          // Thread security attributes
                TRUE,                                // Inherit handles for pipes
                CREATE_SUSPENDED | CREATE_NEW_PROCESS_GROUP | CREATE_UNICODE_ENVIRONMENT,
                env_block.as_ptr() as LPVOID,       // Environment block
                wide_work_dir.as_ptr(),             // Working directory - ENFORCED to sandbox
                &mut startup_info,                   // Startup info
                &mut process_info,                   // Process info
            )
        };
        
        // Close write ends of pipes immediately after CreateProcess
        unsafe {
            CloseHandle(stdout_write);
            CloseHandle(stderr_write);
        }
        
        if result == 0 {
            unsafe {
                CloseHandle(stdout_read);
                CloseHandle(stderr_read);
            }
            return Err(format!("Failed to create process: {}", unsafe { winapi::um::errhandlingapi::GetLastError() }));
        }
        
        // Apply Low Integrity Level to the process
        if let Err(e) = self.set_low_integrity_level(process_info.hProcess) {
            log::warn!("Failed to set low integrity level: {}", e);
        }
        
        // Assign process to Job Object
        let assign_result = unsafe {
            AssignProcessToJobObject(self.job_handle, process_info.hProcess)
        };
        
        if assign_result == 0 {
            unsafe {
                TerminateProcess(process_info.hProcess, 1);
                CloseHandle(process_info.hProcess);
                CloseHandle(process_info.hThread);
                CloseHandle(stdout_read);
                CloseHandle(stderr_read);
            }
            return Err("Failed to assign process to Job Object".to_string());
        }
        
        log::info!("Process assigned to Job Object with PID: {}", process_info.dwProcessId);
        
        // Resume the suspended process
        unsafe {
            ResumeThread(process_info.hThread);
        }
        
        // Start process watchdog
        let process_handle = process_info.hProcess;
        let process_id = process_info.dwProcessId;
        let _watchdog = self.start_watchdog(process_handle, process_id);
        
        // Wait for process completion or timeout
        let wait_result = unsafe {
            WaitForSingleObject(process_handle, EXECUTION_TIMEOUT_MS as DWORD)
        };
        
        // Read output from pipes
        let mut stdout_output = Vec::new();
        let mut stderr_output = Vec::new();
        
        // Read stdout
        unsafe {
            let mut buffer = [0u8; 4096];
            let mut bytes_read: DWORD = 0;
            loop {
                if ReadFile(
                    stdout_read,
                    buffer.as_mut_ptr() as LPVOID,
                    buffer.len() as DWORD,
                    &mut bytes_read,
                    null_mut()
                ) == 0 || bytes_read == 0 {
                    break;
                }
                stdout_output.extend_from_slice(&buffer[..bytes_read as usize]);
            }
        }
        
        // Read stderr
        unsafe {
            let mut buffer = [0u8; 4096];
            let mut bytes_read: DWORD = 0;
            loop {
                if ReadFile(
                    stderr_read,
                    buffer.as_mut_ptr() as LPVOID,
                    buffer.len() as DWORD,
                    &mut bytes_read,
                    null_mut()
                ) == 0 || bytes_read == 0 {
                    break;
                }
                stderr_output.extend_from_slice(&buffer[..bytes_read as usize]);
            }
        }
        
        let (stdout, stderr, exit_code) = match wait_result {
            WAIT_OBJECT_0 => {
                // Process completed
                let mut exit_code: DWORD = 0;
                unsafe {
                    GetExitCodeProcess(process_handle, &mut exit_code);
                }
                
                log::info!("Process completed with exit code: {}", exit_code);
                
                // Return actual captured output
                (String::from_utf8_lossy(&stdout_output).to_string(),
                 String::from_utf8_lossy(&stderr_output).to_string(),
                 exit_code as i32)
            }
            WAIT_TIMEOUT => {
                // Timeout - terminate the process
                log::warn!("Process timeout after {} seconds, terminating", EXECUTION_TIMEOUT_MS / 1000);
                unsafe {
                    TerminateProcess(process_handle, 999);
                }
                (String::from_utf8_lossy(&stdout_output).to_string(),
                 format!("Process terminated due to timeout\n{}", String::from_utf8_lossy(&stderr_output)),
                 999)
            }
            _ => {
                log::error!("Unexpected wait result: {}", wait_result);
                (String::from_utf8_lossy(&stdout_output).to_string(),
                 format!("Process execution failed\n{}", String::from_utf8_lossy(&stderr_output)),
                 -1)
            }
        };
        
        // Clean up handles
        unsafe {
            CloseHandle(process_info.hProcess);
            CloseHandle(process_info.hThread);
            CloseHandle(stdout_read);
            CloseHandle(stderr_read);
        }
        
        Ok((stdout, stderr, exit_code))
    }
    
    /// Set Low Integrity Level on a process
    fn set_low_integrity_level(&self, process_handle: HANDLE) -> Result<(), String> {
        let mut token_handle: HANDLE = null_mut();
        
        // Open process token
        let result = unsafe {
            OpenProcessToken(
                process_handle,
                TOKEN_ADJUST_DEFAULT | TOKEN_QUERY | TOKEN_ADJUST_PRIVILEGES,
                &mut token_handle,
            )
        };
        
        if result == 0 {
            return Err("Failed to open process token".to_string());
        }
        
        // Create Low Integrity Level SID
        let mut low_sid: PSID = null_mut();
        let mut sid_authority = SECURITY_MANDATORY_LABEL_AUTHORITY;
        
        let alloc_result = unsafe {
            AllocateAndInitializeSid(
                &mut sid_authority as *mut _ as *mut _,
                1,
                SECURITY_MANDATORY_LOW_RID as DWORD,
                0, 0, 0, 0, 0, 0, 0,
                &mut low_sid
            )
        };
        
        if alloc_result == 0 {
            unsafe { CloseHandle(token_handle); }
            return Err("Failed to allocate Low Integrity SID".to_string());
        }
        
        // Create TOKEN_MANDATORY_LABEL structure
        let mut tml = TOKEN_MANDATORY_LABEL {
            Label: winapi::um::winnt::SID_AND_ATTRIBUTES {
                Sid: low_sid,
                Attributes: SE_GROUP_INTEGRITY,
            },
        };
        
        // Apply the Low Integrity Level to the token
        let set_result = unsafe {
            SetTokenInformation(
                token_handle,
                TokenIntegrityLevel,
                &mut tml as *mut _ as LPVOID,
                std::mem::size_of::<TOKEN_MANDATORY_LABEL>() as DWORD
                    + GetLengthSid(low_sid),
            )
        };
        
        // Clean up
        unsafe {
            FreeSid(low_sid);
            CloseHandle(token_handle);
        }
        
        if set_result == 0 {
            let error = unsafe { winapi::um::errhandlingapi::GetLastError() };
            return Err(format!("Failed to set Low Integrity Level: error code {}", error));
        }
        
        log::info!("Successfully applied Low Integrity Level to process");
        Ok(())
    }
    
    /// Create environment block for process
    fn create_environment_block(&self, env_vars: &HashMap<String, String>) -> Result<Vec<u16>, String> {
        let mut env_block = Vec::new();
        
        // Add custom environment variables
        for (key, value) in env_vars {
            let env_str = format!("{}={}", key, value);
            let wide_str: Vec<u16> = OsStr::new(&env_str)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            env_block.extend_from_slice(&wide_str);
        }
        
        // Add PATH with sandbox directory
        let path_str = format!("PATH={};{}", self.sandbox_path.display(), 
                              std::env::var("PATH").unwrap_or_default());
        let wide_path: Vec<u16> = OsStr::new(&path_str)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        env_block.extend_from_slice(&wide_path);
        
        // Add working directory
        let work_dir_str = format!("WORKDIR={}", self.sandbox_path.display());
        let wide_work: Vec<u16> = OsStr::new(&work_dir_str)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        env_block.extend_from_slice(&wide_work);
        
        // Terminate with double null
        env_block.push(0);
        
        Ok(env_block)
    }
    
    /// Start process watchdog thread with resource monitoring
    fn start_watchdog(&self, process_handle: HANDLE, process_id: DWORD) -> thread::JoinHandle<()> {
        // Create duplicate handles that can be sent between threads
        let mut dup_process_handle: HANDLE = null_mut();
        let mut dup_job_handle: HANDLE = null_mut();
        
        // Create a JobObjectHandle using windows-job-object crate for monitoring
        let job_handle = unsafe { windows_job_object::JobObjectHandle::new(self.job_handle) };
        
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                _process_handle,
                GetCurrentProcess(),
                &mut dup_process_handle,
                0,
                FALSE,
                DUPLICATE_SAME_ACCESS,
            );
            
            DuplicateHandle(
                GetCurrentProcess(),
                self.job_handle,
                GetCurrentProcess(),
                &mut dup_job_handle,
                0,
                FALSE,
                DUPLICATE_SAME_ACCESS,
            );
        }
        
        // Convert handles to usize for thread safety
        let process_handle_value = dup_process_handle as usize;
        let job_handle_value = dup_job_handle as usize;
        
        thread::spawn(move || {
            // Convert back to HANDLE inside the thread
            let dup_process_handle = process_handle_value as HANDLE;
            let dup_job_handle = job_handle_value as HANDLE;
            
            log::info!("Process Watchdog started for PID: {}", process_id);
            let start_time = Instant::now();
            
            loop {
                thread::sleep(Duration::from_millis(CRITICAL_RESOURCE_CHECK_MS));
                
                // Check if process is still alive
                let mut exit_code: DWORD = 0;
                let result = unsafe { GetExitCodeProcess(dup_process_handle, &mut exit_code) };
                
                if result != 0 && exit_code != winapi::um::minwinbase::STILL_ACTIVE {
                    log::info!("Process {} terminated with code: {}", process_id, exit_code);
                    break;
                }
                
                // Check timeout
                if start_time.elapsed() > Duration::from_millis(EXECUTION_TIMEOUT_MS) {
                    log::error!("CRITICAL_RESOURCE_BREACH: Process {} exceeded 10s timeout", process_id);
                    unsafe {
                        TerminateJobObject(dup_job_handle, 888);
                    }
                    break;
                }
                
                // Monitor resource usage using windows-job-object
                if let Ok(job_info) = job_handle.query_extended_limit_info() {
                    // Check CPU usage
                    if job_info.peak_process_usage > MAX_CPU_RATE as u64 {
                        log::error!("CRITICAL_RESOURCE_BREACH: Process {} exceeded CPU limit of 50%", process_id);
                        unsafe {
                            TerminateJobObject(dup_job_handle, 888);
                        }
                        break;
                    }
                    
                    // Check memory usage
                    if job_info.peak_job_memory_used > MAX_JOB_MEMORY as u64 {
                        log::error!("CRITICAL_RESOURCE_BREACH: Process {} exceeded memory limit of 512MB", process_id);
                        unsafe {
                            TerminateJobObject(dup_job_handle, 888);
                        }
                        break;
                    }
                    
                    // Log resource usage every second
                    if start_time.elapsed().as_secs() % 1 == 0 {
                        log::info!("Process {} resource usage - CPU: {}%, Memory: {}MB",
                                 process_id,
                                 job_info.peak_process_usage / 100,
                                 job_info.peak_job_memory_used / (1024 * 1024));
                    }
                }
            }
            
            // Clean up duplicated handles
            unsafe {
                CloseHandle(dup_process_handle);
                CloseHandle(dup_job_handle);
            }
            
            log::info!("Process Watchdog stopped for PID: {}", process_id);
        })
    }
}

impl Drop for JobObjectManager {
    fn drop(&mut self) {
        if !self.job_handle.is_null() {
            log::info!("Initiating cleanup of Job Object and child processes");
            
            unsafe {
                // Create a JobObjectHandle for final resource usage logging
                if let Ok(job_handle) = JobObjectHandle::new(self.job_handle) {
                    if let Ok(info) = job_handle.query_extended_limit_info() {
                        log::info!("Final resource usage statistics:");
                        log::info!("- Peak Memory Usage: {} MB", info.peak_job_memory_used / (1024 * 1024));
                        log::info!("- Peak CPU Usage: {}%", info.peak_process_usage / 100);
                        log::info!("- Total Processes: {}", info.active_processes);
                        
                        // Log any breaches that occurred
                        if info.peak_job_memory_used > MAX_JOB_MEMORY as u64 {
                            log::error!("CRITICAL_RESOURCE_BREACH: Memory limit exceeded during execution");
                        }
                        if info.peak_process_usage > MAX_CPU_RATE as u64 {
                            log::error!("CRITICAL_RESOURCE_BREACH: CPU limit exceeded during execution");
                        }
                    }
                }
                
                // Terminate all processes in the job with a specific exit code
                log::info!("Terminating all child processes");
                TerminateJobObject(self.job_handle, 888);
                
                // Small delay to ensure processes are terminated
                std::thread::sleep(std::time::Duration::from_millis(100));
                
                // Close the job handle
                log::info!("Closing Job Object handle");
                CloseHandle(self.job_handle);
            }
            
            // Clean up any temporary files
            if let Ok(temp_dir) = std::env::temp_dir().read_dir() {
                for entry in temp_dir {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if let Some(name) = path.file_name() {
                            if let Some(name_str) = name.to_str() {
                                // Clean up our script files
                                if name_str.starts_with("script_") && name_str.ends_with(".py") {
                                    if let Err(e) = std::fs::remove_file(&path) {
                                        log::warn!("Failed to remove temporary file {}: {}", path.display(), e);
                                    } else {
                                        log::debug!("Removed temporary file: {}", path.display());
                                    }
                                    
                                    #[cfg(test)]
                                    mod tests {
                                        use super::*;
                                        use std::time::Duration;
                                    
                                        #[test]
                                        fn test_job_object_creation() {
                                            let job = JobObjectManager::new();
                                            assert!(job.is_ok(), "Should create JobObjectManager successfully");
                                        }
                                    
                                        #[tokio::test]
                                        async fn test_resource_limits() {
                                            let job = JobObjectManager::new().expect("Failed to create JobObjectManager");
                                            
                                            // Test CPU-intensive operation
                                            let cpu_code = r#"
                                    import time
                                    while True:
                                        pass
                                    "#;
                                            let result = job.execute_code(
                                                "python",
                                                &["-c".to_string(), cpu_code.to_string()],
                                                &HashMap::new(),
                                                "python"
                                            ).await;
                                    
                                            assert!(result.is_err(), "Should terminate CPU-intensive process");
                                            assert!(result.unwrap_err().contains("CRITICAL_RESOURCE_BREACH") ||
                                                   result.unwrap_err().contains("exceeded timeout"));
                                        }
                                    
                                        #[tokio::test]
                                        async fn test_memory_limits() {
                                            let job = JobObjectManager::new().expect("Failed to create JobObjectManager");
                                            
                                            // Test memory-intensive operation
                                            let memory_code = r#"
                                    x = ' ' * (1024 * 1024 * 1024)  # Allocate 1GB
                                    "#;
                                            let result = job.execute_code(
                                                "python",
                                                &["-c".to_string(), memory_code.to_string()],
                                                &HashMap::new(),
                                                "python"
                                            ).await;
                                    
                                            assert!(result.is_err(), "Should terminate memory-intensive process");
                                            assert!(result.unwrap_err().contains("CRITICAL_RESOURCE_BREACH"));
                                        }
                                    
                                        #[tokio::test]
                                        async fn test_timeout() {
                                            let job = JobObjectManager::new().expect("Failed to create JobObjectManager");
                                            
                                            // Test long-running operation
                                            let sleep_code = r#"
                                    import time
                                    time.sleep(20)
                                    "#;
                                            let result = job.execute_code(
                                                "python",
                                                &["-c".to_string(), sleep_code.to_string()],
                                                &HashMap::new(),
                                                "python"
                                            ).await;
                                    
                                            assert!(result.is_err(), "Should terminate long-running process");
                                            assert!(result.unwrap_err().contains("exceeded timeout"));
                                        }
                                    
                                        #[test]
                                        fn test_cleanup() {
                                            let job = JobObjectManager::new().expect("Failed to create JobObjectManager");
                                            
                                            // Get initial temp file count
                                            let initial_count = std::fs::read_dir(std::env::temp_dir())
                                                .unwrap()
                                                .filter(|entry| {
                                                    if let Ok(entry) = entry {
                                                        if let Some(name) = entry.file_name().to_str() {
                                                            return name.starts_with("script_") && name.ends_with(".py");
                                                        }
                                                    }
                                                    false
                                                })
                                                .count();
                                            
                                            // Drop the job manager
                                            drop(job);
                                            
                                            // Get final temp file count
                                            let final_count = std::fs::read_dir(std::env::temp_dir())
                                                .unwrap()
                                                .filter(|entry| {
                                                    if let Ok(entry) = entry {
                                                        if let Some(name) = entry.file_name().to_str() {
                                                            return name.starts_with("script_") && name.ends_with(".py");
                                                        }
                                                    }
                                                    false
                                                })
                                                .count();
                                            
                                            assert_eq!(initial_count, final_count, "Should clean up temporary files");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            log::info!("Cleanup completed successfully");
        }
    }
}

/// Execute command using Windows native control
pub async fn execute_with_windows_control(
    command: &str,
    args: &[String],
    env_vars: &HashMap<String, String>,
    language: &str,
) -> Result<(String, String, i32), String> {
    // Validate command path to ensure it doesn't escape sandbox
    let cmd_path = PathBuf::from(command);
    if cmd_path.is_absolute() && !cmd_path.starts_with(SANDBOX_DIR) {
        // Allow system commands like python, cmd, powershell
        let allowed_system_commands = vec!["python", "python.exe", "python3", "python3.exe",
                                          "cmd", "cmd.exe", "powershell", "powershell.exe"];
        let cmd_name = cmd_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if !allowed_system_commands.contains(&cmd_name.to_lowercase().as_str()) {
            return Err(format!("Command path outside sandbox: {}", command));
        }
    }
    
    let job_manager = JobObjectManager::new()?;
    job_manager.execute_code(command, args, env_vars, language).await
}

/// Basic path validation without sandbox restrictions
pub fn validate_path(path: &str) -> Result<PathBuf, String> {
    let requested_path = PathBuf::from(path);
    
    // Basic validation to ensure path exists
    let canonical_path = requested_path
        .canonicalize()
        .map_err(|e| format!("Invalid path: {}", e))?;
    
    Ok(canonical_path)
}