//! Agent 配置管理：发现、校验、备份、写入、bridge 修复。
//!
//! Settings › Agents › 配置管理 与命令面板共用此模块。

pub mod adapters;
pub mod backup;
pub mod bridge;
pub mod document;
pub mod model;
pub mod schema;

pub use adapters::scan_all;
pub use backup::{create_backup, list_backups, restore_backup, restore_latest, BackupFileEntry, BackupManifest, DEFAULT_KEEP};
pub use bridge::{command_is_bridge_managed, health_check_bridge, run_bridge_install};
pub use model::*;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use document::{
    apply_json_pointer_replace, apply_text_replace, apply_toml_replace, atomic_write,
    line_diff, read_document, redact_for_display,
};
use schema::validate_before_write;

/// 扫描当前环境的全部配置条目。
pub fn discover(repo_root: Option<PathBuf>) -> Vec<AgentConfigItem> {
    let ctx = ScanContext::from_env(repo_root);
    adapters::scan_all(&ctx)
}

/// 读条目内容（展示用，已打码）。
pub fn read_item_display(item: &AgentConfigItem) -> Result<String> {
    if item.format == DocumentFormat::Directory {
        let names = adapters::list_skill_names(&item.source_path);
        return Ok(names.join("\n"));
    }
    let raw = read_document(&item.source_path, item.format)?;
    if item.pointer.is_empty() {
        return Ok(redact_for_display(&raw, item.format));
    }
    // pointer：抽出片段再打码
    match item.format {
        DocumentFormat::Json => {
            let value: serde_json::Value = serde_json::from_str(&raw)?;
            let frag = value
                .pointer(&item.pointer)
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let text = serde_json::to_string_pretty(&frag)?;
            Ok(redact_for_display(&text, DocumentFormat::Json))
        }
        DocumentFormat::Toml => Ok(redact_for_display(&raw, DocumentFormat::Toml)),
        other => Ok(redact_for_display(&raw, other)),
    }
}

/// 读条目原始内容（写入前对比用，仍打码敏感字段）。
pub fn read_item_raw_redacted(item: &AgentConfigItem) -> Result<String> {
    read_item_display(item)
}

/// 预览写入后的 diff。`new_contents` 是 pointer 片段或整文件。
pub fn preview_write(item: &AgentConfigItem, new_contents: &str) -> Result<(String, usize, usize)> {
    if item.read_only {
        return Err(anyhow!("只读条目，不可编辑"));
    }
    if item.format == DocumentFormat::Directory {
        return Err(anyhow!("目录条目请在文件管理器或 Skill 管理器中操作"));
    }
    let before_full = read_document(&item.source_path, item.format)?;
    let after_full = match item.format {
        DocumentFormat::Json => {
            apply_json_pointer_replace(&before_full, &item.pointer, new_contents)?
        }
        DocumentFormat::Toml => {
            if item.pointer.is_empty() {
                apply_toml_replace(&before_full, "", new_contents)?
            } else {
                // 简化：pointer 非空时要求用户编辑整文件
                return Err(anyhow!(
                    "TOML 条目请切换到整文件编辑（清空 pointer 的 Settings 项）"
                ));
            }
        }
        DocumentFormat::Markdown => apply_text_replace(&before_full, &item.pointer, new_contents)?,
        DocumentFormat::Directory => unreachable!(),
    };
    validate_before_write(item.agent, item.kind, item.format, &after_full)?;
    let before_show = redact_for_display(&before_full, item.format);
    let after_show = redact_for_display(&after_full, item.format);
    Ok(line_diff(&before_show, &after_show))
}

/// 确认写入：备份 → 校验 → 原子写。
pub fn commit_write(item: &AgentConfigItem, new_contents: &str, note: &str) -> Result<backup::BackupManifest> {
    if item.read_only {
        return Err(anyhow!("只读条目，不可编辑"));
    }
    if item.format == DocumentFormat::Directory {
        return Err(anyhow!("目录条目不可整文件写入"));
    }
    let before_full = read_document(&item.source_path, item.format)?;
    let after_full = match item.format {
        DocumentFormat::Json => {
            apply_json_pointer_replace(&before_full, &item.pointer, new_contents)?
        }
        DocumentFormat::Toml => apply_toml_replace(&before_full, "", new_contents)?,
        DocumentFormat::Markdown => apply_text_replace(&before_full, &item.pointer, new_contents)?,
        DocumentFormat::Directory => unreachable!(),
    };
    validate_before_write(item.agent, item.kind, item.format, &after_full)?;

    let mut paths = vec![item.source_path.clone()];
    // Codex hooks 改动时顺带备份 config.toml（trusted_hash）
    if item.needs_codex_retrust {
        if let Some(home) = dirs::home_dir() {
            let cfg = home.join(".codex/config.toml");
            if cfg.exists() {
                paths.push(cfg);
            }
        }
    }
    let manifest = create_backup(&paths, note, DEFAULT_KEEP)?;
    atomic_write(&item.source_path, &after_full)
        .with_context(|| format!("写入 {}", item.source_path.display()))?;
    Ok(manifest)
}

/// Claude settings 生效链。
pub fn claude_settings_effective_chain(repo_root: Option<&Path>) -> Vec<EffectiveLayer> {
    let home = dirs::home_dir();
    let mut layers = Vec::new();
    let managed = PathBuf::from(
        "/Library/Application Support/ClaudeCode/managed-settings.json",
    );
    layers.push(EffectiveLayer {
        scope: ConfigScope::Managed,
        exists: managed.exists(),
        wins: false,
        path: managed,
    });
    if let Some(repo) = repo_root {
        let local = repo.join(".claude/settings.local.json");
        layers.push(EffectiveLayer {
            scope: ConfigScope::Local,
            exists: local.exists(),
            wins: false,
            path: local,
        });
        let project = repo.join(".claude/settings.json");
        layers.push(EffectiveLayer {
            scope: ConfigScope::Project,
            exists: project.exists(),
            wins: false,
            path: project,
        });
    }
    if let Some(home) = home {
        let user = home.join(".claude/settings.json");
        layers.push(EffectiveLayer {
            scope: ConfigScope::User,
            exists: user.exists(),
            wins: false,
            path: user,
        });
    }
    // 标记最高存在层为 wins（简化：不模拟完整合并）
    if let Some(layer) = layers.iter_mut().find(|l| l.exists) {
        layer.wins = true;
    }
    layers
}

/// 打开路径：返回应用内编辑器可用的绝对路径。
pub fn openable_path(item: &AgentConfigItem) -> &Path {
    &item.source_path
}

/// 体检汇总。
pub fn run_health_check(items: &[AgentConfigItem]) -> Vec<HealthFinding> {
    let mut per_agent = std::collections::HashMap::<AgentKind, bool>::new();
    for item in items {
        let has = matches!(item.managed_by, ManagedBy::Bridge);
        per_agent
            .entry(item.agent)
            .and_modify(|v| *v |= has)
            .or_insert(has);
    }
    let pairs: Vec<_> = per_agent.into_iter().collect();
    health_check_bridge(&pairs)
}

/// 文件是否像在 git 仓库内（粗检）。
pub fn path_looks_project_tracked(path: &Path) -> bool {
    let mut cur = path.parent();
    while let Some(dir) = cur {
        if dir.join(".git").exists() {
            return true;
        }
        cur = dir.parent();
    }
    false
}

/// 确保备份根目录存在。
pub fn ensure_backup_root() -> Result<PathBuf> {
    let root = backup::backups_root();
    fs::create_dir_all(&root)?;
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn discover_does_not_panic() {
        let items = discover(None);
        // 本机通常至少有 Claude / Codex / Grok 之一
        let _ = items.len();
    }

    #[test]
    fn commit_write_json_roundtrip() {
        let dir = tempdir().unwrap();
        // 把 backups 测开：直接测 apply + atomic
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{"hooks":{"Stop":[]}}"#).unwrap();
        let item = AgentConfigItem {
            id: "t".into(),
            agent: AgentKind::Claude,
            kind: ConfigKind::Hooks,
            scope: ConfigScope::User,
            title: "Stop".into(),
            source_path: path.clone(),
            pointer: "/hooks/Stop".into(),
            format: DocumentFormat::Json,
            executable: true,
            managed_by: ManagedBy::None,
            project_tracked: false,
            needs_codex_retrust: false,
            summary: String::new(),
            read_only: false,
        };
        let before = read_document(&path, DocumentFormat::Json).unwrap();
        let after =
            apply_json_pointer_replace(&before, &item.pointer, r#"[{"type":"command","command":"echo"}]"#)
                .unwrap();
        validate_before_write(item.agent, item.kind, item.format, &after).unwrap();
        atomic_write(&path, &after).unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("echo"));
    }
}
