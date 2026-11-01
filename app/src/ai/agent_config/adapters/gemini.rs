//! Gemini — 只读展示。

use super::util::*;
use crate::ai::agent_config::model::*;

pub fn scan_gemini(ctx: &ScanContext) -> Vec<AgentConfigItem> {
    let mut out = Vec::new();
    push_file(
        &mut out,
        ctx.home_path(".gemini/settings.json"),
        AgentKind::Gemini,
        ConfigKind::Settings,
        ConfigScope::User,
        DocumentFormat::Json,
        false,
        false,
    );
    push_rules_file(
        &mut out,
        ctx.home_path(".gemini/GEMINI.md"),
        AgentKind::Gemini,
        ConfigScope::User,
        false,
    );
    push_dir(
        &mut out,
        ctx.home_path(".gemini/skills"),
        AgentKind::Gemini,
        ConfigKind::Skills,
        ConfigScope::User,
        false,
        false,
    );
    // 强制只读
    for item in &mut out {
        item.read_only = true;
    }
    out
}
