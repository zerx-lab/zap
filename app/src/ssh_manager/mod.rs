//! SSH 管理器 UI(左侧 Tool Panel)。当前为骨架,内容待 Commit 2b 实现:
//! 树状文件夹/服务器列表 + 右侧详情表单。
//!
//! 数据层在独立 crate `warp_ssh_manager`(`crates/warp_ssh_manager/`)。

pub mod notifier;
pub mod onekey;
pub mod panel;
pub mod password_prompt;
pub mod secret_injector;
pub mod server_view;
pub mod startup_command_injector;
pub mod su_password_injector;

pub use notifier::{SshTreeChangedEvent, SshTreeChangedNotifier};
pub use panel::SshManagerPanel;
// Re-exports for downstream UI consumers (Commit 2b).
#[allow(unused_imports)]
pub use panel::{SshManagerPanelAction, SshManagerPanelEvent};
