use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum NodeKind {
    Folder,
    Server,
}

impl NodeKind {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            NodeKind::Folder => "folder",
            NodeKind::Server => "server",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "folder" => Some(NodeKind::Folder),
            "server" => Some(NodeKind::Server),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AuthType {
    Password,
    Key,
}

impl AuthType {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            AuthType::Password => "password",
            AuthType::Key => "key",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "password" => Some(AuthType::Password),
            "key" => Some(AuthType::Key),
            _ => None,
        }
    }
}

/// 树节点(folder 或 server),不含 server-only metadata。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SshNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub kind: NodeKind,
    pub name: String,
    pub sort_order: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    /// 仅对 folder 有意义,UI 据此决定是否隐藏子节点。SQLite 持久化让重启后
    /// 状态保持。
    pub is_collapsed: bool,
}

/// Server 节点的连接配置。`password` / `passphrase` 不在此处 — 走 keychain。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SshServerInfo {
    pub node_id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: AuthType,
    /// auth_type=Key 时使用,绝对路径或 `~` 开头(由调用方 `shellexpand`)。
    pub key_path: Option<String>,
    pub startup_command: Option<String>,
    pub notes: Option<String>,
    pub last_connected_at: Option<NaiveDateTime>,
}

impl SshServerInfo {
    pub fn new_default(node_id: String) -> Self {
        Self {
            node_id,
            host: String::new(),
            port: 22,
            username: String::new(),
            auth_type: AuthType::Password,
            key_path: None,
            startup_command: None,
            notes: None,
            last_connected_at: None,
        }
    }
}
