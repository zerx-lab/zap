use crate::ai::agent::conversation::ConversationStatus;
use crate::terminal::cli_agent_sessions::CLIAgentSessionStatus;
use crate::terminal::CLIAgent;

/// workspace 行活动槽要展示的 agent 身份。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceAgentIdentity {
    Cli(CLIAgent),
    Oz { ambient: bool },
}

/// 计入活动槽的会话阶段。结束 / 出错不进入此枚举。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceAgentPhase {
    InProgress,
    Blocked,
}

/// 某个 workspace 当前应展示的 agent 活动。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceAgentActivity {
    pub identity: WorkspaceAgentIdentity,
    pub phase: WorkspaceAgentPhase,
}

/// workspace 行左侧活动槽: 头像、绿点、空槽互斥。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceActivitySlot {
    Empty,
    RunningDot,
    Agent(WorkspaceAgentActivity),
}

/// 同一 workspace 内多个命中时取扫描顺序中最后一个。
pub(crate) fn last_agent_activity(
    activities: impl IntoIterator<Item = WorkspaceAgentActivity>,
) -> Option<WorkspaceAgentActivity> {
    activities.into_iter().last()
}

/// 有 agent 活动时只占头像槽; 否则回退绿点或空槽。
pub(crate) fn workspace_activity_slot(
    agent: Option<WorkspaceAgentActivity>,
    has_running_terminal: bool,
) -> WorkspaceActivitySlot {
    match agent {
        Some(activity) => WorkspaceActivitySlot::Agent(activity),
        None if has_running_terminal => WorkspaceActivitySlot::RunningDot,
        None => WorkspaceActivitySlot::Empty,
    }
}

impl WorkspaceAgentActivity {
    /// InProgress 需要呼吸环; Blocked 为静态环。
    pub(crate) fn should_breathe(self) -> bool {
        match self.phase {
            WorkspaceAgentPhase::InProgress => true,
            WorkspaceAgentPhase::Blocked => false,
        }
    }
}

pub(crate) struct OzConversationSource {
    pub status: ConversationStatus,
    pub is_empty: bool,
    pub is_entirely_passive: bool,
    pub is_ambient: bool,
}

/// 同一 terminal 上的候选顺序: ambient → Oz → CLI。last_agent_activity 因此让 CLI 覆盖 Oz。
pub(crate) fn activities_from_terminal_sources(
    cli: Option<(CLIAgent, CLIAgentSessionStatus)>,
    oz: Option<OzConversationSource>,
    ambient_in_progress: bool,
) -> Vec<WorkspaceAgentActivity> {
    let mut activities = Vec::new();

    if ambient_in_progress {
        activities.push(WorkspaceAgentActivity {
            identity: WorkspaceAgentIdentity::Oz { ambient: true },
            phase: WorkspaceAgentPhase::InProgress,
        });
    }

    if let Some(oz) = oz {
        if !oz.is_empty && !oz.is_entirely_passive {
            if let Some(phase) = phase_from_conversation_status(&oz.status) {
                activities.push(WorkspaceAgentActivity {
                    identity: WorkspaceAgentIdentity::Oz {
                        ambient: oz.is_ambient,
                    },
                    phase,
                });
            }
        }
    }

    if let Some((agent, status)) = cli {
        if let Some(phase) = phase_from_cli_status(&status) {
            activities.push(WorkspaceAgentActivity {
                identity: WorkspaceAgentIdentity::Cli(agent),
                phase,
            });
        }
    }

    activities
}

fn phase_from_cli_status(status: &CLIAgentSessionStatus) -> Option<WorkspaceAgentPhase> {
    match status {
        CLIAgentSessionStatus::InProgress => Some(WorkspaceAgentPhase::InProgress),
        CLIAgentSessionStatus::Blocked { .. } => Some(WorkspaceAgentPhase::Blocked),
        CLIAgentSessionStatus::Success => None,
    }
}

fn phase_from_conversation_status(status: &ConversationStatus) -> Option<WorkspaceAgentPhase> {
    match status {
        ConversationStatus::InProgress => Some(WorkspaceAgentPhase::InProgress),
        ConversationStatus::Blocked { .. } => Some(WorkspaceAgentPhase::Blocked),
        ConversationStatus::Success | ConversationStatus::Error | ConversationStatus::Cancelled => {
            None
        }
    }
}

#[cfg(test)]
#[path = "workspace_agent_activity_tests.rs"]
mod tests;
