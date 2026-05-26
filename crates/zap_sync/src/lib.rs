//! zap_sync - 云同步通用基础设施
//!
// author: logic
// date: 2026-05-24

pub mod crypto;
pub mod gist_client;
pub mod sync_engine;
pub mod types;

pub use crypto::CryptoError;
pub use gist_client::{GistClient, GistClientError, GistOps};
pub use sync_engine::{SyncDataProvider, SyncEngine};
pub use types::*;
