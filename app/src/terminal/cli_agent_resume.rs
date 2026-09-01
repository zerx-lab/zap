//! 把仍在跑的 CLI agent 会话写成可 resume 的命令,供重启后自动接回。

use serde::{Deserialize, Serialize};

use super::cli_agent_sessions::CLIAgentSession;
use super::CLIAgent;

/// 退出时从 pane 抽出、重启后用来拼 resume 命令的数据。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliAgentResumeSnapshot {
    pub agent: CLIAgent,
    pub session_id: Option<String>,
    pub original_command: Option<String>,
}

impl CliAgentResumeSnapshot {
    /// 仅当该会话本地、能拼出 resume 命令时才返回快照。
    pub(crate) fn from_active_session(
        session: &CLIAgentSession,
        original_command: Option<String>,
    ) -> Option<Self> {
        if session.is_remote() {
            return None;
        }
        let snapshot = Self {
            agent: session.agent,
            session_id: session
                .session_context
                .session_id
                .as_deref()
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_owned),
            original_command: original_command
                .as_deref()
                .map(str::trim)
                .filter(|command| !command.is_empty())
                .map(str::to_owned),
        };
        snapshot.resume_command().is_some().then_some(snapshot)
    }

    /// 重启后要写入 input 并执行的 resume 命令。
    pub fn resume_command(&self) -> Option<String> {
        build_resume_command(
            self.agent,
            self.session_id.as_deref(),
            self.original_command.as_deref(),
        )
    }
}

/// Resume 只有在 login shell bootstrap 完成后才能从队列取出。
/// 过早 take 会把命令写进仍会被 bootstrap 清掉的 input,重启后会话接不回去。
pub(crate) fn take_resume_command_if_shell_ready(
    pending: &mut Option<String>,
    is_login_shell_bootstrapped: bool,
) -> Option<String> {
    if is_login_shell_bootstrapped {
        pending.take()
    } else {
        None
    }
}

/// 按各 CLI 的 resume 语法拼命令。session_id 优先;没有 id 时仅 Claude / Codex 回退到 continue/last。
pub fn build_resume_command(
    agent: CLIAgent,
    session_id: Option<&str>,
    original_command: Option<&str>,
) -> Option<String> {
    let syntax = resume_syntax(agent)?;
    if let Some(session_id) = session_id.map(str::trim).filter(|id| !id.is_empty()) {
        if let Some(original) = original_command
            .map(str::trim)
            .filter(|command| !command.is_empty() && is_simple_command_line(command))
        {
            if let Some(command) = inject_resume_into_original(agent, original, syntax, session_id)
            {
                return Some(command);
            }
        }
        return Some(canonical_resume(agent, syntax, session_id));
    }
    continue_without_session_id(agent).map(str::to_owned)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResumeSyntax {
    /// `claude --resume <id>`
    LongFlag,
    /// `copilot --resume=<id>`
    EqualsFlag,
    /// `codex resume <id>`
    Subcommand,
    /// `opencode -s <id>`
    ShortSessionFlag,
    /// `pi --session <id>`
    SessionFlag,
}

fn resume_syntax(agent: CLIAgent) -> Option<ResumeSyntax> {
    match agent {
        CLIAgent::Claude | CLIAgent::Grok | CLIAgent::Gemini | CLIAgent::Omp => {
            Some(ResumeSyntax::LongFlag)
        }
        CLIAgent::Copilot => Some(ResumeSyntax::EqualsFlag),
        CLIAgent::Codex => Some(ResumeSyntax::Subcommand),
        CLIAgent::OpenCode => Some(ResumeSyntax::ShortSessionFlag),
        CLIAgent::Pi => Some(ResumeSyntax::SessionFlag),
        CLIAgent::Amp
        | CLIAgent::Droid
        | CLIAgent::Auggie
        | CLIAgent::CursorCli
        | CLIAgent::Goose
        | CLIAgent::DeepSeek
        | CLIAgent::Antigravity
        | CLIAgent::Unknown => None,
    }
}

fn continue_without_session_id(agent: CLIAgent) -> Option<&'static str> {
    match agent {
        CLIAgent::Claude => Some("claude --continue"),
        CLIAgent::Codex => Some("codex resume --last"),
        CLIAgent::Gemini
        | CLIAgent::Amp
        | CLIAgent::Droid
        | CLIAgent::OpenCode
        | CLIAgent::Copilot
        | CLIAgent::Pi
        | CLIAgent::Auggie
        | CLIAgent::CursorCli
        | CLIAgent::Goose
        | CLIAgent::DeepSeek
        | CLIAgent::Antigravity
        | CLIAgent::Omp
        | CLIAgent::Grok
        | CLIAgent::Unknown => None,
    }
}

fn canonical_resume(agent: CLIAgent, syntax: ResumeSyntax, session_id: &str) -> String {
    let quoted = shell_quote(session_id);
    let prefix = agent.command_prefix();
    match syntax {
        ResumeSyntax::LongFlag => format!("{prefix} --resume {quoted}"),
        ResumeSyntax::EqualsFlag => format!("{prefix} --resume={quoted}"),
        ResumeSyntax::Subcommand => format!("{prefix} resume {quoted}"),
        ResumeSyntax::ShortSessionFlag => format!("{prefix} -s {quoted}"),
        ResumeSyntax::SessionFlag => format!("{prefix} --session {quoted}"),
    }
}

fn inject_resume_into_original(
    agent: CLIAgent,
    original: &str,
    syntax: ResumeSyntax,
    session_id: &str,
) -> Option<String> {
    let tokens: Vec<&str> = original.split_whitespace().collect();
    let binary = tokens.first().copied()?;
    if !agent.matches_command_prefix(binary) {
        return None;
    }
    let stripped = strip_resume_tokens(&tokens[1..], syntax);
    let quoted = shell_quote(session_id);
    let mut out = Vec::with_capacity(stripped.len() + 3);
    out.push(binary.to_string());
    out.extend(stripped);
    match syntax {
        ResumeSyntax::LongFlag => {
            out.push("--resume".to_owned());
            out.push(quoted);
        }
        ResumeSyntax::EqualsFlag => out.push(format!("--resume={quoted}")),
        ResumeSyntax::Subcommand => {
            out.push("resume".to_owned());
            out.push(quoted);
        }
        ResumeSyntax::ShortSessionFlag => {
            out.push("-s".to_owned());
            out.push(quoted);
        }
        ResumeSyntax::SessionFlag => {
            out.push("--session".to_owned());
            out.push(quoted);
        }
    }
    Some(out.join(" "))
}

fn strip_resume_tokens(args: &[&str], syntax: ResumeSyntax) -> Vec<String> {
    let mut out = Vec::new();
    let mut skip_next = false;
    for (index, token) in args.iter().copied().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        let next = args.get(index + 1).copied();
        match syntax {
            ResumeSyntax::LongFlag => {
                if token == "--continue" || token == "-c" {
                    continue;
                }
                if token == "--resume" || token == "-r" {
                    if next.is_some_and(|value| !value.starts_with('-')) {
                        skip_next = true;
                    }
                    continue;
                }
            }
            ResumeSyntax::EqualsFlag => {
                if token.starts_with("--resume=") {
                    continue;
                }
                if token == "--resume" {
                    if next.is_some_and(|value| !value.starts_with('-')) {
                        skip_next = true;
                    }
                    continue;
                }
            }
            ResumeSyntax::Subcommand => {
                if token == "resume" {
                    if next == Some("--last") {
                        skip_next = true;
                    } else if next.is_some_and(|value| !value.starts_with('-')) {
                        skip_next = true;
                    }
                    continue;
                }
            }
            ResumeSyntax::ShortSessionFlag => {
                if token == "-s" || token == "--session" {
                    if next.is_some_and(|value| !value.starts_with('-')) {
                        skip_next = true;
                    }
                    continue;
                }
            }
            ResumeSyntax::SessionFlag => {
                if token == "--session" {
                    if next.is_some_and(|value| !value.starts_with('-')) {
                        skip_next = true;
                    }
                    continue;
                }
            }
        }
        out.push(token.to_owned());
    }
    out
}

fn is_simple_command_line(command: &str) -> bool {
    !command.chars().any(|ch| {
        matches!(
            ch,
            '|' | ';' | '&' | '`' | '\n' | '"' | '\'' | '(' | ')' | '<' | '>' | '\\'
        )
    })
}

fn shell_quote(value: &str) -> String {
    if value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
#[path = "cli_agent_resume_tests.rs"]
mod tests;
