use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;
use tempfile::NamedTempFile;
use warp_cli::agent::Harness;
use warp_managed_secrets::ManagedSecretValue;
use warpui::{ModelHandle, ModelSpawner};

use crate::ai::ambient_agents::AmbientAgentTaskId;
use crate::server::server_api::ServerApi;
use crate::terminal::model::block::BlockId;
use crate::terminal::CLIAgent;

use super::super::terminal::{CommandHandle, TerminalDriver};
use super::super::{AgentDriver, AgentDriverError};
use super::{write_temp_file, HarnessRunner, ResumePayload, SavePoint, ThirdPartyHarness};

pub(crate) struct WarpAiHarness;

const WARP_AI_CLI_NAME: &str = "warp-ai";

const DEFAULT_SYSTEM_PROMPT: &str = "\
You are a system administration AI agent running inside a terminal. You execute tasks \
on Mac and Linux machines by running shell commands, reading and writing files, and \
inspecting system state.

## Available Tools

You have access to these tools:
- **run_command**: Execute any shell command via `sh -c`. You get stdout, stderr, and exit code.
- **read_file**: Read a file's contents.
- **write_file**: Write content to a file (creates parent dirs if needed).
- **list_directory**: List files with details (`ls -la`).

## Guidelines

1. **Execute, don't suggest.** When asked to do something, use the tools to actually do it. \
   Don't just print commands for the user to run.
2. **Verify your work.** After making changes, run commands to confirm the result \
   (e.g., check a service is running, verify a file was written correctly).
3. **Be safe.** Before destructive operations (rm, format, drop), confirm what will be affected. \
   Use dry-run flags when available.
4. **Handle errors.** If a command fails, read the error, diagnose the issue, and try to fix it.
5. **Know the OS.** Detect the OS early (`uname -a`, `sw_vers`) and use appropriate commands. \
   macOS and Linux differ in package managers, service management, and file locations.
6. **One step at a time.** For complex tasks, break them into steps. Run a command, check the \
   result, then proceed.
7. **Prefer idempotent commands.** Use `mkdir -p`, `install -d`, `>>` for append, etc.
8. **Current directory.** Assume you're in the user's current working directory unless they specify otherwise.

## Common Tasks

- Package management: apt, yum, brew, pacman
- Service management: systemctl, launchctl, service
- File management: create, edit, move, copy, permissions
- User management: useradd, usermod, groups
- Network: curl, wget, netstat, ss, ping, dig
- Process management: ps, top, kill, lsof
- Disk: df, du, mount, fdisk/parted
- Logs: journalctl, tail, grep in /var/log";

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl ThirdPartyHarness for WarpAiHarness {
    fn harness(&self) -> Harness {
        Harness::WarpAi
    }

    fn cli_agent(&self) -> CLIAgent {
        CLIAgent::WarpAi
    }

    fn requires_server_prompt_resolution(&self) -> bool {
        false
    }

    fn validate(&self) -> Result<(), AgentDriverError> {
        // Check for warp-ai next to the Warp binary first (bundled build)
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(dir) = exe_path.parent() {
                let bundled = dir.join(WARP_AI_CLI_NAME);
                if bundled.exists() {
                    return Ok(());
                }
            }
        }
        // Fall back to PATH check
        validate_cli_installed(self.cli_agent().command_prefix(), self.install_docs_url())
    }

    fn prepare_environment_config(
        &self,
        _working_dir: &Path,
        _system_prompt: Option<&str>,
        _secrets: &HashMap<String, ManagedSecretValue>,
    ) -> Result<(), AgentDriverError> {
        // No special environment setup needed beyond the system prompt file
        // which is handled in build_runner.
        Ok(())
    }

    fn build_runner(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        _resumption_prompt: Option<&str>,
        _working_dir: &Path,
        _task_id: Option<AmbientAgentTaskId>,
        _server_api: Arc<ServerApi>,
        terminal_driver: ModelHandle<TerminalDriver>,
        _resume: Option<ResumePayload>,
    ) -> Result<Box<dyn HarnessRunner>, AgentDriverError> {
        Ok(Box::new(WarpAiHarnessRunner::new(
            prompt,
            system_prompt,
            terminal_driver,
        )?))
    }
}

fn warp_ai_command(cli_name: &str, prompt_path: &str, system_prompt_path: &str) -> String {
    format!(
        "{cli_name} --prompt-file '{prompt_path}' --system-prompt-file '{system_prompt_path}'"
    )
}

enum WarpAiRunnerState {
    Preexec,
    Running { block_id: BlockId },
}

struct WarpAiHarnessRunner {
    command: String,
    /// Held so temp files are cleaned up when the runner is dropped.
    _temp_prompt_file: NamedTempFile,
    _temp_system_prompt_file: NamedTempFile,
    terminal_driver: ModelHandle<TerminalDriver>,
    state: Mutex<WarpAiRunnerState>,
}

impl WarpAiHarnessRunner {
    fn new(
        prompt: &str,
        system_prompt: Option<&str>,
        terminal_driver: ModelHandle<TerminalDriver>,
    ) -> Result<Self, AgentDriverError> {
        let temp_prompt_file = write_temp_file("warp_ai_prompt_", prompt)?;

        let resolved_system_prompt = system_prompt
            .map(|s| s.to_owned())
            .or_else(|| load_custom_system_prompt())
            .unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_owned());

        let temp_system_prompt_file =
            write_temp_file("warp_ai_sys_", &resolved_system_prompt)?;

        let prompt_path = temp_prompt_file.path().display().to_string();
        let sys_path = temp_system_prompt_file.path().display().to_string();

        Ok(Self {
            command: warp_ai_command(WARP_AI_CLI_NAME, &prompt_path, &sys_path),
            _temp_prompt_file: temp_prompt_file,
            _temp_system_prompt_file: temp_system_prompt_file,
            terminal_driver,
            state: Mutex::new(WarpAiRunnerState::Preexec),
        })
    }
}

/// Load a custom system prompt from `~/.config/warp-ai/system-prompt.txt` if it exists.
fn load_custom_system_prompt() -> Option<String> {
    dirs::home_dir()
        .map(|p| p.join(".config/warp-ai/system-prompt.txt"))
        .filter(|p| p.exists())
        .and_then(|p| std::fs::read_to_string(p).ok())
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl HarnessRunner for WarpAiHarnessRunner {
    async fn start(
        &self,
        foreground: &ModelSpawner<AgentDriver>,
    ) -> Result<CommandHandle, AgentDriverError> {
        let command = self.command.clone();
        let terminal_driver = self.terminal_driver.clone();
        let command_handle = foreground
            .spawn(move |_, ctx| {
                terminal_driver.update(ctx, |driver, ctx| driver.execute_command(&command, ctx))
            })
            .await??
            .await?;

        *self.state.lock() = WarpAiRunnerState::Running {
            block_id: command_handle.block_id().clone(),
        };

        Ok(command_handle)
    }

    async fn exit(&self, foreground: &ModelSpawner<AgentDriver>) -> Result<()> {
        log::info!("Sending Ctrl-C to warp-ai CLI");
        let terminal_driver = self.terminal_driver.clone();
        foreground
            .spawn(move |_, ctx| {
                terminal_driver.update(ctx, |driver, ctx| {
                    driver.send_text_to_cli("\x03".to_string(), ctx);
                });
            })
            .await
            .map_err(|_| anyhow::anyhow!("Agent driver dropped while sending exit"))
    }

    async fn save_conversation(
        &self,
        _save_point: SavePoint,
        _foreground: &ModelSpawner<AgentDriver>,
    ) -> Result<()> {
        // No server to save to — conversations are local only.
        Ok(())
    }
}
