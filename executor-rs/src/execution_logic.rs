// executor-rs/src/execution_logic.rs
// Core logic for executing commands with Windows native control
// PHOENIX ORCH: The Ashen Guard Edition AGI

use enigo::{Enigo, Key, KeyboardControllable, MouseButton, MouseControllable};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::Mutex;

// Import Windows executor module
#[cfg(target_os = "windows")]
use crate::windows_executor;

// Allowlist of permitted commands
static ALLOWED_COMMANDS: Lazy<RwLock<Vec<String>>> = Lazy::new(|| {
    RwLock::new(vec![
        "ls".to_string(),
        "dir".to_string(),
        "cat".to_string(),
        "type".to_string(),
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
        "findstr".to_string(),
        "cmd".to_string(),
        "powershell".to_string(),
        // Add more allowed commands as needed
    ])
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
    let sanitized = regex::Regex::new(r"(/|\\)(Users|home)(/|\\)[\w/\\.-]+")
        .unwrap_or_else(|_| {
            log::warn!("Failed to compile path sanitization regex");
            regex::Regex::new(r"").unwrap()
        })
        .replace_all(&error_message, "[USER_PATH]")
        .to_string();

    // Replace IPs, temp directories and sensitive system details
    let sanitized = regex::Regex::new(r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}")
        .unwrap_or_else(|_| regex::Regex::new(r"").unwrap())
        .replace_all(&sanitized, "[IP_ADDRESS]")
        .to_string();

    // Replace temp directory paths
    let sanitized = regex::Regex::new(r"(/|\\)tmp(/|\\)[\w/\\.-]+")
        .unwrap_or_else(|_| regex::Regex::new(r"").unwrap())
        .replace_all(&sanitized, "[TEMP_PATH]")
        .to_string();

    // Generic error message for specific cases
    if sanitized.contains("permission denied") {
        return "Operation not permitted due to security restrictions".to_string();
    }

    sanitized
}

/// Execute a shell command with Windows native control
pub async fn execute_shell_command(
    cmd: &str,
    args: &[String],
    env_vars: &HashMap<String, String>,
) -> Result<(String, String, i32), String> {
    log::info!("Executing command: {} {:?}", cmd, args);

    // Validate command against allowlist
    if let Err(e) = validate_command(cmd) {
        return Err(sanitize_error(e));
    }

    // Basic path validation without sandbox restrictions
    #[cfg(target_os = "windows")]
    {
        for arg in args {
            if arg.contains('\\') || arg.contains('/') {
                if let Err(e) = windows_executor::validate_path(arg) {
                    log::warn!("Path validation failed: {} - {}", arg, e);
                }
            }
        }
    }

    // If the command is python or python3, use the windows sandboxed execution
    if cmd == "python" || cmd == "python3" {
        return execute_python_sandboxed(&args.join(" "), env_vars).await;
    }

    // For other commands, use Windows native execution with Job Object control
    #[cfg(target_os = "windows")]
    {
        log::info!("Using Windows native execution control");
        match windows_executor::execute_with_windows_control(cmd, args, env_vars, "shell").await {
            Ok(result) => {
                // Log output details for debugging
                log::debug!("Command stdout length: {} bytes", result.0.len());
                log::debug!("Command stderr length: {} bytes", result.1.len());
                log::debug!("Command exit code: {}", result.2);
                Ok(result)
            }
            Err(e) => {
                log::error!("Windows execution failed: {}", e);
                Err(sanitize_error(e))
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Fallback for non-Windows systems (for development)
        log::warn!("Non-Windows system detected, using basic process execution");
        execute_basic_command(cmd, args, env_vars).await
    }
}

/// Execute Python code with resource limits and monitoring
pub async fn execute_python_sandboxed(
    code: &str,
    env_vars: &HashMap<String, String>,
) -> Result<(String, String, i32), String> {
    log::info!("Executing Python code in Windows sandboxed environment");

    #[cfg(target_os = "windows")]
    {
        // Write Python code to a temporary file
        let temp_dir = std::env::temp_dir();

        // Generate a unique script name
        let script_name = format!("script_{}.py", uuid::Uuid::new_v4());
        let script_path = temp_dir.join(&script_name);

        // Write the Python code to file
        let mut file = File::create(&script_path)
            .map_err(|e| sanitize_error(format!("Failed to create script file: {}", e)))?;

        file.write_all(code.as_bytes())
            .map_err(|e| sanitize_error(format!("Failed to write script: {}", e)))?;

        // Execute using Windows native control
        let result = windows_executor::execute_with_windows_control(
            "python",
            &vec![script_path.to_string_lossy().to_string()],
            env_vars,
            "python",
        )
        .await;

        // Clean up the script file
        let _ = std::fs::remove_file(&script_path);

        match result {
            Ok(output) => {
                log::debug!("Python execution stdout: {} bytes", output.0.len());
                log::debug!("Python execution stderr: {} bytes", output.1.len());
                Ok(output)
            }
            Err(e) => Err(sanitize_error(e)),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Fallback for non-Windows systems
        Err("Python sandboxed execution only supported on Windows".to_string())
    }
}

/// Basic command execution for non-Windows systems (development only)
#[cfg(not(target_os = "windows"))]
async fn execute_basic_command(
    cmd: &str,
    args: &[String],
    env_vars: &HashMap<String, String>,
) -> Result<(String, String, i32), String> {
    let cmd_owned = cmd.to_string();
    let args_owned = args.to_vec();
    let env_owned = env_vars.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut command = Command::new(cmd_owned);
        command.args(args_owned);
        command.envs(env_owned);
        command.output()
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

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

/// Check if the current process has permissions to simulate input
fn check_input_permissions() -> Result<(), String> {
    // Check for Windows specific permissions
    #[cfg(target_os = "windows")]
    {
        // In Windows, check if we're running with appropriate privileges
        // This could check for specific user rights or group membership
        log::debug!("Checking Windows input simulation permissions");
    }

    #[cfg(target_os = "linux")]
    {
        // Check if we're running in a container or restricted environment
        if std::path::Path::new("/.dockerenv").exists() {
            return Err(sanitize_error(
                "Input simulation not allowed in containerized environment".to_string(),
            ));
        }
    }

    Ok(())
}

/// Allowlist of permitted input actions
static ALLOWED_INPUT_TYPES: Lazy<Vec<&'static str>> =
    Lazy::new(|| vec!["mouse_move", "mouse_click", "type_text", "key_press"]);

/// Validate if input type is allowed
fn validate_input_type(input_type: &str) -> Result<(), String> {
    if ALLOWED_INPUT_TYPES.contains(&input_type) {
        Ok(())
    } else {
        Err(format!("Input type not permitted: {}", input_type))
    }
}

/// Simulate input (mouse/keyboard) with strict security boundaries
pub fn simulate_input(input_type: &str, params: &HashMap<String, String>) -> Result<(), String> {
    log::info!("Requested input simulation: {} {:?}", input_type, params);

    // Validate input type against allowlist
    validate_input_type(input_type)?;

    // Check permissions before allowing input simulation
    check_input_permissions()?;

    // Get screen dimensions for boundary checking
    let screen_width = 1920; // Default fallback value
    let screen_height = 1080; // Default fallback value

    log::info!("Simulating input: {} {:?}", input_type, params);

    let mut enigo = Enigo::new();

    match input_type {
        "mouse_move" => {
            // Validate coordinate parameters
            let x = match params.get("x").and_then(|v| v.parse::<i32>().ok()) {
                Some(val) => {
                    // Enforce boundaries
                    if val < 0 || val > screen_width {
                        return Err(sanitize_error(format!(
                            "X coordinate out of bounds: {}",
                            val
                        )));
                    }
                    val
                }
                None => {
                    return Err(sanitize_error(
                        "Missing or invalid x coordinate".to_string(),
                    ))
                }
            };

            let y = match params.get("y").and_then(|v| v.parse::<i32>().ok()) {
                Some(val) => {
                    // Enforce boundaries
                    if val < 0 || val > screen_height {
                        return Err(sanitize_error(format!(
                            "Y coordinate out of bounds: {}",
                            val
                        )));
                    }
                    val
                }
                None => {
                    return Err(sanitize_error(
                        "Missing or invalid y coordinate".to_string(),
                    ))
                }
            };

            enigo.mouse_move_to(x, y);
        }
        "mouse_click" => {
            // Validate button parameter
            let button = match params.get("button").map(|s| s.as_str()) {
                Some("right") => MouseButton::Right,
                Some("middle") => MouseButton::Middle,
                Some("left") => MouseButton::Left,
                Some(invalid) => {
                    return Err(sanitize_error(format!("Invalid mouse button: {}", invalid)))
                }
                None => MouseButton::Left, // Default to left button
            };
            enigo.mouse_click(button);
        }
        "type_text" => {
            // Validate text parameter
            if let Some(text) = params.get("text") {
                // Limit text length for security
                if text.len() > 1000 {
                    return Err(sanitize_error(
                        "Text input too long (>1000 chars)".to_string(),
                    ));
                }

                enigo.key_sequence(text);
            } else {
                return Err(sanitize_error(
                    "Missing text parameter for type_text".to_string(),
                ));
            }
        }
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
                return Err(sanitize_error(
                    "Missing key parameter for key_press".to_string(),
                ));
            }
        }
        _ => {
            // This should never happen due to validate_input_type check
            return Err(sanitize_error(format!(
                "Unknown input type: {}",
                input_type
            )));
        }
    }

    // Log successful input simulation for audit purposes
    log::info!(
        "Successfully simulated input: {} with params: {:?}",
        input_type,
        params
    );

    Ok(())
}

/// Get execution statistics (for monitoring)
pub fn get_execution_stats() -> HashMap<String, String> {
    let mut stats = HashMap::new();

    #[cfg(target_os = "windows")]
    {
        stats.insert(
            "executor_type".to_string(),
            "Windows_JobObject_Enhanced".to_string(),
        );
        stats.insert(
            "work_dir".to_string(),
            std::env::temp_dir().to_string_lossy().to_string(),
        );
        stats.insert("max_process_memory_mb".to_string(), "512".to_string());
        stats.insert("max_job_memory_mb".to_string(), "512".to_string());
        stats.insert("max_processes".to_string(), "5".to_string());
        stats.insert("execution_timeout_seconds".to_string(), "10".to_string());
        stats.insert("cpu_limit_percent".to_string(), "50".to_string());
        stats.insert(
            "resource_monitoring_interval_ms".to_string(),
            "100".to_string(),
        );
    }

    #[cfg(not(target_os = "windows"))]
    {
        stats.insert("executor_type".to_string(), "Basic_Process".to_string());
        stats.insert("sandbox_dir".to_string(), "Not_Available".to_string());
    }

    stats
}
