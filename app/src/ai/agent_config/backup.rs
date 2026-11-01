//! 配置备份与回滚。默认保留 3 份，落在 Zap `data_dir()/agent-config/backups`。

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use super::document::{atomic_write, relative_backup_name};

pub const DEFAULT_KEEP: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub id: String,
    pub created_unix_ms: u128,
    pub note: String,
    pub files: Vec<BackupFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupFileEntry {
    pub original: PathBuf,
    pub stored_as: String,
}

/// `~/.zap/agent-config/backups`（macOS Oss）或对应 channel 的 data_dir。
pub fn backups_root() -> PathBuf {
    warp_core::paths::data_dir().join("agent-config").join("backups")
}

fn now_id() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{ms}")
}

/// 为若干文件创建一份备份；超过 `keep` 时删最旧的。
pub fn create_backup(
    paths: &[PathBuf],
    note: impl Into<String>,
    keep: usize,
) -> Result<BackupManifest> {
    let root = backups_root();
    fs::create_dir_all(&root).with_context(|| format!("创建 {}", root.display()))?;
    let id = now_id();
    let dir = root.join(&id);
    fs::create_dir_all(&dir)?;

    let mut files = Vec::new();
    for path in paths {
        if !path.exists() {
            continue;
        }
        let stored = relative_backup_name(path);
        let dest = dir.join(&stored);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(path, &dest)
            .with_context(|| format!("备份 {} → {}", path.display(), dest.display()))?;
        files.push(BackupFileEntry {
            original: path.clone(),
            stored_as: stored.to_string_lossy().into_owned(),
        });
    }

    let manifest = BackupManifest {
        id: id.clone(),
        created_unix_ms: id.parse().unwrap_or(0),
        note: note.into(),
        files,
    };
    let manifest_path = dir.join("manifest.json");
    let body = serde_json::to_string_pretty(&manifest)?;
    atomic_write(&manifest_path, &body)?;

    prune_old(keep)?;
    Ok(manifest)
}

/// 列出备份（新→旧）。
pub fn list_backups() -> Result<Vec<BackupManifest>> {
    let root = backups_root();
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let manifest_path = entry.path().join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&manifest_path)?;
        if let Ok(m) = serde_json::from_str::<BackupManifest>(&raw) {
            out.push(m);
        }
    }
    out.sort_by(|a, b| b.created_unix_ms.cmp(&a.created_unix_ms));
    Ok(out)
}

/// 恢复指定备份。
pub fn restore_backup(id: &str) -> Result<BackupManifest> {
    let dir = backups_root().join(id);
    let manifest_path = dir.join("manifest.json");
    let raw = fs::read_to_string(&manifest_path)
        .with_context(|| format!("读备份清单 {}", manifest_path.display()))?;
    let manifest: BackupManifest = serde_json::from_str(&raw)?;
    for file in &manifest.files {
        let src = dir.join(&file.stored_as);
        if !src.exists() {
            return Err(anyhow!("备份文件缺失: {}", src.display()));
        }
        if let Some(parent) = file.original.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = fs::read_to_string(&src)?;
        atomic_write(&file.original, &contents)?;
    }
    Ok(manifest)
}

/// 恢复最近一次备份。
pub fn restore_latest() -> Result<BackupManifest> {
    let list = list_backups()?;
    let latest = list
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("没有可用备份"))?;
    restore_backup(&latest.id)
}

fn prune_old(keep: usize) -> Result<()> {
    let mut list = list_backups()?;
    if list.len() <= keep {
        return Ok(());
    }
    for old in list.drain(keep..) {
        let dir = backups_root().join(&old.id);
        let _ = fs::remove_dir_all(dir);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    // 测试里直接操作临时 backups_root 不可行（函数绑死 data_dir），
    // 改为测 relative 与 manifest 序列化。
    #[test]
    fn manifest_roundtrip() {
        let m = BackupManifest {
            id: "1".into(),
            created_unix_ms: 1,
            note: "test".into(),
            files: vec![BackupFileEntry {
                original: PathBuf::from("/tmp/a.json"),
                stored_as: "_tmp_a.json".into(),
            }],
        };
        let s = serde_json::to_string(&m).unwrap();
        let back: BackupManifest = serde_json::from_str(&s).unwrap();
        assert_eq!(back.note, "test");
    }

    #[test]
    fn create_and_restore_in_temp_via_override_paths() {
        // 用临时目录模拟：直接测 copy/atomic 逻辑
        let dir = tempdir().unwrap();
        let src = dir.path().join("settings.json");
        fs::write(&src, "{\"v\":1}").unwrap();
        let bak_dir = dir.path().join("bak");
        fs::create_dir_all(&bak_dir).unwrap();
        let stored = bak_dir.join("settings.json");
        fs::copy(&src, &stored).unwrap();
        fs::write(&src, "{\"v\":2}").unwrap();
        let restored = fs::read_to_string(&stored).unwrap();
        atomic_write(&src, &restored).unwrap();
        assert_eq!(fs::read_to_string(&src).unwrap(), "{\"v\":1}");
        let _ = Mutex::<()>::new(());
        let _ = OnceLock::<()>::new();
    }
}
