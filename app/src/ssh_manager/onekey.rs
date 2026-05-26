//! OneKey 凭据加载:从 SSH Manager 持久化层 + Keychain/DPAPI/Linux Keyring
//! 读出所有已保存的 server 凭据,供 `TerminalView` 在检测到 PTY 密码提示时
//! 弹出选择菜单。
//!
//! ## 注意
//!
//! - 内部调用 `warp_ssh_manager::with_conn`(同步 Mutex + SQLite)和
//!   `KeychainSecretStore::get`(同步 OS API),**不可以**在 UI 主线程直接
//!   同步调用——server 一多就会卡顿。调用方需走 `tokio::task::spawn_blocking`。
//! - secret 全程用 `Zeroizing<String>` 持有,丢弃时自动清零。

use anyhow::Result;
use zeroize::Zeroizing;

use warp_ssh_manager::{
    AuthType, KeychainSecretStore, NodeKind, SecretKind, SshRepository, SshSecretStore,
};

pub struct OneKeyCredential {
    pub label: String,
    pub subtitle: String,
    pub secret: Zeroizing<String>,
}

pub fn load_saved_ssh_credentials() -> Result<Vec<OneKeyCredential>> {
    let store = KeychainSecretStore;
    warp_ssh_manager::with_conn(|conn| {
        let nodes = SshRepository::list_nodes(conn)?;
        let mut credentials = Vec::new();

        for node in nodes {
            if node.kind != NodeKind::Server {
                continue;
            }
            let Some(server) = SshRepository::get_server(conn, &node.id)? else {
                continue;
            };
            let kind = match server.auth_type {
                AuthType::Password => SecretKind::Password,
                AuthType::Key => SecretKind::Passphrase,
            };
            let secret = match store.get(&node.id, kind) {
                Ok(Some(secret)) if !secret.is_empty() => secret,
                Ok(Some(_)) | Ok(None) => continue,
                Err(e) => {
                    log::warn!("onekey: failed to read saved ssh credential: {e}");
                    continue;
                }
            };
            let target = if server.username.is_empty() {
                format!("{}:{}", server.host, server.port)
            } else {
                format!("{}@{}:{}", server.username, server.host, server.port)
            };
            // kind 由 auth_type 推出,只能是 Password / Passphrase 两者,RootPassword
            // 不在 OneKey 本身的范围内(走独立的 su 弹窗确认流程)。
            let subtitle = match server.auth_type {
                AuthType::Password => target,
                AuthType::Key => {
                    let key_path = server.key_path.as_deref().unwrap_or("key");
                    format!("{key_path} for {target}")
                }
            };
            credentials.push(OneKeyCredential {
                label: node.name,
                subtitle,
                secret,
            });
        }

        Ok(credentials)
    })
}
