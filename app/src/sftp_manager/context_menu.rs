//! 右键上下文菜单渲染组件
//!
//! 提供文件条目的右键菜单渲染，包括打开、下载、重命名、删除、详情等操作。
//! author: logic
//! date: 2026-05-26

use pathfinder_geometry::vector::Vector2F;
use warp_core::ui::appearance::Appearance;
use warpui::elements::{
    Border, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Dismiss, Flex, Hoverable,
    MainAxisSize, ParentElement, Radius, SavePosition, Text,
};
use warpui::platform::Cursor;
use warpui::Element;

/// 右键菜单宽度
const CONTEXT_MENU_WIDTH: f32 = 150.0;

use crate::sftp_manager::browser::SftpBrowserAction;

/// 右键菜单状态
#[derive(Debug)]
pub struct ContextMenuState {
    /// 关联的文件条目索引
    pub entry_index: usize,
    /// 菜单弹出位置
    pub position: Vector2F,
}

impl ContextMenuState {
    /// 创建新的右键菜单状态
    pub fn new(entry_index: usize, position: Vector2F) -> Self {
        Self {
            entry_index,
            position,
        }
    }
}

/// 菜单项定义
struct MenuItem {
    /// 显示标签
    label: String,
    /// 关联动作
    action: SftpBrowserAction,
}

/// 构建文件右键菜单项列表
fn build_file_menu_items(entry_index: usize) -> Vec<MenuItem> {
    vec![
        MenuItem {
            label: String::from("打开"),
            action: SftpBrowserAction::OpenEntry(entry_index),
        },
        MenuItem {
            label: String::from("下载"),
            action: SftpBrowserAction::DownloadEntry(entry_index),
        },
        MenuItem {
            label: String::from("重命名"),
            action: SftpBrowserAction::RenameEntry(entry_index),
        },
        MenuItem {
            label: String::from("删除"),
            action: SftpBrowserAction::DeleteEntry(entry_index),
        },
        MenuItem {
            label: String::from("详细信息"),
            action: SftpBrowserAction::DetailsEntry(entry_index),
        },
    ]
}

/// 渲染单个菜单项
fn render_menu_item(
    label: &str,
    action: SftpBrowserAction,
    appearance: &Appearance,
    position_id: &str,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let text_color = theme.active_ui_text_color();
    let hover_bg = theme.surface_3();
    let default_bg = theme.surface_2();
    let ui_font = appearance.ui_font_family();
    let ui_font_size = appearance.ui_font_size();
    let label_owned = label.to_string();

    let item_el = Hoverable::new(Default::default(), move |state| {
        let bg = if state.is_hovered() || state.is_clicked() {
            hover_bg
        } else {
            default_bg
        };
        let text_el = Text::new_inline(label_owned.clone(), ui_font, ui_font_size)
            .with_color(text_color.into())
            .finish();
        Container::new(text_el)
            .with_background(bg)
            .with_padding_left(12.0)
            .with_padding_right(12.0)
            .with_padding_top(6.0)
            .with_padding_bottom(6.0)
            .finish()
    })
    .with_cursor(Cursor::PointingHand)
    .on_mouse_down(move |ctx, _, _| {
        ctx.dispatch_typed_action(action.clone());
        ctx.dispatch_typed_action(SftpBrowserAction::CloseContextMenu);
    })
    .finish();

    SavePosition::new(item_el, position_id).finish()
}

/// 渲染右键上下文菜单
pub fn render_context_menu(state: &ContextMenuState, appearance: &Appearance) -> Box<dyn Element> {
    let theme = appearance.theme();
    let menu_items = build_file_menu_items(state.entry_index);

    let mut col = Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_main_axis_size(MainAxisSize::Min);

    for item in &menu_items {
        let position_id = match &item.action {
            SftpBrowserAction::OpenEntry(_) => "sftp_ctx:open",
            SftpBrowserAction::DownloadEntry(_) => "sftp_ctx:download",
            SftpBrowserAction::RenameEntry(_) => "sftp_ctx:rename",
            SftpBrowserAction::DeleteEntry(_) => "sftp_ctx:delete",
            SftpBrowserAction::DetailsEntry(_) => "sftp_ctx:details",
            SftpBrowserAction::NavigateTo(_)
            | SftpBrowserAction::GoUp
            | SftpBrowserAction::GoBack
            | SftpBrowserAction::GoForward
            | SftpBrowserAction::Refresh
            | SftpBrowserAction::SelectEntry(_)
            | SftpBrowserAction::UploadFile
            | SftpBrowserAction::NewFolder
            | SftpBrowserAction::ConfirmDelete
            | SftpBrowserAction::ConfirmRename
            | SftpBrowserAction::ConfirmNewFolder
            | SftpBrowserAction::ConfirmOverwrite
            | SftpBrowserAction::ContextMenu { .. }
            | SftpBrowserAction::CloseContextMenu
            | SftpBrowserAction::CloseDialog
            | SftpBrowserAction::SetSearchFilter(_)
            | SftpBrowserAction::ClearSearchFilter
            | SftpBrowserAction::NavigateUp
            | SftpBrowserAction::DeleteSelected
            | SftpBrowserAction::CreateFolder
            | SftpBrowserAction::DragFilesEnter
            | SftpBrowserAction::DragFilesLeave
            | SftpBrowserAction::DragAndDropFiles(_)
            | SftpBrowserAction::ExecuteUpload(_)
            | SftpBrowserAction::DownloadSaveAs { .. }
            | SftpBrowserAction::ConfirmMove
            | SftpBrowserAction::CancelTransfer(_)
            | SftpBrowserAction::ToggleTransferPanel
            | SftpBrowserAction::ConfirmCloseTransferPanel => "sftp_ctx:unknown",
        };
        let el = render_menu_item(&item.label, item.action.clone(), appearance, position_id);
        col.add_child(el);
    }

    let menu_inner = ConstrainedBox::new(
        Container::new(col.finish())
            .with_background(theme.surface_2())
            .with_border(Border::all(1.0).with_border_color(theme.surface_3().into()))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.0)))
            .with_uniform_padding(4.0)
            .finish(),
    )
    .with_width(CONTEXT_MENU_WIDTH)
    .finish();

    Dismiss::new(menu_inner)
        .prevent_interaction_with_other_elements()
        .on_dismiss(|ctx, _| {
            ctx.dispatch_typed_action(SftpBrowserAction::CloseContextMenu);
        })
        .finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pathfinder_geometry::vector::Vector2F;
    use warpui::TypedActionView;

    // ============================================================
    // ContextMenuState 测试
    // ============================================================

    /// 测试 ContextMenuState 构造函数正确设置字段
    #[test]
    fn test_context_menu_state_new() {
        let position = Vector2F::new(100.0, 200.0);
        let state = ContextMenuState::new(3, position);
        assert_eq!(state.entry_index, 3);
        assert_eq!(state.position, position);
    }

    /// 测试 ContextMenuState 在 index=0 时正确构造
    #[test]
    fn test_context_menu_state_zero_index() {
        let position = Vector2F::new(0.0, 0.0);
        let state = ContextMenuState::new(0, position);
        assert_eq!(state.entry_index, 0);
        assert_eq!(state.position, position);
    }

    /// 测试 ContextMenuState 大索引值
    #[test]
    fn test_context_menu_state_large_index() {
        let position = Vector2F::new(500.0, 600.0);
        let state = ContextMenuState::new(usize::MAX, position);
        assert_eq!(state.entry_index, usize::MAX);
    }

    /// 测试 ContextMenuState 负坐标（Vector2F 支持负值）
    #[test]
    fn test_context_menu_state_negative_position() {
        let position = Vector2F::new(-50.0, -100.0);
        let state = ContextMenuState::new(1, position);
        assert_eq!(state.position, position);
    }

    // ============================================================
    // build_file_menu_items 测试
    // ============================================================

    /// 测试菜单项数量为 5
    #[test]
    fn test_build_file_menu_items_count() {
        let items = build_file_menu_items(0);
        assert_eq!(items.len(), 5, "应有 5 个菜单项");
    }

    /// 测试菜单项标签正确
    #[test]
    fn test_build_file_menu_items_labels() {
        let items = build_file_menu_items(0);
        let expected_labels = ["打开", "下载", "重命名", "删除", "详细信息"];
        for (item, expected) in items.iter().zip(expected_labels.iter()) {
            assert_eq!(&item.label.as_str(), expected, "标签应为 {}", expected);
        }
    }

    /// 测试菜单项动作绑定正确的 index
    #[test]
    fn test_build_file_menu_items_actions_index() {
        let index = 7;
        let items = build_file_menu_items(index);

        assert!(matches!(&items[0].action, SftpBrowserAction::OpenEntry(7)));
        assert!(matches!(
            &items[1].action,
            SftpBrowserAction::DownloadEntry(7)
        ));
        assert!(matches!(
            &items[2].action,
            SftpBrowserAction::RenameEntry(7)
        ));
        assert!(matches!(
            &items[3].action,
            SftpBrowserAction::DeleteEntry(7)
        ));
        assert!(matches!(
            &items[4].action,
            SftpBrowserAction::DetailsEntry(7)
        ));
    }

    /// 测试 index=0 时菜单项动作正确
    #[test]
    fn test_build_file_menu_items_zero_index() {
        let items = build_file_menu_items(0);
        assert!(matches!(&items[0].action, SftpBrowserAction::OpenEntry(0)));
        assert!(matches!(
            &items[3].action,
            SftpBrowserAction::DeleteEntry(0)
        ));
        assert!(matches!(
            &items[4].action,
            SftpBrowserAction::DetailsEntry(0)
        ));
    }

    // ============================================================
    // render_context_menu 渲染测试（通过 browser 视图验证）
    // ============================================================

    /// 通过 browser 视图触发 ContextMenu 后渲染不 panic
    #[test]
    fn test_render_context_menu_via_browser() {
        use crate::settings_view::keybindings::KeybindingChangedNotifier;
        use crate::test_util::settings::initialize_settings_for_tests;
        use warp_core::ui::appearance::Appearance;

        warpui::App::test((), |mut app| async move {
            initialize_settings_for_tests(&mut app);
            app.add_singleton_model(|_| Appearance::mock());
            app.add_singleton_model(|_| KeybindingChangedNotifier::mock());
            app.add_singleton_model(|_| crate::workspace::ToastStack);

            let temp_db = std::env::temp_dir().join("warp_sftp_ctx_test.sqlite");
            let _ = warp_ssh_manager::set_database_path(temp_db);

            let (_, view) = app.add_window(warpui::platform::WindowStyle::NotStealFocus, |ctx| {
                crate::sftp_manager::browser::SftpBrowserView::new("test-node".to_string(), ctx)
            });

            // 触发右键菜单
            view.update(&mut app, |view, ctx| {
                view.handle_action(
                    &SftpBrowserAction::ContextMenu {
                        index: 2,
                        position: Vector2F::new(150.0, 250.0),
                    },
                    ctx,
                );
            });

            // 渲染不应 panic（视图会自动重新渲染）
            view.read(&app, |view, _| {
                assert!(view.context_menu.is_some(), "菜单应已打开");
            });
        });
    }
}
