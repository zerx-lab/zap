Send text or control bytes to the stdin / PTY of a long-running shell command that was previously started with `run_shell_command` (with `wait_until_complete=false`).

You will need the `command_id` from the `LongRunningCommandSnapshot` returned by the original `run_shell_command` call.

Common uses:
- Send input to an interactive REPL (Python/Node/bun shell, `dotenv` prompts, etc.).
- Reply to a TUI prompt.
- Terminate the process: pass raw byte `\x03` (Ctrl-C) or `\x04` (Ctrl-D / EOF).

Pass `raw=true` to send raw bytes (no trailing newline added). Otherwise the input is treated as a line — a newline is appended automatically.

After sending, follow up with `read_shell_command_output` to read the response.
