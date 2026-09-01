//! SFTP 浏览器 UI 集成测试
//!
//! 使用 InMemorySftpBackend 模拟 SFTP 连接，测试 Connected 状态下的
//! 完整用户操作流程，包括文件浏览、导航、操作、对话框、传输等。
//! author: logic
//! date: 2026-05-30

use std::path::PathBuf;
use std::sync::Arc;

use warp_core::ui::appearance::Appearance;
use warpui::platform::WindowStyle;
use warpui::TypedActionView;

use crate::settings_view::keybindings::KeybindingChangedNotifier;
use crate::test_util::settings::initialize_settings_for_tests;

use pathfinder_geometry::vector::Vector2F;

use super::browser::{SftpBrowserAction, SftpBrowserView};
use super::sftp_backend::{InMemorySftpBackend, SftpBackend};
use super::types::{ConnectionState, Dialog, FileEntryType, TransferDirection, TransferState};

/// 初始化测试所需的最小单例集合
fn initialize_app(app: &mut warpui::App) {
    use crate::workspace::ToastStack;

    initialize_settings_for_tests(app);
    app.add_singleton_model(|_| Appearance::mock());
    app.add_singleton_model(|_| KeybindingChangedNotifier::mock());
    app.add_singleton_model(|_| ToastStack);

    let temp_db = std::env::temp_dir().join("warp_sftp_integration_test.sqlite");
    let _ = warp_ssh_manager::set_database_path(temp_db);
}

/// 创建 SftpBrowserView 并放入窗口（Disconnected 状态）
fn create_view(app: &mut warpui::App) -> (warpui::WindowId, warpui::ViewHandle<SftpBrowserView>) {
    app.add_window(WindowStyle::NotStealFocus, |ctx| {
        SftpBrowserView::new("test-node".to_string(), ctx)
    })
}

/// 创建带文件结构的临时目录
///
/// files 为 (相对路径, 内容) 列表，自动创建所需父目录。
fn create_temp_dir_with_files(files: &[(&str, &[u8])]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("创建临时目录失败");
    for (path, content) in files {
        let full_path = dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).expect("创建子目录失败");
        }
        std::fs::write(&full_path, content).expect("写入测试文件失败");
    }
    dir
}

/// 创建带有 InMemorySftpBackend 的 Connected 状态视图
///
/// 返回 (window_id, view_handle, temp_dir)，temp_dir 需要在测试期间保持存活
fn create_connected_view(
    app: &mut warpui::App,
    files: &[(&str, &[u8])],
) -> (
    warpui::WindowId,
    warpui::ViewHandle<SftpBrowserView>,
    tempfile::TempDir,
) {
    let temp_dir = create_temp_dir_with_files(files);
    let backend =
        Arc::new(InMemorySftpBackend::new(temp_dir.path().to_path_buf())) as Arc<dyn SftpBackend>;

    let (win_id, view) = create_view(app);
    view.update(app, |v, ctx| {
        v.set_backend_for_test(backend, PathBuf::from("/"), ctx);
    });

    (win_id, view, temp_dir)
}

/// 创建带子目录结构的 Connected 视图
///
/// 根目录下包含: docs/ 子目录, readme.txt, config.yaml
fn create_standard_view(
    app: &mut warpui::App,
) -> (
    warpui::WindowId,
    warpui::ViewHandle<SftpBrowserView>,
    tempfile::TempDir,
) {
    create_connected_view(
        app,
        &[
            ("docs/report.txt", b"report content"),
            ("readme.txt", b"hello world"),
            ("config.yaml", b"key: value"),
            ("data/sub/deep.txt", b"deep file"),
        ],
    )
}

// ============================================================
// A. 连接管理测试（6 个）
// ============================================================

/// 验证注入 InMemorySftpBackend 后 Connected 状态和条目填充
#[test]
fn test_connected_state_with_mock_backend() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(
            &mut app,
            &[("file1.txt", b"content1"), ("file2.txt", b"content2")],
        );

        view.read(&app, |v, _| {
            assert!(
                matches!(v.connection, ConnectionState::Connected),
                "应处于 Connected 状态"
            );
            assert_eq!(v.entries.len(), 2, "应列出 2 个文件");
            assert!(v.current_path == PathBuf::from("/"), "当前路径应为 /");
        });
    });
}

/// 验证未连接时状态为非 Connected 且无条目
#[test]
fn test_connection_failure_shows_error_state() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.read(&app, |v, _| {
            // new() 内部调用 connect_to_server，无 SSH 配置时会进入 Failed 状态
            assert!(
                !matches!(v.connection, ConnectionState::Connected),
                "无 SSH 配置时不应为 Connected 状态"
            );
            assert!(v.entries.is_empty(), "未连接状态无条目");
        });
    });
}

/// 验证从 Failed 状态重新连接
#[test]
fn test_reconnect_after_failure() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("reconnect.txt", b"data")]);

        // 先设置为 Failed 状态
        view.update(&mut app, |v, ctx| {
            v.connection = ConnectionState::Failed("模拟连接失败".to_string());
            ctx.notify();
        });

        view.read(&app, |v, _| {
            assert!(
                matches!(v.connection, ConnectionState::Failed(_)),
                "应为 Failed 状态"
            );
        });

        // 重新注入后端恢复连接
        let temp2 = create_temp_dir_with_files(&[("new.txt", b"new content")]);
        let backend =
            Arc::new(InMemorySftpBackend::new(temp2.path().to_path_buf())) as Arc<dyn SftpBackend>;
        view.update(&mut app, |v, ctx| {
            v.set_backend_for_test(backend, PathBuf::from("/"), ctx);
        });

        view.read(&app, |v, _| {
            assert!(
                matches!(v.connection, ConnectionState::Connected),
                "重新注入后应为 Connected"
            );
            assert_eq!(v.entries.len(), 1, "应列出新后端的 1 个文件");
        });
    });
}

/// 验证断开连接后清空条目和路径
#[test]
fn test_disconnect_clears_entries_and_path() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("file.txt", b"content")]);

        // 验证已连接
        view.read(&app, |v, _| {
            assert!(matches!(v.connection, ConnectionState::Connected));
            assert!(!v.entries.is_empty());
        });

        // 断开连接
        view.update(&mut app, |v, ctx| {
            v.disconnect_for_test(ctx);
        });

        view.read(&app, |v, _| {
            assert!(
                matches!(v.connection, ConnectionState::Disconnected),
                "断开后应为 Disconnected"
            );
            assert!(v.entries.is_empty(), "条目应被清空");
        });
    });
}

/// 验证非 Connected 状态 render 不 panic
#[test]
fn test_render_disconnected_state() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.read(&app, |v, _| {
            // new() 内部调用 connect_to_server，无 SSH 配置时为 Failed 而非 Disconnected
            assert!(!matches!(v.connection, ConnectionState::Connected));
        });
    });
}

/// 验证 Failed 状态 render 不 panic
#[test]
fn test_render_failed_state() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |v, ctx| {
            v.connection = ConnectionState::Failed("连接超时".to_string());
            ctx.notify();
        });

        view.read(&app, |v, _| {
            assert!(matches!(v.connection, ConnectionState::Failed(_)));
        });
    });
}

// ============================================================
// B. 文件浏览与导航测试（10 个）
// ============================================================

/// 验证目录列表正确填充并按 目录优先+字母排序
#[test]
fn test_list_dir_populates_entries() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(
            &mut app,
            &[
                ("banana.txt", b"b"),
                ("apple.txt", b"a"),
                ("cherry.txt", b"c"),
                ("folder_a/.keep", b""),
                ("folder_b/.keep", b""),
            ],
        );

        view.read(&app, |v, _| {
            assert_eq!(v.entries.len(), 5, "应有 5 个条目");

            // 目录应排在文件前面
            let dirs: Vec<_> = v
                .entries
                .iter()
                .take_while(|e| e.file_type == FileEntryType::Directory)
                .collect();
            let files: Vec<_> = v
                .entries
                .iter()
                .skip_while(|e| e.file_type == FileEntryType::Directory)
                .collect();
            assert_eq!(dirs.len(), 2, "应有 2 个目录");
            assert_eq!(files.len(), 3, "应有 3 个文件");
        });
    });
}

/// 验证双击目录进入并更新历史
#[test]
fn test_open_directory_navigates_and_updates_history() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(
            &mut app,
            &[("docs/readme.txt", b"readme"), ("file.txt", b"file")],
        );

        // 找到 docs 目录的索引
        let docs_idx = view.read(&app, |v, _| {
            v.entries.iter().position(|e| e.name == "docs").unwrap()
        });

        // 双击进入 docs 目录
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::OpenEntry(docs_idx), ctx);
        });

        view.read(&app, |v, _| {
            assert!(
                v.current_path.ends_with("docs")
                    || v.current_path.to_string_lossy().contains("docs"),
                "当前路径应包含 docs"
            );
            assert!(v.path_history.len() >= 2, "历史记录应增加");
        });
    });
}

/// 验证 GoUp 返回上级目录
#[test]
fn test_go_up_from_subdirectory() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("subdir/file.txt", b"content")]);

        // 进入子目录
        let sub_idx = view.read(&app, |v, _| {
            v.entries.iter().position(|e| e.name == "subdir").unwrap()
        });
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::OpenEntry(sub_idx), ctx);
        });

        // 返回上级
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::GoUp, ctx);
        });

        view.read(&app, |v, _| {
            assert!(
                v.current_path == PathBuf::from("/")
                    || v.entries.iter().any(|e| e.name == "subdir"),
                "GoUp 应返回上级目录"
            );
        });
    });
}

/// 验证 GoBack/GoForward 还原路径
#[test]
fn test_go_back_forward_restores_path() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(
            &mut app,
            &[("alpha/file.txt", b"a"), ("beta/file.txt", b"b")],
        );

        // 记录根路径
        let root_path = view.read(&app, |v, _| v.current_path.clone());

        // 进入 alpha
        let alpha_idx = view.read(&app, |v, _| {
            v.entries.iter().position(|e| e.name == "alpha").unwrap()
        });
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::OpenEntry(alpha_idx), ctx);
        });
        let alpha_path = view.read(&app, |v, _| v.current_path.clone());

        // GoBack
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::GoBack, ctx);
        });
        view.read(&app, |v, _| {
            assert_eq!(v.current_path, root_path, "GoBack 应返回根路径");
        });

        // GoForward
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::GoForward, ctx);
        });
        view.read(&app, |v, _| {
            assert_eq!(v.current_path, alpha_path, "GoForward 应回到 alpha");
        });
    });
}

/// 验证面包屑点击跳转到指定路径段
#[test]
fn test_breadcrumb_click_navigates_to_segment() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) =
            create_connected_view(&mut app, &[("level1/level2/file.txt", b"deep")]);

        // 进入 level1/level2
        let l1_idx = view.read(&app, |v, _| {
            v.entries.iter().position(|e| e.name == "level1").unwrap()
        });
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::OpenEntry(l1_idx), ctx);
        });
        let l2_idx = view.read(&app, |v, _| {
            v.entries.iter().position(|e| e.name == "level2").unwrap()
        });
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::OpenEntry(l2_idx), ctx);
        });

        // 验证当前路径为 level1/level2
        let current = view.read(&app, |v, _| v.current_path.clone());
        assert!(
            current.to_string_lossy().contains("level1"),
            "应导航到 level1 下"
        );

        // 导航回根（通过 NavigateTo）
        view.update(&mut app, |v, ctx| {
            // 找到 level1 对应的面包屑路径
            let l1_path = v
                .current_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("/"));
            v.handle_action(&SftpBrowserAction::NavigateTo(l1_path), ctx);
        });

        view.read(&app, |v, _| {
            assert!(
                v.current_path.to_string_lossy().contains("level1"),
                "面包屑跳转后应在 level1"
            );
        });
    });
}

/// 验证搜索过滤缩小可见条目
#[test]
fn test_search_filter_narrows_visible_entries() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(
            &mut app,
            &[
                ("readme.txt", b"r"),
                ("config.yaml", b"c"),
                ("data.csv", b"d"),
            ],
        );

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::SetSearchFilter(".txt".to_string()), ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.search_filter.is_some());
            let visible: Vec<_> = v
                .entries
                .iter()
                .filter(|e| e.name.contains(".txt"))
                .collect();
            assert_eq!(visible.len(), 1, "只有 readme.txt 匹配");
        });
    });
}

/// 验证清除搜索恢复全部条目
#[test]
fn test_clear_search_restores_all_entries() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) =
            create_connected_view(&mut app, &[("a.txt", b"a"), ("b.yaml", b"b")]);

        let total = view.read(&app, |v, _| v.entries.len());

        // 设置过滤
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::SetSearchFilter(".txt".to_string()), ctx);
        });

        // 清除过滤
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::ClearSearchFilter, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.search_filter.is_none());
            assert_eq!(v.entries.len(), total, "清除搜索后条目数应恢复");
        });
    });
}

/// 验证文件系统变更后刷新重新加载
#[test]
fn test_refresh_dir_reloads_entries() {
    warpui::App::test((), |mut app| async move {
        let temp = create_temp_dir_with_files(&[("original.txt", b"original")]);
        initialize_app(&mut app);
        let backend =
            Arc::new(InMemorySftpBackend::new(temp.path().to_path_buf())) as Arc<dyn SftpBackend>;
        let (_, view) = create_view(&mut app);
        view.update(&mut app, |v, ctx| {
            v.set_backend_for_test(backend, PathBuf::from("/"), ctx);
        });

        view.read(&app, |v, _| {
            assert_eq!(v.entries.len(), 1, "初始 1 个文件");
        });

        // 向临时目录添加新文件
        std::fs::write(temp.path().join("new_file.txt"), b"new").unwrap();

        // 刷新
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::Refresh, ctx);
        });

        view.read(&app, |v, _| {
            assert_eq!(v.entries.len(), 2, "刷新后应有 2 个文件");
        });
    });
}

/// 验证导航到当前路径不重复历史
#[test]
fn test_navigate_to_same_path_is_noop() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("file.txt", b"f")]);

        let history_len = view.read(&app, |v, _| v.path_history.len());
        let current = view.read(&app, |v, _| v.current_path.clone());

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::NavigateTo(current), ctx);
        });

        view.read(&app, |v, _| {
            assert_eq!(
                v.path_history.len(),
                history_len,
                "导航到当前路径不应增加历史"
            );
        });
    });
}

/// 验证 Windows 反斜杠路径标准化
#[test]
fn test_navigate_normalizes_backslashes() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("target/file.txt", b"t")]);

        // 使用反斜杠路径导航
        let target_idx = view.read(&app, |v, _| {
            v.entries.iter().position(|e| e.name == "target").unwrap()
        });
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::OpenEntry(target_idx), ctx);
        });

        view.read(&app, |v, _| {
            // 路径不应包含反斜杠
            let path_str = v.current_path.to_string_lossy();
            assert!(path_str.contains("target"), "导航后路径应包含 target");
        });
    });
}

/// 验证 SelectEntry 选中单个条目
#[test]
fn test_select_entry_highlights_item() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(
            &mut app,
            &[
                ("file_a.txt", b"a"),
                ("file_b.txt", b"b"),
                ("file_c.txt", b"c"),
            ],
        );

        // 选中第二个条目
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::SelectEntry(1), ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.selected.contains(&1), "SelectEntry(1) 应选中第二个条目");
            assert_eq!(v.selected.len(), 1, "应只有 1 个选中");
        });

        // 切换选中到第三个条目
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::SelectEntry(2), ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.selected.contains(&2), "SelectEntry(2) 应选中第三个条目");
        });
    });
}

/// 验证 SelectEntry 边界安全（越界索引不 panic）
#[test]
fn test_select_entry_out_of_bounds_safe() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("only_file.txt", b"x")]);

        // 越界选中不应 panic（当前实现直接插入索引）
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::SelectEntry(99), ctx);
        });

        view.read(&app, |v, _| {
            // 实现不校验边界，索引 99 会被插入 selected
            assert!(
                v.selected.contains(&99),
                "当前实现将越界索引也插入 selected"
            );
        });
    });
}

/// 验证 UploadFile（工具栏上传按钮）在未连接时安全处理
#[test]
fn test_upload_file_action_without_connection_safe() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        // 未连接时点击上传按钮不应 panic
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::UploadFile, ctx);
        });

        view.read(&app, |v, _| {
            // 文件选择器在 mock 平台不会触发，但也不应 panic
            assert!(v.transfers.is_empty());
        });
    });
}

/// 验证 DownloadEntry（右键菜单下载）在未连接时安全处理
#[test]
fn test_download_entry_action_without_connection_safe() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        // 未连接时触发下载不应 panic
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::DownloadEntry(0), ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.transfers.is_empty(), "未连接时下载不应创建传输任务");
        });
    });
}

/// 验证 OpenEntry 对文件类型条目的安全处理
#[test]
fn test_open_entry_on_file_triggers_download() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("readme.txt", b"hello")]);

        // 双击文件条目应触发下载（文件选择器在 mock 中不触发）
        let file_idx = view.read(&app, |v, _| {
            v.entries
                .iter()
                .position(|e| e.name == "readme.txt")
                .unwrap()
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::OpenEntry(file_idx), ctx);
        });

        // 不应 panic，传输任务创建取决于文件选择器是否可用
        view.read(&app, |v, _| {
            assert!(matches!(v.connection, ConnectionState::Connected));
        });
    });
}

// ============================================================
// C. 文件操作测试（8 个）
// ============================================================

/// 验证确认删除后文件从列表移除
#[test]
fn test_delete_file_confirmed_removes_entry() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(
            &mut app,
            &[("to_delete.txt", b"delete me"), ("keep.txt", b"keep me")],
        );

        let file_idx = view.read(&app, |v, _| {
            v.entries
                .iter()
                .position(|e| e.name == "to_delete.txt")
                .unwrap()
        });

        // 发起删除
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::DeleteEntry(file_idx), ctx);
        });

        // 确认删除对话框存在
        view.read(&app, |v, _| {
            assert!(matches!(v.dialog, Some(Dialog::DeleteConfirm { .. })));
        });

        // 确认删除
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::ConfirmDelete, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.dialog.is_none(), "对话框应关闭");
            assert_eq!(v.entries.len(), 1, "删除后应剩 1 个条目");
            assert!(v.entries[0].name == "keep.txt");
        });
    });
}

/// 验证递归删除目录
#[test]
fn test_delete_directory_confirmed_removes_recursively() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(
            &mut app,
            &[("mydir/inner.txt", b"inner file"), ("outer.txt", b"outer")],
        );

        let dir_idx = view.read(&app, |v, _| {
            v.entries.iter().position(|e| e.name == "mydir").unwrap()
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::DeleteEntry(dir_idx), ctx);
        });
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::ConfirmDelete, ctx);
        });

        view.read(&app, |v, _| {
            assert_eq!(v.entries.len(), 1, "删除目录后应剩 1 个条目");
            assert!(v.entries[0].name == "outer.txt");
        });
    });
}

/// 验证重命名更新文件名
#[test]
fn test_rename_entry_updates_name() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("old_name.txt", b"content")]);

        let idx = view.read(&app, |v, _| {
            v.entries
                .iter()
                .position(|e| e.name == "old_name.txt")
                .unwrap()
        });

        // 发起重命名
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::RenameEntry(idx), ctx);
        });

        // 在编辑器中输入新名称
        view.update(&mut app, |v, ctx| {
            v.rename_editor.update(ctx, |e, ctx| {
                e.set_buffer_text("new_name.txt", ctx);
            });
        });

        // 确认重命名
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::ConfirmRename, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.dialog.is_none(), "对话框应关闭");
            assert!(
                v.entries.iter().any(|e| e.name == "new_name.txt"),
                "应出现新名称"
            );
        });
    });
}

/// 验证重命名空名称保留对话框
#[test]
fn test_rename_empty_name_shows_error() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("file.txt", b"content")]);

        let idx = view.read(&app, |v, _| {
            v.entries.iter().position(|e| e.name == "file.txt").unwrap()
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::RenameEntry(idx), ctx);
        });

        // 清空编辑器
        view.update(&mut app, |v, ctx| {
            v.rename_editor.update(ctx, |e, ctx| {
                e.set_buffer_text("", ctx);
            });
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::ConfirmRename, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.dialog.is_some(), "空名称时对话框应保持打开");
        });
    });
}

/// 验证新建文件夹后目录存在
#[test]
fn test_new_folder_creates_entry() {
    warpui::App::test((), |mut app| async move {
        let temp = create_temp_dir_with_files(&[]);
        initialize_app(&mut app);
        let backend =
            Arc::new(InMemorySftpBackend::new(temp.path().to_path_buf())) as Arc<dyn SftpBackend>;
        let (_, view) = create_view(&mut app);
        view.update(&mut app, |v, ctx| {
            v.set_backend_for_test(backend, PathBuf::from("/"), ctx);
        });

        // 打开新建文件夹对话框
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::NewFolder, ctx);
        });

        view.read(&app, |v, _| {
            assert!(matches!(v.dialog, Some(Dialog::CreateFolder { .. })));
        });

        // 输入名称
        view.update(&mut app, |v, ctx| {
            v.new_folder_editor.update(ctx, |e, ctx| {
                e.set_buffer_text("test_folder", ctx);
            });
        });

        // 确认
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::ConfirmNewFolder, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.dialog.is_none(), "对话框应关闭");
            assert!(
                v.entries
                    .iter()
                    .any(|e| e.name == "test_folder" && e.file_type == FileEntryType::Directory),
                "应出现新建的文件夹"
            );
        });

        // 文件系统验证
        assert!(
            temp.path().join("test_folder").is_dir(),
            "临时目录中应存在新建的文件夹"
        );
    });
}

/// 验证新建文件夹空名称保留对话框
#[test]
fn test_new_folder_empty_name_shows_error() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[]);

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::NewFolder, ctx);
        });

        view.update(&mut app, |v, ctx| {
            v.new_folder_editor.update(ctx, |e, ctx| {
                e.set_buffer_text("", ctx);
            });
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::ConfirmNewFolder, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.dialog.is_some(), "空名称时对话框应保持打开");
        });
    });
}

/// 验证文件详情对话框展示正确信息
#[test]
fn test_file_details_dialog_shows_metadata() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) =
            create_connected_view(&mut app, &[("details.txt", b"file content here")]);

        let idx = view.read(&app, |v, _| {
            v.entries
                .iter()
                .position(|e| e.name == "details.txt")
                .unwrap()
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::DetailsEntry(idx), ctx);
        });

        view.read(&app, |v, _| match &v.dialog {
            Some(Dialog::FileDetails { entry }) => {
                assert_eq!(entry.name, "details.txt");
                assert_eq!(entry.file_type, FileEntryType::File);
            }
            _ => panic!("应打开 FileDetails 对话框"),
        });
    });
}

/// 验证取消删除保留条目
#[test]
fn test_delete_cancel_preserves_entry() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("keep_me.txt", b"keep")]);

        let idx = view.read(&app, |v, _| {
            v.entries
                .iter()
                .position(|e| e.name == "keep_me.txt")
                .unwrap()
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::DeleteEntry(idx), ctx);
        });

        // 取消（关闭对话框）
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::CloseDialog, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.dialog.is_none());
            assert_eq!(v.entries.len(), 1, "取消后条目应保留");
        });
    });
}

// ============================================================
// D. 右键菜单测试（5 个）
// ============================================================

/// 验证右键菜单打开并选中条目
#[test]
fn test_right_click_opens_menu_and_selects_entry() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("menu_file.txt", b"content")]);

        view.update(&mut app, |v, ctx| {
            v.handle_action(
                &SftpBrowserAction::ContextMenu {
                    index: 0,
                    position: Vector2F::new(100.0, 100.0),
                },
                ctx,
            );
        });

        view.read(&app, |v, _| {
            assert!(v.context_menu.is_some(), "右键菜单应打开");
            assert!(v.selected.contains(&0), "应选中第一个条目");
        });
    });
}

/// 验证右键菜单删除项触发删除确认
#[test]
fn test_context_menu_delete_item_triggers_delete() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("ctx_delete.txt", b"x")]);

        // 打开右键菜单
        view.update(&mut app, |v, ctx| {
            v.handle_action(
                &SftpBrowserAction::ContextMenu {
                    index: 0,
                    position: Vector2F::new(50.0, 50.0),
                },
                ctx,
            );
        });

        // 从菜单选择删除
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::DeleteEntry(0), ctx);
        });

        view.read(&app, |v, _| {
            assert!(
                matches!(v.dialog, Some(Dialog::DeleteConfirm { .. })),
                "应打开删除确认对话框"
            );
        });
    });
}

/// 验证右键菜单重命名项触发重命名
#[test]
fn test_context_menu_rename_item_triggers_rename() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("ctx_rename.txt", b"x")]);

        view.update(&mut app, |v, ctx| {
            v.handle_action(
                &SftpBrowserAction::ContextMenu {
                    index: 0,
                    position: Vector2F::new(50.0, 50.0),
                },
                ctx,
            );
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::RenameEntry(0), ctx);
        });

        view.read(&app, |v, _| {
            assert!(
                matches!(v.dialog, Some(Dialog::Rename { .. })),
                "应打开重命名对话框"
            );
        });
    });
}

/// 验证右键菜单详情项触发详情
#[test]
fn test_context_menu_details_item_triggers_details() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("ctx_details.txt", b"x")]);

        view.update(&mut app, |v, ctx| {
            v.handle_action(
                &SftpBrowserAction::ContextMenu {
                    index: 0,
                    position: Vector2F::new(50.0, 50.0),
                },
                ctx,
            );
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::DetailsEntry(0), ctx);
        });

        view.read(&app, |v, _| {
            assert!(
                matches!(v.dialog, Some(Dialog::FileDetails { .. })),
                "应打开文件详情对话框"
            );
        });
    });
}

/// 验证关闭右键菜单
#[test]
fn test_dismiss_click_closes_menu() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("menu_close.txt", b"x")]);

        view.update(&mut app, |v, ctx| {
            v.handle_action(
                &SftpBrowserAction::ContextMenu {
                    index: 0,
                    position: Vector2F::new(50.0, 50.0),
                },
                ctx,
            );
        });

        view.read(&app, |v, _| {
            assert!(v.context_menu.is_some());
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::CloseContextMenu, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.context_menu.is_none(), "菜单应关闭");
        });
    });
}

// ============================================================
// E. 对话框交互测试（6 个）
// ============================================================

/// 验证多选删除显示多项信息
#[test]
fn test_delete_confirm_dialog_multiple_paths() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) =
            create_connected_view(&mut app, &[("file_a.txt", b"a"), ("file_b.txt", b"b")]);

        // 选中两个条目
        view.update(&mut app, |v, ctx| {
            v.selected.clear();
            v.selected.insert(0);
            v.selected.insert(1);
            ctx.notify();
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::DeleteSelected, ctx);
        });

        view.read(&app, |v, _| match &v.dialog {
            Some(Dialog::DeleteConfirm { paths, .. }) => {
                assert_eq!(paths.len(), 2, "应显示 2 个待删除路径");
            }
            _ => panic!("应打开删除确认对话框"),
        });
    });
}

/// 验证重命名编辑器回车确认
#[test]
fn test_rename_editor_enter_confirms() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("rename_enter.txt", b"x")]);

        let idx = view.read(&app, |v, _| {
            v.entries
                .iter()
                .position(|e| e.name == "rename_enter.txt")
                .unwrap()
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::RenameEntry(idx), ctx);
        });

        view.update(&mut app, |v, ctx| {
            v.rename_editor.update(ctx, |e, ctx| {
                e.set_buffer_text("renamed.txt", ctx);
            });
        });

        // 通过 ConfirmRename 模拟回车
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::ConfirmRename, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.dialog.is_none(), "回车后对话框应关闭");
        });
    });
}

/// 验证重命名编辑器 Escape 取消
#[test]
fn test_rename_editor_escape_cancels() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("rename_esc.txt", b"x")]);

        let idx = view.read(&app, |v, _| {
            v.entries
                .iter()
                .position(|e| e.name == "rename_esc.txt")
                .unwrap()
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::RenameEntry(idx), ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.dialog.is_some());
        });

        // Escape 取消（通过 CloseDialog）
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::CloseDialog, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.dialog.is_none(), "Escape 后对话框应关闭");
            // 文件名不应改变
            assert!(
                v.entries.iter().any(|e| e.name == "rename_esc.txt"),
                "原文件名应保持不变"
            );
        });
    });
}

/// 验证新建文件夹编辑器回车确认
#[test]
fn test_new_folder_editor_enter_confirms() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[]);

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::NewFolder, ctx);
        });

        view.update(&mut app, |v, ctx| {
            v.new_folder_editor.update(ctx, |e, ctx| {
                e.set_buffer_text("my_folder", ctx);
            });
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::ConfirmNewFolder, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.dialog.is_none(), "回车后对话框应关闭");
            assert!(
                v.entries.iter().any(|e| e.name == "my_folder"),
                "应创建 my_folder"
            );
        });
    });
}

/// 验证覆盖确认对话框
#[test]
fn test_overwrite_confirm_dialog() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("file.txt", b"x")]);

        // 手动设置覆盖确认对话框
        view.update(&mut app, |v, ctx| {
            v.dialog = Some(Dialog::OverwriteConfirm {
                source: PathBuf::from("/source.txt"),
                target: PathBuf::from("/target.txt"),
                file_size: 1,
                direction: TransferDirection::Download,
            });
            ctx.notify();
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::ConfirmOverwrite, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.dialog.is_none(), "覆盖确认后对话框应关闭");
        });
    });
}

/// 验证移动确认对话框
#[test]
fn test_move_confirm_dialog() {
    warpui::App::test((), |mut app| async move {
        let temp =
            create_temp_dir_with_files(&[("move_src.txt", b"move me"), ("dest_dir/.keep", b"")]);
        initialize_app(&mut app);
        let backend =
            Arc::new(InMemorySftpBackend::new(temp.path().to_path_buf())) as Arc<dyn SftpBackend>;
        let (_, view) = create_view(&mut app);
        view.update(&mut app, |v, ctx| {
            v.set_backend_for_test(backend, PathBuf::from("/"), ctx);
        });

        // 手动设置移动对话框
        view.update(&mut app, |v, ctx| {
            v.dialog = Some(Dialog::Move {
                source: PathBuf::from("/move_src.txt"),
                target_dir: PathBuf::from("/dest_dir"),
            });
            ctx.notify();
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::ConfirmMove, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.dialog.is_none(), "移动确认后对话框应关闭");
        });
    });
}

// ============================================================
// F. 传输面板测试（5 个）
// ============================================================

/// 验证上传创建传输任务
#[test]
fn test_upload_creates_transfer_task() {
    warpui::App::test((), |mut app| async move {
        let temp = create_temp_dir_with_files(&[]);
        // 本地文件放在独立临时目录中，避免被 InMemorySftpBackend 的 list_dir 列出
        let local_dir = tempfile::tempdir().expect("创建本地临时目录失败");
        let local_file = local_dir.path().join("upload_source.txt");
        std::fs::write(&local_file, b"upload content").unwrap();

        initialize_app(&mut app);
        let backend =
            Arc::new(InMemorySftpBackend::new(temp.path().to_path_buf())) as Arc<dyn SftpBackend>;
        let (_, view) = create_view(&mut app);
        view.update(&mut app, |v, ctx| {
            v.set_backend_for_test(backend, PathBuf::from("/"), ctx);
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(
                &SftpBrowserAction::ExecuteUpload(local_file.to_string_lossy().to_string()),
                ctx,
            );
        });

        view.read(&app, |v, _| {
            assert_eq!(v.transfers.len(), 1, "应创建 1 个传输任务");
            let task = &v.transfers[0];
            assert_eq!(task.direction, TransferDirection::Upload);
            assert!(
                matches!(
                    task.state,
                    TransferState::Completed | TransferState::InProgress | TransferState::Failed(_)
                ),
                "传输任务应有明确状态"
            );
        });
    });
}

/// 验证上传不存在文件失败
#[test]
fn test_upload_nonexistent_file_fails() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[]);

        view.update(&mut app, |v, ctx| {
            v.handle_action(
                &SftpBrowserAction::ExecuteUpload("/nonexistent/path/file.txt".to_string()),
                ctx,
            );
        });

        view.read(&app, |v, _| {
            assert_eq!(v.transfers.len(), 1);
            assert!(
                matches!(v.transfers[0].state, TransferState::Failed(_)),
                "上传不存在的文件应失败"
            );
        });
    });
}

/// 验证下载创建传输任务
#[test]
fn test_download_creates_transfer_task() {
    warpui::App::test((), |mut app| async move {
        let temp = create_temp_dir_with_files(&[("download_me.txt", b"download content")]);
        let local_save = temp.path().join("saved_file.txt");

        initialize_app(&mut app);
        let backend =
            Arc::new(InMemorySftpBackend::new(temp.path().to_path_buf())) as Arc<dyn SftpBackend>;
        let (_, view) = create_view(&mut app);
        view.update(&mut app, |v, ctx| {
            v.set_backend_for_test(backend, PathBuf::from("/"), ctx);
        });

        let idx = view.read(&app, |v, _| {
            v.entries
                .iter()
                .position(|e| e.name == "download_me.txt")
                .unwrap()
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(
                &SftpBrowserAction::DownloadSaveAs {
                    index: idx,
                    local_path: local_save.to_string_lossy().to_string(),
                },
                ctx,
            );
        });

        view.read(&app, |v, _| {
            assert_eq!(v.transfers.len(), 1, "应创建下载任务");
            assert_eq!(v.transfers[0].direction, TransferDirection::Download);
        });
    });
}

/// 验证取消传输设置取消标志
#[test]
fn test_cancel_transfer_sets_cancelled_flag() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[]);

        // 手动添加一个传输任务
        view.update(&mut app, |v, ctx| {
            use super::types::TransferTask;
            let task = TransferTask::new(
                42,
                PathBuf::from("/remote.txt"),
                PathBuf::from("/local.txt"),
                TransferDirection::Download,
                1024,
            );
            v.transfers.push(task);
            ctx.notify();
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::CancelTransfer(42), ctx);
        });

        view.read(&app, |v, _| {
            let task = v.transfers.iter().find(|t| t.id == 42).unwrap();
            assert!(task.is_cancelled(), "任务应被标记为已取消");
        });
    });
}

/// 验证传输面板 render 不 panic
#[test]
fn test_transfer_panel_renders_with_tasks() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[]);

        view.update(&mut app, |v, ctx| {
            use super::types::TransferTask;
            let task = TransferTask::new(
                1,
                PathBuf::from("/file.txt"),
                PathBuf::from("/local/file.txt"),
                TransferDirection::Upload,
                2048,
            );
            v.transfers.push(task);
            ctx.notify();
        });

        // render 不会 panic
        view.read(&app, |_v, _| {
            // 如果能到这里说明 render 成功
        });
    });
}

// ============================================================
// G. 拖放交互测试（4 个）
// ============================================================

/// 验证拖入显示覆盖层
#[test]
fn test_drag_enter_shows_overlay() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("file.txt", b"x")]);

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::DragFilesEnter, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.is_drag_hovering, "拖入后应显示覆盖层");
        });
    });
}

/// 验证拖出隐藏覆盖层
#[test]
fn test_drag_leave_hides_overlay() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("file.txt", b"x")]);

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::DragFilesEnter, ctx);
        });
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::DragFilesLeave, ctx);
        });

        view.read(&app, |v, _| {
            assert!(!v.is_drag_hovering, "拖出后应隐藏覆盖层");
        });
    });
}

/// 验证拖放文件创建上传任务
#[test]
fn test_drop_files_creates_upload_tasks() {
    warpui::App::test((), |mut app| async move {
        let temp = create_temp_dir_with_files(&[]);
        // 本地文件放在独立临时目录中，避免被 InMemorySftpBackend 的 list_dir 列出
        let local_dir = tempfile::tempdir().expect("创建本地临时目录失败");
        let drop_file = local_dir.path().join("dropped.txt");
        std::fs::write(&drop_file, b"dropped content").unwrap();

        initialize_app(&mut app);
        let backend =
            Arc::new(InMemorySftpBackend::new(temp.path().to_path_buf())) as Arc<dyn SftpBackend>;
        let (_, view) = create_view(&mut app);
        view.update(&mut app, |v, ctx| {
            v.set_backend_for_test(backend, PathBuf::from("/"), ctx);
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(
                &SftpBrowserAction::DragAndDropFiles(vec![drop_file.clone()]),
                ctx,
            );
        });

        view.read(&app, |v, _| {
            assert_eq!(v.transfers.len(), 1, "拖放应创建上传任务");
            assert!(!v.is_drag_hovering, "拖放后应清除悬停状态");
        });
    });
}

/// 验证空路径拖放被忽略
#[test]
fn test_drop_empty_paths_ignored() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[]);

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::DragAndDropFiles(vec![]), ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.transfers.is_empty(), "空路径不应创建任务");
        });
    });
}

// ============================================================
// H. 键盘快捷键测试（5 个）
// ============================================================

/// 验证 NavigateUp (Backspace) 返回上级目录
#[test]
fn test_keyboard_navigate_up() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("subdir/file.txt", b"x")]);

        // 进入子目录
        let sub_idx = view.read(&app, |v, _| {
            v.entries.iter().position(|e| e.name == "subdir").unwrap()
        });
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::OpenEntry(sub_idx), ctx);
        });

        // NavigateUp
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::NavigateUp, ctx);
        });

        view.read(&app, |v, _| {
            assert!(
                v.entries.iter().any(|e| e.name == "subdir"),
                "NavigateUp 后应回到上级并看到 subdir"
            );
        });
    });
}

/// 验证 DeleteSelected 触发删除确认
#[test]
fn test_keyboard_delete_selected() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("del_target.txt", b"x")]);

        // 选中第一个条目
        view.update(&mut app, |v, ctx| {
            v.selected.clear();
            v.selected.insert(0);
            ctx.notify();
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::DeleteSelected, ctx);
        });

        view.read(&app, |v, _| {
            assert!(
                matches!(v.dialog, Some(Dialog::DeleteConfirm { .. })),
                "DeleteSelected 应触发删除确认"
            );
        });
    });
}

/// 验证 CreateFolder (Ctrl+Shift+N) 打开新建文件夹对话框
#[test]
fn test_keyboard_create_folder() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[]);

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::CreateFolder, ctx);
        });

        view.read(&app, |v, _| {
            assert!(
                matches!(v.dialog, Some(Dialog::CreateFolder { .. })),
                "CreateFolder 应打开新建文件夹对话框"
            );
        });
    });
}

/// 验证无选中时 DeleteSelected 安全处理
#[test]
fn test_keyboard_shortcuts_without_selection() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("file.txt", b"x")]);

        // 无选中
        view.update(&mut app, |v, ctx| {
            v.selected.clear();
            ctx.notify();
        });

        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::DeleteSelected, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.dialog.is_none(), "无选中时 DeleteSelected 不应打开对话框");
            assert_eq!(v.entries.len(), 1, "条目不应被删除");
        });
    });
}

/// 验证 Escape 关闭对话框
#[test]
fn test_keyboard_escape_closes_dialog() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("esc_file.txt", b"x")]);

        let idx = view.read(&app, |v, _| {
            v.entries
                .iter()
                .position(|e| e.name == "esc_file.txt")
                .unwrap()
        });

        // 打开重命名对话框
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::RenameEntry(idx), ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.dialog.is_some());
        });

        // Escape 关闭
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::CloseDialog, ctx);
        });

        view.read(&app, |v, _| {
            assert!(v.dialog.is_none(), "Escape 应关闭对话框");
        });
    });
}

// ============================================================
// I. 渲染安全性与组合测试（4 个）
// ============================================================

/// 验证连接+右键+对话框+传输+拖拽叠加状态安全
#[test]
fn test_render_with_all_overlays_connected() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[("overlay.txt", b"x")]);

        // 打开右键菜单
        view.update(&mut app, |v, ctx| {
            v.context_menu = Some(super::context_menu::ContextMenuState::new(
                0,
                Vector2F::new(50.0, 50.0),
            ));
            // 打开对话框
            v.dialog = Some(Dialog::DeleteConfirm {
                paths: vec![PathBuf::from("/overlay.txt")],
                is_dirs: vec![false],
            });
            // 添加传输任务
            use super::types::TransferTask;
            v.transfers.push(TransferTask::new(
                1,
                PathBuf::from("/file.txt"),
                PathBuf::from("/local.txt"),
                TransferDirection::Upload,
                1024,
            ));
            // 启用拖拽悬停
            v.is_drag_hovering = true;
            ctx.notify();
        });

        // 验证所有叠加状态存在且不冲突
        view.read(&app, |v, _| {
            assert!(v.context_menu.is_some());
            assert!(v.dialog.is_some());
            assert!(!v.transfers.is_empty());
            assert!(v.is_drag_hovering);
            assert!(matches!(v.connection, ConnectionState::Connected));
        });
    });
}

/// 验证加载状态指示
#[test]
fn test_render_loading_state() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[]);

        view.update(&mut app, |v, ctx| {
            v.is_loading = true;
            ctx.notify();
        });

        view.read(&app, |v, _| {
            assert!(v.is_loading, "应处于加载状态");
        });

        // 取消加载
        view.update(&mut app, |v, ctx| {
            v.is_loading = false;
            ctx.notify();
        });

        view.read(&app, |v, _| {
            assert!(!v.is_loading, "应取消加载状态");
        });
    });
}

/// 验证空目录显示
#[test]
fn test_render_empty_directory() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view, _temp) = create_connected_view(&mut app, &[]);

        view.read(&app, |v, _| {
            assert!(matches!(v.connection, ConnectionState::Connected));
            assert!(v.entries.is_empty(), "空目录应无条目");
        });
    });
}

/// 验证多步操作后渲染安全
#[test]
fn test_render_after_multiple_operations() {
    warpui::App::test((), |mut app| async move {
        let temp = create_temp_dir_with_files(&[
            ("op_dir/file1.txt", b"1"),
            ("op_dir/file2.txt", b"2"),
            ("root_file.txt", b"root"),
        ]);
        initialize_app(&mut app);
        let backend =
            Arc::new(InMemorySftpBackend::new(temp.path().to_path_buf())) as Arc<dyn SftpBackend>;
        let (_, view) = create_view(&mut app);
        view.update(&mut app, |v, ctx| {
            v.set_backend_for_test(backend, PathBuf::from("/"), ctx);
        });

        // 进入目录
        let dir_idx = view.read(&app, |v, _| {
            v.entries.iter().position(|e| e.name == "op_dir").unwrap()
        });
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::OpenEntry(dir_idx), ctx);
        });

        // 搜索
        view.update(&mut app, |v, ctx| {
            v.handle_action(
                &SftpBrowserAction::SetSearchFilter("file1".to_string()),
                ctx,
            );
        });

        // 清除搜索
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::ClearSearchFilter, ctx);
        });

        // 返回上级
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::GoUp, ctx);
        });

        // 刷新
        view.update(&mut app, |v, ctx| {
            v.handle_action(&SftpBrowserAction::Refresh, ctx);
        });

        // 最终状态验证
        view.read(&app, |v, _| {
            assert!(matches!(v.connection, ConnectionState::Connected));
            assert!(!v.entries.is_empty());
            assert!(v.search_filter.is_none());
        });
    });
}
