//! SFTP 会话管理模块
//!
//! 封装 SSH2 连接建立、认证和 SFTP 子系统通道创建。
//! author: logic
//! date: 2026-05-31

use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::error::SftpError;
use crate::sftp::Sftp;

/// 默认连接超时时间（10 秒）
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// 认证方式
#[derive(Debug, Clone)]
pub enum AuthMethod {
    Password {
        password: String,
    },
    PublicKey {
        key_path: PathBuf,
        passphrase: Option<String>,
    },
}

/// SFTP 会话，封装 ssh2 连接
pub struct SftpSession {
    session: Arc<ssh2::Session>,
    _tcp: TcpStream,
    /// 标记是否已显式断开连接，防止 Drop 双重 disconnect
    disconnected: Arc<AtomicBool>,
}

impl SftpSession {
    /// 通过指定参数建立 SSH 连接
    ///
    /// # 参数
    /// - `host`: 服务器地址
    /// - `port`: 服务器端口
    /// - `username`: 用户名
    /// - `auth`: 认证方式
    /// - `timeout`: 可选的超时时间，None 使用默认 10 秒
    pub fn connect(
        host: &str,
        port: u16,
        username: &str,
        auth: AuthMethod,
        timeout: Option<Duration>,
    ) -> Result<Self, SftpError> {
        let effective_timeout = timeout.unwrap_or(DEFAULT_TIMEOUT);
        let addr = format!("{host}:{port}");

        // 通过 ToSocketAddrs 进行 DNS 解析，支持主机名和 IP 地址
        let socket_addr = addr
            .to_socket_addrs()
            .map_err(|e| SftpError::ConnectionFailed(format!("地址解析失败: {e}")))?
            .next()
            .ok_or_else(|| SftpError::ConnectionFailed(format!("DNS 解析无结果: {addr}")))?;

        // 使用带超时的 TCP 连接
        let tcp = TcpStream::connect_timeout(&socket_addr, effective_timeout).map_err(|e| {
            if e.kind() == std::io::ErrorKind::TimedOut {
                SftpError::Timeout
            } else {
                SftpError::ConnectionFailed(format!("连接 {addr} 失败: {e}"))
            }
        })?;

        let mut session = ssh2::Session::new()
            .map_err(|e| SftpError::ConnectionFailed(format!("创建 SSH 会话失败: {e}")))?;

        let tcp_for_session = tcp
            .try_clone()
            .map_err(|e| SftpError::ConnectionFailed(format!("克隆 TCP 流失败: {e}")))?;
        session.set_tcp_stream(tcp_for_session);

        // 设置 SSH 会话超时（毫秒），影响 handshake 和后续所有阻塞操作
        session.set_timeout(effective_timeout.as_millis() as u32);

        session.handshake().map_err(|e| {
            if is_timeout_error(&e) {
                SftpError::Timeout
            } else {
                SftpError::ConnectionFailed(format!("SSH 握手失败: {e}"))
            }
        })?;

        match &auth {
            AuthMethod::Password { password } => {
                session.userauth_password(username, password).map_err(|e| {
                    if is_timeout_error(&e) {
                        SftpError::Timeout
                    } else {
                        SftpError::AuthFailed(format!("密码认证失败: {e}"))
                    }
                })?;
            }
            AuthMethod::PublicKey {
                key_path,
                passphrase,
            } => {
                let pass = passphrase.as_deref();
                session
                    .userauth_pubkey_file(username, None, key_path, pass)
                    .map_err(|e| {
                        if is_timeout_error(&e) {
                            SftpError::Timeout
                        } else {
                            SftpError::AuthFailed(format!("密钥认证失败: {e}"))
                        }
                    })?;
            }
        }

        if !session.authenticated() {
            return Err(SftpError::AuthFailed("认证未通过".into()));
        }

        // 设置操作超时（30秒），避免网络异常时操作无限阻塞
        session.set_timeout(30_000);

        Ok(Self {
            session: Arc::new(session),
            _tcp: tcp,
            disconnected: Arc::new(AtomicBool::new(false)),
        })
    }

    /// 获取 SFTP 通道
    pub fn sftp(&self) -> Result<Sftp, SftpError> {
        let sftp = self.session.sftp()?;
        Ok(Sftp::new(sftp))
    }

    /// 断开连接
    pub fn disconnect(&self) -> Result<(), SftpError> {
        if self.disconnected.swap(true, Ordering::SeqCst) {
            // 已经断开过，跳过
            return Ok(());
        }
        self.session.disconnect(None, "bye", None)?;
        Ok(())
    }

    /// 检查连接是否存活
    pub fn is_authenticated(&self) -> bool {
        self.session.authenticated()
    }
}

impl Drop for SftpSession {
    fn drop(&mut self) {
        if !self.disconnected.swap(true, Ordering::SeqCst) {
            let _ = self.session.disconnect(None, "bye", None);
        }
    }
}

/// 判断 ssh2 错误是否为超时错误
fn is_timeout_error(error: &ssh2::Error) -> bool {
    // ssh2 错误码 Session(-37) 对应 LIBSSH2_ERROR_SOCKET_TIMEOUT
    error.code() == ssh2::ErrorCode::Session(-37)
}
