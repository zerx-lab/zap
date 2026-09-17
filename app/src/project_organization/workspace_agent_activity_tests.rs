use crate::ai::agent::conversation::ConversationStatus;
use crate::terminal::cli_agent_sessions::CLIAgentSessionStatus;
use crate::terminal::CLIAgent;

use super::{
    activities_from_terminal_sources, last_agent_activity, workspace_activity_slot,
    OzConversationSource, WorkspaceActivitySlot, WorkspaceAgentActivity, WorkspaceAgentIdentity,
    WorkspaceAgentPhase,
};

fn grok_running() -> WorkspaceAgentActivity {
    WorkspaceAgentActivity {
        identity: WorkspaceAgentIdentity::Cli(CLIAgent::Grok),
        phase: WorkspaceAgentPhase::InProgress,
    }
}

fn claude_blocked() -> WorkspaceAgentActivity {
    WorkspaceAgentActivity {
        identity: WorkspaceAgentIdentity::Cli(CLIAgent::Claude),
        phase: WorkspaceAgentPhase::Blocked,
    }
}

#[test]
fn last_agent_activity_returns_later_candidate() {
    assert_eq!(
        last_agent_activity([grok_running(), claude_blocked()]),
        Some(claude_blocked())
    );
}

#[test]
fn last_agent_activity_returns_none_when_empty() {
    assert_eq!(last_agent_activity([]), None);
}

#[test]
fn activity_slot_prefers_agent_over_running_dot() {
    assert_eq!(
        workspace_activity_slot(Some(grok_running()), true),
        WorkspaceActivitySlot::Agent(grok_running())
    );
}

#[test]
fn activity_slot_falls_back_to_running_dot() {
    assert_eq!(
        workspace_activity_slot(None, true),
        WorkspaceActivitySlot::RunningDot
    );
}

#[test]
fn activity_slot_is_empty_when_idle() {
    assert_eq!(
        workspace_activity_slot(None, false),
        WorkspaceActivitySlot::Empty
    );
}

#[test]
fn in_progress_activity_should_breathe() {
    assert!(grok_running().should_breathe());
}

#[test]
fn blocked_activity_should_not_breathe() {
    assert!(!claude_blocked().should_breathe());
}

#[test]
fn cli_in_progress_is_collected() {
    let activities = activities_from_terminal_sources(
        Some((CLIAgent::Grok, CLIAgentSessionStatus::InProgress)),
        None,
        false,
    );
    assert_eq!(activities, vec![grok_running()]);
}

#[test]
fn cli_success_is_ignored() {
    let activities = activities_from_terminal_sources(
        Some((CLIAgent::Grok, CLIAgentSessionStatus::Success)),
        None,
        false,
    );
    assert!(activities.is_empty());
}

#[test]
fn oz_blocked_is_collected() {
    let activities = activities_from_terminal_sources(
        None,
        Some(OzConversationSource {
            status: ConversationStatus::Blocked {
                blocked_action: "ask".to_string(),
            },
            is_empty: false,
            is_entirely_passive: false,
            is_ambient: false,
        }),
        false,
    );
    assert_eq!(
        activities,
        vec![WorkspaceAgentActivity {
            identity: WorkspaceAgentIdentity::Oz { ambient: false },
            phase: WorkspaceAgentPhase::Blocked,
        }]
    );
}

#[test]
fn empty_or_passive_oz_is_ignored() {
    let activities = activities_from_terminal_sources(
        None,
        Some(OzConversationSource {
            status: ConversationStatus::InProgress,
            is_empty: true,
            is_entirely_passive: false,
            is_ambient: false,
        }),
        false,
    );
    assert!(activities.is_empty());
}

#[test]
fn entirely_passive_oz_in_progress_is_ignored() {
    let activities = activities_from_terminal_sources(
        None,
        Some(OzConversationSource {
            status: ConversationStatus::InProgress,
            is_empty: false,
            is_entirely_passive: true,
            is_ambient: false,
        }),
        false,
    );
    assert!(activities.is_empty());
}

#[test]
fn oz_error_and_cancelled_are_ignored() {
    for status in [ConversationStatus::Error, ConversationStatus::Cancelled] {
        let activities = activities_from_terminal_sources(
            None,
            Some(OzConversationSource {
                status,
                is_empty: false,
                is_entirely_passive: false,
                is_ambient: false,
            }),
            false,
        );
        assert!(activities.is_empty());
    }
}

#[test]
fn cli_wins_over_oz_on_the_same_terminal() {
    let activities = activities_from_terminal_sources(
        Some((CLIAgent::Grok, CLIAgentSessionStatus::InProgress)),
        Some(OzConversationSource {
            status: ConversationStatus::Blocked {
                blocked_action: "ask".to_string(),
            },
            is_empty: false,
            is_entirely_passive: false,
            is_ambient: false,
        }),
        false,
    );
    assert_eq!(last_agent_activity(activities), Some(grok_running()));
}

#[test]
fn ambient_in_progress_uses_oz_cloud_identity() {
    let activities = activities_from_terminal_sources(None, None, true);
    assert_eq!(
        activities,
        vec![WorkspaceAgentActivity {
            identity: WorkspaceAgentIdentity::Oz { ambient: true },
            phase: WorkspaceAgentPhase::InProgress,
        }]
    );
}

#[test]
fn ambient_oz_source_uses_oz_cloud_identity() {
    let activities = activities_from_terminal_sources(
        None,
        Some(OzConversationSource {
            status: ConversationStatus::InProgress,
            is_empty: false,
            is_entirely_passive: false,
            is_ambient: true,
        }),
        false,
    );
    assert_eq!(
        activities,
        vec![WorkspaceAgentActivity {
            identity: WorkspaceAgentIdentity::Oz { ambient: true },
            phase: WorkspaceAgentPhase::InProgress,
        }]
    );
}
