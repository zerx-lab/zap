//! SFTP 浏览器视图 UI 单元测试
//!
//! 验证视图状态管理、Action 处理逻辑。使用 App::test() + mock 平台，
//! 不依赖真实 SSH 连接（视图初始为 Disconnected 状态）。
//! author: logic
//! date: 2026-05-27

use std::path::PathBuf;

use warp_core::ui::appearance::Appearance;
use warpui::platform::WindowStyle;
use warpui::TypedActionView;

use crate::settings_view::keybindings::KeybindingChangedNotifier;
use crate::test_util::settings::initialize_settings_for_tests;

use pathfinder_geometry::vector::Vector2F;

use super::browser::{SftpBrowserAction, SftpBrowserView};
use super::types::{ConnectionState, Dialog, TransferDirection, TransferState};
use crate::editor::EditorView;

/// 初始化测试所需的最小单例集合
fn initialize_app(app: &mut warpui::App) {
    use crate::workspace::ToastStack;

    initialize_settings_for_tests(app);
    app.add_singleton_model(|_| Appearance::mock());
    app.add_singleton_model(|_| KeybindingChangedNotifier::mock());
    app.add_singleton_model(|_| ToastStack);

    // SSH 管理器需要一个 SQLite 路径；使用临时文件，查询失败不 panic
    let temp_db = std::env::temp_dir().join("warp_sftp_test.sqlite");
    let _ = warp_ssh_manager::set_database_path(temp_db);
}

/// 创建 SftpBrowserView 并放入窗口
///
/// 视图初始状态为 Disconnected（无 SSH 连接），不影响 UI 状态逻辑测试。
fn create_view(app: &mut warpui::App) -> (warpui::WindowId, warpui::ViewHandle<SftpBrowserView>) {
    app.add_window(WindowStyle::NotStealFocus, |ctx| {
        SftpBrowserView::new("test-node".to_string(), ctx)
    })
}

// ============================================================
// 拖拽状态测试
// ============================================================

/// 验证 DragFilesEnter 设置 is_drag_hovering 为 true
#[test]
fn test_drag_files_enter() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::DragFilesEnter, ctx);
        });

        view.read(&app, |view, _| {
            assert!(
                view.is_drag_hovering,
                "DragFilesEnter 后 is_drag_hovering 应为 true"
            );
        });
    });
}

/// 验证 DragFilesLeave 设置 is_drag_hovering 为 false
#[test]
fn test_drag_files_leave() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        // 先进入悬停状态
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::DragFilesEnter, ctx);
        });
        // 再离开
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::DragFilesLeave, ctx);
        });

        view.read(&app, |view, _| {
            assert!(
                !view.is_drag_hovering,
                "DragFilesLeave 后 is_drag_hovering 应为 false"
            );
        });
    });
}

/// 验证 DragAndDropFiles 重置 is_drag_hovering
#[test]
fn test_drag_and_drop_resets_hover() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        // 先进入悬停
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::DragFilesEnter, ctx);
        });
        // 释放文件（无 SFTP 连接，传输会失败但不崩溃）
        view.update(&mut app, |view, ctx| {
            view.handle_action(
                &SftpBrowserAction::DragAndDropFiles(vec![PathBuf::from("/tmp/test.txt")]),
                ctx,
            );
        });

        view.read(&app, |view, _| {
            assert!(
                !view.is_drag_hovering,
                "DragAndDropFiles 后 is_drag_hovering 应重置为 false"
            );
        });
    });
}

// ============================================================
// 选择状态测试
// ============================================================

/// 验证 SelectEntry 选中条目
#[test]
fn test_select_entry() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::SelectEntry(0), ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.selected.contains(&0), "SelectEntry(0) 后应选中索引 0");
        });
    });
}

/// 验证 SelectEntry 切换选中（单选模式：再次选中同一项仍保持选中）
#[test]
fn test_toggle_select_entry() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        // 选中索引 2
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::SelectEntry(2), ctx);
        });
        view.read(&app, |view, _| {
            assert!(view.selected.contains(&2));
        });

        // 选中索引 5 → 清除之前的，只保留 5
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::SelectEntry(5), ctx);
        });
        view.read(&app, |view, _| {
            assert!(!view.selected.contains(&2), "SelectEntry(5) 后应取消选中 2");
            assert!(view.selected.contains(&5), "SelectEntry(5) 后应选中 5");
        });
    });
}

// ============================================================
// 搜索过滤测试
// ============================================================

/// 验证 SetSearchFilter 设置搜索文本
#[test]
fn test_set_search_filter() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::SetSearchFilter("txt".to_string()), ctx);
        });

        view.read(&app, |view, _| {
            assert_eq!(view.search_filter.as_deref(), Some("txt"));
        });
    });
}

/// 验证 ClearSearchFilter 清除搜索文本
#[test]
fn test_clear_search_filter() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        // 先设置
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::SetSearchFilter("log".to_string()), ctx);
        });
        // 再清除
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::ClearSearchFilter, ctx);
        });

        view.read(&app, |view, _| {
            assert!(
                view.search_filter.is_none(),
                "ClearSearchFilter 后应为 None"
            );
        });
    });
}

// ============================================================
// 导航测试
// ============================================================

/// 验证在根目录 NavigateUp 不改变路径
#[test]
fn test_navigate_up_from_root() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.read(&app, |view, _| {
            assert_eq!(view.current_path, PathBuf::from("/"));
        });

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::NavigateUp, ctx);
        });

        view.read(&app, |view, _| {
            assert_eq!(
                view.current_path,
                PathBuf::from("/"),
                "根目录 NavigateUp 应保持不变"
            );
        });
    });
}

// ============================================================
// 初始状态测试
// ============================================================

/// 验证视图初始状态正确
#[test]
fn test_initial_state() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.read(&app, |view, _| {
            assert!(view.entries.is_empty(), "初始条目列表应为空");
            assert!(view.selected.is_empty(), "初始选中集合应为空");
            assert!(view.transfers.is_empty(), "初始传输列表应为空");
            assert!(view.search_filter.is_none(), "初始搜索过滤应为 None");
            assert!(!view.is_drag_hovering, "初始拖拽悬停应为 false");
        });
    });
}

// ============================================================
// 右键菜单测试
// ============================================================

/// 验证 ContextMenu action 设置 context_menu 状态并选中条目
#[test]
fn test_context_menu_sets_state() {
    use pathfinder_geometry::vector::Vector2F;

    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        let position = Vector2F::new(100.0, 200.0);
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::ContextMenu { index: 3, position }, ctx);
        });

        view.read(&app, |view, _| {
            assert!(
                view.context_menu.is_some(),
                "ContextMenu 后 context_menu 应为 Some"
            );
            let cm = view.context_menu.as_ref().unwrap();
            assert_eq!(cm.entry_index, 3, "entry_index 应为 3");
            assert_eq!(cm.position, position, "position 应与传入值一致");
            assert!(view.selected.contains(&3), "ContextMenu 后应选中索引 3");
        });
    });
}

/// 验证 CloseContextMenu 清除 context_menu 状态
#[test]
fn test_close_context_menu_clears_state() {
    use pathfinder_geometry::vector::Vector2F;

    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        // 先打开右键菜单
        view.update(&mut app, |view, ctx| {
            view.handle_action(
                &SftpBrowserAction::ContextMenu {
                    index: 1,
                    position: Vector2F::new(50.0, 50.0),
                },
                ctx,
            );
        });
        view.read(&app, |view, _| {
            assert!(view.context_menu.is_some(), "应已打开菜单");
        });

        // 关闭菜单
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::CloseContextMenu, ctx);
        });

        view.read(&app, |view, _| {
            assert!(
                view.context_menu.is_none(),
                "CloseContextMenu 后 context_menu 应为 None"
            );
        });
    });
}

/// 验证 ContextMenu 会替换之前的菜单状态
#[test]
fn test_context_menu_replaces_previous() {
    use pathfinder_geometry::vector::Vector2F;

    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        // 打开第一个菜单
        view.update(&mut app, |view, ctx| {
            view.handle_action(
                &SftpBrowserAction::ContextMenu {
                    index: 0,
                    position: Vector2F::new(10.0, 10.0),
                },
                ctx,
            );
        });

        // 打开第二个菜单（不同位置和索引）
        let new_position = Vector2F::new(300.0, 400.0);
        view.update(&mut app, |view, ctx| {
            view.handle_action(
                &SftpBrowserAction::ContextMenu {
                    index: 5,
                    position: new_position,
                },
                ctx,
            );
        });

        view.read(&app, |view, _| {
            let cm = view.context_menu.as_ref().unwrap();
            assert_eq!(cm.entry_index, 5, "应更新为新的 entry_index");
            assert_eq!(cm.position, new_position, "应更新为新的 position");
            assert!(view.selected.contains(&5), "应选中新索引 5");
            assert!(!view.selected.contains(&0), "应取消选中旧索引 0");
        });
    });
}

// ============================================================
// 右键菜单边界条件测试
// ============================================================

/// 验证 ContextMenu index=0 时正确处理
#[test]
fn test_context_menu_zero_index() {
    use pathfinder_geometry::vector::Vector2F;

    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        let position = Vector2F::new(0.0, 0.0);
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::ContextMenu { index: 0, position }, ctx);
        });

        view.read(&app, |view, _| {
            let cm = view.context_menu.as_ref().unwrap();
            assert_eq!(cm.entry_index, 0, "index=0 应正确保存");
            assert_eq!(cm.position, position, "position 应正确保存");
            assert!(view.selected.contains(&0), "应选中索引 0");
        });
    });
}

/// 验证 ContextMenu 大索引值不 panic
#[test]
fn test_context_menu_large_index() {
    use pathfinder_geometry::vector::Vector2F;

    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        let position = Vector2F::new(500.0, 600.0);
        view.update(&mut app, |view, ctx| {
            view.handle_action(
                &SftpBrowserAction::ContextMenu {
                    index: 999,
                    position,
                },
                ctx,
            );
        });

        view.read(&app, |view, _| {
            let cm = view.context_menu.as_ref().unwrap();
            assert_eq!(cm.entry_index, 999, "大索引应正确保存");
            assert!(view.selected.contains(&999), "应选中大索引");
        });
    });
}

/// 验证 ContextMenu 负坐标正确处理
#[test]
fn test_context_menu_negative_position() {
    use pathfinder_geometry::vector::Vector2F;

    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        let position = Vector2F::new(-50.0, -100.0);
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::ContextMenu { index: 1, position }, ctx);
        });

        view.read(&app, |view, _| {
            let cm = view.context_menu.as_ref().unwrap();
            assert_eq!(cm.position, position, "负坐标应正确保存");
        });
    });
}

/// 验证 CloseContextMenu 在没有菜单打开时不 panic
#[test]
fn test_close_context_menu_when_none() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        // 初始状态没有菜单
        view.read(&app, |view, _| {
            assert!(view.context_menu.is_none(), "初始应无菜单");
        });

        // 直接关闭不应 panic
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::CloseContextMenu, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.context_menu.is_none(), "关闭后仍应为 None");
        });
    });
}

/// 验证 ContextMenu 清除之前的选择并选中新条目
#[test]
fn test_context_menu_clears_previous_selection() {
    use pathfinder_geometry::vector::Vector2F;

    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        // 先选中条目 2 和 3（通过两次 SelectEntry）
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::SelectEntry(2), ctx);
        });
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::SelectEntry(3), ctx);
        });
        view.read(&app, |view, _| {
            assert!(view.selected.contains(&3), "应选中 3");
            assert!(!view.selected.contains(&2), "单选模式应清除 2");
        });

        // 右键点击条目 7
        view.update(&mut app, |view, ctx| {
            view.handle_action(
                &SftpBrowserAction::ContextMenu {
                    index: 7,
                    position: Vector2F::new(200.0, 300.0),
                },
                ctx,
            );
        });

        view.read(&app, |view, _| {
            assert!(view.selected.contains(&7), "应选中 7");
            assert!(!view.selected.contains(&3), "应清除旧选择 3");
            assert_eq!(view.selected.len(), 1, "应只有一个选中项");
        });
    });
}

/// 验证多次打开关闭菜单不泄漏状态
#[test]
fn test_context_menu_multiple_open_close_cycles() {
    use pathfinder_geometry::vector::Vector2F;

    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        for i in 0..5 {
            // 打开菜单
            view.update(&mut app, |view, ctx| {
                view.handle_action(
                    &SftpBrowserAction::ContextMenu {
                        index: i,
                        position: Vector2F::new(i as f32 * 10.0, i as f32 * 20.0),
                    },
                    ctx,
                );
            });
            view.read(&app, |view, _| {
                assert!(view.context_menu.is_some(), "第 {i} 次打开应成功");
            });

            // 关闭菜单
            view.update(&mut app, |view, ctx| {
                view.handle_action(&SftpBrowserAction::CloseContextMenu, ctx);
            });
            view.read(&app, |view, _| {
                assert!(view.context_menu.is_none(), "第 {i} 次关闭后应为 None");
            });
        }
    });
}

// ============================================================
// 菜单项动作测试
// ============================================================

/// 验证 SftpBrowserAction::DetailsEntry 变体正确构造
#[test]
fn test_action_details_entry() {
    let action = SftpBrowserAction::DetailsEntry(42);
    assert!(matches!(action, SftpBrowserAction::DetailsEntry(42)));
}

/// 验证 SftpBrowserAction::DeleteEntry 变体正确构造
#[test]
fn test_action_delete_entry() {
    let action = SftpBrowserAction::DeleteEntry(10);
    assert!(matches!(action, SftpBrowserAction::DeleteEntry(10)));
}

/// 验证 SftpBrowserAction::RenameEntry 变体正确构造
#[test]
fn test_action_rename_entry() {
    let action = SftpBrowserAction::RenameEntry(5);
    assert!(matches!(action, SftpBrowserAction::RenameEntry(5)));
}

/// 验证 SftpBrowserAction::DownloadEntry 变体正确构造
#[test]
fn test_action_download_entry() {
    let action = SftpBrowserAction::DownloadEntry(3);
    assert!(matches!(action, SftpBrowserAction::DownloadEntry(3)));
}

/// 验证 SftpBrowserAction::OpenEntry 变体正确构造
#[test]
fn test_action_open_entry() {
    let action = SftpBrowserAction::OpenEntry(1);
    assert!(matches!(action, SftpBrowserAction::OpenEntry(1)));
}

/// 验证 SftpBrowserAction::ContextMenu 变体正确构造
#[test]
fn test_action_context_menu_variant() {
    use pathfinder_geometry::vector::Vector2F;
    let action = SftpBrowserAction::ContextMenu {
        index: 3,
        position: Vector2F::new(100.0, 200.0),
    };
    assert!(matches!(
        action,
        SftpBrowserAction::ContextMenu { index: 3, .. }
    ));
}

/// 验证 SftpBrowserAction::CloseContextMenu 变体正确构造
#[test]
fn test_action_close_context_menu_variant() {
    let action = SftpBrowserAction::CloseContextMenu;
    assert!(matches!(action, SftpBrowserAction::CloseContextMenu));
}

// ============================================================
// DeleteEntry 动作处理测试
// ============================================================

/// 验证 DeleteEntry 在无 SFTP 连接时不 panic
#[test]
fn test_delete_entry_no_connection() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        // 无 SFTP 连接时执行 DeleteEntry 不应 panic
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::DeleteEntry(0), ctx);
        });
    });
}

/// 验证 RenameEntry 在无 SFTP 连接时不 panic
#[test]
fn test_rename_entry_no_connection() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::RenameEntry(0), ctx);
        });
    });
}

// ============================================================
// Category 1: 对话框操作无连接安全测试
// ============================================================

/// 验证 ConfirmDelete 无 dialog、无连接时不 panic
#[test]
fn test_confirm_delete_no_connection_no_dialog() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::ConfirmDelete, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

/// 验证 ConfirmDelete 有 dialog 但无连接时安全处理
#[test]
fn test_confirm_delete_no_connection_with_dialog() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.dialog = Some(Dialog::DeleteConfirm {
                paths: vec![PathBuf::from("/tmp/test")],
                is_dirs: vec![false],
            });
            view.handle_action(&SftpBrowserAction::ConfirmDelete, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

/// 验证 ConfirmRename 无 dialog、无连接时不 panic
#[test]
fn test_confirm_rename_no_connection_no_dialog() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::ConfirmRename, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

/// 验证 ConfirmRename 有 dialog 但无连接时提示错误并关闭 dialog
#[test]
fn test_confirm_rename_no_connection_with_dialog() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.dialog = Some(Dialog::Rename {
                path: PathBuf::from("/home/old.txt"),
                original_name: "old.txt".to_string(),
            });
            // 先输入非空名称以跳过空名称检查
            view.rename_editor.update(ctx, |e: &mut EditorView, ctx| {
                e.set_buffer_text("new_name", ctx);
            });
            view.handle_action(&SftpBrowserAction::ConfirmRename, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

/// 验证 ConfirmNewFolder 无 dialog、无连接时不 panic
#[test]
fn test_confirm_new_folder_no_connection_no_dialog() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::ConfirmNewFolder, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

/// 验证 ConfirmNewFolder 有 dialog 但无连接时提示错误并关闭 dialog
#[test]
fn test_confirm_new_folder_no_connection_with_dialog() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.dialog = Some(Dialog::CreateFolder {
                parent_path: PathBuf::from("/home"),
            });
            // 先输入非空名称以跳过空名称检查
            view.new_folder_editor
                .update(ctx, |e: &mut EditorView, ctx| {
                    e.set_buffer_text("new_folder", ctx);
                });
            view.handle_action(&SftpBrowserAction::ConfirmNewFolder, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

/// 验证 ConfirmMove 无 dialog、无连接时不 panic
#[test]
fn test_confirm_move_no_connection_no_dialog() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::ConfirmMove, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

/// 验证 ConfirmMove 有 dialog 但无连接时提示错误并关闭 dialog
#[test]
fn test_confirm_move_no_connection_with_dialog() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.dialog = Some(Dialog::Move {
                source: PathBuf::from("/home/file.txt"),
                target_dir: PathBuf::from("/home/backup"),
            });
            view.handle_action(&SftpBrowserAction::ConfirmMove, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

// ============================================================
// Category 2: 导航边界测试
// ============================================================

/// 验证 NavigateTo 当前路径时不产生重复历史
#[test]
fn test_navigate_to_same_path() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::NavigateTo(PathBuf::from("/")), ctx);
        });

        view.read(&app, |view, _| {
            assert_eq!(view.current_path, PathBuf::from("/"));
            assert_eq!(view.path_history.len(), 1);
        });
    });
}

/// 验证 NavigateTo 深层路径正确更新
#[test]
fn test_navigate_to_deep_path() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(
                &SftpBrowserAction::NavigateTo(PathBuf::from("/a/b/c/d")),
                ctx,
            );
        });

        view.read(&app, |view, _| {
            assert_eq!(view.current_path, PathBuf::from("/a/b/c/d"));
            assert_eq!(view.path_history.len(), 2);
        });
    });
}

/// 验证 NavigateTo 将反斜杠规范化为正斜杠
#[test]
fn test_navigate_to_backslash_path() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(
                &SftpBrowserAction::NavigateTo(PathBuf::from(r"home\user")),
                ctx,
            );
        });

        view.read(&app, |view, _| {
            assert_eq!(view.current_path, PathBuf::from("home/user"));
        });
    });
}

/// 验证 GoBack 在初始历史位置不做操作
#[test]
fn test_go_back_at_initial() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::GoBack, ctx);
        });

        view.read(&app, |view, _| {
            assert_eq!(view.current_path, PathBuf::from("/"));
        });
    });
}

/// 验证 GoForward 在初始历史位置不做操作
#[test]
fn test_go_forward_at_initial() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::GoForward, ctx);
        });

        view.read(&app, |view, _| {
            assert_eq!(view.current_path, PathBuf::from("/"));
        });
    });
}

/// 验证 GoUp 从根路径不做操作
#[test]
fn test_go_up_from_root_via_action() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::GoUp, ctx);
        });

        view.read(&app, |view, _| {
            assert_eq!(view.current_path, PathBuf::from("/"));
        });
    });
}

/// 验证多步导航后 GoBack/GoForward 历史追踪正确
#[test]
fn test_multiple_navigate_then_back_forward() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::NavigateTo(PathBuf::from("/home")), ctx);
            view.handle_action(&SftpBrowserAction::NavigateTo(PathBuf::from("/var")), ctx);
        });

        view.read(&app, |view, _| {
            assert_eq!(view.current_path, PathBuf::from("/var"));
            assert_eq!(view.path_history.len(), 3);
            assert_eq!(view.history_index, 2);
        });

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::GoBack, ctx);
        });

        view.read(&app, |view, _| {
            assert_eq!(view.current_path, PathBuf::from("/home"));
            assert_eq!(view.history_index, 1);
        });

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::GoForward, ctx);
        });

        view.read(&app, |view, _| {
            assert_eq!(view.current_path, PathBuf::from("/var"));
            assert_eq!(view.history_index, 2);
        });
    });
}

// ============================================================
// Category 3: 对话框开关循环测试
// ============================================================

/// 验证 NewFolder 打开 CreateFolder 对话框
#[test]
fn test_new_folder_opens_dialog() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::NewFolder, ctx);
        });

        view.read(&app, |view, _| {
            assert!(matches!(view.dialog, Some(Dialog::CreateFolder { .. })));
        });
    });
}

/// 验证 CloseDialog 清除对话框
#[test]
fn test_close_dialog_clears() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::NewFolder, ctx);
            view.handle_action(&SftpBrowserAction::CloseDialog, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

/// 验证 ConfirmOverwrite 关闭对话框
#[test]
fn test_confirm_overwrite_closes_dialog() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.dialog = Some(Dialog::OverwriteConfirm {
                source: PathBuf::from("/a"),
                target: PathBuf::from("/b"),
                file_size: 0,
                direction: TransferDirection::Download,
            });
            view.handle_action(&SftpBrowserAction::ConfirmOverwrite, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

/// 验证 CloseDialog 在无 dialog 时不 panic
#[test]
fn test_close_dialog_when_none() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::CloseDialog, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

/// 验证对话框多次开关循环稳定性
#[test]
fn test_dialog_multiple_cycles() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        for _ in 0..3 {
            view.update(&mut app, |view, ctx| {
                view.handle_action(&SftpBrowserAction::NewFolder, ctx);
            });
            view.read(&app, |view, _| {
                assert!(view.dialog.is_some());
            });

            view.update(&mut app, |view, ctx| {
                view.handle_action(&SftpBrowserAction::CloseDialog, ctx);
            });
            view.read(&app, |view, _| {
                assert!(view.dialog.is_none());
            });
        }
    });
}

// ============================================================
// Category 4: 传输任务生命周期测试
// ============================================================

/// 验证取消不存在的任务 ID 不 panic
#[test]
fn test_cancel_transfer_nonexistent_id() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::CancelTransfer(999), ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.transfers.is_empty());
        });
    });
}

/// 验证取消 ID 为 0 的不存在任务不 panic
#[test]
fn test_cancel_transfer_zero_id() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::CancelTransfer(0), ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.transfers.is_empty());
        });
    });
}

/// 验证 DownloadSaveAs 越界索引不 panic 且不创建孤立任务
#[test]
fn test_download_save_as_out_of_range() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(
                &SftpBrowserAction::DownloadSaveAs {
                    index: 100,
                    local_path: "/tmp/out.txt".to_string(),
                },
                ctx,
            );
        });

        view.read(&app, |view, _| {
            assert!(view.transfers.is_empty());
            assert_eq!(view.next_transfer_id, 1);
        });
    });
}

/// 验证 DownloadSaveAs 在空条目列表 index=0 不 panic
#[test]
fn test_download_save_as_zero_index_empty() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(
                &SftpBrowserAction::DownloadSaveAs {
                    index: 0,
                    local_path: "/tmp/out.txt".to_string(),
                },
                ctx,
            );
        });

        view.read(&app, |view, _| {
            assert!(view.transfers.is_empty());
        });
    });
}

/// 验证 ExecuteUpload 不存在的本地文件且无连接时任务标记为 Failed
#[test]
fn test_execute_upload_nonexistent_file() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(
                &SftpBrowserAction::ExecuteUpload("/no/such/file.txt".to_string()),
                ctx,
            );
        });

        view.read(&app, |view, _| {
            assert_eq!(view.transfers.len(), 1);
            assert!(matches!(view.transfers[0].state, TransferState::Failed(_)));
        });
    });
}

// ============================================================
// Category 5: DetailsEntry 边界测试
// ============================================================

/// 验证 DetailsEntry 越界索引不 panic
#[test]
fn test_details_entry_out_of_range() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::DetailsEntry(999), ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

/// 验证 DetailsEntry 在空条目 index=0 不 panic
#[test]
fn test_details_entry_zero_empty() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::DetailsEntry(0), ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

/// 验证 DetailsEntry 极大索引不 panic
#[test]
fn test_details_entry_usize_max() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::DetailsEntry(usize::MAX), ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

// ============================================================
// Category 6: OpenEntry / DownloadEntry 无条目测试
// ============================================================

/// 验证 OpenEntry 越界索引不 panic，路径不变
#[test]
fn test_open_entry_out_of_range() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::OpenEntry(999), ctx);
        });

        view.read(&app, |view, _| {
            assert_eq!(view.current_path, PathBuf::from("/"));
        });
    });
}

/// 验证 OpenEntry 在空条目 index=0 不 panic
#[test]
fn test_open_entry_zero_empty() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::OpenEntry(0), ctx);
        });

        view.read(&app, |view, _| {
            assert_eq!(view.current_path, PathBuf::from("/"));
        });
    });
}

/// 验证 DownloadEntry 在空条目列表不 panic
#[test]
fn test_download_entry_empty_entries() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::DownloadEntry(0), ctx);
        });
        // 不 panic 即通过
    });
}

// ============================================================
// Category 7: 选择与删除边界测试
// ============================================================

/// 验证 DeleteSelected 在空选择集不 panic
#[test]
fn test_delete_selected_empty_selection() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::DeleteSelected, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

/// 验证 DeleteSelected 有选择但无条目不 panic
#[test]
fn test_delete_selected_no_entries() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::SelectEntry(0), ctx);
            view.handle_action(&SftpBrowserAction::DeleteSelected, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_none());
        });
    });
}

/// 验证 SelectEntry 接受 usize::MAX 不 panic
#[test]
fn test_select_entry_usize_max() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::SelectEntry(usize::MAX), ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.selected.contains(&usize::MAX));
        });
    });
}

/// 验证多次 SelectEntry 每次清除前一个选择
#[test]
fn test_multiple_select_clears_previous() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::SelectEntry(1), ctx);
            view.handle_action(&SftpBrowserAction::SelectEntry(3), ctx);
            view.handle_action(&SftpBrowserAction::SelectEntry(7), ctx);
        });

        view.read(&app, |view, _| {
            assert_eq!(view.selected.len(), 1);
            assert!(view.selected.contains(&7));
            assert!(!view.selected.contains(&1));
            assert!(!view.selected.contains(&3));
        });
    });
}

// ============================================================
// Category 8: Render 安全性测试
// ============================================================

/// 验证初始状态一致性（构造函数会尝试连接，可能为 Failed 或 Disconnected）
#[test]
fn test_render_disconnected_state() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.read(&app, |view, _| {
            // 构造函数调用 connect_to_server，测试环境无 SSH 服务，状态为 Failed
            assert!(matches!(
                view.connection,
                ConnectionState::Failed(_) | ConnectionState::Disconnected
            ));
            assert!(!view.is_loading);
            assert!(view.entries.is_empty());
            assert!(view.dialog.is_none());
            assert!(view.context_menu.is_none());
        });
    });
}

/// 验证 drag hover 状态正确设置
#[test]
fn test_render_with_drag_hover() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::DragFilesEnter, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.is_drag_hovering);
        });
    });
}

/// 验证搜索过滤状态正确设置
#[test]
fn test_render_with_search_filter() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::SetSearchFilter("test".to_string()), ctx);
        });

        view.read(&app, |view, _| {
            assert_eq!(view.search_filter.as_deref(), Some("test"));
        });
    });
}

/// 验证右键菜单状态正确设置
#[test]
fn test_render_with_context_menu() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(
                &SftpBrowserAction::ContextMenu {
                    index: 0,
                    position: Vector2F::new(10.0, 20.0),
                },
                ctx,
            );
        });

        view.read(&app, |view, _| {
            assert!(view.context_menu.is_some());
        });
    });
}

/// 验证对话框打开状态正确设置
#[test]
fn test_render_with_dialog_open() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::NewFolder, ctx);
        });

        view.read(&app, |view, _| {
            assert!(view.dialog.is_some());
        });
    });
}

/// 验证传输任务创建后状态正确
#[test]
fn test_render_with_transfer_task() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(
                &SftpBrowserAction::ExecuteUpload("/tmp/x.txt".to_string()),
                ctx,
            );
        });

        view.read(&app, |view, _| {
            assert_eq!(view.transfers.len(), 1);
        });
    });
}

/// 验证所有叠加层同时存在时不 panic
#[test]
fn test_render_all_overlays_combined() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::DragFilesEnter, ctx);
            view.handle_action(&SftpBrowserAction::SetSearchFilter("x".to_string()), ctx);
            view.handle_action(
                &SftpBrowserAction::ContextMenu {
                    index: 0,
                    position: Vector2F::new(5.0, 5.0),
                },
                ctx,
            );
            view.handle_action(&SftpBrowserAction::NewFolder, ctx);
            view.handle_action(
                &SftpBrowserAction::ExecuteUpload("/tmp/test.txt".to_string()),
                ctx,
            );
        });

        view.read(&app, |view, _| {
            assert!(view.is_drag_hovering);
            assert!(view.search_filter.is_some());
            assert!(view.context_menu.is_some());
            assert!(view.dialog.is_some());
            assert_eq!(view.transfers.len(), 1);
        });
    });
}

/// 验证所有叠加层关闭后状态正确清零
#[test]
fn test_render_after_close_all_overlays() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        // 开启所有叠加层
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::DragFilesEnter, ctx);
            view.handle_action(&SftpBrowserAction::SetSearchFilter("x".to_string()), ctx);
            view.handle_action(
                &SftpBrowserAction::ContextMenu {
                    index: 0,
                    position: Vector2F::new(5.0, 5.0),
                },
                ctx,
            );
            view.handle_action(&SftpBrowserAction::NewFolder, ctx);
        });

        // 关闭所有叠加层
        view.update(&mut app, |view, ctx| {
            view.handle_action(&SftpBrowserAction::DragFilesLeave, ctx);
            view.handle_action(&SftpBrowserAction::ClearSearchFilter, ctx);
            view.handle_action(&SftpBrowserAction::CloseContextMenu, ctx);
            view.handle_action(&SftpBrowserAction::CloseDialog, ctx);
        });

        view.read(&app, |view, _| {
            assert!(!view.is_drag_hovering);
            assert!(view.search_filter.is_none());
            assert!(view.context_menu.is_none());
            assert!(view.dialog.is_none());
        });
    });
}

/// 验证初始路径历史状态
#[test]
fn test_render_path_history_initial() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.read(&app, |view, _| {
            assert_eq!(view.path_history, vec![PathBuf::from("/")]);
            assert_eq!(view.history_index, 0);
        });
    });
}

/// 验证初始 is_loading 为 false
#[test]
fn test_render_is_loading_initial_false() {
    warpui::App::test((), |mut app| async move {
        initialize_app(&mut app);
        let (_, view) = create_view(&mut app);

        view.read(&app, |view, _| {
            assert!(!view.is_loading);
        });
    });
}
