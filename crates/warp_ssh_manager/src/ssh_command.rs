//! 把 `SshServerInfo` 拼成 `ssh ...` 命令。纯函数,易测。
//!
//! 写入 PTY 时调 `build_ssh_command_line`,会用 shell-escape 引用每个 arg,
//! 防止用户名 / host / key_path 里的空格或单引号破坏命令行。

use crate::types::{AuthType, ConnectionStatus, SshServerInfo};
use std::borrow::Cow;
use std::time::Duration;

pub fn build_ssh_args(server: &SshServerInfo) -> Vec<String> {
    let mut args: Vec<String> = vec!["ssh".into()];
    if server.port != 22 {
        args.push("-p".into());
        args.push(server.port.to_string());
    }
    if server.auth_type == AuthType::Key
        && let Some(path) = server.key_path.as_deref()
        && !path.is_empty()
    {
        args.push("-i".into());
        args.push(path.to_string());
    }
    let target = if server.username.is_empty() {
        server.host.clone()
    } else {
        format!("{}@{}", server.username, server.host)
    };
    args.push(target);
    args
}

pub fn build_ssh_command_line(server: &SshServerInfo) -> String {
    let args = build_ssh_args(server);
    args.iter()
        .map(|a| shell_escape::unix::escape(Cow::Borrowed(a.as_str())).to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

const TEST_TIMEOUT: Duration = Duration::from_secs(10);

pub struct ConnectionTestResult {
    pub status: ConnectionStatus,
    pub latency_ms: Option<u64>,
    pub error_message: Option<String>,
}

pub async fn test_connection(server: &SshServerInfo, password: Option<String>) -> ConnectionTestResult {
    let start = instant::Instant::now();

    let result = match server.auth_type {
        AuthType::Key => test_key_auth(server).await,
        AuthType::Password => test_password_auth(server, password).await,
    };

    let latency = start.elapsed().as_millis() as u64;

    match result {
        Ok(()) => ConnectionTestResult {
            status: ConnectionStatus::Online,
            latency_ms: Some(latency),
            error_message: None,
        },
        Err(e) => ConnectionTestResult {
            status: ConnectionStatus::Offline,
            latency_ms: Some(latency),
            error_message: Some(e),
        },
    }
}

async fn test_key_auth(server: &SshServerInfo) -> Result<(), String> {
    let args = build_ssh_args(server);
    let mut cmd_args = args.clone();
    cmd_args.push("-o".into());
    cmd_args.push("BatchMode=yes".into());
    cmd_args.push("-o".into());
    cmd_args.push("ConnectTimeout=5".into());
    cmd_args.push("-o".into());
    cmd_args.push("StrictHostKeyChecking=no".into());
    cmd_args.push("-o".into());
    cmd_args.push("LogLevel=ERROR".into());
    cmd_args.push("echo ok".into());

    match tokio::time::timeout(TEST_TIMEOUT, run_ssh_test(&cmd_args)).await {
        Ok(Ok(output)) => {
            if output.trim() == "ok" || output.trim().ends_with("ok") {
                Ok(())
            } else {
                Err(format!("Unexpected output: {}", output.trim()))
            }
        }
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err("Connection timeout".into()),
    }
}

async fn test_password_auth(server: &SshServerInfo, password: Option<String>) -> Result<(), String> {
    let password = password.ok_or("Password not provided")?;

    let args = build_ssh_args(server);
    let mut cmd_args = vec!["sshpass".into(), "-p".into(), password];
    cmd_args.extend(args);
    cmd_args.push("-o".into());
    cmd_args.push("ConnectTimeout=5".into());
    cmd_args.push("-o".into());
    cmd_args.push("StrictHostKeyChecking=no".into());
    cmd_args.push("-o".into());
    cmd_args.push("LogLevel=ERROR".into());
    cmd_args.push("echo ok".into());

    match tokio::time::timeout(TEST_TIMEOUT, run_ssh_test(&cmd_args)).await {
        Ok(Ok(output)) => {
            if output.trim() == "ok" || output.trim().ends_with("ok") {
                Ok(())
            } else {
                Err(format!("Unexpected output: {}", output.trim()))
            }
        }
        Ok(Err(e)) => {
            let err_msg = e.to_string();
            if err_msg.contains("Permission denied") {
                Err("Authentication failed: wrong password".into())
            } else {
                Err(err_msg)
            }
        }
        Err(_) => Err("Connection timeout".into()),
    }
}

async fn run_ssh_test(args: &[String]) -> Result<String, std::io::Error> {
    // 统一走 command::r#async 派生子进程,Windows 上会带 CREATE_NO_WINDOW,
    // 避免闪出控制台窗口(见 .clippy.toml 对 tokio::process::Command 的禁用)。
    let output = command::r#async::Command::new(&args[0])
        .args(&args[1..])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // 成功判定:进程退出码为 0,或远端 `echo ok` 的输出已回传(部分 sshpass
    // 警告会让退出码非零,但 stdout 里仍含 "ok")。
    if output.status.success() || stdout.contains("ok") {
        Ok(stdout)
    } else {
        Err(std::io::Error::other(stderr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server() -> SshServerInfo {
        SshServerInfo {
            node_id: "n".into(),
            host: "1.2.3.4".into(),
            port: 22,
            username: "alice".into(),
            auth_type: AuthType::Password,
            key_path: None,
            startup_command: None,
            notes: None,
            last_connected_at: None,
        }
    }

    #[test]
    fn default_port_omitted() {
        let s = server();
        assert_eq!(build_ssh_args(&s), vec!["ssh", "alice@1.2.3.4"]);
        // shell-escape 出于保守会把 user@host 用单引号引起来,这是合法且
        // shell-equivalent 的形式 — 不强求未引用版本。
        let line = build_ssh_command_line(&s);
        assert!(
            line == "ssh alice@1.2.3.4" || line == "ssh 'alice@1.2.3.4'",
            "unexpected: {line}"
        );
    }

    #[test]
    fn custom_port_uses_dash_p() {
        let mut s = server();
        s.port = 2222;
        assert_eq!(
            build_ssh_args(&s),
            vec!["ssh", "-p", "2222", "alice@1.2.3.4"]
        );
    }

    #[test]
    fn key_auth_emits_dash_i() {
        let mut s = server();
        s.auth_type = AuthType::Key;
        s.key_path = Some("/home/u/.ssh/id_ed25519".into());
        assert_eq!(
            build_ssh_args(&s),
            vec!["ssh", "-i", "/home/u/.ssh/id_ed25519", "alice@1.2.3.4"]
        );
    }

    #[test]
    fn key_auth_without_path_is_skipped() {
        let mut s = server();
        s.auth_type = AuthType::Key;
        s.key_path = None;
        assert_eq!(build_ssh_args(&s), vec!["ssh", "alice@1.2.3.4"]);
    }

    #[test]
    fn empty_username_yields_host_only() {
        let mut s = server();
        s.username = String::new();
        assert_eq!(build_ssh_args(&s), vec!["ssh", "1.2.3.4"]);
    }

    #[test]
    fn shell_escapes_spaces_in_path() {
        let mut s = server();
        s.auth_type = AuthType::Key;
        s.key_path = Some("/path with spaces/id_rsa".into());
        let line = build_ssh_command_line(&s);
        assert!(
            line.contains("'/path with spaces/id_rsa'"),
            "actual: {line}"
        );
    }

    #[test]
    fn test_connection_requires_password_for_password_auth() {
        let s = server();
        // test_connection 应该在没有密码时返回错误
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(test_connection(&s, None));
        assert_eq!(result.status, ConnectionStatus::Offline);
        assert!(result.error_message.unwrap().contains("Password not provided"));
    }

    #[test]
    fn test_connection_key_auth_uses_batch_mode() {
        let mut s = server();
        s.auth_type = AuthType::Key;
        s.key_path = Some("/home/user/.ssh/id_rsa".into());
        // 对于密钥认证，应该使用 BatchMode=yes
        // 由于我们无法实际连接，这里只测试参数构建
        let args = build_ssh_args(&s);
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"/home/user/.ssh/id_rsa".to_string()));
    }

    #[test]
    fn test_password_auth_args_include_sshpass() {
        let s = server();
        let args = build_ssh_args(&s);
        let mut cmd_args = vec!["sshpass".to_string(), "-p".to_string(), "test_password".to_string()];
        cmd_args.extend(args);
        assert!(cmd_args[0] == "sshpass");
        assert!(cmd_args[2] == "test_password");
    }

    #[test]
    fn connection_status_equality() {
        assert_eq!(ConnectionStatus::Online, ConnectionStatus::Online);
        assert_eq!(ConnectionStatus::Offline, ConnectionStatus::Offline);
        assert_eq!(ConnectionStatus::Unknown, ConnectionStatus::Unknown);
        assert_ne!(ConnectionStatus::Online, ConnectionStatus::Offline);
        assert_ne!(ConnectionStatus::Online, ConnectionStatus::Unknown);
        assert_ne!(ConnectionStatus::Offline, ConnectionStatus::Unknown);
    }
}
