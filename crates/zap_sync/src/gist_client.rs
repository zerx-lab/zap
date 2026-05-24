//! Gist API 客户端
//!
// author: logic
// date: 2026-05-24

/// Gist API 客户端错误
#[derive(Debug)]
pub struct GistClientError {
    pub message: String,
}

impl std::fmt::Display for GistClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for GistClientError {}

/// Gist API 客户端（将在后续任务中实现）
pub struct GistClient {
    _placeholder: (),
}

impl GistClient {
    /// 占位构造函数
    pub fn new() -> Self {
        Self { _placeholder: () }
    }
}

impl Default for GistClient {
    fn default() -> Self {
        Self::new()
    }
}
