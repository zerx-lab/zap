use super::{build_resume_command, take_resume_command_if_shell_ready, CliAgentResumeSnapshot};
use crate::terminal::cli_agent_sessions::{
    CLIAgentInputState, CLIAgentSession, CLIAgentSessionContext, CLIAgentSessionStatus,
};
use crate::terminal::CLIAgent;

fn session(agent: CLIAgent, session_id: Option<&str>, remote: bool) -> CLIAgentSession {
    CLIAgentSession {
        agent,
        status: CLIAgentSessionStatus::InProgress,
        session_context: CLIAgentSessionContext {
            session_id: session_id.map(str::to_owned),
            ..Default::default()
        },
        input_state: CLIAgentInputState::Closed,
        should_auto_toggle_input: false,
        listener: None,
        plugin_version: None,
        remote_host: remote.then(|| "user@host".to_owned()),
        draft_text: None,
        custom_command_prefix: None,
    }
}

#[test]
fn claude_resume_uses_session_id() {
    assert_eq!(
        build_resume_command(CLIAgent::Claude, Some("abc-123"), None).as_deref(),
        Some("claude --resume abc-123"),
    );
}

#[test]
fn claude_resume_preserves_original_flags() {
    assert_eq!(
        build_resume_command(
            CLIAgent::Claude,
            Some("abc-123"),
            Some("claude --dangerously-skip-permissions --model opus"),
        )
        .as_deref(),
        Some("claude --dangerously-skip-permissions --model opus --resume abc-123"),
    );
}

#[test]
fn claude_resume_strips_existing_resume_and_continue() {
    assert_eq!(
        build_resume_command(
            CLIAgent::Claude,
            Some("new-id"),
            Some("claude --continue --resume old-id --dangerously-skip-permissions"),
        )
        .as_deref(),
        Some("claude --dangerously-skip-permissions --resume new-id"),
    );
}

#[test]
fn claude_without_session_id_falls_back_to_continue() {
    assert_eq!(
        build_resume_command(CLIAgent::Claude, None, Some("claude --model opus")).as_deref(),
        Some("claude --continue"),
    );
}

#[test]
fn codex_resume_is_a_subcommand() {
    assert_eq!(
        build_resume_command(
            CLIAgent::Codex,
            Some("sess_1"),
            Some("codex --dangerously-bypass-approvals-and-sandbox"),
        )
        .as_deref(),
        Some("codex --dangerously-bypass-approvals-and-sandbox resume sess_1"),
    );
}

#[test]
fn codex_without_session_id_resumes_last() {
    assert_eq!(
        build_resume_command(CLIAgent::Codex, None, None).as_deref(),
        Some("codex resume --last"),
    );
}

#[test]
fn grok_resume_uses_long_flag() {
    assert_eq!(
        build_resume_command(CLIAgent::Grok, Some("sid"), Some("grok --always-approve")).as_deref(),
        Some("grok --always-approve --resume sid"),
    );
}

#[test]
fn grok_without_session_id_is_skipped() {
    assert_eq!(build_resume_command(CLIAgent::Grok, None, None), None);
}

#[test]
fn copilot_uses_equals_flag() {
    assert_eq!(
        build_resume_command(CLIAgent::Copilot, Some("abc"), None).as_deref(),
        Some("copilot --resume=abc"),
    );
}

#[test]
fn opencode_and_pi_use_session_flags() {
    assert_eq!(
        build_resume_command(CLIAgent::OpenCode, Some("sid"), None).as_deref(),
        Some("opencode -s sid"),
    );
    assert_eq!(
        build_resume_command(CLIAgent::Pi, Some("sid"), None).as_deref(),
        Some("pi --session sid"),
    );
}

#[test]
fn unsupported_agent_returns_none() {
    assert_eq!(build_resume_command(CLIAgent::Amp, Some("sid"), None), None);
}

#[test]
fn complex_original_command_falls_back_to_canonical() {
    assert_eq!(
        build_resume_command(
            CLIAgent::Claude,
            Some("sid"),
            Some("claude --model opus | tee log"),
        )
        .as_deref(),
        Some("claude --resume sid"),
    );
}

#[test]
fn quotes_session_ids_with_special_characters() {
    assert_eq!(
        build_resume_command(CLIAgent::Claude, Some("id with space"), None).as_deref(),
        Some("claude --resume 'id with space'"),
    );
}

#[test]
fn snapshot_skips_remote_and_unresumable_sessions() {
    assert!(CliAgentResumeSnapshot::from_active_session(
        &session(CLIAgent::Claude, Some("sid"), true),
        None,
    )
    .is_none());
    assert!(CliAgentResumeSnapshot::from_active_session(
        &session(CLIAgent::Amp, Some("sid"), false),
        None,
    )
    .is_none());
}

#[test]
fn snapshot_keeps_local_claude_session() {
    let snapshot = CliAgentResumeSnapshot::from_active_session(
        &session(CLIAgent::Claude, Some("sid"), false),
        Some("claude --dangerously-skip-permissions".to_owned()),
    )
    .expect("local claude session should persist");
    assert_eq!(
        snapshot.resume_command().as_deref(),
        Some("claude --dangerously-skip-permissions --resume sid"),
    );
}

#[test]
fn queued_resume_stays_until_shell_is_bootstrapped() {
    let mut pending = Some("claude --resume sid".to_owned());
    assert_eq!(
        take_resume_command_if_shell_ready(&mut pending, false),
        None
    );
    assert_eq!(pending.as_deref(), Some("claude --resume sid"));
}

#[test]
fn queued_resume_is_taken_after_shell_bootstraps() {
    let mut pending = Some("claude --resume sid".to_owned());
    assert_eq!(
        take_resume_command_if_shell_ready(&mut pending, true).as_deref(),
        Some("claude --resume sid"),
    );
    assert_eq!(pending, None);
}
