//! 各 CLI agent 的路径表与扫描。

mod claude;
mod codex;
mod cursor;
mod gemini;
mod grok;
mod util;

pub use claude::scan_claude;
pub use codex::scan_codex;
pub use cursor::scan_cursor;
pub use gemini::scan_gemini;
pub use grok::scan_grok;

use super::model::{AgentConfigItem, AgentKind, ConfigKind, ConfigScope, DocumentFormat, ManagedBy, ScanContext};
use std::fs;

pub fn scan_all(ctx: &ScanContext) -> Vec<AgentConfigItem> {
    let mut items = Vec::new();
    items.extend(scan_claude(ctx));
    items.extend(scan_codex(ctx));
    items.extend(scan_grok(ctx));
    items.extend(scan_agents_shared(ctx));
    items.extend(scan_gemini(ctx));
    items.extend(scan_cursor(ctx));
    items
}

fn scan_agents_shared(ctx: &ScanContext) -> Vec<AgentConfigItem> {
    let mut out = Vec::new();
    if let Some(dir) = ctx.home_path(".agents/skills") {
        if dir.is_dir() {
            let count = fs::read_dir(&dir).map(|rd| rd.count()).unwrap_or(0);
            out.push(AgentConfigItem {
                id: AgentConfigItem::new_id(
                    AgentKind::Agents,
                    ConfigKind::Skills,
                    ConfigScope::User,
                    "skills",
                ),
                agent: AgentKind::Agents,
                kind: ConfigKind::Skills,
                scope: ConfigScope::User,
                title: format!("skills ({count})"),
                source_path: dir,
                pointer: String::new(),
                format: DocumentFormat::Directory,
                executable: false,
                managed_by: ManagedBy::None,
                project_tracked: false,
                needs_codex_retrust: false,
                summary: "共享技能扇出源".into(),
                read_only: false,
            });
        }
    }
    if let Some(path) = ctx.home_path(".agents/.mcp.json") {
        if path.exists() {
            out.push(AgentConfigItem {
                id: AgentConfigItem::new_id(
                    AgentKind::Agents,
                    ConfigKind::Mcp,
                    ConfigScope::User,
                    ".mcp.json",
                ),
                agent: AgentKind::Agents,
                kind: ConfigKind::Mcp,
                scope: ConfigScope::User,
                title: ".mcp.json".into(),
                source_path: path,
                pointer: String::new(),
                format: DocumentFormat::Json,
                executable: true,
                managed_by: ManagedBy::None,
                project_tracked: false,
                needs_codex_retrust: false,
                summary: "共享 MCP".into(),
                read_only: false,
            });
        }
    }
    out
}

pub fn list_skill_names(dir: &std::path::Path) -> Vec<String> {
    let Ok(rd) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = rd
        .flatten()
        .filter(|e| {
            e.file_type()
                .map(|t| t.is_dir() || t.is_symlink())
                .unwrap_or(false)
        })
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with('.'))
        .collect();
    names.sort();
    names
}
