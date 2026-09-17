//! 传输面板渲染组件
//!
//! 提供文件传输进度面板的渲染功能，包括传输方向图标、状态标签、进度条和传输列表。
//! author: logic
//! date: 2026-05-26

use warp_core::ui::appearance::Appearance;
use warpui::elements::{
    Clipped, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Flex, Hoverable,
    MainAxisAlignment, MainAxisSize, MouseStateHandle, ParentElement, Radius, SavePosition,
    Shrinkable, Text,
};
use warpui::platform::Cursor;
use warpui::Element;

use crate::sftp_manager::browser::SftpBrowserAction;
use crate::sftp_manager::types::{TransferDirection, TransferState, TransferTask};
use crate::ui_components::icons::Icon;

/// 进度条高度
const PROGRESS_BAR_HEIGHT: f32 = 4.0;
/// 面板内边距
const PANEL_PADDING: f32 = 8.0;

/// 渲染传输方向图标
fn render_direction_icon(
    direction: &TransferDirection,
    appearance: &Appearance,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let icon_color = theme.sub_text_color(theme.background());

    let icon = match direction {
        TransferDirection::Upload => Icon::UploadCloud,
        TransferDirection::Download => Icon::Download,
    };

    ConstrainedBox::new(icon.to_warpui_icon(icon_color).finish())
        .with_width(14.0)
        .with_height(14.0)
        .finish()
}

/// 渲染传输状态标签
fn render_state_label(state: &TransferState, appearance: &Appearance) -> Box<dyn Element> {
    let theme = appearance.theme();
    let ui_font = appearance.ui_font_family();
    let ui_font_size = appearance.ui_font_size();

    let (label, color) = match state {
        TransferState::Pending => (
            String::from("等待中"),
            theme.sub_text_color(theme.background()),
        ),
        TransferState::InProgress => (String::from("传输中"), theme.accent()),
        TransferState::Completed => (String::from("已完成"), theme.ui_green_color().into()),
        TransferState::Failed(_) => (String::from("失败"), theme.ui_error_color().into()),
        TransferState::Cancelled => (
            String::from("已取消"),
            theme.sub_text_color(theme.background()),
        ),
    };

    Text::new_inline(label, ui_font, ui_font_size)
        .with_color(color.into())
        .finish()
}

/// 渲染进度条
fn render_progress_bar(progress: u8, appearance: &Appearance) -> Box<dyn Element> {
    let theme = appearance.theme();

    if progress == 0 {
        return ConstrainedBox::new(
            Container::new(Flex::row().finish())
                .with_background(theme.surface_3())
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(2.0)))
                .finish(),
        )
        .with_height(PROGRESS_BAR_HEIGHT)
        .finish();
    }

    let remaining = 100u8.saturating_sub(progress);

    // 进度填充
    let fill = ConstrainedBox::new(
        Container::new(Flex::row().finish())
            .with_background(theme.accent())
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(2.0)))
            .finish(),
    )
    .with_height(PROGRESS_BAR_HEIGHT)
    .finish();

    // 空白部分
    let spacer = Shrinkable::new(
        remaining as f32,
        ConstrainedBox::new(Flex::row().finish())
            .with_height(PROGRESS_BAR_HEIGHT)
            .finish(),
    )
    .finish();

    ConstrainedBox::new(
        Container::new(
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(Shrinkable::new(progress as f32, fill).finish())
                .with_child(spacer)
                .finish(),
        )
        .with_background(theme.surface_3())
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(2.0)))
        .finish(),
    )
    .with_height(PROGRESS_BAR_HEIGHT)
    .finish()
}

/// 渲染单个传输行
fn render_transfer_row(task: &TransferTask, appearance: &Appearance) -> Box<dyn Element> {
    // 方向图标
    let dir_icon = render_direction_icon(&task.direction, appearance);

    // 文件名
    let file_name = task
        .source_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let name_el = Text::new_inline(
        file_name,
        appearance.ui_font_family(),
        appearance.ui_font_size(),
    )
    .with_color(appearance.theme().active_ui_text_color().into())
    .finish();

    // 状态标签
    let state_el = render_state_label(&task.state, appearance);

    // 第一行：图标 + 文件名 + 状态 + 取消按钮
    let mut top_row = Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_spacing(6.0)
        .with_child(dir_icon)
        .with_child(Shrinkable::new(1.0, name_el).finish())
        .with_child(state_el);

    // 传输中的任务显示取消按钮
    if matches!(task.state, TransferState::InProgress) {
        let task_id = task.id;
        let icon_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background());
        let position_id = format!("sftp_btn:cancel_transfer:{task_id}");

        let cancel_el = Hoverable::new(Default::default(), move |_| {
            let icon_el = ConstrainedBox::new(Icon::X.to_warpui_icon(icon_color).finish())
                .with_width(12.0)
                .with_height(12.0)
                .finish();
            Container::new(icon_el).with_uniform_padding(2.0).finish()
        })
        .with_cursor(Cursor::PointingHand)
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(SftpBrowserAction::CancelTransfer(task_id));
        })
        .finish();

        let positioned = SavePosition::new(cancel_el, &position_id).finish();
        top_row = top_row.with_child(Clipped::new(positioned).finish());
    }

    let mut col = Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_spacing(4.0)
        .with_child(top_row.finish());

    // 进度条（仅传输中显示）
    if matches!(task.state, TransferState::InProgress) {
        let bar = render_progress_bar(task.progress_percent(), appearance);
        col.add_child(bar);
    }

    Container::new(col.finish())
        .with_padding_top(4.0)
        .with_padding_bottom(4.0)
        .finish()
}

/// 渲染文件传输面板（主入口）
///
/// 始终显示传输任务列表，标题栏右侧包含关闭按钮。
pub fn render_transfer_panel(
    transfers: &[TransferTask],
    appearance: &Appearance,
    close_btn_state: MouseStateHandle,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let text_color = theme.active_ui_text_color();
    let ui_font = appearance.ui_font_family();
    let ui_font_size = appearance.ui_font_size();

    // 标题栏
    let count = transfers.len();
    let title_text = format!("传输 ({count})");

    let title_el = Text::new_inline(title_text, ui_font, ui_font_size)
        .with_color(text_color.into())
        .finish();

    // 关闭按钮
    let icon_color = theme.sub_text_color(theme.background());
    let close_btn = Hoverable::new(close_btn_state, move |_| {
        let icon_el = ConstrainedBox::new(Icon::X.to_warpui_icon(icon_color).finish())
            .with_width(12.0)
            .with_height(12.0)
            .finish();
        Container::new(icon_el)
            .with_padding_left(4.0)
            .with_padding_right(4.0)
            .with_padding_top(4.0)
            .with_padding_bottom(4.0)
            .finish()
    })
    .with_cursor(Cursor::PointingHand)
    .on_click(|ctx, _, _| {
        ctx.dispatch_typed_action(SftpBrowserAction::ToggleTransferPanel);
    })
    .finish();

    let header = Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_main_axis_size(MainAxisSize::Max)
        .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
        .with_child(title_el)
        .with_child(close_btn)
        .finish();

    let mut col = Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_child(header);

    let rows_col = {
        let mut inner = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(4.0);
        for task in transfers {
            let row = render_transfer_row(task, appearance);
            inner.add_child(row);
        }
        inner.finish()
    };
    col.add_child(rows_col);

    Container::new(col.finish())
        .with_uniform_padding(PANEL_PADDING)
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.0)))
        .with_background(theme.surface_2())
        .finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::rc::Rc;

    use pathfinder_geometry::vector::vec2f;
    use warpui::platform::WindowStyle;
    use warpui::{
        App, AppContext, Entity, Event, Presenter, SingletonEntity, TypedActionView, View,
        ViewContext, WindowInvalidation,
    };

    struct TransferPanelTestView {
        transfers: Vec<TransferTask>,
        close_btn_state: MouseStateHandle,
    }

    impl TransferPanelTestView {
        /// 创建用于验证传输面板点击行为的测试视图
        fn new() -> Self {
            Self {
                transfers: vec![make_transfer_task(1)],
                close_btn_state: MouseStateHandle::default(),
            }
        }
    }

    impl Entity for TransferPanelTestView {
        type Event = ();
    }

    impl TypedActionView for TransferPanelTestView {
        type Action = SftpBrowserAction;

        /// 处理传输面板派发的测试动作
        fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
            if matches!(action, SftpBrowserAction::CancelTransfer(_)) {
                ctx.notify();
            }
        }
    }

    impl View for TransferPanelTestView {
        fn ui_name() -> &'static str {
            "TransferPanelTestView"
        }

        /// 渲染测试用传输面板
        fn render(&self, app: &AppContext) -> Box<dyn Element> {
            let appearance = Appearance::as_ref(app);
            render_transfer_panel(&self.transfers, appearance, self.close_btn_state.clone())
        }
    }

    /// 初始化传输面板测试所需的外观单例
    fn initialize_app(app: &mut App) {
        app.add_singleton_model(|_| Appearance::mock());
    }

    /// 创建一个测试用传输任务
    fn make_transfer_task(id: usize) -> TransferTask {
        TransferTask::new(
            id,
            PathBuf::from(format!("/remote/file_{id}.txt")),
            PathBuf::from(format!("/local/file_{id}.txt")),
            TransferDirection::Download,
            1024,
        )
    }

    /// 验证点击传输面板背景区域不会影响传输内容展示
    #[test]
    fn clicking_panel_background_does_not_toggle_transfer_panel() {
        App::test((), |mut app| async move {
            initialize_app(&mut app);
            let (window_id, view) =
                app.add_window(WindowStyle::NotStealFocus, |_| TransferPanelTestView::new());
            let root_view_id = app.root_view_id(window_id).expect("测试窗口应包含根视图");
            let presenter = Rc::new(RefCell::new(Presenter::new(window_id)));
            let invalidation = WindowInvalidation {
                updated: HashSet::from([root_view_id]),
                ..Default::default()
            };

            app.update({
                let presenter = presenter.clone();
                move |ctx| {
                    presenter.borrow_mut().invalidate(invalidation, ctx);
                    presenter
                        .borrow_mut()
                        .build_scene(vec2f(320., 120.), 1., None, ctx);

                    ctx.simulate_window_event(
                        Event::LeftMouseDown {
                            position: vec2f(4., 12.),
                            modifiers: Default::default(),
                            click_count: 1,
                            is_first_mouse: false,
                        },
                        window_id,
                        presenter.clone(),
                    );
                    ctx.simulate_window_event(
                        Event::LeftMouseUp {
                            position: vec2f(4., 12.),
                            modifiers: Default::default(),
                        },
                        window_id,
                        presenter,
                    );
                }
            });

            view.read(&app, |view, _| {
                assert_eq!(
                    view.transfers.len(),
                    1,
                    "点击传输面板背景区域后传输内容应保持显示"
                );
            });
        });
    }
}
