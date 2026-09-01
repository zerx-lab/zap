use warpui::EntityId;

use crate::project_organization::domain::RepositoryWorkspaceId;
use crate::project_organization::workspace_agent_activity::{
    WorkspaceActivitySlot, WorkspaceAgentActivity, workspace_activity_slot,
};

/// 树内页签节点的稳定身份,对应所属 PaneGroup。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ProjectTreeTabId(pub EntityId);

/// 页签子节点左侧活动槽。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TabNodeActivity {
    Agent(WorkspaceAgentActivity),
    RunningDot,
    Idle,
}

/// 某个 workspace 下一行页签的展示数据。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectTreeTabNode {
    pub id: ProjectTreeTabId,
    pub title: String,
    pub activity: TabNodeActivity,
    pub is_active: bool,
}

/// 将单页签的 agent / 长任务判定压成树节点活动槽。
pub(crate) fn tab_node_activity(
    agent: Option<WorkspaceAgentActivity>,
    has_running_terminal: bool,
) -> TabNodeActivity {
    match workspace_activity_slot(agent, has_running_terminal) {
        WorkspaceActivitySlot::Agent(activity) => TabNodeActivity::Agent(activity),
        WorkspaceActivitySlot::RunningDot => TabNodeActivity::RunningDot,
        WorkspaceActivitySlot::Empty => TabNodeActivity::Idle,
    }
}

/// 折叠父节点只用通用绿点;展开后活动落在子节点上,父节点永不画 agent 头像。
pub(crate) fn workspace_parent_activity_slot(
    expanded: bool,
    any_child_busy: bool,
) -> WorkspaceActivitySlot {
    if expanded || !any_child_busy {
        WorkspaceActivitySlot::Empty
    } else {
        WorkspaceActivitySlot::RunningDot
    }
}

/// 当前活动页签只属于当前活动 workspace;后台 workspace 的上次活动页签不高亮。
pub(crate) fn tab_is_active(
    tab_workspace_id: RepositoryWorkspaceId,
    tab_index: usize,
    active_workspace_id: Option<RepositoryWorkspaceId>,
    active_tab_index: usize,
) -> bool {
    active_workspace_id == Some(tab_workspace_id) && tab_index == active_tab_index
}

/// repository workspace 页签标题在编号前的解析结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ResolvedWorkspaceTabLabel {
    Named(String),
    IdleTerminal,
}

/// 解析 terminal 页签在树 / TabBar 上应显示的标题。
/// 自定义名永远优先;非 terminal 页签沿用现有 display_title。
pub(crate) fn resolve_terminal_tab_label(
    custom_title: Option<&str>,
    is_focused_terminal: bool,
    fallback_display_title: &str,
    cli_agent_title: Option<&str>,
    conversation_title: Option<&str>,
    is_long_running: bool,
    osc_title: &str,
    working_directory: &str,
    last_completed_command: Option<&str>,
) -> ResolvedWorkspaceTabLabel {
    let working_directory = working_directory.trim();
    if let Some(custom) = custom_title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .filter(|title| !title_is_working_directory(title, working_directory))
    {
        return ResolvedWorkspaceTabLabel::Named(custom.to_owned());
    }
    if !is_focused_terminal {
        let fallback = fallback_display_title.trim();
        if fallback.is_empty() || title_is_working_directory(fallback, working_directory) {
            return ResolvedWorkspaceTabLabel::IdleTerminal;
        }
        return ResolvedWorkspaceTabLabel::Named(fallback.to_owned());
    }
    if let Some(title) = nonempty_title(cli_agent_title)
        .filter(|title| !title_is_working_directory(title, working_directory))
    {
        return ResolvedWorkspaceTabLabel::Named(title.to_owned());
    }

    let osc_title = osc_title.trim();
    // 空闲时 OSC 几乎总是 cwd(绝对路径 / ~/... / 末级目录名),不能当页签名。
    // 只有长任务且标题明显不是目录时才用,例如 nvim、htop。
    if is_long_running
        && !osc_title.is_empty()
        && !title_is_working_directory(osc_title, working_directory)
    {
        return ResolvedWorkspaceTabLabel::Named(osc_title.to_owned());
    }
    if let Some(title) = nonempty_title(conversation_title)
        .filter(|title| !title_is_working_directory(title, working_directory))
    {
        return ResolvedWorkspaceTabLabel::Named(title.to_owned());
    }
    if let Some(command) = last_completed_command.and_then(meaningful_command) {
        return ResolvedWorkspaceTabLabel::Named(command.to_owned());
    }
    ResolvedWorkspaceTabLabel::IdleTerminal
}

fn title_is_working_directory(title: &str, working_directory: &str) -> bool {
    osc_title_represents_working_directory(title, working_directory)
        || title.starts_with('/')
        || title.starts_with("~/")
        || title.starts_with("./")
}

/// Shell 常把 OSC 标题设成 cwd 的绝对路径、`~/...` 或最后一级目录名,
/// 和 display_working_directory 的字符串并不相等,不能当成进程名。
fn osc_title_represents_working_directory(osc_title: &str, working_directory: &str) -> bool {
    let osc_title = osc_title.trim().trim_end_matches('/');
    if osc_title.is_empty() {
        return true;
    }
    let working_directory = working_directory.trim().trim_end_matches('/');
    let osc_title = strip_shell_title_host_prefix(osc_title);
    if working_directory.is_empty() {
        return osc_title.starts_with('/') || osc_title.starts_with("~/");
    }
    if osc_title == working_directory {
        return true;
    }
    let cwd_base = last_path_component(working_directory);
    if osc_title == cwd_base {
        return true;
    }
    let cwd_tail = path_suffix(working_directory, 2);
    !cwd_tail.is_empty()
        && cwd_tail.contains('/')
        && (path_suffix(osc_title, 2) == cwd_tail || osc_title.ends_with(&format!("/{cwd_tail}")))
}

fn strip_shell_title_host_prefix(osc_title: &str) -> &str {
    if let Some((host, rest)) = osc_title.split_once(": ") {
        if !host.contains('/') {
            return rest.trim();
        }
    }
    if let Some((host, rest)) = osc_title.split_once(':') {
        if !host.contains('/') && (rest.starts_with('/') || rest.starts_with('~')) {
            return rest.trim();
        }
    }
    osc_title
}

fn last_path_component(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(path)
}

fn path_suffix(path: &str, component_count: usize) -> String {
    path.split('/')
        .filter(|part| !part.is_empty() && *part != "~")
        .rev()
        .take(component_count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("/")
}

/// 空闲 terminal 按当前 workspace 从左到右编号为 Terminal 1、Terminal 2。
pub(crate) fn assign_idle_terminal_numbers(labels: Vec<ResolvedWorkspaceTabLabel>) -> Vec<String> {
    let mut next_number = 1usize;
    labels
        .into_iter()
        .map(|label| match label {
            ResolvedWorkspaceTabLabel::Named(name) => name,
            ResolvedWorkspaceTabLabel::IdleTerminal => {
                let title = default_terminal_tab_title(next_number);
                next_number += 1;
                title
            }
        })
        .collect()
}

pub(crate) fn default_terminal_tab_title(number: usize) -> String {
    crate::t!("repository-workspace-terminal-tab-title", number = number)
}

fn nonempty_title(title: Option<&str>) -> Option<&str> {
    title.map(str::trim).filter(|title| !title.is_empty())
}

fn meaningful_command(command: &str) -> Option<&str> {
    let trimmed = command.trim();
    if trimmed.is_empty() || is_trivial_shell_command(trimmed) {
        None
    } else {
        Some(trimmed)
    }
}

fn is_trivial_shell_command(command: &str) -> bool {
    if command.contains("&&") || command.contains("||") {
        return false;
    }
    matches!(
        first_shell_token(command),
        Some("cd" | "ls" | "ll" | "la" | "pwd" | "clear" | "reset" | "exit" | "true")
    )
}

fn first_shell_token(command: &str) -> Option<&str> {
    let mut rest = command.trim_start();
    loop {
        let token = rest.split_whitespace().next()?;
        if is_env_assignment(token) {
            rest = rest[token.len()..].trim_start();
            continue;
        }
        return Some(token);
    }
}

fn is_env_assignment(token: &str) -> bool {
    let Some(eq) = token.find('=') else {
        return false;
    };
    eq > 0 && !token[..eq].contains(['/', '\\'])
}

impl TabNodeActivity {
    pub(crate) fn is_busy(self) -> bool {
        match self {
            Self::Agent(_) | Self::RunningDot => true,
            Self::Idle => false,
        }
    }

    pub(crate) fn agent(self) -> Option<WorkspaceAgentActivity> {
        match self {
            Self::Agent(activity) => Some(activity),
            Self::RunningDot | Self::Idle => None,
        }
    }

    pub(crate) fn should_breathe(self) -> bool {
        self.agent()
            .is_some_and(WorkspaceAgentActivity::should_breathe)
    }
}

#[cfg(test)]
#[path = "project_tree_tab_tests.rs"]
mod tests;
