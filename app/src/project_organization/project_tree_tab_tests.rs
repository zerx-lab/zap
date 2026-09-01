use warpui::EntityId;

use crate::project_organization::domain::RepositoryWorkspaceId;
use crate::project_organization::workspace_agent_activity::{
    WorkspaceActivitySlot, WorkspaceAgentActivity, WorkspaceAgentIdentity, WorkspaceAgentPhase,
};
use crate::terminal::CLIAgent;

use super::{
    ProjectTreeTabId, ProjectTreeTabNode, ResolvedWorkspaceTabLabel, TabNodeActivity,
    assign_idle_terminal_numbers, default_terminal_tab_title, resolve_terminal_tab_label,
    tab_is_active, tab_node_activity, workspace_parent_activity_slot,
};

fn grok_running() -> WorkspaceAgentActivity {
    WorkspaceAgentActivity {
        identity: WorkspaceAgentIdentity::Cli(CLIAgent::Grok),
        phase: WorkspaceAgentPhase::InProgress,
    }
}

fn oz_blocked() -> WorkspaceAgentActivity {
    WorkspaceAgentActivity {
        identity: WorkspaceAgentIdentity::Oz { ambient: false },
        phase: WorkspaceAgentPhase::Blocked,
    }
}

#[test]
fn tab_node_activity_prefers_agent_over_running_dot() {
    assert_eq!(
        tab_node_activity(Some(grok_running()), true),
        TabNodeActivity::Agent(grok_running())
    );
}

#[test]
fn tab_node_activity_falls_back_to_running_dot() {
    assert_eq!(tab_node_activity(None, true), TabNodeActivity::RunningDot);
}

#[test]
fn tab_node_activity_is_idle_when_empty() {
    assert_eq!(tab_node_activity(None, false), TabNodeActivity::Idle);
}

#[test]
fn expanded_parent_never_shows_activity_slot() {
    assert_eq!(
        workspace_parent_activity_slot(true, true),
        WorkspaceActivitySlot::Empty
    );
    assert_eq!(
        workspace_parent_activity_slot(true, false),
        WorkspaceActivitySlot::Empty
    );
}

#[test]
fn collapsed_parent_shows_running_dot_when_any_child_is_busy() {
    assert_eq!(
        workspace_parent_activity_slot(false, true),
        WorkspaceActivitySlot::RunningDot
    );
}

#[test]
fn collapsed_idle_parent_is_empty() {
    assert_eq!(
        workspace_parent_activity_slot(false, false),
        WorkspaceActivitySlot::Empty
    );
}

#[test]
fn custom_title_wins_over_command_and_cwd() {
    assert_eq!(
        resolve_terminal_tab_label(
            Some("Build"),
            true,
            "~/zap",
            Some("claude"),
            Some("conversation"),
            false,
            "~/zap",
            "~/zap",
            Some("cargo test"),
        ),
        ResolvedWorkspaceTabLabel::Named("Build".to_string())
    );
}

#[test]
fn non_terminal_tab_keeps_display_title() {
    assert_eq!(
        resolve_terminal_tab_label(
            None,
            false,
            "main.rs",
            None,
            None,
            false,
            "",
            "",
            Some("cargo test"),
        ),
        ResolvedWorkspaceTabLabel::Named("main.rs".to_string())
    );
}

#[test]
fn cli_agent_title_beats_last_command() {
    assert_eq!(
        resolve_terminal_tab_label(
            None,
            true,
            "~/zap",
            Some("fix the pager"),
            None,
            false,
            "~/zap",
            "~/zap",
            Some("cargo test"),
        ),
        ResolvedWorkspaceTabLabel::Named("fix the pager".to_string())
    );
}

#[test]
fn long_running_process_title_beats_last_command() {
    assert_eq!(
        resolve_terminal_tab_label(
            None,
            true,
            "~/zap",
            None,
            None,
            true,
            "nvim src/lib.rs",
            "~/zap",
            Some("ls"),
        ),
        ResolvedWorkspaceTabLabel::Named("nvim src/lib.rs".to_string())
    );
}

#[test]
fn last_command_replaces_cwd_title() {
    assert_eq!(
        resolve_terminal_tab_label(
            None,
            true,
            "~/zap",
            None,
            None,
            false,
            "~/zap",
            "~/zap",
            Some("cargo nextest run"),
        ),
        ResolvedWorkspaceTabLabel::Named("cargo nextest run".to_string())
    );
}

#[test]
fn idle_terminal_ignores_osc_path_title() {
    assert_eq!(
        resolve_terminal_tab_label(
            None,
            true,
            "feature-601-dump-sample",
            None,
            None,
            false,
            "feature-601-dump-sample",
            "~/.warp/worktrees/repo/feature-601-dump-sample",
            None,
        ),
        ResolvedWorkspaceTabLabel::IdleTerminal
    );
}

#[test]
fn idle_terminal_ignores_custom_title_that_is_cwd() {
    assert_eq!(
        resolve_terminal_tab_label(
            Some("~/.warp/worktrees/repo/feature-601-dump-sample"),
            true,
            "~/.warp/worktrees/repo/feature-601-dump-sample",
            None,
            None,
            false,
            "~/.warp/worktrees/repo/feature-601-dump-sample",
            "~/.warp/worktrees/repo/feature-601-dump-sample",
            Some("git status"),
        ),
        ResolvedWorkspaceTabLabel::Named("git status".to_string())
    );
}

#[test]
fn path_like_osc_title_is_treated_as_cwd() {
    assert_eq!(
        resolve_terminal_tab_label(
            None,
            true,
            "/Users/me/.warp/worktrees/repo/feature-601-dump-sample",
            None,
            None,
            false,
            "/Users/me/.warp/worktrees/repo/feature-601-dump-sample",
            "~/.warp/worktrees/repo/feature-601-dump-sample",
            Some("git status"),
        ),
        ResolvedWorkspaceTabLabel::Named("git status".to_string())
    );
    assert_eq!(
        resolve_terminal_tab_label(
            None,
            true,
            "~/.warp/worktrees/repo/feature-601-dump-sample",
            None,
            None,
            false,
            "feature-601-dump-sample",
            "~/.warp/worktrees/repo/feature-601-dump-sample",
            None,
        ),
        ResolvedWorkspaceTabLabel::IdleTerminal
    );
}

#[test]
fn host_prefixed_osc_path_is_treated_as_cwd() {
    assert_eq!(
        resolve_terminal_tab_label(
            None,
            true,
            "~/zap",
            None,
            None,
            false,
            "host: ~/zap",
            "~/zap",
            Some("git status"),
        ),
        ResolvedWorkspaceTabLabel::Named("git status".to_string())
    );
}

#[test]
fn trivial_commands_fall_through_to_idle_terminal() {
    assert_eq!(
        resolve_terminal_tab_label(
            None,
            true,
            "~/zap",
            None,
            None,
            false,
            "~/zap",
            "~/zap",
            Some("ls -la"),
        ),
        ResolvedWorkspaceTabLabel::IdleTerminal
    );
    assert_eq!(
        resolve_terminal_tab_label(
            None,
            true,
            "~/zap",
            None,
            None,
            false,
            "~/zap",
            "~/zap",
            Some("cd src"),
        ),
        ResolvedWorkspaceTabLabel::IdleTerminal
    );
}

#[test]
fn compound_command_starting_with_cd_is_kept() {
    assert_eq!(
        resolve_terminal_tab_label(
            None,
            true,
            "~/zap",
            None,
            None,
            false,
            "~/zap",
            "~/zap",
            Some("cd src && cargo test"),
        ),
        ResolvedWorkspaceTabLabel::Named("cd src && cargo test".to_string())
    );
}

#[test]
fn idle_terminals_are_numbered_in_workspace_order() {
    assert_eq!(
        assign_idle_terminal_numbers(vec![
            ResolvedWorkspaceTabLabel::IdleTerminal,
            ResolvedWorkspaceTabLabel::Named("cargo test".to_string()),
            ResolvedWorkspaceTabLabel::IdleTerminal,
        ]),
        vec![
            default_terminal_tab_title(1),
            "cargo test".to_string(),
            default_terminal_tab_title(2),
        ]
    );
}

#[test]
fn tab_is_active_only_for_current_workspace_active_index() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let workspace_b = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
    assert!(tab_is_active(workspace_a, 1, Some(workspace_a), 1));
    assert!(!tab_is_active(workspace_a, 0, Some(workspace_a), 1));
    assert!(!tab_is_active(workspace_b, 1, Some(workspace_a), 1));
    assert!(!tab_is_active(workspace_a, 1, None, 1));
}

#[test]
fn in_progress_tab_activity_should_breathe() {
    assert!(TabNodeActivity::Agent(grok_running()).should_breathe());
    assert!(!TabNodeActivity::Agent(oz_blocked()).should_breathe());
    assert!(!TabNodeActivity::RunningDot.should_breathe());
    assert!(!TabNodeActivity::Idle.should_breathe());
}

#[test]
fn tab_node_busy_covers_agent_and_running_dot() {
    assert!(TabNodeActivity::Agent(grok_running()).is_busy());
    assert!(TabNodeActivity::RunningDot.is_busy());
    assert!(!TabNodeActivity::Idle.is_busy());
}

#[test]
fn project_tree_tab_id_is_stable_entity_id() {
    let id = ProjectTreeTabId(EntityId::from_usize(42));
    let node = ProjectTreeTabNode {
        id,
        title: "agent".to_string(),
        activity: TabNodeActivity::Idle,
        is_active: true,
    };
    assert_eq!(node.id, ProjectTreeTabId(EntityId::from_usize(42)));
}
