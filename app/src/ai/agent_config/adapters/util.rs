use std::fs;
use std::path::{Path, PathBuf};

use crate::ai::agent_config::bridge::command_is_bridge_managed;
use crate::ai::agent_config::model::*;

pub fn truncate(s: &str, n: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= n {
        t.to_string()
    } else {
        let cut: String = t.chars().take(n.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

pub fn item(
    agent: AgentKind,
    kind: ConfigKind,
    scope: ConfigScope,
    title: &str,
    source_path: PathBuf,
    pointer: String,
    format: DocumentFormat,
    executable: bool,
    managed_by: ManagedBy,
    project_tracked: bool,
    needs_codex_retrust: bool,
    summary: String,
    read_only: bool,
) -> AgentConfigItem {
    AgentConfigItem {
        id: AgentConfigItem::new_id(agent, kind, scope, title),
        agent,
        kind,
        scope,
        title: title.to_string(),
        source_path,
        pointer,
        format,
        executable,
        managed_by,
        project_tracked,
        needs_codex_retrust,
        summary,
        read_only,
    }
}

pub fn push_rules_file(
    out: &mut Vec<AgentConfigItem>,
    path: Option<PathBuf>,
    agent: AgentKind,
    scope: ConfigScope,
    project_tracked: bool,
) {
    let Some(path) = path else { return };
    if !path.exists() {
        return;
    }
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("rules")
        .to_string();
    out.push(item(
        agent,
        ConfigKind::Rules,
        scope,
        &name,
        path,
        String::new(),
        DocumentFormat::Markdown,
        false,
        ManagedBy::None,
        project_tracked,
        false,
        "规则文件".into(),
        !agent.is_editable(),
    ));
}

pub fn push_dir(
    out: &mut Vec<AgentConfigItem>,
    path: Option<PathBuf>,
    agent: AgentKind,
    kind: ConfigKind,
    scope: ConfigScope,
    project_tracked: bool,
    executable: bool,
) {
    let Some(path) = path else { return };
    if !path.is_dir() {
        return;
    }
    let count = fs::read_dir(&path).map(|rd| rd.count()).unwrap_or(0);
    let title = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("dir");
    out.push(item(
        agent,
        kind,
        scope,
        &format!("{title} ({count})"),
        path,
        String::new(),
        DocumentFormat::Directory,
        executable,
        ManagedBy::None,
        project_tracked,
        false,
        format!("{count} 项"),
        !agent.is_editable(),
    ));
}

pub fn push_file(
    out: &mut Vec<AgentConfigItem>,
    path: Option<PathBuf>,
    agent: AgentKind,
    kind: ConfigKind,
    scope: ConfigScope,
    format: DocumentFormat,
    executable: bool,
    project_tracked: bool,
) {
    let Some(path) = path else { return };
    if !path.exists() {
        return;
    }
    let title = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file")
        .to_string();
    out.push(item(
        agent,
        kind,
        scope,
        &title,
        path,
        String::new(),
        format,
        executable,
        ManagedBy::None,
        project_tracked,
        false,
        String::new(),
        !agent.is_editable(),
    ));
}

pub fn collect_commands(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(cmd)) = map.get("command") {
                out.push(truncate(cmd, 60));
            }
            for v in map.values() {
                collect_commands(v, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_commands(v, out);
            }
        }
        _ => {}
    }
}

pub fn summarize_hook_commands(body: &serde_json::Value) -> String {
    let mut cmds = Vec::new();
    collect_commands(body, &mut cmds);
    if cmds.is_empty() {
        "—".into()
    } else {
        cmds.join(" · ")
    }
}

pub fn managed_from_text(text: &str) -> ManagedBy {
    if command_is_bridge_managed(text) {
        ManagedBy::Bridge
    } else {
        ManagedBy::None
    }
}

pub fn path_exists(path: &Path) -> bool {
    path.exists()
}
