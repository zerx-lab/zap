//! Codex 配置发现。

use std::fs;
use std::path::Path;

use super::util::*;
use crate::ai::agent_config::bridge::command_is_bridge_managed;
use crate::ai::agent_config::model::*;

pub fn scan_codex(ctx: &ScanContext) -> Vec<AgentConfigItem> {
    let mut out = Vec::new();
    push_file(
        &mut out,
        ctx.home_path(".codex/config.toml"),
        AgentKind::Codex,
        ConfigKind::Settings,
        ConfigScope::User,
        DocumentFormat::Toml,
        false,
        false,
    );
    if let Some(path) = ctx.home_path(".codex/hooks.json") {
        if path.exists() {
            out.extend(expand_codex_hooks(&path, ConfigScope::User, false));
        }
    }
    push_rules_file(
        &mut out,
        ctx.home_path(".codex/AGENTS.md"),
        AgentKind::Codex,
        ConfigScope::User,
        false,
    );
    push_rules_file(
        &mut out,
        ctx.repo_path("AGENTS.md"),
        AgentKind::Codex,
        ConfigScope::Project,
        true,
    );
    push_dir(
        &mut out,
        ctx.home_path(".codex/skills"),
        AgentKind::Codex,
        ConfigKind::Skills,
        ConfigScope::User,
        false,
        false,
    );
    push_dir(
        &mut out,
        ctx.home_path(".codex/hooks"),
        AgentKind::Codex,
        ConfigKind::Hooks,
        ConfigScope::User,
        false,
        true,
    );
    // MCP 在 config.toml 的 [mcp_servers]
    if let Some(path) = ctx.home_path(".codex/config.toml") {
        if path.exists() {
            out.push(item(
                AgentKind::Codex,
                ConfigKind::Mcp,
                ConfigScope::User,
                "mcp_servers",
                path.clone(),
                "mcp_servers".into(),
                DocumentFormat::Toml,
                true,
                ManagedBy::None,
                false,
                false,
                "config.toml › mcp_servers".into(),
                false,
            ));
        }
    }
    out
}

fn expand_codex_hooks(
    path: &Path,
    scope: ConfigScope,
    project_tracked: bool,
) -> Vec<AgentConfigItem> {
    let mut out = Vec::new();
    let raw = fs::read_to_string(path).unwrap_or_default();
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        out.push(item(
            AgentKind::Codex,
            ConfigKind::Hooks,
            scope,
            "hooks.json",
            path.to_path_buf(),
            String::new(),
            DocumentFormat::Json,
            true,
            ManagedBy::None,
            project_tracked,
            true,
            "无法解析".into(),
            false,
        ));
        return out;
    };
    out.push(item(
        AgentKind::Codex,
        ConfigKind::Hooks,
        scope,
        "hooks.json",
        path.to_path_buf(),
        String::new(),
        DocumentFormat::Json,
        true,
        ManagedBy::None,
        project_tracked,
        true,
        "整文件".into(),
        false,
    ));
    let hooks = value.get("hooks").and_then(|h| h.as_object());
    if let Some(hooks) = hooks {
        for (event, body) in hooks {
            let summary = summarize_hook_commands(body);
            let managed = if command_is_bridge_managed(&body.to_string()) {
                ManagedBy::Bridge
            } else {
                ManagedBy::None
            };
            out.push(item(
                AgentKind::Codex,
                ConfigKind::Hooks,
                scope,
                event,
                path.to_path_buf(),
                format!("/hooks/{event}"),
                DocumentFormat::Json,
                true,
                managed,
                project_tracked,
                true,
                truncate(&summary, 100),
                false,
            ));
        }
    }
    out
}
