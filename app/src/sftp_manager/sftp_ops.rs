//! SFTP 操作封装层
//!
//! 将 zap_sftp 协议层 API 封装为 UI 层可直接使用的高级操作。
//! author: logic
//! date: 2026-05-26

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use warp_ssh_manager::secrets::SshSecretStore;
use warp_ssh_manager::types::{AuthType, ResolvedSshAuth, SshServerInfo};
use warp_ssh_manager::SshRepository;
use zap_sftp::session::{AuthMethod, SftpSession};
use zap_sftp::types::OpenOptions;
use zap_sftp::Sftp;

use super::types::{FileEntry, FileEntryType};

/// SFTP 操作错误
#[derive(Debug)]
pub enum SftpOpsError {
    /// 连接错误
    Connection(String),
    /// 操作错误
    Operation(String),
    /// 本地 IO 错误
    LocalIo(String),
    /// 未找到凭据
    NoCredentials(String),
    /// 传输已取消
    Cancelled,
}

impl std::fmt::Display for SftpOpsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SftpOpsError::Connection(msg) => write!(f, "连接错误: {msg}"),
            SftpOpsError::Operation(msg) => write!(f, "操作错误: {msg}"),
            SftpOpsError::LocalIo(msg) => write!(f, "本地 IO 错误: {msg}"),
            SftpOpsError::NoCredentials(msg) => write!(f, "未找到凭据: {msg}"),
            SftpOpsError::Cancelled => write!(f, "传输已取消"),
        }
    }
}

impl From<zap_sftp::SftpError> for SftpOpsError {
    fn from(e: zap_sftp::SftpError) -> Self {
        SftpOpsError::Operation(e.to_string())
    }
}

impl From<std::io::Error> for SftpOpsError {
    fn from(e: std::io::Error) -> Self {
        SftpOpsError::LocalIo(e.to_string())
    }
}

/// 进度回调类型
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send>;

/// 连接超时时间
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// 使用服务器配置建立 SFTP 连接
pub fn connect_from_server(
    server: &SshServerInfo,
    secret_store: &dyn SshSecretStore,
) -> Result<SftpSession, SftpOpsError> {
    let resolved_auth = resolve_sftp_auth(server)?;
    let auth = build_auth_method(server, &resolved_auth, secret_store)?;
    SftpSession::connect(
        &server.host,
        server.port,
        &resolved_auth.username,
        auth,
        Some(CONNECT_TIMEOUT),
    )
    .map_err(|e| SftpOpsError::Connection(e.to_string()))
}

fn resolve_sftp_auth(server: &SshServerInfo) -> Result<ResolvedSshAuth, SftpOpsError> {
    warp_ssh_manager::with_conn(|conn| Ok(SshRepository::resolve_server_auth(conn, server)?))
        .map_err(|e| SftpOpsError::NoCredentials(format!("解析认证失败: {e}")))
}

/// 列出远程目录内容，转换为 UI 层 FileEntry
pub fn list_dir(sftp: &Sftp, path: &Path) -> Result<Vec<FileEntry>, SftpOpsError> {
    let entries = sftp.read_dir(path)?;
    let result = entries
        .into_iter()
        .map(|entry| {
            let file_type = match entry.metadata.file_type {
                zap_sftp::types::FileType::Dir => FileEntryType::Directory,
                zap_sftp::types::FileType::File => FileEntryType::File,
                zap_sftp::types::FileType::Symlink => FileEntryType::Symlink,
                zap_sftp::types::FileType::Other => FileEntryType::Other,
            };
            let modified = entry.metadata.modified.map(|t| {
                let datetime: chrono::DateTime<chrono::Local> = t.into();
                datetime.format("%Y-%m-%d %H:%M").to_string()
            });
            let perms = &entry.metadata.permissions;
            let owner = bool_to_rwx(perms.owner_read, perms.owner_write, perms.owner_exec);
            let group = bool_to_rwx(perms.group_read, perms.group_write, perms.group_exec);
            let other = bool_to_rwx(perms.other_read, perms.other_write, perms.other_exec);
            let permissions = Some(format!("{owner}{group}{other}"));
            FileEntry {
                name: entry.name,
                path: entry.path,
                file_type,
                size: entry.metadata.size,
                modified,
                permissions,
            }
        })
        .collect();
    Ok(result)
}

/// 删除远程文件
pub fn delete_file(sftp: &Sftp, path: &Path) -> Result<(), SftpOpsError> {
    sftp.remove_file(path)?;
    Ok(())
}

/// 递归删除远程目录
pub fn delete_dir_recursive(sftp: &Sftp, path: &Path) -> Result<(), SftpOpsError> {
    let entries = sftp.read_dir(path)?;
    for entry in entries {
        match entry.metadata.file_type {
            zap_sftp::types::FileType::Dir => {
                delete_dir_recursive(sftp, &entry.path)?;
            }
            zap_sftp::types::FileType::File
            | zap_sftp::types::FileType::Symlink
            | zap_sftp::types::FileType::Other => {
                sftp.remove_file(&entry.path)?;
            }
        }
    }
    sftp.remove_dir(path)?;
    Ok(())
}

/// 创建远程目录
pub fn create_dir(sftp: &Sftp, path: &Path) -> Result<(), SftpOpsError> {
    sftp.create_dir(path)?;
    Ok(())
}

/// 重命名远程文件或目录
pub fn rename(sftp: &Sftp, old_path: &Path, new_path: &Path) -> Result<(), SftpOpsError> {
    let opts = zap_sftp::types::RenameOptions {
        overwrite: false,
        atomic: false,
        native: false,
    };
    sftp.rename(old_path, new_path, opts)?;
    Ok(())
}

/// 流式上传本地文件到远程
///
/// 使用临时文件模式：先上传到 .sftp_partial 后缀的临时路径，
/// 完成后 rename 到目标路径，取消或失败时清理临时文件，
/// 避免截断已有远程文件导致数据丢失。
pub fn upload_file_streaming(
    sftp: &Sftp,
    local_path: &Path,
    remote_path: &Path,
    progress_cb: Option<&ProgressCallback>,
    cancel_flag: &AtomicBool,
) -> Result<(), SftpOpsError> {
    let mut local_file =
        fs::File::open(local_path).map_err(|e| SftpOpsError::LocalIo(e.to_string()))?;
    let total_size = local_file.metadata().map(|m| m.len()).unwrap_or(0);

    // 使用临时路径上传，避免截断已有文件
    let remote_display = remote_path.display();
    let temp_remote_path = PathBuf::from(format!("{remote_display}.sftp_partial"));
    let mut remote_file = sftp.open(&temp_remote_path, OpenOptions::write())?;

    const CHUNK_SIZE: usize = 32 * 1024;
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut transferred: u64 = 0;

    let result = (|| -> Result<(), SftpOpsError> {
        loop {
            if cancel_flag.load(Ordering::SeqCst) {
                return Err(SftpOpsError::Cancelled);
            }
            let n = std::io::Read::read(&mut local_file, &mut buf)
                .map_err(|e| SftpOpsError::LocalIo(e.to_string()))?;
            if n == 0 {
                break;
            }
            remote_file.write_all(&buf[..n])?;
            transferred += n as u64;
            if let Some(cb) = progress_cb {
                cb(transferred, total_size);
            }
        }
        remote_file.flush()?;
        Ok(())
    })();

    match &result {
        Ok(()) => {
            // 上传成功：rename 临时文件到目标路径
            let rename_result = sftp.rename(
                &temp_remote_path,
                remote_path,
                zap_sftp::types::RenameOptions {
                    overwrite: true,
                    atomic: false,
                    native: false,
                },
            );

            // 部分服务器不支持 OVERWRITE 标志，使用备份重命名策略避免数据丢失
            let rename_result = match rename_result {
                Ok(()) => Ok(()),
                Err(_) => {
                    let remote_display = remote_path.display();
                    let backup_path = PathBuf::from(format!("{remote_display}.sftp_backup"));
                    let backup_created = sftp
                        .rename(
                            remote_path,
                            &backup_path,
                            zap_sftp::types::RenameOptions {
                                overwrite: false,
                                atomic: false,
                                native: false,
                            },
                        )
                        .is_ok();

                    match sftp.rename(
                        &temp_remote_path,
                        remote_path,
                        zap_sftp::types::RenameOptions {
                            overwrite: false,
                            atomic: false,
                            native: false,
                        },
                    ) {
                        Ok(()) => {
                            if backup_created {
                                let _ = sftp.remove_file(&backup_path);
                            }
                            Ok(())
                        }
                        Err(e) => {
                            // 重命名失败：恢复备份
                            if backup_created {
                                let _ = sftp.rename(
                                    &backup_path,
                                    remote_path,
                                    zap_sftp::types::RenameOptions {
                                        overwrite: false,
                                        atomic: false,
                                        native: false,
                                    },
                                );
                            }
                            Err(e)
                        }
                    }
                }
            };

            if let Err(e) = rename_result {
                // rename 失败时保留远程临时文件，避免数据丢失
                let temp_display = temp_remote_path.display();
                return Err(SftpOpsError::Operation(format!(
                    "重命名远程临时文件失败: {e}。临时文件: {temp_display}"
                )));
            }
        }
        Err(_) => {
            // 取消或失败：清理临时文件
            let _ = sftp.remove_file(&temp_remote_path);
        }
    }

    result
}

/// 流式下载远程文件到本地
///
/// 使用临时文件模式：先写入 .sftp_partial 后缀的临时文件，
/// 完成后 rename 到目标路径，取消或失败时清理临时文件，
/// 避免截断已有本地文件导致数据丢失。
pub fn download_file_streaming(
    sftp: &Sftp,
    remote_path: &Path,
    local_path: &Path,
    progress_cb: Option<&ProgressCallback>,
    cancel_flag: &AtomicBool,
) -> Result<(), SftpOpsError> {
    let mut remote_file = sftp.open(remote_path, OpenOptions::read())?;
    let metadata = remote_file.stat()?;
    let total_size = metadata.size;

    if let Some(parent) = local_path.parent() {
        fs::create_dir_all(parent).map_err(|e| SftpOpsError::LocalIo(e.to_string()))?;
    }

    // 使用临时路径下载，避免截断已有文件
    let local_display = local_path.display();
    let temp_local_path = PathBuf::from(format!("{local_display}.sftp_partial"));
    let mut local_file =
        fs::File::create(&temp_local_path).map_err(|e| SftpOpsError::LocalIo(e.to_string()))?;

    const CHUNK_SIZE: usize = 32 * 1024;
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut transferred: u64 = 0;

    let result = (|| -> Result<(), SftpOpsError> {
        loop {
            if cancel_flag.load(Ordering::SeqCst) {
                return Err(SftpOpsError::Cancelled);
            }
            let n = remote_file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            local_file
                .write_all(&buf[..n])
                .map_err(|e| SftpOpsError::LocalIo(e.to_string()))?;
            transferred += n as u64;
            if let Some(cb) = progress_cb {
                cb(transferred, total_size);
            }
        }
        local_file
            .flush()
            .map_err(|e| SftpOpsError::LocalIo(e.to_string()))?;
        Ok(())
    })();

    match &result {
        Ok(()) => {
            // 下载成功：rename 临时文件到目标路径
            if let Err(e) = fs::rename(&temp_local_path, local_path) {
                // rename 失败时保留本地临时文件，避免数据丢失
                let temp_display = temp_local_path.display();
                return Err(SftpOpsError::LocalIo(format!(
                    "重命名失败: {e}。已下载的临时文件保留在: {temp_display}"
                )));
            }
        }
        Err(_) => {
            // 取消或失败：清理临时文件
            let _ = fs::remove_file(&temp_local_path);
        }
    }

    result
}

/// 递归上传本地目录到远程
pub fn upload_dir_recursive(
    sftp: &Sftp,
    local_dir: &Path,
    remote_dir: &Path,
    progress_cb: Option<&ProgressCallback>,
    cancel_flag: &AtomicBool,
) -> Result<(), SftpOpsError> {
    if cancel_flag.load(Ordering::SeqCst) {
        return Err(SftpOpsError::Cancelled);
    }

    sftp.create_dir(remote_dir)?;

    let entries = fs::read_dir(local_dir).map_err(|e| SftpOpsError::LocalIo(e.to_string()))?;

    for entry in entries {
        if cancel_flag.load(Ordering::SeqCst) {
            return Err(SftpOpsError::Cancelled);
        }

        let entry = entry.map_err(|e| SftpOpsError::LocalIo(e.to_string()))?;
        let file_name = entry.file_name();
        let remote_path = normalize_remote_path(&remote_dir.join(&file_name));

        let file_type = entry
            .file_type()
            .map_err(|e| SftpOpsError::LocalIo(e.to_string()))?;

        if file_type.is_dir() {
            upload_dir_recursive(sftp, &entry.path(), &remote_path, progress_cb, cancel_flag)?;
        } else {
            upload_file_streaming(sftp, &entry.path(), &remote_path, progress_cb, cancel_flag)?;
        }
    }

    Ok(())
}

/// 递归下载远程目录到本地
pub fn download_dir_recursive(
    sftp: &Sftp,
    remote_dir: &Path,
    local_dir: &Path,
    progress_cb: Option<&ProgressCallback>,
    cancel_flag: &AtomicBool,
) -> Result<(), SftpOpsError> {
    if cancel_flag.load(Ordering::SeqCst) {
        return Err(SftpOpsError::Cancelled);
    }

    fs::create_dir_all(local_dir).map_err(|e| SftpOpsError::LocalIo(e.to_string()))?;

    let entries = sftp.read_dir(remote_dir)?;

    for entry in entries {
        if cancel_flag.load(Ordering::SeqCst) {
            return Err(SftpOpsError::Cancelled);
        }

        // 路径遍历防护：验证远程服务器返回的文件名安全性
        if entry.name.is_empty()
            || entry.name.starts_with('/')
            || entry.name.starts_with('\\')
            || entry.name.contains("..")
            || entry.name.contains('/')
            || entry.name.contains('\\')
        {
            continue;
        }

        let safe_remote_path = normalize_remote_path(&remote_dir.join(&entry.name));
        let local_path = local_dir.join(&entry.name);

        match entry.metadata.file_type {
            zap_sftp::types::FileType::Dir => {
                download_dir_recursive(
                    sftp,
                    &safe_remote_path,
                    &local_path,
                    progress_cb,
                    cancel_flag,
                )?;
            }
            zap_sftp::types::FileType::File
            | zap_sftp::types::FileType::Symlink
            | zap_sftp::types::FileType::Other => {
                download_file_streaming(
                    sftp,
                    &safe_remote_path,
                    &local_path,
                    progress_cb,
                    cancel_flag,
                )?;
            }
        }
    }

    Ok(())
}

/// 根据服务器配置构建认证方式
fn build_auth_method(
    server: &SshServerInfo,
    resolved_auth: &ResolvedSshAuth,
    secret_store: &dyn SshSecretStore,
) -> Result<AuthMethod, SftpOpsError> {
    match resolved_auth.auth_type {
        AuthType::Password | AuthType::OneKey => {
            let password = secret_store
                .get(&resolved_auth.secret_lookup_id, resolved_auth.secret_kind)
                .map_err(|e| SftpOpsError::NoCredentials(format!("读取密码失败: {e}")))?
                .ok_or_else(|| {
                    SftpOpsError::NoCredentials(format!("服务器 {} 未存储密码", server.host))
                })?;
            Ok(AuthMethod::Password {
                password: password.to_string(),
            })
        }
        AuthType::Key => {
            let key_path = resolved_auth.key_path.as_ref().ok_or_else(|| {
                SftpOpsError::NoCredentials("密钥认证但未指定密钥路径".to_string())
            })?;
            let expanded = shellexpand_path(key_path);
            let passphrase = secret_store
                .get(&resolved_auth.secret_lookup_id, resolved_auth.secret_kind)
                .ok()
                .flatten()
                .map(|p| p.to_string());
            Ok(AuthMethod::PublicKey {
                key_path: PathBuf::from(expanded),
                passphrase,
            })
        }
    }
}

/// 展开路径中的 ~ 为用户主目录
fn shellexpand_path(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            let home_path = home.display();
            let suffix = &path[2..];
            return format!("{home_path}/{suffix}");
        }
    }
    path.to_string()
}

/// 将读/写/执行布尔值转换为 rwx 权限字符串
pub(crate) fn bool_to_rwx(read: bool, write: bool, exec: bool) -> String {
    let mut s = String::with_capacity(3);
    s.push(if read { 'r' } else { '-' });
    s.push(if write { 'w' } else { '-' });
    s.push(if exec { 'x' } else { '-' });
    s
}

/// 规范化远程路径，将 Windows 反斜杠替换为正斜杠
///
/// 远程服务器（Linux）只接受正斜杠路径分隔符，
/// 在 Windows 上 PathBuf::join 会产生反斜杠，必须转换。
pub(crate) fn normalize_remote_path(path: &PathBuf) -> PathBuf {
    PathBuf::from(path.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 SftpOpsError::Connection Display 输出
    #[test]
    fn test_sftp_ops_error_display_connection() {
        assert_eq!(
            SftpOpsError::Connection("refused".into()).to_string(),
            "连接错误: refused"
        );
    }

    /// 测试 SftpOpsError::Operation Display 输出
    #[test]
    fn test_sftp_ops_error_display_operation() {
        assert_eq!(
            SftpOpsError::Operation("not found".into()).to_string(),
            "操作错误: not found"
        );
    }

    /// 测试 SftpOpsError::LocalIo Display 输出
    #[test]
    fn test_sftp_ops_error_display_local_io() {
        assert_eq!(
            SftpOpsError::LocalIo("disk full".into()).to_string(),
            "本地 IO 错误: disk full"
        );
    }

    /// 测试 SftpOpsError::NoCredentials Display 输出
    #[test]
    fn test_sftp_ops_error_display_no_credentials() {
        assert_eq!(
            SftpOpsError::NoCredentials("no key".into()).to_string(),
            "未找到凭据: no key"
        );
    }

    /// 测试 SftpOpsError::Cancelled Display 输出
    #[test]
    fn test_sftp_ops_error_display_cancelled() {
        assert_eq!(SftpOpsError::Cancelled.to_string(), "传输已取消");
    }

    /// 测试从 std::io::Error 转换为 SftpOpsError
    #[test]
    fn test_sftp_ops_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let ops_err: SftpOpsError = io_err.into();
        assert!(matches!(ops_err, SftpOpsError::LocalIo(_)));
    }

    /// 测试从 zap_sftp::SftpError 转换为 SftpOpsError
    #[test]
    fn test_sftp_ops_error_from_sftp_error() {
        let sftp_err = zap_sftp::SftpError::General("test error".into());
        let ops_err: SftpOpsError = sftp_err.into();
        assert!(matches!(ops_err, SftpOpsError::Operation(_)));
    }

    /// 测试 shellexpand_path 展开 ~/ 路径
    #[test]
    fn test_shellexpand_path_home() {
        let home = dirs::home_dir().unwrap_or_default();
        let result = shellexpand_path("~/test");
        if !home.as_os_str().is_empty() {
            assert!(!result.starts_with('~'));
            assert!(result.contains("test"));
        }
    }

    /// 测试 shellexpand_path 不变绝对路径
    #[test]
    fn test_shellexpand_path_absolute() {
        let result = shellexpand_path("/absolute/path");
        assert_eq!(result, "/absolute/path");
    }

    /// 测试 shellexpand_path 不变相对路径
    #[test]
    fn test_shellexpand_path_relative() {
        let result = shellexpand_path("relative/path");
        assert_eq!(result, "relative/path");
    }

    /// 测试 shellexpand_path 仅 ~ 不展开
    #[test]
    fn test_shellexpand_path_tilde_only() {
        let result = shellexpand_path("~");
        assert_eq!(result, "~");
    }

    /// 测试 shellexpand_path 空路径
    #[test]
    fn test_shellexpand_path_empty() {
        let result = shellexpand_path("");
        assert_eq!(result, "");
    }

    // ==================== bool_to_rwx 测试 ====================

    /// 测试全部权限 rwx
    #[test]
    fn test_bool_to_rwx_all_true() {
        assert_eq!(bool_to_rwx(true, true, true), "rwx");
    }

    /// 测试全部无权限
    #[test]
    fn test_bool_to_rwx_all_false() {
        assert_eq!(bool_to_rwx(false, false, false), "---");
    }

    /// 测试仅读权限
    #[test]
    fn test_bool_to_rwx_read_only() {
        assert_eq!(bool_to_rwx(true, false, false), "r--");
    }

    /// 测试仅写权限
    #[test]
    fn test_bool_to_rwx_write_only() {
        assert_eq!(bool_to_rwx(false, true, false), "-w-");
    }

    /// 测试仅执行权限
    #[test]
    fn test_bool_to_rwx_exec_only() {
        assert_eq!(bool_to_rwx(false, false, true), "--x");
    }

    /// 测试读写权限
    #[test]
    fn test_bool_to_rwx_read_write() {
        assert_eq!(bool_to_rwx(true, true, false), "rw-");
    }

    /// 测试读执行权限
    #[test]
    fn test_bool_to_rwx_read_exec() {
        assert_eq!(bool_to_rwx(true, false, true), "r-x");
    }

    /// 测试写执行权限
    #[test]
    fn test_bool_to_rwx_write_exec() {
        assert_eq!(bool_to_rwx(false, true, true), "-wx");
    }

    /// 测试返回值长度始终为 3
    #[test]
    fn test_bool_to_rwx_length() {
        for r in [true, false] {
            for w in [true, false] {
                for x in [true, false] {
                    assert_eq!(bool_to_rwx(r, w, x).len(), 3);
                }
            }
        }
    }

    /// 测试每个位置字符只可能是目标字符
    #[test]
    fn test_bool_to_rwx_valid_chars() {
        for r in [true, false] {
            for w in [true, false] {
                for x in [true, false] {
                    let s = bool_to_rwx(r, w, x);
                    let chars: Vec<char> = s.chars().collect();
                    assert!((chars[0] == 'r') || (chars[0] == '-'));
                    assert!((chars[1] == 'w') || (chars[1] == '-'));
                    assert!((chars[2] == 'x') || (chars[2] == '-'));
                }
            }
        }
    }

    // ==================== SftpOpsError 边界场景测试 ====================

    /// 测试 SftpOpsError::Connection 空消息
    #[test]
    fn test_sftp_ops_error_connection_empty() {
        assert_eq!(
            SftpOpsError::Connection(String::new()).to_string(),
            "连接错误: "
        );
    }

    /// 测试 SftpOpsError::Operation 空消息
    #[test]
    fn test_sftp_ops_error_operation_empty() {
        assert_eq!(
            SftpOpsError::Operation(String::new()).to_string(),
            "操作错误: "
        );
    }

    /// 测试 SftpOpsError::LocalIo 空消息
    #[test]
    fn test_sftp_ops_error_local_io_empty() {
        assert_eq!(
            SftpOpsError::LocalIo(String::new()).to_string(),
            "本地 IO 错误: "
        );
    }

    /// 测试 SftpOpsError::NoCredentials 空消息
    #[test]
    fn test_sftp_ops_error_no_credentials_empty() {
        assert_eq!(
            SftpOpsError::NoCredentials(String::new()).to_string(),
            "未找到凭据: "
        );
    }

    /// 测试 SftpOpsError::Cancelled 始终为固定文本
    #[test]
    fn test_sftp_ops_error_cancelled_consistent() {
        let s1 = SftpOpsError::Cancelled.to_string();
        let s2 = SftpOpsError::Cancelled.to_string();
        assert_eq!(s1, s2);
        assert_eq!(s1, "传输已取消");
    }

    /// 测试 shellexpand_path 多级 ~/ 展开
    #[test]
    fn test_shellexpand_path_home_nested() {
        let result = shellexpand_path("~/a/b/c");
        assert!(!result.starts_with('~'));
        assert!(result.contains("a/b/c"));
    }

    /// 测试 shellexpand_path 仅 ~ 后跟 / 无附加路径
    #[test]
    fn test_shellexpand_path_home_root() {
        let result = shellexpand_path("~/");
        let home = dirs::home_dir().unwrap_or_default();
        if !home.as_os_str().is_empty() {
            assert!(!result.starts_with('~'));
        }
    }
}
