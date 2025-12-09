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

/// Execute a shell command
pub async fn execute_shell_command(
    cmd: &str,
    args: &[String],
    env_vars: &HashMap<String, String>
) -> Result<(String, String, i32), String> {
    log::info!("Executing command: {} {:?}", cmd, args);

    // If the command is python or python3, use the sandboxed execution
    if cmd == "python" || cmd == "python3" {
        // Check if we're executing a file or code
        if args.len() == 1 && (args[0].ends_with(".py") || !args[0].contains(" ")) {
            // Likely a file path
            let file_path = &args[0];
            if let Ok(code) = std::fs::read_to_string(file_path) {
                return execute_python_sandboxed(&code, env_vars).await;
            } else {
                return Err(format!("Failed to read Python file: {}", file_path));
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
            Err(err_msg)
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
    
    // Create a temporary directory for code execution
    let temp_dir = std::env::temp_dir().join(&container_name);
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;
    
    // Write Python code to a file
    let script_path = temp_dir.join("script.py");
    let mut file = File::create(&script_path)
        .map_err(|e| format!("Failed to create script file: {}", e))?;
    file.write_all(code.as_bytes())
        .map_err(|e| format!("Failed to write to script file: {}", e))?;

    // Create mount for the temp directory
    let mount = Mount {
        target: Some("/workspace".to_string()),
        source: Some(temp_dir.to_string_lossy().to_string()),
        typ: Some(MountTypeEnum::BIND),
        read_only: Some(false),
        ..Default::default()
    };

    // Set up container with strict security constraints
    let host_config = HostConfig {
        mounts: Some(vec![mount]),
        // Security settings
        cap_drop: Some(vec![
            "NET_ADMIN".to_string(),
            "SYS_ADMIN".to_string(),
            "SYS_PTRACE".to_string(),
        ]),
        security_opt: Some(vec![
            "no-new-privileges:true".to_string(),
            "seccomp=unconfined".to_string(),
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
            return Err(format!("Failed to create container: {}", e));
        }
    };

    let start_options = StartContainerOptions::<String>::default();
    if let Err(e) = docker.start_container(&container_id, Some(start_options)).await {
        // Clean up container and temp directory
        let _ = docker.remove_container(&container_id, None).await;
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(format!("Failed to start container: {}", e));
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
            return Err(format!("Failed to wait for container: {}", e));
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
            return Err(format!("Failed to get container logs: {}", e));
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

    let _ = docker.remove_container(&container_id, remove_options).await;
    let _ = std::fs::remove_dir_all(&temp_dir);

    log::info!("Python execution completed with exit code: {}", exit_code);
    Ok((stdout, stderr, exit_code))
}

/// Simulate input (mouse/keyboard)
pub fn simulate_input(
    input_type: &str, 
    params: &HashMap<String, String>
) -> Result<(), String> {
    log::info!("Simulating input: {} {:?}", input_type, params);

    let mut enigo = Enigo::new();

    match input_type {
        "mouse_move" => {
            let x = params.get("x").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
            let y = params.get("y").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
            enigo.mouse_move_to(x, y);
        },
        "mouse_click" => {
            let button = match params.get("button").map(|s| s.as_str()) {
                Some("right") => MouseButton::Right,
                Some("middle") => MouseButton::Middle,
                _ => MouseButton::Left,
            };
            enigo.mouse_click(button);
        },
        "type_text" => {
            if let Some(text) = params.get("text") {
                enigo.key_sequence(text);
            }
        },
        "key_press" => {
            if let Some(key_name) = params.get("key") {
                // Simple mapping for common keys
                // In a real app, this would need a comprehensive lookup
                match key_name.to_lowercase().as_str() {
                    "enter" | "return" => enigo.key_click(Key::Return),
                    "space" => enigo.key_click(Key::Space),
                    "tab" => enigo.key_click(Key::Tab),
                    "escape" | "esc" => enigo.key_click(Key::Escape),
                    "backspace" => enigo.key_click(Key::Backspace),
                    "delete" => enigo.key_click(Key::Delete),
                    k => {
                        // If single character, type it
                        if k.len() == 1 {
                            let c = k.chars().next().unwrap();
                            enigo.key_click(Key::Layout(c));
                        } else {
                            log::warn!("Unknown key: {}", k);
                            return Err(format!("Unknown key: {}", k));
                        }
                    }
                }
            }
        },
        _ => {
            return Err(format!("Unknown input type: {}", input_type));
        }
    }

    Ok(())
}
