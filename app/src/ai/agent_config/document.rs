//! 配置文档读写：JSON / TOML / Markdown，原子写，敏感字段打码。

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde_json::Value as JsonValue;

use super::model::DocumentFormat;

const SENSITIVE_KEY_HINTS: &[&str] = &[
    "token",
    "api_key",
    "apikey",
    "secret",
    "password",
    "passwd",
    "auth",
    "credential",
    "authorization",
];

/// 读入整份文档。
pub fn read_document(path: &Path, _format: DocumentFormat) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("读取 {}", path.display()))
}

/// 原子写入：先写临时文件再 rename。
pub fn atomic_write(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建目录 {}", parent.display()))?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp-{}",
        path.extension().and_then(|e| e.to_str()).unwrap_or("cfg"),
        std::process::id()
    ));
    {
        let mut f = fs::File::create(&tmp)
            .with_context(|| format!("创建临时文件 {}", tmp.display()))?;
        f.write_all(contents.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path).with_context(|| {
        let _ = fs::remove_file(&tmp);
        format!("替换 {}", path.display())
    })?;
    Ok(())
}

/// 对 JSON 文本做 pretty 格式化（失败则原样返回）。
pub fn pretty_json(raw: &str) -> Result<String> {
    let value: JsonValue = serde_json::from_str(raw).context("解析 JSON")?;
    Ok(serde_json::to_string_pretty(&value)?)
}

/// 用 toml_edit 解析并序列化，尽量保留格式。
pub fn parse_toml_document(raw: &str) -> Result<toml_edit::DocumentMut> {
    raw.parse::<toml_edit::DocumentMut>()
        .map_err(|e| anyhow!("解析 TOML: {e}"))
}

/// 把 pointer 路径上的 JSON 值替换为 `new_value` 的序列化结果。
/// pointer 空字符串表示整文件替换。
pub fn apply_json_pointer_replace(
    raw: &str,
    pointer: &str,
    new_fragment: &str,
) -> Result<String> {
    if pointer.is_empty() {
        // 整文件：校验新内容是合法 JSON 后 pretty 写出
        let v: JsonValue = serde_json::from_str(new_fragment).context("新内容不是合法 JSON")?;
        return Ok(serde_json::to_string_pretty(&v)?);
    }
    let mut root: JsonValue = serde_json::from_str(raw).context("原文件不是合法 JSON")?;
    let fragment: JsonValue =
        serde_json::from_str(new_fragment).context("新片段不是合法 JSON")?;
    let target = root
        .pointer_mut(pointer)
        .ok_or_else(|| anyhow!("找不到 JSON pointer `{pointer}`"))?;
    *target = fragment;
    Ok(serde_json::to_string_pretty(&root)?)
}

/// TOML：空 pointer = 整文件；否则按 dotted path 设为字符串/内联表（简化：整段替换为解析后的 item）。
pub fn apply_toml_replace(raw: &str, pointer: &str, new_fragment: &str) -> Result<String> {
    if pointer.is_empty() {
        let doc = parse_toml_document(new_fragment)?;
        return Ok(doc.to_string());
    }
    let mut doc = parse_toml_document(raw)?;
    let replacement = new_fragment
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| anyhow!("新 TOML 片段解析失败: {e}"))?;
    // 简化：把 pointer 当作顶层 key 路径，用整个 replacement 的第一个表项，
    // 更常见场景是整文件编辑（pointer 空）。对 dotted path，写入字符串值。
    set_toml_dotted(&mut doc, pointer, toml_edit::value(new_fragment.trim()))?;
    let _ = replacement;
    Ok(doc.to_string())
}

fn set_toml_dotted(
    doc: &mut toml_edit::DocumentMut,
    dotted: &str,
    value: toml_edit::Item,
) -> Result<()> {
    let parts: Vec<&str> = dotted.split('.').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return Err(anyhow!("空 TOML pointer"));
    }
    let mut current = doc.as_item_mut();
    for (i, part) in parts.iter().enumerate() {
        let is_last = i + 1 == parts.len();
        if is_last {
            let table = current
                .as_table_like_mut()
                .ok_or_else(|| anyhow!("路径 `{dotted}` 中段不是表"))?;
            table.insert(part, value.clone());
            return Ok(());
        }
        let table = current
            .as_table_like_mut()
            .ok_or_else(|| anyhow!("路径 `{dotted}` 中段不是表"))?;
        if table.get(part).is_none() {
            table.insert(part, toml_edit::table());
        }
        current = table
            .get_mut(part)
            .ok_or_else(|| anyhow!("无法进入 `{part}`"))?;
    }
    Ok(())
}

/// Markdown / 纯文本：整文件替换。
pub fn apply_text_replace(_raw: &str, pointer: &str, new_fragment: &str) -> Result<String> {
    if !pointer.is_empty() {
        return Err(anyhow!("Markdown 仅支持整文件编辑"));
    }
    Ok(new_fragment.to_string())
}

/// 对展示用文本打码敏感字段（JSON）。
pub fn redact_for_display(raw: &str, format: DocumentFormat) -> String {
    match format {
        DocumentFormat::Json => redact_json_text(raw),
        DocumentFormat::Toml => redact_toml_text(raw),
        _ => raw.to_string(),
    }
}

fn key_looks_sensitive(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    SENSITIVE_KEY_HINTS
        .iter()
        .any(|hint| lower.contains(hint))
}

fn redact_json_text(raw: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<JsonValue>(raw) else {
        return raw.to_string();
    };
    redact_json_value(&mut value);
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| raw.to_string())
}

fn redact_json_value(value: &mut JsonValue) {
    match value {
        JsonValue::Object(map) => {
            for (k, v) in map.iter_mut() {
                if key_looks_sensitive(k) && v.is_string() {
                    *v = JsonValue::String("***".into());
                } else {
                    redact_json_value(v);
                }
            }
        }
        JsonValue::Array(arr) => {
            for v in arr {
                redact_json_value(v);
            }
        }
        _ => {}
    }
}

fn redact_toml_text(raw: &str) -> String {
    let Ok(mut doc) = raw.parse::<toml_edit::DocumentMut>() else {
        return raw.to_string();
    };
    redact_toml_item(doc.as_item_mut());
    doc.to_string()
}

fn redact_toml_item(item: &mut toml_edit::Item) {
    if let Some(table) = item.as_table_like_mut() {
        let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
        for key in keys {
            if key_looks_sensitive(&key) {
                if let Some(val) = table.get_mut(&key) {
                    if val.is_str() {
                        *val = toml_edit::value("***");
                    }
                }
            } else if let Some(child) = table.get_mut(&key) {
                redact_toml_item(child);
            }
        }
    } else if let Some(arr) = item.as_array_of_tables_mut() {
        for table in arr.iter_mut() {
            let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
            for key in keys {
                if key_looks_sensitive(&key) {
                    if let Some(val) = table.get_mut(&key) {
                        if val.is_str() {
                            *val = toml_edit::value("***");
                        }
                    }
                } else if let Some(child) = table.get_mut(&key) {
                    redact_toml_item(child);
                }
            }
        }
    }
}

/// 简单行级 diff（展示用），返回 (unified 风格文本, +n, -n)。
pub fn line_diff(before: &str, after: &str) -> (String, usize, usize) {
    let before_lines: Vec<&str> = before.lines().collect();
    let after_lines: Vec<&str> = after.lines().collect();
    // Myers 太重；用 LCS 简化：按行集合对齐（足够配置文件）。
    let mut out = String::new();
    let mut adds = 0usize;
    let mut dels = 0usize;
    let mut i = 0usize;
    let mut j = 0usize;
    while i < before_lines.len() || j < after_lines.len() {
        if i < before_lines.len() && j < after_lines.len() && before_lines[i] == after_lines[j] {
            out.push_str(&format!("  {}\n", before_lines[i]));
            i += 1;
            j += 1;
        } else if j < after_lines.len()
            && (i >= before_lines.len()
                || !after_lines[j..].contains(&before_lines[i]))
        {
            out.push_str(&format!("+ {}\n", after_lines[j]));
            adds += 1;
            j += 1;
        } else if i < before_lines.len() {
            out.push_str(&format!("- {}\n", before_lines[i]));
            dels += 1;
            i += 1;
        } else {
            break;
        }
    }
    (out, adds, dels)
}

/// 备份目录下的相对路径辅助。
pub fn relative_backup_name(path: &Path) -> PathBuf {
    // 把绝对路径拍扁成安全文件名
    let s = path.to_string_lossy();
    let flat: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect();
    PathBuf::from(flat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn atomic_write_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        atomic_write(&path, "{\"a\":1}\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":1}\n");
    }

    #[test]
    fn json_pointer_replace() {
        let raw = r#"{"hooks":{"Stop":[{"x":1}]}}"#;
        let out = apply_json_pointer_replace(raw, "/hooks/Stop", "[{\"y\":2}]").unwrap();
        let v: JsonValue = serde_json::from_str(&out).unwrap();
        assert_eq!(v["hooks"]["Stop"][0]["y"], 2);
    }

    #[test]
    fn redact_token_in_env() {
        let raw = r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"sk-secret","FOO":"bar"}}"#;
        let red = redact_for_display(raw, DocumentFormat::Json);
        assert!(red.contains("***"));
        assert!(red.contains("bar"));
        assert!(!red.contains("sk-secret"));
    }

    #[test]
    fn line_diff_counts() {
        let (text, a, d) = line_diff("a\nb\n", "a\nc\n");
        assert!(a >= 1 && d >= 1);
        assert!(text.contains("- b") || text.contains("-b") || text.contains("- b\n") || text.contains("- b"));
        assert!(text.contains('+'));
    }
}
