//! 轻量 schema / 结构校验。不做完整 JSON Schema，只拦明显坏文件。

use anyhow::{anyhow, Result};
use serde_json::Value as JsonValue;

use super::model::{AgentKind, ConfigKind, DocumentFormat};

const CLAUDE_HOOK_EVENTS: &[&str] = &[
    "Notification",
    "PermissionRequest",
    "PostToolUse",
    "PostToolUseFailure",
    "PreCompact",
    "PreToolUse",
    "SessionEnd",
    "SessionStart",
    "Stop",
    "UserPromptSubmit",
];

const CODEX_HOOK_EVENTS: &[&str] = &[
    "Interrupt",
    "PermissionRequest",
    "PostToolUse",
    "PreToolUse",
    "SessionStart",
    "Stop",
    "SubagentStart",
    "SubagentStop",
    "UserPromptSubmit",
];

/// 校验即将写入的内容。
pub fn validate_before_write(
    agent: AgentKind,
    kind: ConfigKind,
    format: DocumentFormat,
    contents: &str,
) -> Result<()> {
    match format {
        DocumentFormat::Json => validate_json(agent, kind, contents),
        DocumentFormat::Toml => validate_toml(agent, kind, contents),
        DocumentFormat::Markdown => {
            if contents.len() > 500_000 {
                return Err(anyhow!("Markdown 超过 500KB"));
            }
            Ok(())
        }
        DocumentFormat::Directory => Err(anyhow!("目录条目不可整文件写入")),
    }
}

fn validate_json(agent: AgentKind, kind: ConfigKind, contents: &str) -> Result<()> {
    let value: JsonValue =
        serde_json::from_str(contents).map_err(|e| anyhow!("JSON 语法错误: {e}"))?;
    match (agent, kind) {
        (AgentKind::Claude, ConfigKind::Settings | ConfigKind::Hooks) => {
            if let Some(hooks) = value.get("hooks") {
                validate_hook_object(hooks, CLAUDE_HOOK_EVENTS)?;
            }
            Ok(())
        }
        (AgentKind::Codex, ConfigKind::Hooks) => {
            if let Some(hooks) = value.get("hooks").or(Some(&value)) {
                if hooks.get("hooks").is_some() {
                    validate_hook_object(&hooks["hooks"], CODEX_HOOK_EVENTS)?;
                } else if hooks.is_object() {
                    validate_hook_object(hooks, CODEX_HOOK_EVENTS)?;
                }
            }
            Ok(())
        }
        (_, ConfigKind::Mcp) => {
            if value.get("mcpServers").is_none()
                && value.get("mcp_servers").is_none()
                && !value.is_object()
            {
                return Err(anyhow!("MCP JSON 需要对象根"));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_hook_object(hooks: &JsonValue, allowed: &[&str]) -> Result<()> {
    let Some(obj) = hooks.as_object() else {
        return Err(anyhow!("hooks 必须是对象"));
    };
    for key in obj.keys() {
        if !allowed.iter().any(|a| a == key) {
            // 未知事件名只警告式放行——新版本 CLI 可能加事件。
            log::warn!("未知 hook 事件名: {key}");
        }
    }
    Ok(())
}

fn validate_toml(agent: AgentKind, kind: ConfigKind, contents: &str) -> Result<()> {
    contents
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| anyhow!("TOML 语法错误: {e}"))?;
    let _ = (agent, kind);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_json() {
        let err = validate_before_write(
            AgentKind::Claude,
            ConfigKind::Settings,
            DocumentFormat::Json,
            "{",
        );
        assert!(err.is_err());
    }

    #[test]
    fn accepts_claude_settings() {
        validate_before_write(
            AgentKind::Claude,
            ConfigKind::Settings,
            DocumentFormat::Json,
            r#"{"model":"x","hooks":{"Stop":[]}}"#,
        )
        .unwrap();
    }
}
