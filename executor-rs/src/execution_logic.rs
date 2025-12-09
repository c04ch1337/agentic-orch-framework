// executor-rs/src/execution_logic.rs
// Core logic for executing commands and simulating input

use std::process::Command;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use once_cell::sync::Lazy;
use bollard::Docker;
use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions, WaitContainerOptions};
use bollard::models::{HostConfig, Mount, MountTypeEnum};
use enigo::{Enigo, MouseControllable, KeyboardControllable, Key, MouseButton};
use std::sync::RwLock;

// Allowlist of permitted commands
static ALLOWED_COMMANDS: Lazy<RwLock<Vec<String>>> = Lazy::new(|| {
    RwLock::new(vec![
        "ls".to_string(),
        "cat".to_string(),
        "echo".to_string(),
        "cd".to_string(),
        "pwd".to_string(),
        "mkdir".to_string(),
        "python".to_string(),
        "python3".to_string(),
        "pip".to_string(),
        "pip3".to_string(),
        "grep".to_string(),
        "find".to_string(),
        // Add more allowed commands as needed
    ])
});

// Global Docker client
static DOCKER_CLIENT: Lazy<Option<Arc<Mutex<Docker>>>> = Lazy::new(|| {
    match Docker::connect_with_local_defaults() {
        Ok(docker) => Some(Arc::new(Mutex::new(docker))),
        Err(e) => {
            log::error!("Failed to connect to Docker daemon: {}", e);
            None
        }
    }
});

/// Validate if command is permitted based on allowlist
fn validate_command(cmd: &str) -> Result<(), String> {
    let allowed_commands = ALLOWED_COMMANDS.read().unwrap();
    
    if allowed_commands.iter().any(|allowed| allowed == cmd) {
        Ok(())
    } else {
        Err(format!("Command not permitted: {}", cmd))
    }
}

/// Sanitize error messages to prevent information leakage
fn sanitize_error(error_message: String) -> String {
    // Log the original error for debugging, but don't return it to the client
    log::debug!("Original error: {}", error_message);
    
    // Replace absolute paths with generic indicators
    let sanitized = regex::Regex::new(r"(/|\\)(Users|home)(/|\\)[\w/\\.-]+").unwrap_or_else(|_| {
        log::warn!("Failed to compile path sanitization regex");
        regex::Regex::new(r"").unwrap()
    })
    .replace_all(&error_message, "[USER_PATH]")
    .to_string();
    
    // Replace IPs, temp directories and sensitive system details
    let sanitized = regex::Regex::new(r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}").unwrap_or_else(|_| {
        regex::Regex::new(r"").unwrap()
    })
    .replace_all(&sanitized, "[IP_ADDRESS]")
    .to_string();
    
    // Replace temp directory paths
    let sanitized = regex::Regex::new(r"(/|\\)tmp(/|\\)[\w/\\.-]+").unwrap_or_else(|_| {
        regex::Regex::new(r"").unwrap()
    })
    .replace_all(&sanitized, "[TEMP_PATH]")
    .to_string();
    
    // Generic error message for specific cases
    if sanitized.contains("permission denied") {
        return "Operation not permitted due to security restrictions".to_string();
    }
    
    sanitized
}

/// Execute a shell command
pub async fn execute_shell_command(
    cmd: &str,
    args: &[String],
    env_vars: &HashMap<String, String>
) -> Result<(String, String, i32), String> {
    log::info!("Executing command: {} {:?}", cmd, args);
    
    // Validate command against allowlist
    if let Err(e) = validate_command(cmd) {
        return Err(sanitize_error(e));
    }

    // If the command is python or python3, use the sandboxed execution
    if cmd == "python" || cmd == "python3" {
        // Check if we're executing a file or code
        if args.len() == 1 && (args[0].ends_with(".py") || !args[0].contains(" ")) {
            // Likely a file path
            let file_path = &args[0];
            if let Ok(code) = std::fs::read_to_string(file_path) {
                return execute_python_sandboxed(&code, env_vars).await;
            } else {
                return Err(sanitize_error(format!("Failed to read Python file: {}", file_path)));
            }
        } else {
            // Join args into a single string as code
            let script_arg = args.join(" ");
            return execute_python_sandboxed(&script_arg, env_vars).await;
        }
    }

    // For non-Python commands, execute in a separate thread to avoid blocking
    let cmd_owned = cmd.to_string();
    let args_owned = args.to_vec();
    let env_owned = env_vars.clone();
    
    // Execute the command in a blocking task to avoid blocking the async runtime
    let result = tokio::task::spawn_blocking(move || {
        let mut command = Command::new(cmd_owned);
        command.args(args_owned);
        command.envs(env_owned);

        command.output()
    }).await.map_err(|e| format!("Task join error: {}", e))?;

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);
            
            Ok((stdout, stderr, exit_code))
        }
        Err(e) => {
            let err_msg = format!("Failed to execute command: {}", e);
            log::error!("{}", err_msg);
            Err(sanitize_error(err_msg))
        }
    }
}

/// Execute Python code in a Docker container with restrictions
pub async fn execute_python_sandboxed(
    code: &str,
    env_vars: &HashMap<String, String>
) -> Result<(String, String, i32), String> {
    log::info!("Executing Python code in sandboxed environment");

    // Get Docker client
    let docker_client = match &*DOCKER_CLIENT {
        Some(client) => client.clone(),
        None => return Err("Docker client not initialized".to_string()),
    };
    let docker = docker_client.lock().await;

    // Generate a unique container name
    let container_name = format!("agi-python-sandbox-{}", uuid::Uuid::new_v4());
    
    // Create a temporary directory for code execution with proper permissions
    let temp_dir = std::env::temp_dir().join(&container_name);
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    // Set directory permissions to be restrictive (700 - only owner can access)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp_dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("Failed to set temp directory permissions: {}", e))?;
    }

    // Write Python code to a file
    let script_path = temp_dir.join("script.py");
    let mut file = File::create(&script_path)
        .map_err(|e| format!("Failed to create script file: {}", e))?;
    file.write_all(code.as_bytes())
        .map_err(|e| format!("Failed to write to script file: {}", e))?;

    // Set file permissions to be restrictive (600 - only owner can read/write)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set script file permissions: {}", e))?;
    }

    // Create mount for the temp directory with proper security settings
    let mount = Mount {
        target: Some("/workspace".to_string()),
        source: Some(temp_dir.to_string_lossy().to_string()),
        typ: Some(MountTypeEnum::BIND),
        read_only: Some(false), // Need write access for script output
        // Additional mount options for security
        bind_options: Some(bollard::models::MountBindOptions {
            propagation: Some("private".to_string()), // Prevent mount propagation
            ..Default::default()
        }),
        ..Default::default()
    };

    // Set up container with strict security constraints
    let host_config = HostConfig {
        mounts: Some(vec![mount]),
        // Security settings
        // Drop all capabilities first (aggressive security posture)
        cap_drop: Some(vec![
            "ALL".to_string(),
        ]),
        // Only add back the specific capabilities needed
        cap_add: Some(vec![
            "CHOWN".to_string(),      // Allows changing ownership of files
            "SETUID".to_string(),     // Needed for running python properly
            "SETGID".to_string(),     // Needed for running python properly
            "DAC_OVERRIDE".to_string(), // For temporary file operations
        ]),
        security_opt: Some(vec![
            "no-new-privileges:true".to_string(),
            "seccomp=default".to_string(), // Use Docker's default secure profile
        ]),
        // Resource constraints
        memory: Some(100 * 1024 * 1024), // 100MB memory limit
        cpu_quota: Some(50000),          // Limit CPU usage to 50% of one core
        cpu_period: Some(100000),
        // Network restrictions
        network_mode: Some("none".to_string()), // No network access
        ..Default::default()
    };

    // Convert environment variables
    let env: Vec<String> = env_vars
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();

    // Create container config
    let container_config = Config {
        image: Some("python:3.9-slim".to_string()),
        cmd: Some(vec!["python3".to_string(), "/workspace/script.py".to_string()]),
        env: Some(env),
        working_dir: Some("/workspace".to_string()),
        host_config: Some(host_config),
        ..Default::default()
    };

    let options = Some(CreateContainerOptions {
        name: container_name.clone(),
        ..Default::default()
    });

    // Create and start container
    let container_id = match docker.create_container(options, container_config).await {
        Ok(container) => container.id,
        Err(e) => {
            // Clean up temp directory
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(sanitize_error(format!("Failed to create container: {}", e)));
        }
    };

    let start_options = StartContainerOptions::<String>::default();
    if let Err(e) = docker.start_container(&container_id, Some(start_options)).await {
        // Clean up container and temp directory
        let _ = docker.remove_container(&container_id, None).await;
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(sanitize_error(format!("Failed to start container: {}", e)));
    }

    // Wait for container to finish
    let wait_options = Some(WaitContainerOptions {
        condition: "not-running".to_string(),
    });

    let wait_result = match docker.wait_container(&container_id, wait_options).try_collect::<Vec<_>>().await {
        Ok(result) => result,
        Err(e) => {
            // Clean up container and temp directory
            let _ = docker.remove_container(&container_id, None).await;
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(sanitize_error(format!("Failed to wait for container: {}", e)));
        }
    };

    // Get exit code
    let exit_code = wait_result.first()
        .and_then(|r| r.status_code)
        .unwrap_or(-1);
    
    // Get logs
    let logs_options = bollard::container::LogsOptions::<String> {
        stdout: true,
        stderr: true,
        ..Default::default()
    };
    
    // Collect logs
    let mut stdout = String::new();
    let mut stderr = String::new();

    let logs = match docker.logs(&container_id, Some(logs_options)).try_collect::<Vec<_>>().await {
        Ok(logs) => logs,
        Err(e) => {
            // Clean up container and temp directory
            let _ = docker.remove_container(&container_id, None).await;
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(sanitize_error(format!("Failed to get container logs: {}", e)));
        }
    };

    // Process logs
    for log in logs {
        match log {
            bollard::container::LogOutput::StdOut { message } => {
                stdout.push_str(&String::from_utf8_lossy(&message));
            },
            bollard::container::LogOutput::StdErr { message } => {
                stderr.push_str(&String::from_utf8_lossy(&message));
            },
            _ => {}
        }
    }

    // Remove the container
    let remove_options = Some(RemoveContainerOptions {
        force: true,
        ..Default::default()
    });

    // Ensure proper cleanup of container and temporary files
    if let Err(e) = docker.remove_container(&container_id, remove_options).await {
        log::warn!("Failed to remove container {}: {}", container_id, e);
    }
    
    // Securely clean up the temporary directory
    match std::fs::remove_dir_all(&temp_dir) {
        Ok(_) => log::debug!("Successfully removed temporary directory: {}", temp_dir.display()),
        Err(e) => log::warn!("Failed to remove temporary directory {}: {}", temp_dir.display(), e),
    }

    log::info!("Python execution completed with exit code: {}", exit_code);
    Ok((stdout, stderr, exit_code))
}

/// Check if the current process has permissions to simulate input
fn check_input_permissions() -> Result<(), String> {
    // In a production system, this would check:
    // 1. If the process is running with appropriate permissions
    // 2. If the environment allows input simulation (e.g., not in a container)
    // 3. If the system security policy allows input simulation
    
    // For now, we'll implement a basic check
    #[cfg(target_os = "linux")]
    {
        // Check if we're running in a container or restricted environment
        if std::path::Path::new("/.dockerenv").exists() {
            return Err(sanitize_error("Input simulation not allowed in containerized environment".to_string()));
        }
    }
    
    // Additional checks could be added here, such as:
    // - Check for specific environment variables that control this feature
    // - Verify user/process has appropriate permissions
    // - Check against a system-wide security policy
    
    Ok(())
}

/// Allowlist of permitted input actions
static ALLOWED_INPUT_TYPES: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "mouse_move",
        "mouse_click",
        "type_text",
        "key_press",
    ]
});

/// Validate if input type is allowed
fn validate_input_type(input_type: &str) -> Result<(), String> {
    if ALLOWED_INPUT_TYPES.contains(&input_type) {
        Ok(())
    } else {
        Err(format!("Input type not permitted: {}", input_type))
    }
}

/// Simulate input (mouse/keyboard) with strict security boundaries
pub fn simulate_input(
    input_type: &str,
    params: &HashMap<String, String>
) -> Result<(), String> {
    log::info!("Requested input simulation: {} {:?}", input_type, params);
    
    // Validate input type against allowlist
    validate_input_type(input_type)?;
    
    // Check permissions before allowing input simulation
    check_input_permissions()?;
    
    // Get screen dimensions for boundary checking
    let screen_width = 1920;  // Default fallback value
    let screen_height = 1080; // Default fallback value
    
    #[cfg(feature = "get_screen_size")]
    {
        // In a real implementation, we'd get actual screen dimensions
        // This is placeholder code for the concept
        // screen_width = get_actual_width();
        // screen_height = get_actual_height();
    }
    
    log::info!("Simulating input: {} {:?}", input_type, params);

    let mut enigo = Enigo::new();

    match input_type {
        "mouse_move" => {
            // Validate coordinate parameters
            let x = match params.get("x").and_then(|v| v.parse::<i32>().ok()) {
                Some(val) => {
                    // Enforce boundaries
                    if val < 0 || val > screen_width {
                        return Err(sanitize_error(format!("X coordinate out of bounds: {}", val)));
                    }
                    val
                },
                None => return Err(sanitize_error("Missing or invalid x coordinate".to_string()))
            };
            
            let y = match params.get("y").and_then(|v| v.parse::<i32>().ok()) {
                Some(val) => {
                    // Enforce boundaries
                    if val < 0 || val > screen_height {
                        return Err(sanitize_error(format!("Y coordinate out of bounds: {}", val)));
                    }
                    val
                },
                None => return Err(sanitize_error("Missing or invalid y coordinate".to_string()))
            };
            
            enigo.mouse_move_to(x, y);
        },
        "mouse_click" => {
            // Validate button parameter
            let button = match params.get("button").map(|s| s.as_str()) {
                Some("right") => MouseButton::Right,
                Some("middle") => MouseButton::Middle,
                Some("left") => MouseButton::Left,
                Some(invalid) => return Err(sanitize_error(format!("Invalid mouse button: {}", invalid))),
                None => MouseButton::Left, // Default to left button
            };
            enigo.mouse_click(button);
        },
        "type_text" => {
            // Validate text parameter
            if let Some(text) = params.get("text") {
                // Limit text length for security
                if text.len() > 1000 {
                    return Err(sanitize_error("Text input too long (>1000 chars)".to_string()));
                }
                
                // Optionally validate content (no dangerous sequences, etc.)
                // For example, block specific character sequences that could be harmful
                
                enigo.key_sequence(text);
            } else {
                return Err(sanitize_error("Missing text parameter for type_text".to_string()));
            }
        },
        "key_press" => {
            // Validate key parameter
            if let Some(key_name) = params.get("key") {
                // Limit key name length
                if key_name.len() > 20 {
                    return Err(sanitize_error("Key name too long".to_string()));
                }
                
                // Allowlist approach to keys
                match key_name.to_lowercase().as_str() {
                    "enter" | "return" => enigo.key_click(Key::Return),
                    "space" => enigo.key_click(Key::Space),
                    "tab" => enigo.key_click(Key::Tab),
                    "escape" | "esc" => enigo.key_click(Key::Escape),
                    "backspace" => enigo.key_click(Key::Backspace),
                    "delete" => enigo.key_click(Key::Delete),
                    // Only allow single-character keys if they're valid printable ASCII
                    k if k.len() == 1 => {
                        let c = k.chars().next().unwrap();
                        if c.is_ascii() && !c.is_ascii_control() {
                            enigo.key_click(Key::Layout(c));
                        } else {
                            return Err(sanitize_error(format!("Invalid key character: {}", k)));
                        }
                    }
                    k => {
                        log::warn!("Rejected key: {}", k);
                        return Err(sanitize_error(format!("Unsupported key: {}", k)));
                    }
                }
            } else {
                return Err(sanitize_error("Missing key parameter for key_press".to_string()));
            }
        },
        _ => {
            // This should never happen due to validate_input_type check
            return Err(sanitize_error(format!("Unknown input type: {}", input_type)));
        }
    }

    // Log successful input simulation for audit purposes
    log::info!("Successfully simulated input: {} with params: {:?}", input_type, params);
    
    Ok(())
}
