//! SFTP 集成测试辅助函数
//!
//! 提供 SFTP 浏览器视图获取、mock 后端创建、面板打开与注入等辅助功能。
//! author: logic
//! date: 2026-05-30

use std::path::PathBuf;
use std::sync::Arc;

use warpui::{App, ViewHandle, WindowId};

use crate::sftp_manager::browser::SftpBrowserView;
use crate::sftp_manager::sftp_backend::{InMemorySftpBackend, SftpBackend};

// 重新导出，供集成测试通过 warp::integration_testing::sftp 使用
pub use crate::sftp_manager::browser::SftpBrowserAction;
pub use crate::sftp_manager::types::{ConnectionState, Dialog};

/// 获取 SFTP 浏览器视图句柄
///
/// 在指定窗口中查找 SftpBrowserView 实例。
/// author: logic
/// date: 2026-05-30
pub fn sftp_browser_view(app: &App, window_id: WindowId) -> ViewHandle<SftpBrowserView> {
    let views: Vec<ViewHandle<SftpBrowserView>> = app
        .views_of_type(window_id)
        .expect("should have views for window");
    views
        .into_iter()
        .next()
        .expect("should have at least one SFTP browser view")
}

/// 创建带预设文件结构的临时目录和 mock 后端
///
/// files 为 (相对路径, 内容) 列表，自动创建所需父目录。
/// author: logic
/// date: 2026-05-30
pub fn create_mock_backend(files: &[(&str, &[u8])]) -> (tempfile::TempDir, Arc<dyn SftpBackend>) {
    let temp_dir = tempfile::tempdir().expect("创建临时目录失败");
    for (path, content) in files {
        let full_path = temp_dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).expect("创建子目录失败");
        }
        std::fs::write(&full_path, content).expect("写入测试文件失败");
    }
    let backend =
        Arc::new(InMemorySftpBackend::new(temp_dir.path().to_path_buf())) as Arc<dyn SftpBackend>;
    (temp_dir, backend)
}

/// 打开 SFTP 面板并注入 mock 后端
///
/// 返回 (window_id, temp_dir)，temp_dir 需要在测试期间保持存活。
/// author: logic
/// date: 2026-05-30
pub fn open_sftp_pane_with_mock(
    app: &mut App,
    files: &[(&str, &[u8])],
) -> (WindowId, tempfile::TempDir) {
    let window_id = app.read(|ctx| ctx.windows().active_window().expect("应有活跃窗口"));

    let workspace = super::view_getters::workspace_view(app, window_id);
    app.update(|ctx| {
        workspace.update(ctx, |ws, ctx| {
            ws.open_sftp_pane("__mock_sftp_test__".to_string(), ctx);
        });
    });

    let (temp_dir, backend) = create_mock_backend(files);
    let view = sftp_browser_view(app, window_id);
    view.update(app, |v, ctx| {
        v.inject_mock_backend(backend, PathBuf::from("/"), ctx);
    });

    (window_id, temp_dir)
}
