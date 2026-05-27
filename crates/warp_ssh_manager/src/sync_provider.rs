//! SSH 数据同步提供者，实现 SyncDataProvider trait
//!
// author: logic
// date: 2026-05-26

use crate::db::with_conn;
use crate::repository::{SshRepository, SyncMetaRepository};
use crate::secrets::{KeychainSecretStore, SecretKind, SshSecretStore};
use crate::types::NodeKind;
use diesel::connection::{Connection, SimpleConnection};
use diesel::{QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use zap_sync::crypto;
use zap_sync::{SyncDataProvider, SyncEngineError, SyncVersionStore};
use zeroize::Zeroizing;

/// keychain 三种凭据 kind,用于 collect/apply/orphan-cleanup 时统一遍历
const ALL_SECRET_KINDS: [SecretKind; 3] = [
    SecretKind::Password,
    SecretKind::Passphrase,
    SecretKind::RootPassword,
];

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
                    // 区分 keychain 错误与"用户没设密码":
                    // - Ok(Some) = 有密码,加密上传
                    // - Ok(None) = 用户确实没设,字段写 None
                    // - Err = 中止整次上传,避免把瞬时 keychain 故障序列化为
                    //   "无密码"覆盖其他设备的真实密码(PR #161 review #5)
                    let password = read_secret(&self.secret_store, &node.id, SecretKind::Password)?;
                    let passphrase =
                        read_secret(&self.secret_store, &node.id, SecretKind::Passphrase)?;
                    let root_password =
                        read_secret(&self.secret_store, &node.id, SecretKind::RootPassword)?;

                    sync_servers.push(SyncServer {
                        node_id: server.node_id.clone(),
                        host: server.host.clone(),
                        port: server.port,
                        username: server.username.clone(),
                        auth_type: server.auth_type.as_db_str().to_string(),
                        key_path: server.key_path.clone(),
                        startup_command: server.startup_command.clone(),
                        notes: server.notes.clone(),
                        password_encrypted: encrypt_optional(token, password.as_deref())?,
                        passphrase_encrypted: encrypt_optional(token, passphrase.as_deref())?,
                        root_password_encrypted: encrypt_optional(token, root_password.as_deref())?,
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

        // ---- 阶段 0 ---- 全部解密 + 收集 explicit-clear 列表
        // pending_secrets: 远程明确给了密文 → 需要写入 keychain
        // explicit_clears: 远程明确给了 None → 需要 delete keychain(用户在其他设备清空了密码,
        //                  不清理会导致本地继续用旧密码,违反用户意图;PR #161 七轮 review)
        struct PendingSecret {
            node_id: String,
            kind: SecretKind,
            value: String,
        }
        let mut pending_secrets: Vec<PendingSecret> = Vec::new();
        let mut explicit_clears: Vec<(String, SecretKind)> = Vec::new();
        for server in &ssh_data.servers {
            for (kind, enc) in [
                (SecretKind::Password, &server.password_encrypted),
                (SecretKind::Passphrase, &server.passphrase_encrypted),
                (SecretKind::RootPassword, &server.root_password_encrypted),
            ] {
                match enc {
                    Some(enc) => {
                        let value = crypto::decrypt(token, enc)
                            .map_err(|e| SyncEngineError::Crypto(e.to_string()))?;
                        pending_secrets.push(PendingSecret {
                            node_id: server.node_id.clone(),
                            kind,
                            value,
                        });
                    }
                    None => {
                        explicit_clears.push((server.node_id.clone(), kind));
                    }
                }
            }
        }

        // ---- 阶段 0.5 ---- 拓扑排序节点,父节点先于子节点;orphan(parent 不在数据集中)
        // 视作根节点插入,避免 SQLite FK 违规整事务回滚
        let sorted_nodes = topologically_sort_nodes(&ssh_data.nodes);

        // ---- 阶段 0.6 ---- 收集本地原有 node_id,供后续 orphan keychain 清理
        let existing_node_ids: Vec<String> = with_conn(|conn| {
            Ok(persistence::schema::ssh_nodes::table
                .select(persistence::schema::ssh_nodes::id)
                .load::<String>(conn)?)
        })
        .map_err(|e| SyncEngineError::Provider(e.to_string()))?;

        // ---- 阶段 1 ---- 先写 keychain。任一失败 → 立即中止,不动 DB。
        // 跟踪 (node_id, kind, prior_value) 列表,DB 阶段失败时:
        // - prior_value=Some(v) → restore 回旧值(避免覆盖了用户既有密码)
        // - prior_value=None    → delete(避免污染)
        // 真正的"原子回滚"以 secret_store.set 的幂等覆盖语义为基础(PR #161 三轮 review)
        let mut written_secrets: Vec<WrittenSecret> = Vec::new();
        for s in &pending_secrets {
            // 写入前快照原值,以便后续 rollback 可以真正恢复旧值。
            // 真实的 keychain 错误中止整个流程,但 NoBackend(headless Linux 等)按"无旧值"处理。
            // 此设计与 collect_data 的 read_secret 一致 — 同样的环境约束。
            let prior_value = match self.secret_store.get(&s.node_id, s.kind) {
                // store.get 已经返回 Option<Zeroizing<String>>,直接用,保留零化语义
                Ok(opt) => opt,
                Err(e) => {
                    // 与 read_secret 同等严格:keychain 任何错误都中止,避免无法 rollback
                    rollback_keychain_writes(&self.secret_store, &written_secrets);
                    return Err(SyncEngineError::Provider(format!(
                        "读取 keychain 旧值失败 ({}, {:?}): {e}。已回滚 {} 项,请确认密钥库可用后重试下载",
                        s.node_id, s.kind, written_secrets.len()
                    )));
                }
            };
            if let Err(e) = self.secret_store.set(&s.node_id, s.kind, &s.value) {
                rollback_keychain_writes(&self.secret_store, &written_secrets);
                return Err(SyncEngineError::Provider(format!(
                    "写入 keychain 失败 ({}, {:?}): {e},请检查密钥库权限后重试下载",
                    s.node_id, s.kind
                )));
            }
            written_secrets.push(WrittenSecret {
                node_id: s.node_id.clone(),
                kind: s.kind,
                prior_value,
            });
        }

        // ---- 阶段 2 ---- DB 事务:DELETE + 按拓扑顺序 INSERT
        let db_result = with_conn(|conn| {
            conn.transaction::<(), anyhow::Error, _>(|conn| {
                conn.batch_execute("DELETE FROM ssh_servers; DELETE FROM ssh_nodes;")?;

                for node in &sorted_nodes {
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
                }
                Ok(())
            })
        });
        if let Err(e) = db_result {
            // DB 失败 → 回滚刚写入的 keychain,避免长期残留指向不存在 node 的密钥
            let rolled = written_secrets.len();
            rollback_keychain_writes(&self.secret_store, &written_secrets);
            return Err(SyncEngineError::Provider(format!(
                "DB 写入失败 ({e});已回滚 {rolled} 项 keychain 写入"
            )));
        }

        // ---- 阶段 3a ---- 清理 explicit-clear:节点仍存在但远程把对应 *_encrypted 设为 None
        // 用户在其他设备清空了某项密码 → 必须 delete 本地 keychain,否则 connect 时会继续用旧密码,
        // 违背用户清除意图(PR #161 七轮 review)
        for (node_id, kind) in &explicit_clears {
            if let Err(e) = self.secret_store.delete(node_id, *kind) {
                log::warn!("清理 explicit-clear keychain 项失败 {node_id}/{:?}: {e}", kind);
            }
        }

        // ---- 阶段 3b ---- 清理 orphan keychain:本地原有但远程已删除的 node_id 对应的密码,
        // 必须显式 delete,否则同 UUID 节点重新出现时会读到陈旧密码 (PR #161 review #4)
        let new_node_ids: HashSet<&str> =
            ssh_data.nodes.iter().map(|n| n.id.as_str()).collect();
        for old_id in &existing_node_ids {
            if new_node_ids.contains(old_id.as_str()) {
                continue;
            }
            for kind in ALL_SECRET_KINDS {
                if let Err(e) = self.secret_store.delete(old_id, kind) {
                    log::warn!("清理 orphan keychain 项失败 {old_id}/{:?}: {e}", kind);
                }
            }
        }

        Ok(())
    }
}

/// apply_data Phase 1 已写入的 keychain 条目记录,带原值快照用于真正回滚。
/// `prior_value` 用 `Zeroizing<String>` 持有,保证回滚链上明文密码 drop 时被零化。
struct WrittenSecret {
    node_id: String,
    kind: SecretKind,
    prior_value: Option<Zeroizing<String>>,
}

/// 真正的"回滚":对每个已被覆盖的条目:
/// - prior_value=Some → 写回旧值,避免用户既有密码被吞
/// - prior_value=None → delete,避免 orphan
/// 任何步骤失败仅 log,不阻塞调用方(尽力而为)。
fn rollback_keychain_writes<S: SshSecretStore + ?Sized>(
    store: &S,
    written: &[WrittenSecret],
) {
    for entry in written {
        let res = match &entry.prior_value {
            Some(v) => store.set(&entry.node_id, entry.kind, v.as_str()),
            None => store.delete(&entry.node_id, entry.kind),
        };
        if let Err(e) = res {
            log::warn!(
                "回滚 keychain 写入失败 {}/{:?}: {e}(secret 可能保持新值或成为 orphan)",
                entry.node_id, entry.kind
            );
        }
    }
}

/// 读取 keychain 凭据。
/// - `Ok(Some)` = 有密码,加密上传
/// - `Ok(None)` = 用户没设密码(合法状态),字段写 None
/// - `Err` = keychain 故障 (NoBackend / Locked / 权限拒绝)
///
/// 注意:对 NoBackend 不做 fallback。上层 keyring crate 把锁定的 keychain 和
/// 完全无 backend 都映射成 NoBackend,无法可靠区分(keyring 3.6 documented 行为)。
/// 把 NoBackend 当成 Ok(None) 会让"锁定" 这种瞬时故障静默丢密码 → 云端被清空,
/// 重装后无法恢复(KDF/格式仍是待优化项)。
/// headless Linux / CI 用户若全程无密码,upload 不会触发此函数;一旦遇到 Err,
/// 错误信息明确指引用户解锁/启用 keychain。
fn read_secret(
    store: &dyn SshSecretStore,
    node_id: &str,
    kind: SecretKind,
) -> Result<Option<String>, SyncEngineError> {
    match store.get(node_id, kind) {
        Ok(opt) => Ok(opt.map(|z| z.to_string())),
        Err(e) => Err(SyncEngineError::Provider(format!(
            "读取 keychain 失败 ({node_id}, {kind:?}): {e}。\
             keychain 可能被锁定或当前环境无 backend(headless Linux / WSL 等)。\
             请解锁 keychain 或启用 secret-service / Credential Manager 后重试上传。\
             若该服务器确实不需要密码同步,可在 SSH 管理器中清除该字段。"
        ))),
    }
}

fn encrypt_optional(
    token: &str,
    value: Option<&str>,
) -> Result<Option<String>, SyncEngineError> {
    match value {
        None => Ok(None),
        // 空字符串视为"无密码",不上传(与既往行为兼容,避免空字符串密文污染)
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => Ok(Some(
            crypto::encrypt(token, s)
                .map_err(|e| SyncEngineError::Crypto(e.to_string()))?,
        )),
    }
}

/// BFS 拓扑排序:父节点先于子节点。parent_id 引用数据集外节点的孤儿节点,
/// 视作根节点附加到末尾,parent_id 清空,避免 SQLite FK 约束失败让整个 download 回滚。
fn topologically_sort_nodes(nodes: &[SyncNode]) -> Vec<SyncNode> {
    use std::collections::HashMap;
    let mut by_parent: HashMap<Option<&str>, Vec<&SyncNode>> = HashMap::new();
    for n in nodes {
        by_parent.entry(n.parent_id.as_deref()).or_default().push(n);
    }

    let mut result: Vec<SyncNode> = Vec::with_capacity(nodes.len());
    let mut seen: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<&SyncNode> = VecDeque::new();
    if let Some(roots) = by_parent.get(&None) {
        for r in roots {
            queue.push_back(*r);
        }
    }
    while let Some(node) = queue.pop_front() {
        if !seen.insert(node.id.clone()) {
            continue;
        }
        result.push(node.clone());
        if let Some(children) = by_parent.get(&Some(node.id.as_str())) {
            for c in children {
                queue.push_back(*c);
            }
        }
    }

    // 剩余节点要么是 orphan(parent_id 指向数据集外),要么属于一个循环。
    // 两种都把 parent_id 清空降级为根插入(可恢复且无数据丢失),并显式日志告警,
    // 让用户能在日志中看到数据被结构化重置。
    for n in nodes {
        if !seen.contains(&n.id) {
            if has_cycle_membership(n, nodes) {
                log::warn!(
                    "apply_data: 节点 {} 处于循环引用中(parent_id {:?}),已降级为根节点",
                    n.id, n.parent_id
                );
            } else {
                log::warn!(
                    "apply_data: 节点 {} 的 parent_id {:?} 在数据集中不存在,作为根节点插入",
                    n.id, n.parent_id
                );
            }
            let mut orphan = n.clone();
            orphan.parent_id = None;
            result.push(orphan);
        }
    }

    result
}

/// 判断节点 `start` 是否在循环中(从它出发沿 parent_id 链最终回到自身或环上)。
/// 用于区分日志中的 "orphan" vs "cycle";限制最大遍历步数防止指数复杂度。
fn has_cycle_membership(start: &SyncNode, all: &[SyncNode]) -> bool {
    let by_id: std::collections::HashMap<&str, &SyncNode> =
        all.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut current = start;
    let mut visited: HashSet<&str> = HashSet::new();
    let max_steps = all.len() + 1;
    for _ in 0..max_steps {
        let Some(pid) = current.parent_id.as_deref() else {
            return false;
        };
        if !visited.insert(current.id.as_str()) {
            // 走过同一节点 → 循环
            return true;
        }
        match by_id.get(pid) {
            Some(parent) => current = parent,
            None => return false, // parent 在数据集外 → orphan,不是 cycle
        }
    }
    // 超过 max_steps 还没结束 → 一定有环
    true
}

/// 数据库同步版本存储适配器
pub struct DbVersionStore;

impl SyncVersionStore for DbVersionStore {
    fn get_sync_version(&self) -> Result<i64, SyncEngineError> {
        with_conn(|c| Ok(SyncMetaRepository::get_sync_version(c)?))
            .map_err(|e| SyncEngineError::VersionStore(e.to_string()))
    }

    fn set_sync_version(&self, version: i64) -> Result<(), SyncEngineError> {
        with_conn(|c| Ok(SyncMetaRepository::set_sync_version(c, version)?))
            .map_err(|e| SyncEngineError::VersionStore(e.to_string()))
    }

    fn update_sync_meta(&self, time: &str, platform: &str) -> Result<(), SyncEngineError> {
        with_conn(|c| Ok(SyncMetaRepository::update_sync_meta(c, time, platform)?))
            .map_err(|e| SyncEngineError::VersionStore(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_key() {
        let provider = SshSyncProvider::new();
        assert_eq!(provider.section_key(), "ssh");
    }

    #[test]
    fn test_sync_node_serialization_roundtrip() {
        let node = SyncNode {
            id: "n1".to_string(),
            parent_id: Some("p1".to_string()),
            kind: "folder".to_string(),
            name: "Prod".to_string(),
            sort_order: 0,
            is_collapsed: true,
        };
        let json = serde_json::to_string(&node).unwrap();
        let parsed: SyncNode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "n1");
        assert_eq!(parsed.parent_id, Some("p1".to_string()));
        assert_eq!(parsed.kind, "folder");
        assert_eq!(parsed.name, "Prod");
        assert_eq!(parsed.sort_order, 0);
        assert!(parsed.is_collapsed);
    }

    #[test]
    fn test_sync_server_serialization_with_secrets() {
        let server = SyncServer {
            node_id: "s1".to_string(),
            host: "example.com".to_string(),
            port: 22,
            username: "root".to_string(),
            auth_type: "password".to_string(),
            key_path: Some("/key".to_string()),
            startup_command: None,
            notes: Some("test".to_string()),
            password_encrypted: Some("enc123".to_string()),
            passphrase_encrypted: None,
            root_password_encrypted: Some("enc456".to_string()),
        };
        let json = serde_json::to_string(&server).unwrap();
        let parsed: SyncServer = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.node_id, "s1");
        assert_eq!(parsed.port, 22);
        assert_eq!(parsed.password_encrypted, Some("enc123".to_string()));
        assert_eq!(parsed.passphrase_encrypted, None);
        assert_eq!(parsed.root_password_encrypted, Some("enc456".to_string()));
    }

    #[test]
    fn test_sync_server_no_secrets() {
        let server = SyncServer {
            node_id: "s2".to_string(),
            host: "host".to_string(),
            port: 2222,
            username: "admin".to_string(),
            auth_type: "key".to_string(),
            key_path: None,
            startup_command: None,
            notes: None,
            password_encrypted: None,
            passphrase_encrypted: None,
            root_password_encrypted: None,
        };
        let json = serde_json::to_string(&server).unwrap();
        let parsed: SyncServer = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.password_encrypted, None);
        assert_eq!(parsed.passphrase_encrypted, None);
        assert_eq!(parsed.root_password_encrypted, None);
    }

    #[test]
    fn test_ssh_sync_data_roundtrip() {
        let data = SshSyncData {
            nodes: vec![
                SyncNode {
                    id: "n1".to_string(),
                    parent_id: None,
                    kind: "folder".to_string(),
                    name: "Root".to_string(),
                    sort_order: 0,
                    is_collapsed: false,
                },
            ],
            servers: vec![
                SyncServer {
                    node_id: "s1".to_string(),
                    host: "h".to_string(),
                    port: 22,
                    username: "u".to_string(),
                    auth_type: "password".to_string(),
                    key_path: None,
                    startup_command: None,
                    notes: None,
                    password_encrypted: Some("enc".to_string()),
                    passphrase_encrypted: None,
                    root_password_encrypted: None,
                },
            ],
        };
        let json = serde_json::to_string(&data).unwrap();
        let parsed: SshSyncData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.nodes.len(), 1);
        assert_eq!(parsed.servers.len(), 1);
        assert_eq!(parsed.nodes[0].id, "n1");
        assert_eq!(parsed.servers[0].password_encrypted, Some("enc".to_string()));
    }

    #[test]
    fn test_ssh_sync_data_default_empty() {
        let data = SshSyncData::default();
        assert!(data.nodes.is_empty());
        assert!(data.servers.is_empty());
    }

    #[test]
    fn test_sync_node_null_parent() {
        let node = SyncNode {
            id: "root".to_string(),
            parent_id: None,
            kind: "folder".to_string(),
            name: "R".to_string(),
            sort_order: 0,
            is_collapsed: false,
        };
        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("\"parent_id\":null"), "parent_id=None 应序列化为 null");
        let parsed: SyncNode = serde_json::from_str(&json).unwrap();
        assert!(parsed.parent_id.is_none());
    }
}
