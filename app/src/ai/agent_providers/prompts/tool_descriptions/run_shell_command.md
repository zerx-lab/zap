Executes a given shell command in the user's current shell session.

All commands run in the current working directory by default. The command will be presented to the user for approval before execution unless an auto-approve / allowlist rule applies.

IMPORTANT: This tool is for terminal operations like git, npm, docker, build/test scripts, etc. DO NOT use it for file operations (reading, writing, editing, searching, finding files) — use the specialized tools (`read_files`, `apply_file_diffs`, `grep`, `file_glob`) instead.

# Long-running commands (CRITICAL)

For dev servers, watchers, `tail -f`, interactive REPLs, or any process that does **not exit on its own**, you MUST set `wait_until_complete=false`. Otherwise the current turn will hang forever waiting for the command to terminate.

When `wait_until_complete=false`, the tool returns a `LongRunningCommandSnapshot` with a `command_id`. Use it for follow-ups:
- `read_shell_command_output` — fetch latest stdout
- `write_to_long_running_shell_command` — send stdin / control bytes (e.g. `\x03` for Ctrl-C to terminate)

For one-shot commands (`ls`, `git status`, `cargo test`, etc.) keep the default `wait_until_complete=true`.

# Approval flags

- `is_read_only=true` — command only reads info (ls/cat/git status/grep/...) and is safe to auto-approve.
- `is_risky=true` — command is destructive (rm -rf, force-push, schema migration, killing processes). Set this so the user gets a more visible confirmation.
- `uses_pager=true` — command may invoke a pager (less/more/git log). Prefer appending `| cat` to avoid blocking.

# Quoting and parallelism

- Always quote file paths that contain spaces with double quotes:
  - `mkdir "/Users/name/My Documents"` (correct)
  - `mkdir /Users/name/My Documents` (incorrect — fails)
- When you need to run several independent commands (e.g. `git status` and `git diff`), batch them as multiple parallel `tool_calls` in a single response rather than chaining with `&&`.
- Use `;` only when commands must run sequentially but you don't care if earlier ones fail.
- Avoid `cd <dir> && <cmd>` patterns. The shell session inherits the user's working directory.

# Tools you should NOT shell out to

Avoid using shell with these — use the dedicated tool instead:

| Don't | Use |
|---|---|
| `find`, `ls -R` | `file_glob` |
| `grep`, `rg` | `grep` |
| `cat`, `head`, `tail` | `read_files` |
| `sed`, `awk` | `apply_file_diffs` |
| `echo > file`, `cat <<EOF > file` | `apply_file_diffs` (operation: `create`) |

NEVER use `echo` / `printf` to communicate with the user. Output normal text in your reply instead.

# Git safety

- NEVER run `git config`, `git push --force`, `git reset --hard`, `git checkout --`, `git rebase -i`, or commit hook bypass flags (`--no-verify`, `--no-gpg-sign`) unless the user explicitly requested them.
- NEVER commit or push unless the user explicitly asked.
- For amends: only amend if the user asked, the previous commit is yours, and it has not been pushed.
- For PRs/commits: when the user asks, gather context first (`git status`, `git diff`, `git log`) in parallel, then craft the message.
