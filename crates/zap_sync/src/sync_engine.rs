//! 同步引擎
//!
// author: logic
// date: 2026-05-24

use crate::types::SyncEngineError;

/// 同步数据提供者 trait（由调用方实现）
pub trait SyncDataProvider: Send + Sync {
    /// 收集需要同步的数据
    fn collect(&self) -> Result<serde_json::Map<String, serde_json::Value>, SyncEngineError>;
    /// 应用远程同步数据
    fn apply(&self, data: &serde_json::Map<String, serde_json::Value>) -> Result<(), SyncEngineError>;
}

/// 同步引擎（将在后续任务中实现）
pub struct SyncEngine {
    _placeholder: (),
}

impl SyncEngine {
    /// 占位构造函数
    pub fn new() -> Self {
        Self { _placeholder: () }
    }
}

impl Default for SyncEngine {
    fn default() -> Self {
        Self::new()
    }
}
