// executor-rs/src/execution_logic.rs
// Core logic for executing commands and simulating input

use std::process::Command;
use std::collections::HashMap;
use enigo::{Enigo, MouseControllable, KeyboardControllable, Key, MouseButton};

/// Execute a shell command
pub fn execute_shell_command(
    cmd: &str, 
    args: &[String],
    env_vars: &HashMap<String, String>
) -> Result<(String, String, i32), String> {
    log::info!("Executing command: {} {:?}", cmd, args);

    let mut command = Command::new(cmd);
    command.args(args);
    command.envs(env_vars);

    // TODO: Add sandboxing logic here (e.g., using bollard to run in container)
    // For now, we run directly on host but with logging

    match command.output() {
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
