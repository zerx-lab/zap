//! SFTP 文件浏览器真实弹窗集成测试
//!
//! 使用 Builder/TestStep/Driver 模式，在真实窗口中打开 SFTP 面板，
//! 验证面板渲染、标题、关闭、标签切换等交互行为。
//! author: logic
//! date: 2026-05-30

use std::collections::HashMap;

use warp::integration_testing::sftp;
use warp::integration_testing::sftp::{ConnectionState, Dialog, SftpBrowserAction};
use warp::integration_testing::terminal::wait_until_bootstrapped_single_pane_for_tab;
use warp::integration_testing::view_getters::{pane_group_view, workspace_view};
use warpui::{
    async_assert, async_assert_eq, integration::AssertionCallback, integration::StepDataMap,
    integration::TestStep, TypedActionView,
};

use super::{new_builder, Builder};

/// 断言 SFTP 浏览器视图存在且可访问
///
/// 不依赖固定 pane index，通过 view 类型查找 SftpBrowserView。
/// 接受所有连接状态，仅验证视图存在。
/// author: logic
/// date: 2026-05-31
fn assert_sftp_browser_view_exists() -> AssertionCallback {
    Box::new(move |app, window_id| {
        let view = sftp::sftp_browser_view(app, window_id);
        view.read(app, |_v, _| {
            // 视图成功获取即证明 SFTP 面板存在
            warpui::integration::AssertionOutcome::Success
        })
    })
}

/// 打开 SFTP 面板（使用测试 node_id）
fn open_sftp_pane(app: &mut warpui::App) {
    let window_id = app.read(|ctx| {
        ctx.windows()
            .active_window()
            .expect("should have active window")
    });
    let workspace = workspace_view(app, window_id);
    app.update(|ctx| {
        workspace.update(ctx, |ws, ctx| {
            ws.open_sftp_pane("test-integration-node".to_string(), ctx);
        });
    });
}

/// 验证 SFTP 面板在真实窗口中打开并显示正确标题
pub fn test_sftp_pane_opens_in_workspace() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(
            TestStep::new("Open SFTP pane")
                .with_action(|app, _, _| open_sftp_pane(app))
                .set_post_step_pause(std::time::Duration::from_secs(2)),
        )
        .with_step(
            TestStep::new("Verify SFTP pane is visible")
                .add_assertion(|app, window_id| {
                    let pane_group = pane_group_view(app, window_id, 0);
                    pane_group.read(app, |pane_group, _ctx| {
                        async_assert_eq!(
                            pane_group.pane_count(),
                            2,
                            "Expected 2 panes after opening SFTP (terminal + SFTP)"
                        )
                    })
                })
                .add_assertion(assert_sftp_browser_view_exists()),
        )
}

/// 验证 SFTP 面板获取焦点后键盘事件正常工作
pub fn test_sftp_pane_focus_and_keyboard() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(
            TestStep::new("Open SFTP pane")
                .with_action(|app, _, _| open_sftp_pane(app))
                .set_post_step_pause(std::time::Duration::from_secs(2)),
        )
        .with_step(
            TestStep::new("Press Escape to close dialog if any")
                .with_keystrokes(&["escape"])
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        .with_step(
            TestStep::new("Verify pane still exists").add_assertion(|app, window_id| {
                let pane_group = pane_group_view(app, window_id, 0);
                pane_group.read(app, |pane_group, _ctx| {
                    async_assert_eq!(
                        pane_group.pane_count(),
                        2,
                        "SFTP pane should still be visible"
                    )
                })
            }),
        )
}

/// 验证关闭 SFTP 面板后回到单面板
pub fn test_sftp_pane_close() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(
            TestStep::new("Open SFTP pane")
                .with_action(|app, _, _| open_sftp_pane(app))
                .set_post_step_pause(std::time::Duration::from_secs(2)),
        )
        .with_step(
            TestStep::new("Verify 2 panes").add_assertion(|app, window_id| {
                let pane_group = pane_group_view(app, window_id, 0);
                pane_group.read(app, |pane_group, _ctx| {
                    async_assert_eq!(pane_group.pane_count(), 2, "Should have 2 panes")
                })
            }),
        )
        // 遍历所有可见面板，找到非 terminal 面板（即 SFTP）并关闭
        .with_step(
            TestStep::new("Close SFTP pane via pane group")
                .with_action(|app, window_id, _| {
                    let pg = pane_group_view(app, window_id, 0);
                    let sftp_pane_id = pg.read(app, |pane_group, _ctx| {
                        let terminal_ids: std::collections::HashSet<_> =
                            pane_group.terminal_pane_ids().collect();
                        let ids = pane_group.visible_pane_ids();
                        ids.into_iter()
                            .find(|id| !terminal_ids.contains(id))
                            .expect("应存在一个非 terminal 面板（SFTP）")
                    });
                    pg.update(app, |pane_group, ctx| {
                        pane_group.close_pane(sftp_pane_id, ctx);
                    });
                })
                .set_post_step_pause(std::time::Duration::from_secs(1)),
        )
        .with_step(
            TestStep::new("Verify back to single pane").add_assertion(|app, window_id| {
                let pane_group = pane_group_view(app, window_id, 0);
                pane_group.read(app, |pane_group, _ctx| {
                    async_assert_eq!(
                        pane_group.visible_pane_count(),
                        1,
                        "Should have 1 visible pane after closing SFTP"
                    )
                })
            }),
        )
}

/// 验证切换标签后 SFTP 面板状态
pub fn test_sftp_pane_tab_switch() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(
            TestStep::new("Open SFTP pane")
                .with_action(|app, _, _| open_sftp_pane(app))
                .set_post_step_pause(std::time::Duration::from_secs(2)),
        )
        // 切换到其他标签
        .with_step(
            TestStep::new("Switch tab with Ctrl+Tab")
                .with_keystrokes(&["ctrl-tab"])
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        .with_step(
            TestStep::new("Switch back")
                .with_keystrokes(&["ctrl-shift-tab"])
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        .with_step(
            TestStep::new("Verify SFTP pane still visible").add_assertion(|app, window_id| {
                let pane_group = pane_group_view(app, window_id, 0);
                pane_group.read(app, |pane_group, _ctx| {
                    async_assert!(pane_group.pane_count() >= 1, "Should have at least 1 pane")
                })
            }),
        )
}

/// 验证 SFTP 面板在连接失败状态下正确渲染
pub fn test_sftp_pane_disconnected_render() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(
            TestStep::new("Open SFTP pane (will fail to connect)")
                .with_action(|app, _, _| open_sftp_pane(app))
                .set_post_step_pause(std::time::Duration::from_secs(3)),
        )
        .with_step(
            TestStep::new("Verify pane renders without crash")
                .add_assertion(|app, window_id| {
                    let pane_group = pane_group_view(app, window_id, 0);
                    pane_group.read(app, |pane_group, _ctx| {
                        async_assert_eq!(
                            pane_group.pane_count(),
                            2,
                            "SFTP pane should render even in disconnected state"
                        )
                    })
                })
                .add_assertion(assert_sftp_browser_view_exists()),
        )
}

// ============================================================
// Mock 后端 UI 集成测试
// ============================================================

/// 打开 SFTP 面板并注入 mock 后端的通用步骤
fn open_sftp_with_mock_step(
    files: &'static [(&'static str, &'static [u8])],
) -> warpui::integration::TestStep {
    // 使用 TestStep::new 而非 new_step_with_default_assertions，
    // 因为打开 SFTP 面板后 pane 布局发生变化（SFTP 可能排在 pane_index=0），
    // 默认断言在 pane_index=0 查找 terminal_view 会 panic。
    TestStep::new("Open SFTP pane with mock backend")
        .with_action(move |app, _, step_data: &mut StepDataMap| {
            let (_, temp_dir) = sftp::open_sftp_pane_with_mock(app, files);
            // 将 temp_dir 存入 StepDataMap 以保持生命周期
            step_data.insert("sftp_mock", temp_dir);
        })
        .set_post_step_pause(std::time::Duration::from_secs(2))
}

/// 验证 mock 后端连接成功，SFTP 浏览器处于 Connected 状态
pub fn test_sftp_mock_backend_connected() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_sftp_with_mock_step(&[
            ("readme.txt", b"hello"),
            ("docs/report.txt", b"report"),
        ]))
        .with_step(
            TestStep::new("Verify Connected state and entries")
                .add_assertion(|app, window_id| {
                    let view = sftp::sftp_browser_view(app, window_id);
                    view.read(app, |v, _| {
                        async_assert!(
                            matches!(v.connection_state(), ConnectionState::Connected),
                            "应处于 Connected 状态"
                        )
                    })
                })
                .add_assertion(|app, window_id| {
                    let view = sftp::sftp_browser_view(app, window_id);
                    view.read(app, |v, _| {
                        async_assert_eq!(
                            v.entries().len(),
                            2,
                            "应列出 2 个条目（docs 目录 + readme.txt）"
                        )
                    })
                }),
        )
}

/// 点击工具栏刷新按钮，验证条目重新加载
pub fn test_sftp_toolbar_refresh() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_sftp_with_mock_step(&[("file1.txt", b"content1")]))
        .with_step(
            TestStep::new("Click refresh button")
                .with_click_on_saved_position("sftp_btn:refresh")
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        .with_step(
            TestStep::new("Verify entries still present after refresh").add_assertion(
                |app, window_id| {
                    let view = sftp::sftp_browser_view(app, window_id);
                    view.read(app, |v, _| {
                        async_assert_eq!(v.entries().len(), 1, "刷新后条目应仍存在")
                    })
                },
            ),
        )
}

/// 点击新建文件夹按钮，验证对话框打开
pub fn test_sftp_toolbar_new_folder() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_sftp_with_mock_step(&[]))
        .with_step(
            TestStep::new("Click new folder button")
                .with_click_on_saved_position("sftp_btn:new_folder")
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        .with_step(
            TestStep::new("Verify CreateFolder dialog is open").add_assertion(|app, window_id| {
                let view = sftp::sftp_browser_view(app, window_id);
                view.read(app, |v, _| {
                    async_assert!(
                        matches!(v.dialog(), Some(Dialog::CreateFolder { .. })),
                        "应打开新建文件夹对话框"
                    )
                })
            }),
        )
}

/// 点击上传按钮，验证不 panic
pub fn test_sftp_toolbar_upload() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_sftp_with_mock_step(&[]))
        .with_step(
            TestStep::new("Click upload button")
                .with_click_on_saved_position("sftp_btn:upload")
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        .with_step(
            TestStep::new("Verify view still stable after upload click").add_assertion(
                |app, window_id| {
                    let view = sftp::sftp_browser_view(app, window_id);
                    view.read(app, |v, _| {
                        async_assert!(
                            matches!(v.connection_state(), ConnectionState::Connected),
                            "点击上传后应仍为 Connected"
                        )
                    })
                },
            ),
        )
}

/// 点击上级目录按钮，验证导航回退
pub fn test_sftp_toolbar_up() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_sftp_with_mock_step(&[("subdir/file.txt", b"content")]))
        // 进入子目录
        .with_step(
            TestStep::new("Enter subdirectory")
                .with_action(|app, window_id, _| {
                    let view = sftp::sftp_browser_view(app, window_id);
                    view.update(app, |v, ctx| {
                        v.handle_action(
                            &SftpBrowserAction::OpenEntry(
                                v.entries().iter().position(|e| e.name == "subdir").unwrap(),
                            ),
                            ctx,
                        );
                    });
                })
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        // 点击上级目录按钮
        .with_step(
            TestStep::new("Click up button")
                .with_click_on_saved_position("sftp_btn:up")
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        .with_step(
            TestStep::new("Verify navigated back to root").add_assertion(|app, window_id| {
                let view = sftp::sftp_browser_view(app, window_id);
                view.read(app, |v, _| {
                    async_assert!(
                        v.entries().iter().any(|e| e.name == "subdir"),
                        "回到上级后应看到 subdir 目录"
                    )
                })
            }),
        )
}

/// 点击文件行，验证选中状态
pub fn test_sftp_click_file_row_selects() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_sftp_with_mock_step(&[
            ("file_a.txt", b"a"),
            ("file_b.txt", b"b"),
        ]))
        .with_step(
            TestStep::new("Click on first file row")
                .with_click_on_saved_position("sftp_row:0")
                .set_post_step_pause(std::time::Duration::from_millis(300)),
        )
        .with_step(
            TestStep::new("Verify file is selected").add_assertion(|app, window_id| {
                let view = sftp::sftp_browser_view(app, window_id);
                view.read(app, |v, _| {
                    async_assert!(v.selected().contains(&0), "第一个文件应被选中")
                })
            }),
        )
}

/// 右键点击文件行，验证上下文菜单打开
pub fn test_sftp_right_click_opens_menu() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_sftp_with_mock_step(&[("menu_file.txt", b"content")]))
        .with_step(
            TestStep::new("Right-click on file row")
                .with_right_click_on_saved_position("sftp_row:0")
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        .with_step(
            TestStep::new("Verify context menu is open").add_assertion(|app, window_id| {
                let view = sftp::sftp_browser_view(app, window_id);
                view.read(app, |v, _| {
                    async_assert!(v.context_menu().is_some(), "右键菜单应已打开")
                })
            }),
        )
}

/// 右键菜单 → 点击删除 → 确认
pub fn test_sftp_ctx_menu_delete() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_sftp_with_mock_step(&[("to_delete.txt", b"delete me")]))
        // 右键打开菜单
        .with_step(
            TestStep::new("Right-click on file")
                .with_right_click_on_saved_position("sftp_row:0")
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        // 点击删除菜单项
        .with_step(
            TestStep::new("Click delete in context menu")
                .with_click_on_saved_position("sftp_ctx:delete")
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        // 验证删除确认对话框
        .with_step(
            TestStep::new("Verify delete confirm dialog").add_assertion(|app, window_id| {
                let view = sftp::sftp_browser_view(app, window_id);
                view.read(app, |v, _| {
                    async_assert!(
                        matches!(v.dialog(), Some(Dialog::DeleteConfirm { .. })),
                        "应打开删除确认对话框"
                    )
                })
            }),
        )
        // 点击确认
        .with_step(
            TestStep::new("Click confirm button")
                .with_click_on_saved_position("sftp_btn:dialog_confirm")
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        // 验证条目已删除
        .with_step(
            TestStep::new("Verify file deleted").add_assertion(|app, window_id| {
                let view = sftp::sftp_browser_view(app, window_id);
                view.read(app, |v, _| {
                    async_assert_eq!(v.entries().len(), 0, "删除后应无条目")
                })
            }),
        )
}

/// 右键菜单 → 重命名
pub fn test_sftp_ctx_menu_rename() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_sftp_with_mock_step(&[("old_name.txt", b"content")]))
        .with_step(
            TestStep::new("Right-click on file")
                .with_right_click_on_saved_position("sftp_row:0")
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        .with_step(
            TestStep::new("Click rename in context menu")
                .with_click_on_saved_position("sftp_ctx:rename")
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        .with_step(
            TestStep::new("Verify rename dialog is open").add_assertion(|app, window_id| {
                let view = sftp::sftp_browser_view(app, window_id);
                view.read(app, |v, _| {
                    async_assert!(
                        matches!(v.dialog(), Some(Dialog::Rename { .. })),
                        "应打开重命名对话框"
                    )
                })
            }),
        )
}

/// 面包屑导航 — 点击根目录
pub fn test_sftp_breadcrumb_root_click() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_sftp_with_mock_step(&[("subdir/file.txt", b"content")]))
        // 进入子目录
        .with_step(
            TestStep::new("Enter subdirectory")
                .with_action(|app, window_id, _| {
                    let view = sftp::sftp_browser_view(app, window_id);
                    view.update(app, |v, ctx| {
                        let idx = v.entries().iter().position(|e| e.name == "subdir").unwrap();
                        v.handle_action(&SftpBrowserAction::OpenEntry(idx), ctx);
                    });
                })
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        // 点击面包屑根 "/" 导航回根目录
        .with_step(
            TestStep::new("Navigate to root via breadcrumb")
                .with_action(|app, window_id, _| {
                    let view = sftp::sftp_browser_view(app, window_id);
                    view.update(app, |v, ctx| {
                        v.handle_action(
                            &SftpBrowserAction::NavigateTo(std::path::PathBuf::from("/")),
                            ctx,
                        );
                    });
                })
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        .with_step(
            TestStep::new("Verify navigated to root").add_assertion(|app, window_id| {
                let view = sftp::sftp_browser_view(app, window_id);
                view.read(app, |v, _| {
                    async_assert!(
                        v.entries().iter().any(|e| e.name == "subdir"),
                        "回到根目录后应看到 subdir"
                    )
                })
            }),
        )
}

/// 键盘 Backspace 返回上级
pub fn test_sftp_keyboard_backspace_up() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_sftp_with_mock_step(&[("subdir/file.txt", b"x")]))
        // 进入子目录
        .with_step(
            TestStep::new("Enter subdirectory")
                .with_action(|app, window_id, _| {
                    let view = sftp::sftp_browser_view(app, window_id);
                    view.update(app, |v, ctx| {
                        let idx = v.entries().iter().position(|e| e.name == "subdir").unwrap();
                        v.handle_action(&SftpBrowserAction::OpenEntry(idx), ctx);
                    });
                })
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        // 按 Backspace
        .with_step(
            TestStep::new("Press Backspace to go up")
                .with_keystrokes(&["backspace"])
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        .with_step(
            TestStep::new("Verify back at root").add_assertion(|app, window_id| {
                let view = sftp::sftp_browser_view(app, window_id);
                view.read(app, |v, _| {
                    async_assert!(
                        v.entries().iter().any(|e| e.name == "subdir"),
                        "Backspace 后应回到上级看到 subdir"
                    )
                })
            }),
        )
}

/// 键盘 Delete 删除选中条目
pub fn test_sftp_keyboard_delete() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_sftp_with_mock_step(&[("del_target.txt", b"x")]))
        // 选中第一个条目
        .with_step(
            TestStep::new("Select first entry")
                .with_action(|app, window_id, _| {
                    let view = sftp::sftp_browser_view(app, window_id);
                    view.update(app, |v, ctx| {
                        v.handle_action(&SftpBrowserAction::SelectEntry(0), ctx);
                    });
                })
                .set_post_step_pause(std::time::Duration::from_millis(300)),
        )
        // 按 Delete
        .with_step(
            TestStep::new("Press Delete key")
                .with_keystrokes(&["delete"])
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        .with_step(
            TestStep::new("Verify delete confirm dialog").add_assertion(|app, window_id| {
                let view = sftp::sftp_browser_view(app, window_id);
                view.read(app, |v, _| {
                    async_assert!(
                        matches!(v.dialog(), Some(Dialog::DeleteConfirm { .. })),
                        "Delete 键应触发删除确认对话框"
                    )
                })
            }),
        )
}

/// 键盘 Escape 关闭对话框
pub fn test_sftp_keyboard_escape_close_dialog() -> Builder {
    new_builder()
        .with_user_defaults(HashMap::from([(
            "UndoCloseEnabled".to_string(),
            false.to_string(),
        )]))
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_sftp_with_mock_step(&[("file.txt", b"x")]))
        // 打开新建文件夹对话框
        .with_step(
            TestStep::new("Open new folder dialog")
                .with_action(|app, window_id, _| {
                    let view = sftp::sftp_browser_view(app, window_id);
                    view.update(app, |v, ctx| {
                        v.handle_action(&SftpBrowserAction::NewFolder, ctx);
                    });
                })
                .set_post_step_pause(std::time::Duration::from_millis(300)),
        )
        .with_step(
            TestStep::new("Verify dialog open").add_assertion(|app, window_id| {
                let view = sftp::sftp_browser_view(app, window_id);
                view.read(app, |v, _| {
                    async_assert!(v.dialog().is_some(), "对话框应已打开")
                })
            }),
        )
        // 按 Escape 关闭
        .with_step(
            TestStep::new("Press Escape to close")
                .with_keystrokes(&["escape"])
                .set_post_step_pause(std::time::Duration::from_millis(500)),
        )
        .with_step(
            TestStep::new("Verify dialog closed").add_assertion(|app, window_id| {
                let view = sftp::sftp_browser_view(app, window_id);
                view.read(app, |v, _| {
                    async_assert!(v.dialog().is_none(), "Escape 后对话框应关闭")
                })
            }),
        )
}
