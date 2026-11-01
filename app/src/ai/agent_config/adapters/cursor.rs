//! Cursor — 只读展示。

use super::util::*;
use crate::ai::agent_config::model::*;

pub fn scan_cursor(ctx: &ScanContext) -> Vec<AgentConfigItem> {
    let mut out = Vec::new();
    push_file(
        &mut out,
        ctx.home_path(".cursor/hooks.json"),
        AgentKind::Cursor,
        ConfigKind::Hooks,
        ConfigScope::User,
        DocumentFormat::Json,
        true,
        false,
    );
    push_file(
        &mut out,
        ctx.home_path(".cursor/mcp.json"),
        AgentKind::Cursor,
        ConfigKind::Mcp,
        ConfigScope::User,
        DocumentFormat::Json,
        true,
        false,
    );
    push_dir(
        &mut out,
        ctx.home_path(".cursor/skills"),
        AgentKind::Cursor,
        ConfigKind::Skills,
        ConfigScope::User,
        false,
        false,
    );
    for item in &mut out {
        item.read_only = true;
    }
    out
}
