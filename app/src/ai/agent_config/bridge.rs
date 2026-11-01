//! 调用现有 `zap-cli-agent-bridge/install.sh` 做幂等接线，不移植逻辑进 Rust。

use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, Context, Result};

use super::model::{AgentKind, HealthFinding, HealthSeverity};

/// 默认安装位置。
pub fn bridge_install_root() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".local/share/zap-cli-agent-bridge")
}

pub fn bridge_install_sh() -> PathBuf {
    bridge_install_root().join("install.sh")
}

pub fn bridge_status_bin() -> PathBuf {
    bridge_install_root().join("bin/zap-agent-status")
}

/// 命令是否由 bridge 托管。
pub fn command_is_bridge_managed(cmd: &str) -> bool {
    let lower = cmd.to_ascii_lowercase();
    lower.contains("zap-cli-agent-bridge") || lower.contains("zap-agent-status")
}

/// 跑 `install.sh`（幂等接线）。返回 stdout+stderr。
pub fn run_bridge_install() -> Result<String> {
    let script = bridge_install_sh();
    if !script.exists() {
        return Err(anyhow!(
            "未找到 {}。请先安装 zap-cli-agent-bridge。",
            script.display()
        ));
    }
    let output = Command::new("bash")
        .arg(&script)
        .output()
        .with_context(|| format!("执行 {}", script.display()))?;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    if !output.status.success() {
        return Err(anyhow!(
            "install.sh 退出码 {:?}\n{text}",
            output.status.code()
        ));
    }
    Ok(text)
}

/// 体检：bridge 是否存在、各 agent 是否可能漏接。
pub fn health_check_bridge(items_summaries: &[(AgentKind, bool /*has_bridge_hook*/)]) -> Vec<HealthFinding> {
    let mut out = Vec::new();
    if !bridge_install_sh().exists() {
        out.push(HealthFinding {
            severity: HealthSeverity::Warn,
            agent: AgentKind::Claude,
            title: "未安装 zap-cli-agent-bridge".into(),
            detail: format!("期望路径 {}", bridge_install_sh().display()),
            fixable_by_bridge: false,
        });
        return out;
    }
    if !bridge_status_bin().exists() {
        out.push(HealthFinding {
            severity: HealthSeverity::Warn,
            agent: AgentKind::Claude,
            title: "bridge 入口缺失".into(),
            detail: format!("缺少 {}", bridge_status_bin().display()),
            fixable_by_bridge: true,
        });
    }
    for (agent, has) in items_summaries {
        if matches!(agent, AgentKind::Claude | AgentKind::Codex | AgentKind::Grok) && !*has {
            out.push(HealthFinding {
                severity: HealthSeverity::Info,
                agent: *agent,
                title: format!("{} 未检测到 bridge hook", agent.display_name()),
                detail: "可一键修复接线 statusLine / hooks".into(),
                fixable_by_bridge: true,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_bridge_command() {
        assert!(command_is_bridge_managed(
            "~/.local/share/zap-cli-agent-bridge/bin/zap-agent-status claude"
        ));
        assert!(!command_is_bridge_managed("echo hi"));
    }
}
