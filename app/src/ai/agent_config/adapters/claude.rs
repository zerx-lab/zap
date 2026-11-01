//! Claude Code 配置发现。

use std::fs;
use std::path::Path;

use super::util::*;
use crate::ai::agent_config::bridge::command_is_bridge_managed;
use crate::ai::agent_config::model::*;

pub fn scan_claude(ctx: &ScanContext) -> Vec<AgentConfigItem> {
    let mut out = Vec::new();
    if let Some(path) = ctx.home_path(".claude/settings.json") {
        if path.exists() {
            out.extend(expand_claude_settings(&path, ConfigScope::User, false));
        }
    }
    if let Some(path) = ctx.home_path(".claude/settings.local.json") {
        if path.exists() {
            out.extend(expand_claude_settings(&path, ConfigScope::Local, false));
        }
    }
    if let Some(path) = ctx.repo_path(".claude/settings.json") {
        if path.exists() {
            out.extend(expand_claude_settings(&path, ConfigScope::Project, true));
        }
    }
    if let Some(path) = ctx.repo_path(".claude/settings.local.json") {
        if path.exists() {
            out.extend(expand_claude_settings(&path, ConfigScope::Local, true));
        }
    }

    push_rules_file(
        &mut out,
        ctx.home_path(".claude/CLAUDE.md"),
        AgentKind::Claude,
        ConfigScope::User,
        false,
    );
    push_rules_file(
        &mut out,
        ctx.home_path(".claude/claude.md"),
        AgentKind::Claude,
        ConfigScope::User,
        false,
    );
    push_rules_file(
        &mut out,
        ctx.repo_path("CLAUDE.md"),
        AgentKind::Claude,
        ConfigScope::Project,
        true,
    );
    push_rules_file(
        &mut out,
        ctx.repo_path("AGENTS.md"),
        AgentKind::Claude,
        ConfigScope::Project,
        true,
    );

    push_dir(
        &mut out,
        ctx.home_path(".claude/skills"),
        AgentKind::Claude,
        ConfigKind::Skills,
        ConfigScope::User,
        false,
        false,
    );
    push_dir(
        &mut out,
        ctx.repo_path(".claude/skills"),
        AgentKind::Claude,
        ConfigKind::Skills,
        ConfigScope::Project,
        true,
        false,
    );
    push_dir(
        &mut out,
        ctx.home_path(".claude/commands"),
        AgentKind::Claude,
        ConfigKind::Commands,
        ConfigScope::User,
        false,
        false,
    );
    push_dir(
        &mut out,
        ctx.home_path(".claude/hooks"),
        AgentKind::Claude,
        ConfigKind::Hooks,
        ConfigScope::User,
        false,
        true,
    );

    push_file(
        &mut out,
        ctx.home_path(".claude.json"),
        AgentKind::Claude,
        ConfigKind::Mcp,
        ConfigScope::User,
        DocumentFormat::Json,
        true,
        false,
    );
    push_file(
        &mut out,
        ctx.repo_path(".mcp.json"),
        AgentKind::Claude,
        ConfigKind::Mcp,
        ConfigScope::Project,
        DocumentFormat::Json,
        true,
        true,
    );

    out
}

fn expand_claude_settings(
    path: &Path,
    scope: ConfigScope,
    project_tracked: bool,
) -> Vec<AgentConfigItem> {
    let mut out = Vec::new();
    let raw = fs::read_to_string(path).unwrap_or_default();
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        out.push(item(
            AgentKind::Claude,
            ConfigKind::Settings,
            scope,
            "settings.json",
            path.to_path_buf(),
            String::new(),
            DocumentFormat::Json,
            false,
            ManagedBy::None,
            project_tracked,
            false,
            "无法解析 JSON".into(),
            false,
        ));
        return out;
    };
    let key_count = value.as_object().map(|o| o.len()).unwrap_or(0);
    out.push(item(
        AgentKind::Claude,
        ConfigKind::Settings,
        scope,
        "settings.json",
        path.to_path_buf(),
        String::new(),
        DocumentFormat::Json,
        false,
        ManagedBy::None,
        project_tracked,
        false,
        format!("{key_count} keys"),
        false,
    ));
    if let Some(cmd) = value.pointer("/statusLine/command").and_then(|v| v.as_str()) {
        out.push(item(
            AgentKind::Claude,
            ConfigKind::Hooks,
            scope,
            "statusLine",
            path.to_path_buf(),
            "/statusLine".into(),
            DocumentFormat::Json,
            true,
            managed_from_text(cmd),
            project_tracked,
            false,
            truncate(cmd, 80),
            false,
        ));
    }
    if let Some(hooks) = value.get("hooks").and_then(|h| h.as_object()) {
        for (event, body) in hooks {
            let summary = summarize_hook_commands(body);
            let managed = if command_is_bridge_managed(&body.to_string()) {
                ManagedBy::Bridge
            } else {
                ManagedBy::None
            };
            out.push(item(
                AgentKind::Claude,
                ConfigKind::Hooks,
                scope,
                event,
                path.to_path_buf(),
                format!("/hooks/{event}"),
                DocumentFormat::Json,
                true,
                managed,
                project_tracked,
                false,
                truncate(&summary, 100),
                false,
            ));
        }
    }
    out
}
