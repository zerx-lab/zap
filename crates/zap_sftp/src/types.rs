//! SFTP 协议层公共类型定义
//!
//! 定义文件类型、元数据、打开选项、重命名选项、目录条目等类型，
//! 提供从 ssh2 原始类型到高层类型的转换。
//! author: logic
//! date: 2026-05-31

use std::path::PathBuf;

/// 文件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Dir,
    File,
    Symlink,
    Other,
}

impl FileType {
    /// 从 unix 权限 mode 位解析文件类型
    pub fn from_mode(mode: u32) -> Self {
        match mode & 0o170000 {
            0o040000 => FileType::Dir,
            0o100000 => FileType::File,
            0o120000 => FileType::Symlink,
            _ => FileType::Other,
        }
    }
}

/// 文件权限（Unix 风格）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilePermissions {
    pub owner_read: bool,
    pub owner_write: bool,
    pub owner_exec: bool,
    pub group_read: bool,
    pub group_write: bool,
    pub group_exec: bool,
    pub other_read: bool,
    pub other_write: bool,
    pub other_exec: bool,
}

impl FilePermissions {
    /// 从 unix mode 位解析权限
    pub fn from_mode(mode: u32) -> Self {
        Self {
            owner_read: mode & 0o400 != 0,
            owner_write: mode & 0o200 != 0,
            owner_exec: mode & 0o100 != 0,
            group_read: mode & 0o040 != 0,
            group_write: mode & 0o020 != 0,
            group_exec: mode & 0o010 != 0,
            other_read: mode & 0o004 != 0,
            other_write: mode & 0o002 != 0,
            other_exec: mode & 0o001 != 0,
        }
    }
}

/// 文件元数据
#[derive(Debug, Clone)]
pub struct Metadata {
    pub file_type: FileType,
    pub permissions: FilePermissions,
    pub size: u64,
    pub uid: u32,
    pub gid: u32,
    pub accessed: Option<std::time::SystemTime>,
    pub modified: Option<std::time::SystemTime>,
}

impl Metadata {
    /// 从 ssh2::FileStat 创建
    pub fn from_ssh2(m: ssh2::FileStat) -> Self {
        let file_type = FileType::from_mode(m.perm.unwrap_or(0));
        Self {
            file_type,
            permissions: FilePermissions::from_mode(m.perm.unwrap_or(0)),
            size: m.size.unwrap_or(0),
            uid: m.uid.unwrap_or(0),
            gid: m.gid.unwrap_or(0),
            accessed: m
                .atime
                .map(|t| std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(t)),
            modified: m
                .mtime
                .map(|t| std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(t)),
        }
    }
}

/// 写入模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    Write,
    Append,
}

/// 打开文件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenFileType {
    File,
    Dir,
}

/// 文件打开选项
#[derive(Debug, Clone)]
pub struct OpenOptions {
    pub read: bool,
    pub write: Option<WriteMode>,
    pub create: bool,
    pub truncate: bool,
    pub mode: Option<u32>,
    pub file_type: OpenFileType,
}

impl OpenOptions {
    /// 只读模式
    pub fn read() -> Self {
        Self {
            read: true,
            write: None,
            create: false,
            truncate: false,
            mode: None,
            file_type: OpenFileType::File,
        }
    }

    /// 写入模式（创建+截断）
    pub fn write() -> Self {
        Self {
            read: false,
            write: Some(WriteMode::Write),
            create: true,
            truncate: true,
            mode: Some(0o644),
            file_type: OpenFileType::File,
        }
    }

    /// 追加模式
    pub fn append() -> Self {
        Self {
            read: false,
            write: Some(WriteMode::Append),
            create: true,
            truncate: false,
            mode: Some(0o644),
            file_type: OpenFileType::File,
        }
    }

    /// 创建新文件模式
    pub fn create_new() -> Self {
        Self {
            read: false,
            write: Some(WriteMode::Write),
            create: true,
            truncate: false,
            mode: Some(0o644),
            file_type: OpenFileType::File,
        }
    }
}

/// 重命名选项
#[derive(Debug, Clone, Default)]
pub struct RenameOptions {
    pub overwrite: bool,
    pub atomic: bool,
    pub native: bool,
}

/// 目录条目
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub metadata: Metadata,
}
