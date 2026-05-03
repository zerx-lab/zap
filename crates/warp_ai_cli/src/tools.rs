use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Command;

/// Tool definitions sent to the LLM as available functions.
pub fn tool_definitions() -> Vec<Value> {
    vec![
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "run_command",
                "description": "Execute a shell command and return its stdout, stderr, and exit code. Use this for any system administration task: installing packages, managing services, checking system state, running scripts, etc.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to execute"
                        }
                    },
                    "required": ["command"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the contents of a file. Returns the file content as a string.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Absolute or relative path to the file"
                        }
                    },
                    "required": ["path"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file. Creates the file if it doesn't exist, overwrites if it does. Creates parent directories if needed.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Absolute or relative path to the file"
                        },
                        "content": {
                            "type": "string",
                            "description": "The content to write"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "list_directory",
                "description": "List files and directories at a given path. Returns names with file type indicators.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Absolute or relative path to the directory. Defaults to current directory."
                        }
                    }
                }
            }
        }),
    ]
}

/// A tool call from the LLM.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Result of executing a tool.
#[derive(Debug)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
}

impl ToolResult {
    fn ok(id: String, content: String) -> Self {
        Self {
            tool_call_id: id,
            content,
        }
    }

    fn err(id: String, msg: String) -> Self {
        Self {
            tool_call_id: id,
            content: format!("Error: {msg}"),
        }
    }
}

/// Execute a tool call and return its result.
pub fn execute_tool(call: &ToolCall) -> ToolResult {
    match call.name.as_str() {
        "run_command" => execute_run_command(call),
        "read_file" => execute_read_file(call),
        "write_file" => execute_write_file(call),
        "list_directory" => execute_list_directory(call),
        _ => ToolResult::err(
            call.id.clone(),
            format!("Unknown tool: {}", call.name),
        ),
    }
}

#[derive(Deserialize)]
struct RunCommandArgs {
    command: String,
}

fn execute_run_command(call: &ToolCall) -> ToolResult {
    let args: RunCommandArgs = match serde_json::from_str(&call.arguments) {
        Ok(a) => a,
        Err(e) => return ToolResult::err(call.id.clone(), format!("Invalid arguments: {e}")),
    };

    let output = match Command::new("sh")
        .arg("-c")
        .arg(&args.command)
        .output()
    {
        Ok(o) => o,
        Err(e) => return ToolResult::err(call.id.clone(), format!("Failed to execute: {e}")),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit_code = output.status.code().unwrap_or(-1);

    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str("[stderr]\n");
        result.push_str(&stderr);
    }
    if exit_code != 0 {
        result.push_str(&format!("\n[exit code: {exit_code}]"));
    }

    ToolResult::ok(call.id.clone(), result)
}

#[derive(Deserialize)]
struct ReadFileArgs {
    path: String,
}

fn execute_read_file(call: &ToolCall) -> ToolResult {
    let args: ReadFileArgs = match serde_json::from_str(&call.arguments) {
        Ok(a) => a,
        Err(e) => return ToolResult::err(call.id.clone(), format!("Invalid arguments: {e}")),
    };

    match std::fs::read_to_string(&args.path) {
        Ok(content) => ToolResult::ok(call.id.clone(), content),
        Err(e) => ToolResult::err(
            call.id.clone(),
            format!("Failed to read {}: {e}", args.path),
        ),
    }
}

#[derive(Deserialize)]
struct WriteFileArgs {
    path: String,
    content: String,
}

fn execute_write_file(call: &ToolCall) -> ToolResult {
    let args: WriteFileArgs = match serde_json::from_str(&call.arguments) {
        Ok(a) => a,
        Err(e) => return ToolResult::err(call.id.clone(), format!("Invalid arguments: {e}")),
    };

    // Create parent directories if needed
    if let Some(parent) = std::path::Path::new(&args.path).parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return ToolResult::err(
                    call.id.clone(),
                    format!("Failed to create directories: {e}"),
                );
            }
        }
    }

    match std::fs::write(&args.path, &args.content) {
        Ok(()) => ToolResult::ok(call.id.clone(), format!("Wrote {} bytes to {}", args.content.len(), args.path)),
        Err(e) => ToolResult::err(
            call.id.clone(),
            format!("Failed to write {}: {e}", args.path),
        ),
    }
}

#[derive(Deserialize)]
struct ListDirectoryArgs {
    path: Option<String>,
}

fn execute_list_directory(call: &ToolCall) -> ToolResult {
    let args: ListDirectoryArgs = match serde_json::from_str(&call.arguments) {
        Ok(a) => a,
        Err(e) => return ToolResult::err(call.id.clone(), format!("Invalid arguments: {e}")),
    };

    let path = args.path.as_deref().unwrap_or(".");
    let output = match Command::new("ls")
        .arg("-la")
        .arg(path)
        .output()
    {
        Ok(o) => o,
        Err(e) => return ToolResult::err(call.id.clone(), format!("Failed to list directory: {e}")),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit_code = output.status.code().unwrap_or(-1);

    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str("[stderr]\n");
        result.push_str(&stderr);
    }
    if exit_code != 0 {
        result.push_str(&format!("\n[exit code: {exit_code}]"));
    }

    if exit_code != 0 {
        ToolResult::err(call.id.clone(), result)
    } else {
        ToolResult::ok(call.id.clone(), result)
    }
}
