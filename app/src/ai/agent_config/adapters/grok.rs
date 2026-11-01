//! Grok 配置发现。

use std::fs;

use super::util::*;
use crate::ai::agent_config::bridge::command_is_bridge_managed;
use crate::ai::agent_config::model::*;

pub fn scan_grok(ctx: &ScanContext) -> Vec<AgentConfigItem> {
    let mut out = Vec::new();
    push_file(
        &mut out,
        ctx.home_path(".grok/config.toml"),
        AgentKind::Grok,
        ConfigKind::Settings,
        ConfigScope::User,
        DocumentFormat::Toml,
        false,
        false,
    );
    push_file(
        &mut out,
        ctx.repo_path(".grok/config.toml"),
        AgentKind::Grok,
        ConfigKind::Settings,
        ConfigScope::Project,
        DocumentFormat::Toml,
        false,
        true,
    );
    push_rules_file(
        &mut out,
        ctx.home_path(".grok/AGENTS.md"),
        AgentKind::Grok,
        ConfigScope::User,
        false,
    );
    push_rules_file(
        &mut out,
        ctx.repo_path("AGENTS.md"),
        AgentKind::Grok,
        ConfigScope::Project,
        true,
    );
    push_dir(
        &mut out,
        ctx.home_path(".grok/rules"),
        AgentKind::Grok,
        ConfigKind::Rules,
        ConfigScope::User,
        false,
        false,
    );
    push_dir(
        &mut out,
        ctx.home_path(".grok/skills"),
        AgentKind::Grok,
        ConfigKind::Skills,
        ConfigScope::User,
        false,
        false,
    );
    push_dir(
        &mut out,
        ctx.repo_path(".grok/skills"),
        AgentKind::Grok,
        ConfigKind::Skills,
        ConfigScope::Project,
        true,
        false,
    );
    push_dir(
        &mut out,
        ctx.home_path(".grok/hooks"),
        AgentKind::Grok,
        ConfigKind::Hooks,
        ConfigScope::User,
        false,
        true,
    );
    push_dir(
        &mut out,
        ctx.repo_path(".grok/hooks"),
        AgentKind::Grok,
        ConfigKind::Hooks,
        ConfigScope::Project,
        true,
        true,
    );

    // status_line / hooks from config.toml
    if let Some(path) = ctx.home_path(".grok/config.toml") {
        if path.exists() {
            if let Ok(raw) = fs::read_to_string(&path) {
                if let Ok(doc) = raw.parse::<toml_edit::DocumentMut>() {
                    if let Some(cmd) = doc
                        .get("ui")
                        .and_then(|u| u.get("status_line"))
                        .and_then(|s| s.get("command"))
                        .and_then(|c| c.as_str())
                    {
                        out.push(item(
                            AgentKind::Grok,
                            ConfigKind::Hooks,
                            ConfigScope::User,
                            "status_line",
                            path.clone(),
                            "ui.status_line".into(),
                            DocumentFormat::Toml,
                            true,
                            managed_from_text(cmd),
                            false,
                            false,
                            truncate(cmd, 80),
                            false,
                        ));
                    }
                    // MCP
                    if doc.get("mcp_servers").is_some() {
                        out.push(item(
                            AgentKind::Grok,
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
                    let _ = command_is_bridge_managed;
                }
            }
        }
    }
    out
}
