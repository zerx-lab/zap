# Warp AI — Self-Contained AI Terminal Agent

> This is a fork of [warpdotdev/warp](https://github.com/warpdotdev/warp), modified to add a self-contained AI agent that works with any OpenAI-compatible API — no Warp account or backend server required.

**Author:** [goshitsarch](https://github.com/goshitsarch) (goshitsarch@gmail.com)

> AI tools (OpenClaude with GLM-5.1) were used to accelerate the development of these modifications. All code was designed, reviewed, and validated by a human developer.

---

## What This Fork Adds

### 1. `warp-ai` CLI Agent (`crates/warp_ai_cli/`)

A Rust CLI that acts as a fully autonomous sysadmin agent. It:

- Accepts a prompt and system prompt from files
- Calls any **OpenAI-compatible API** (DeepSeek, LiteLLM, Ollama, Together, local models, etc.)
- Uses **function calling / tool use** to autonomously execute tasks
- Streams responses in real-time to the terminal

**Available tools:**

| Tool | Description |
|------|-------------|
| `run_command` | Execute any shell command via `sh -c` — get stdout, stderr, exit code |
| `read_file` | Read file contents |
| `write_file` | Write to a file (creates parent dirs if needed) |
| `list_directory` | List files with `ls -la` |

The agent loop runs until the task is complete: LLM calls tools, results are sent back, LLM decides next step, repeat. Maximum 50 turns by default.

### 2. WarpAi Harness (`app/src/ai/agent_sdk/driver/harness/warp_ai.rs`)

A [ThirdPartyHarness](https://github.com/warpdotdev/warp/blob/master/app/src/ai/agent_sdk/driver/harness/mod.rs) implementation that integrates the `warp-ai` CLI into Warp's agent mode:

- Spawns `warp-ai` in the terminal
- Writes system prompt to a temp file (customizable via `~/.config/warp-ai/system-prompt.txt`)
- **Bypasses Warp's server** — no `resolve_prompt()` call, no account needed
- Conversations are local-only (no server save)

### 3. Default Harness Setting (Settings UI)

A new "Default AI harness" dropdown in **Settings > Features**:

- **None** — default, uses Warp's built-in Oz agent
- **Warp AI** — new tabs/sessions auto-launch the `warp-ai` CLI agent

Stored in `~/.warp/settings.toml` at `general.default_harness`.

### 4. Server Bypass

Added `requires_server_prompt_resolution()` to the `ThirdPartyHarness` trait. Harnesses that return `false` skip the server-side prompt resolution call. This is the mechanism that makes the whole system work without Warp's backend.

---

## Quick Start

### 1. Build

```bash
# Build Warp with the warp-ai binary included
./script/bootstrap
./script/run
```

The `warp-ai` binary is built automatically as part of the workspace.

### 2. Configure Your AI Provider

Set environment variables (or create a config file):

```bash
# Required: your API key
export WARP_AI_API_KEY="sk-your-key-here"

# Optional: defaults to OpenAI
export WARP_AI_BASE_URL="https://api.deepseek.com/v1"
export WARP_AI_MODEL="deepseek-chat"
```

Or create `~/.config/warp-ai/config.json`:

```json
{
  "api_key": "sk-your-key-here",
  "base_url": "https://api.deepseek.com/v1",
  "model": "deepseek-chat"
}
```

### 3. Use It

**Option A: Auto-launch (recommended)**

1. Open Warp
2. Go to **Settings > Features**
3. Set "Default AI harness" to **Warp AI**
4. New tabs automatically start the AI agent

**Option B: Manual launch**

```bash
warp --harness warp-ai -p "check disk usage and clean up temp files over 1GB"
```

**Option C: Standalone CLI**

```bash
echo "show me running services on this machine" > /tmp/prompt.txt
echo "You are a sysadmin assistant." > /tmp/sys.txt
WARP_AI_API_KEY=sk-xxx warp-ai --prompt-file /tmp/prompt.txt --system-prompt-file /tmp/sys.txt
```

---

## Example Usage

```
You: set up nginx with a reverse proxy to localhost:3000

[run_command] sudo apt install -y nginx
[OK] Reading package lists... Setting up nginx...

[run_command] sudo tee /etc/nginx/sites-available/myapp
[OK] Wrote 247 bytes to /etc/nginx/sites-available/myapp

[run_command] sudo ln -sf /etc/nginx/sites-available/myapp /etc/nginx/sites-enabled/
[OK]

[run_command] sudo nginx -t && sudo systemctl reload nginx
[OK] nginx: configuration syntax is ok...

Nginx is configured and running. Your app at localhost:3000 is now
reverse-proxied through nginx on port 80.
```

---

## Supported Providers

Any API that implements the OpenAI chat completions interface with streaming and function calling:

| Provider | Base URL | Notes |
|----------|----------|-------|
| OpenAI | `https://api.openai.com/v1` | Default |
| DeepSeek | `https://api.deepseek.com/v1` | Good value |
| Together AI | `https://api.together.xyz/v1` | |
| Groq | `https://api.groq.com/openai/v1` | Fast inference |
| LiteLLM Proxy | `http://localhost:4000/v1` | Local proxy for any provider |
| Ollama | `http://localhost:11434/v1` | Local models |
| Any OpenAI-compatible | Custom | |

---

## Custom System Prompt

Override the default sysadmin-focused system prompt by creating:

```
~/.config/warp-ai/system-prompt.txt
```

---

## Architecture

```
User → Warp Terminal → WarpAiHarness → spawns `warp-ai` CLI → OpenAI-compatible API
                                              ↕
                                   run_command / read_file / write_file / list_directory
```

### Request Flow

```
1. User types prompt in Warp
2. WarpAiHarness writes prompt + system prompt to temp files
3. Spawns: warp-ai --prompt-file <path> --system-prompt-file <path>
4. CLI sends prompt + tools to LLM API
5. LLM responds with tool calls (e.g., run_command("mkdir deploy"))
6. CLI executes tools locally, sends results back to LLM
7. Repeat steps 5-6 until LLM says done
8. Response streams to Warp terminal
```

---

## Files Added

| File | Purpose |
|------|---------|
| `crates/warp_ai_cli/Cargo.toml` | CLI crate definition |
| `crates/warp_ai_cli/src/main.rs` | Agent loop entry point |
| `crates/warp_ai_cli/src/client.rs` | OpenAI-compatible streaming client with tool call parsing |
| `crates/warp_ai_cli/src/config.rs` | Config resolution (env > CLI > config file) |
| `crates/warp_ai_cli/src/tools.rs` | Tool definitions and execution |
| `app/src/ai/agent_sdk/driver/harness/warp_ai.rs` | WarpAiHarness + WarpAiHarnessRunner |

## Files Modified

| File | Change |
|------|--------|
| `crates/warp_cli/src/agent.rs` | Added `Harness::WarpAi` enum variant |
| `app/src/terminal/cli_agent.rs` | Added `CLIAgent::WarpAi` + all match arms |
| `app/src/ai/agent_sdk/driver/harness/mod.rs` | Registered harness, added `requires_server_prompt_resolution()` |
| `app/src/ai/agent_sdk/driver.rs` | Server bypass guard in `prepare_harness()` |
| `app/src/ai/agent/conversation.rs` | Added `AIAgentHarness::WarpAi` |
| `app/src/ai/harness_display.rs` | Display name, icon, conversions |
| `app/src/ai/agent_sdk/mod.rs` | Telemetry label |
| `app/src/settings/ai.rs` | `DefaultHarness` enum + setting registration |
| `app/src/settings_view/features_page.rs` | Dropdown widget + auto-launch UI |
| `app/src/pane_group/mod.rs` | Auto-launch on new pane |
| `app/src/workspace/view.rs` | Auto-launch on new tab |
| `app/src/server/telemetry/events.rs` | `CLIAgentType::WarpAi` |
| `app/src/terminal/view/ambient_agent/*.rs` | UI updates |
| `app/src/pane_group/pane/local_harness_launch.rs` | Local launch support |
| `app/src/ai/blocklist/history_model/conversation_loader.rs` | Conversation loading |
| Various other files | Exhaustive match updates for new enum variants |

---

## Limitations

- **Agent mode only** — passive suggestions, autosuggestions, and AI command search still require Warp's backend
- **No conversation resume** — conversations are not persisted between sessions
- **No MCP tools** — the CLI uses its own built-in tools, not Warp's MCP tool system
- **Function calling required** — the LLM must support OpenAI-style function calling for the agent loop to work

---

## License

Same as upstream Warp:
- `warpui_core` and `warpui` crates: [MIT license](LICENSE-MIT)
- Everything else (including new code): [AGPL v3](LICENSE-AGPL)

## Credits

- [Warp](https://github.com/warpdotdev/warp) by the Warp team — the incredible terminal this is built on
- AI development assistance: [OpenClaude](https://github.com/anthropics/claude-code) with GLM-5.1
- Maintained by [goshitsarch](https://github.com/goshitsarch)
