//! SSH 数据同步提供者，实现 SyncDataProvider trait
//!
// author: logic
// date: 2026-05-24

use crate::db::with_conn;
use crate::repository::SshRepository;
use crate::secrets::{KeychainSecretStore, SecretKind, SshSecretStore};
use crate::types::NodeKind;
use diesel::connection::{Connection, SimpleConnection};
use diesel::RunQueryDsl;
use serde::{Deserialize, Serialize};
use zap_sync::crypto;
use zap_sync::{SyncDataProvider, SyncEngineError, SyncVersionStore};

/// SSH 同步用的节点数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub kind: String,
    pub name: String,
    pub sort_order: i32,
    pub is_collapsed: bool,
}

/// SSH 同步用的服务器数据（含加密密码）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncServer {
    pub node_id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String,
    pub key_path: Option<String>,
    pub startup_command: Option<String>,
    pub notes: Option<String>,
    pub password_encrypted: Option<String>,
    pub passphrase_encrypted: Option<String>,
    pub root_password_encrypted: Option<String>,
}

/// SSH 同步数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SshSyncData {
    pub nodes: Vec<SyncNode>,
    pub servers: Vec<SyncServer>,
}

/// SSH 数据同步提供者
pub struct SshSyncProvider {
    secret_store: KeychainSecretStore,
}

impl SshSyncProvider {
    /// 创建新的 SshSyncProvider 实例
    pub fn new() -> Self {
        Self {
            secret_store: KeychainSecretStore::default(),
        }
    }
}

impl SyncDataProvider for SshSyncProvider {
    fn section_key(&self) -> &str {
        "ssh"
    }

    fn collect_data(&self, token: &str) -> Result<serde_json::Value, SyncEngineError> {
        let nodes =
            with_conn(|conn| Ok(SshRepository::list_nodes(conn)?))
                .map_err(|e| SyncEngineError::Provider(e.to_string()))?;

        let mut sync_nodes = Vec::new();
        let mut sync_servers = Vec::new();

        for node in &nodes {
            sync_nodes.push(SyncNode {
                id: node.id.clone(),
                parent_id: node.parent_id.clone(),
                kind: node.kind.as_db_str().to_string(),
                name: node.name.clone(),
                sort_order: node.sort_order,
                is_collapsed: node.is_collapsed,
            });

            if node.kind == NodeKind::Server {
                let server_result =
                    with_conn(|conn| Ok(SshRepository::get_server(conn, &node.id)?))
                        .map_err(|e| SyncEngineError::Provider(e.to_string()))?;
                if let Some(server) = server_result {
                    let password = self
                        .secret_store
                        .get(&node.id, SecretKind::Password)
                        .ok()
                        .flatten()
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let passphrase = self
                        .secret_store
                        .get(&node.id, SecretKind::Passphrase)
                        .ok()
                        .flatten()
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let root_password = self
                        .secret_store
                        .get(&node.id, SecretKind::RootPassword)
                        .ok()
                        .flatten()
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    sync_servers.push(SyncServer {
                        node_id: server.node_id.clone(),
                        host: server.host.clone(),
                        port: server.port,
                        username: server.username.clone(),
                        auth_type: server.auth_type.as_db_str().to_string(),
                        key_path: server.key_path.clone(),
                        startup_command: server.startup_command.clone(),
                        notes: server.notes.clone(),
                        password_encrypted: if password.is_empty() {
                            None
                        } else {
                            Some(
                                crypto::encrypt(token, &password)
                                    .map_err(|e| SyncEngineError::Crypto(e.to_string()))?,
                            )
                        },
                        passphrase_encrypted: if passphrase.is_empty() {
                            None
                        } else {
                            Some(
                                crypto::encrypt(token, &passphrase)
                                    .map_err(|e| SyncEngineError::Crypto(e.to_string()))?,
                            )
                        },
                        root_password_encrypted: if root_password.is_empty() {
                            None
                        } else {
                            Some(
                                crypto::encrypt(token, &root_password)
                                    .map_err(|e| SyncEngineError::Crypto(e.to_string()))?,
                            )
                        },
                    });
                }
            }
        }

        let data = SshSyncData {
            nodes: sync_nodes,
            servers: sync_servers,
        };

        serde_json::to_value(&data)
            .map_err(|e: serde_json::Error| SyncEngineError::Serialization(e.to_string()))
    }

    fn apply_data(&self, token: &str, data: &serde_json::Value) -> Result<(), SyncEngineError> {
        let ssh_data: SshSyncData = serde_json::from_value(data.clone())
            .map_err(|e: serde_json::Error| SyncEngineError::Serialization(e.to_string()))?;

        /// 待写入 keychain 的凭据
        struct PendingSecret {
            node_id: String,
            kind: SecretKind,
            value: String,
        }

        // 阶段一：事务中执行数据库操作，收集解密后的凭据
        let pending_secrets = with_conn(|conn| {
            conn.transaction::<Vec<PendingSecret>, anyhow::Error, _>(|conn| {
                conn.batch_execute("DELETE FROM ssh_servers; DELETE FROM ssh_nodes;")?;

                let mut pending = Vec::new();

                for node in &ssh_data.nodes {
                    let kind = NodeKind::parse(&node.kind)
                        .ok_or_else(|| anyhow::anyhow!("无效的 kind: {}", node.kind))?;

                    diesel::insert_into(persistence::schema::ssh_nodes::table)
                        .values(persistence::model::NewSshNode {
                            id: &node.id,
                            parent_id: node.parent_id.as_deref(),
                            kind: kind.as_db_str(),
                            name: &node.name,
                            sort_order: node.sort_order,
                        })
                        .execute(conn)?;

                    if node.is_collapsed {
                        SshRepository::set_collapsed(conn, &node.id, true)?;
                    }
                }

                for server in &ssh_data.servers {
                    diesel::insert_into(persistence::schema::ssh_servers::table)
                        .values(persistence::model::NewSshServer {
                            node_id: &server.node_id,
                            host: &server.host,
                            port: server.port as i32,
                            username: &server.username,
                            auth_type: &server.auth_type,
                            key_path: server.key_path.as_deref(),
                            startup_command: server.startup_command.as_deref(),
                            notes: server.notes.as_deref(),
                        })
                        .execute(conn)?;

                    if let Some(ref enc) = server.password_encrypted {
                        let password = crypto::decrypt(token, enc)?;
                        pending.push(PendingSecret {
                            node_id: server.node_id.clone(),
                            kind: SecretKind::Password,
                            value: password,
                        });
                    }
                    if let Some(ref enc) = server.passphrase_encrypted {
                        let passphrase = crypto::decrypt(token, enc)?;
                        pending.push(PendingSecret {
                            node_id: server.node_id.clone(),
                            kind: SecretKind::Passphrase,
                            value: passphrase,
                        });
                    }
                    if let Some(ref enc) = server.root_password_encrypted {
                        let root_password = crypto::decrypt(token, enc)?;
                        pending.push(PendingSecret {
                            node_id: server.node_id.clone(),
                            kind: SecretKind::RootPassword,
                            value: root_password,
                        });
                    }
                }

                Ok(pending)
            })
        })
        .map_err(|e| SyncEngineError::Provider(e.to_string()))?;

        // 阶段二：事务提交后写入 keychain
        for secret in pending_secrets {
            if let Err(e) = self.secret_store.set(&secret.node_id, secret.kind, &secret.value) {
                log::warn!("写入 keychain 失败 {}: {e}", secret.node_id);
            }
        }

        Ok(())
    }
}

/// 数据库同步版本存储适配器
pub struct DbVersionStore;

impl SyncVersionStore for DbVersionStore {
    fn get_sync_version(&self) -> Result<i64, SyncEngineError> {
        with_conn(|c| Ok(SshRepository::get_sync_version(c)?))
            .map_err(|e| SyncEngineError::VersionStore(e.to_string()))
    }

    fn set_sync_version(&self, version: i64) -> Result<(), SyncEngineError> {
        with_conn(|c| Ok(SshRepository::set_sync_version(c, version)?))
            .map_err(|e| SyncEngineError::VersionStore(e.to_string()))
    }

    fn update_sync_meta(&self, time: &str, platform: &str) -> Result<(), SyncEngineError> {
        with_conn(|c| Ok(SshRepository::update_sync_meta(c, time, platform)?))
            .map_err(|e| SyncEngineError::VersionStore(e.to_string()))
    }
}
