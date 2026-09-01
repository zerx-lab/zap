//! 对话框渲染组件
//!
//! 提供删除确认、重命名、新建文件夹、文件详情等对话框的渲染功能。
//! author: logic
//! date: 2026-05-26

use std::path::PathBuf;

use warp_core::ui::appearance::Appearance;
use warp_core::ui::icons::Icon;
use warpui::elements::{
    Border, ChildView, Clipped, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment,
    Dismiss, Flex, Hoverable, MainAxisAlignment, MainAxisSize, MouseStateHandle, ParentElement,
    Radius, SavePosition, Shrinkable, Text,
};
use warpui::platform::Cursor;
use warpui::Element;
use warpui::ViewHandle;

use crate::editor::EditorView;
use crate::sftp_manager::browser::SftpBrowserAction;
use crate::sftp_manager::types::{format_size, Dialog, FileEntry, TransferDirection};

/// 对话框最大宽度
const DIALOG_MAX_WIDTH: f32 = 360.0;
/// 对话框最大高度
const DIALOG_MAX_HEIGHT: f32 = 500.0;
/// 对话框内边距
const DIALOG_PADDING: f32 = 16.0;
/// 按钮最小宽度
const BUTTON_MIN_WIDTH: f32 = 80.0;
/// 按钮高度
const BUTTON_HEIGHT: f32 = 32.0;

/// 弹窗外壳容器
///
/// 提供统一的背景色、圆角、边框和内边距。
fn dialog_shell(content: Box<dyn Element>, appearance: &Appearance) -> Box<dyn Element> {
    let theme = appearance.theme();
    Container::new(content)
        .with_background(theme.surface_1())
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.0)))
        .with_border(Border::all(1.0).with_border_fill(theme.surface_3()))
        .with_uniform_padding(DIALOG_PADDING)
        .finish()
}

/// 渲染标题行（标题 + 关闭按钮）
///
/// 标题使用 Shrinkable 包裹以支持自适应宽度，右侧放置 X 关闭按钮。
fn render_title_bar(
    title: &str,
    appearance: &Appearance,
    close_btn_state: MouseStateHandle,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let text_color = theme.active_ui_text_color();
    let ui_font = appearance.ui_font_family();
    let ui_font_size = appearance.ui_font_size();

    let title_el = Shrinkable::new(
        1.0,
        Text::new(title.to_string(), ui_font, ui_font_size)
            .with_color(text_color.into())
            .finish(),
    )
    .finish();

    let close_btn = render_icon_close_button(appearance, close_btn_state);

    Flex::row()
        .with_main_axis_size(MainAxisSize::Max)
        .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(title_el)
        .with_child(close_btn)
        .finish()
}

/// 渲染 X 图标关闭按钮
fn render_icon_close_button(
    appearance: &Appearance,
    mouse_state: MouseStateHandle,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let icon_color = theme.sub_text_color(theme.background());
    let icon_el = ConstrainedBox::new(Icon::X.to_warpui_icon(icon_color).finish())
        .with_width(12.0)
        .with_height(12.0)
        .finish();
    Hoverable::new(mouse_state, move |_| {
        Container::new(icon_el)
            .with_padding_left(4.0)
            .with_padding_right(4.0)
            .with_padding_top(4.0)
            .with_padding_bottom(4.0)
            .finish()
    })
    .with_cursor(Cursor::PointingHand)
    .on_click(|ctx, _, _| {
        ctx.dispatch_typed_action(SftpBrowserAction::CloseDialog);
    })
    .finish()
}

/// 渲染操作按钮组件
///
/// is_accent 为 true 时使用 accent 色背景，否则使用 surface_2 背景。
fn render_button(
    label: &str,
    is_accent: bool,
    appearance: &Appearance,
    action: SftpBrowserAction,
    mouse_state: MouseStateHandle,
    position_id: Option<&str>,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let ui_font = appearance.ui_font_family();
    let ui_font_size = appearance.ui_font_size();
    let bg = if is_accent {
        theme.accent()
    } else {
        theme.surface_2()
    };
    let text_color = if is_accent {
        theme.background()
    } else {
        theme.active_ui_text_color()
    };
    let label_owned = label.to_string();

    let btn_el = Hoverable::new(mouse_state, move |_| {
        let text_el = Text::new(label_owned.clone(), ui_font, ui_font_size)
            .with_color(text_color.into())
            .finish();
        let centered = Flex::row()
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_main_axis_size(MainAxisSize::Max)
            .with_child(text_el)
            .finish();
        Container::new(
            ConstrainedBox::new(centered)
                .with_width(BUTTON_MIN_WIDTH)
                .with_height(BUTTON_HEIGHT)
                .finish(),
        )
        .with_background(bg)
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)))
        .finish()
    })
    .with_cursor(Cursor::PointingHand)
    .on_click(move |ctx, _, _| {
        ctx.dispatch_typed_action(action.clone());
    })
    .finish();

    match position_id {
        Some(id) => SavePosition::new(btn_el, id).finish(),
        None => btn_el,
    }
}

/// 渲染取消按钮
fn render_cancel_button(
    appearance: &Appearance,
    mouse_state: MouseStateHandle,
) -> Box<dyn Element> {
    render_button(
        "取消",
        false,
        appearance,
        SftpBrowserAction::CloseDialog,
        mouse_state,
        Some("sftp_btn:dialog_cancel"),
    )
}

/// 渲染描述性确认对话框（标题 + 描述 + 确认/取消按钮）
///
/// 适用于删除确认、移动确认、覆盖确认等场景。
fn render_confirm_dialog(
    title: &str,
    description: &str,
    confirm_label: &str,
    confirm_action: SftpBrowserAction,
    appearance: &Appearance,
    confirm_btn_state: MouseStateHandle,
    cancel_btn_state: MouseStateHandle,
    close_btn_state: MouseStateHandle,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let sub_color = theme.sub_text_color(theme.background());
    let ui_font = appearance.ui_font_family();
    let ui_font_size = appearance.ui_font_size();

    let title_bar = render_title_bar(title, appearance, close_btn_state);

    let desc_el = Shrinkable::new(
        1.0,
        Text::new(description.to_string(), ui_font, ui_font_size)
            .with_color(sub_color.into())
            .finish(),
    )
    .finish();

    let confirm_btn = render_button(
        confirm_label,
        true,
        appearance,
        confirm_action,
        confirm_btn_state,
        Some("sftp_btn:dialog_confirm"),
    );
    let cancel_btn = render_cancel_button(appearance, cancel_btn_state);

    let buttons = Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_main_axis_alignment(MainAxisAlignment::End)
        .with_spacing(8.0)
        .with_child(confirm_btn)
        .with_child(cancel_btn)
        .finish();

    let content = Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_spacing(12.0)
        .with_child(title_bar)
        .with_child(desc_el)
        .with_child(buttons)
        .finish();

    let dialog_body = ConstrainedBox::new(dialog_shell(content, appearance))
        .with_max_width(DIALOG_MAX_WIDTH)
        .with_max_height(DIALOG_MAX_HEIGHT)
        .finish();

    wrap_dismiss(dialog_body)
}

/// 将弹窗内容包裹在 Dismiss + 居中容器中
fn wrap_dismiss(dialog_content: Box<dyn Element>) -> Box<dyn Element> {
    Dismiss::new(dialog_content)
        .prevent_interaction_with_other_elements()
        .on_dismiss(|ctx, _| {
            ctx.dispatch_typed_action(SftpBrowserAction::CloseDialog);
        })
        .finish()
}

/// 渲染删除确认对话框
fn render_delete_confirm(
    paths: &[PathBuf],
    appearance: &Appearance,
    confirm_btn_state: MouseStateHandle,
    cancel_btn_state: MouseStateHandle,
    close_btn_state: MouseStateHandle,
) -> Box<dyn Element> {
    let count = paths.len();
    let desc = if count == 1 {
        let name = paths[0]
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| paths[0].display().to_string());
        format!("确定要删除 \"{name}\" 吗？此操作不可撤销。")
    } else {
        format!("确定要删除 {count} 个项目吗？此操作不可撤销。")
    };

    render_confirm_dialog(
        "确认删除",
        &desc,
        "删除",
        SftpBrowserAction::ConfirmDelete,
        appearance,
        confirm_btn_state,
        cancel_btn_state,
        close_btn_state,
    )
}

/// 渲染重命名对话框
fn render_rename(
    original_name: &str,
    editor: &ViewHandle<EditorView>,
    appearance: &Appearance,
    confirm_btn_state: MouseStateHandle,
    cancel_btn_state: MouseStateHandle,
    close_btn_state: MouseStateHandle,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let sub_color = theme.sub_text_color(theme.background());
    let ui_font = appearance.ui_font_family();
    let ui_font_size = appearance.ui_font_size();

    // 标题行
    let title_bar = render_title_bar("重命名", appearance, close_btn_state);

    // 当前名称提示
    let hint = format!("当前名称: {original_name}");
    let hint_el = Shrinkable::new(
        1.0,
        Text::new(hint, ui_font, ui_font_size)
            .with_color(sub_color.into())
            .finish(),
    )
    .finish();

    // 编辑器 — Shrinkable + Clipped 防止长文件名溢出
    let editor_el = Container::new(
        Shrinkable::new(1.0, Clipped::new(ChildView::new(editor).finish()).finish()).finish(),
    )
    .with_padding_left(8.0)
    .with_padding_right(8.0)
    .with_padding_top(4.0)
    .with_padding_bottom(4.0)
    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)))
    .with_background(theme.surface_2())
    .finish();

    // 按钮
    let confirm_btn = render_button(
        "确定",
        true,
        appearance,
        SftpBrowserAction::ConfirmRename,
        confirm_btn_state,
        Some("sftp_btn:dialog_confirm"),
    );
    let cancel_btn = render_cancel_button(appearance, cancel_btn_state);

    let buttons = Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_main_axis_alignment(MainAxisAlignment::End)
        .with_spacing(8.0)
        .with_child(confirm_btn)
        .with_child(cancel_btn)
        .finish();

    let content = Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_spacing(12.0)
        .with_child(title_bar)
        .with_child(hint_el)
        .with_child(editor_el)
        .with_child(buttons)
        .finish();

    let dialog_body = ConstrainedBox::new(dialog_shell(content, appearance))
        .with_max_width(DIALOG_MAX_WIDTH)
        .with_max_height(DIALOG_MAX_HEIGHT)
        .finish();

    wrap_dismiss(dialog_body)
}

/// 渲染新建文件夹对话框
fn render_create_folder(
    editor: &ViewHandle<EditorView>,
    appearance: &Appearance,
    confirm_btn_state: MouseStateHandle,
    cancel_btn_state: MouseStateHandle,
    close_btn_state: MouseStateHandle,
) -> Box<dyn Element> {
    let theme = appearance.theme();

    // 标题行
    let title_bar = render_title_bar("新建文件夹", appearance, close_btn_state);

    // 编辑器
    let editor_el = Container::new(
        Shrinkable::new(1.0, Clipped::new(ChildView::new(editor).finish()).finish()).finish(),
    )
    .with_padding_left(8.0)
    .with_padding_right(8.0)
    .with_padding_top(4.0)
    .with_padding_bottom(4.0)
    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)))
    .with_background(theme.surface_2())
    .finish();

    // 按钮
    let confirm_btn = render_button(
        "创建",
        true,
        appearance,
        SftpBrowserAction::ConfirmNewFolder,
        confirm_btn_state,
        Some("sftp_btn:dialog_confirm"),
    );
    let cancel_btn = render_cancel_button(appearance, cancel_btn_state);

    let buttons = Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_main_axis_alignment(MainAxisAlignment::End)
        .with_spacing(8.0)
        .with_child(confirm_btn)
        .with_child(cancel_btn)
        .finish();

    let content = Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_spacing(12.0)
        .with_child(title_bar)
        .with_child(editor_el)
        .with_child(buttons)
        .finish();

    let dialog_body = ConstrainedBox::new(dialog_shell(content, appearance))
        .with_max_width(DIALOG_MAX_WIDTH)
        .with_max_height(DIALOG_MAX_HEIGHT)
        .finish();

    wrap_dismiss(dialog_body)
}

/// 渲染单个属性行（标签 + 值）
fn detail_row(label: &str, value: &str, appearance: &Appearance) -> Box<dyn Element> {
    let theme = appearance.theme();
    let sub_color = theme.sub_text_color(theme.background());
    let text_color = theme.active_ui_text_color();
    let ui_font = appearance.ui_font_family();
    let ui_font_size = appearance.ui_font_size();

    let label_el = ConstrainedBox::new(
        Text::new(label.to_string(), ui_font, ui_font_size)
            .with_color(sub_color.into())
            .finish(),
    )
    .with_width(80.0)
    .finish();

    let value_el = Shrinkable::new(
        1.0,
        Text::new(value.to_string(), ui_font, ui_font_size)
            .with_color(text_color.into())
            .finish(),
    )
    .finish();

    Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_spacing(8.0)
        .with_child(label_el)
        .with_child(value_el)
        .finish()
}

/// 渲染文件详情对话框
fn render_file_details(
    entry: &FileEntry,
    appearance: &Appearance,
    cancel_btn_state: MouseStateHandle,
    close_btn_state: MouseStateHandle,
) -> Box<dyn Element> {
    // 标题行
    let title_bar = render_title_bar("文件详情", appearance, close_btn_state);

    // 类型
    let type_str = match entry.file_type {
        crate::sftp_manager::types::FileEntryType::File => "文件",
        crate::sftp_manager::types::FileEntryType::Directory => "目录",
        crate::sftp_manager::types::FileEntryType::Symlink => "符号链接",
        crate::sftp_manager::types::FileEntryType::Other => "其他",
    };

    // 构建属性行
    let mut rows = Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_spacing(8.0);

    rows.add_child(detail_row("类型", type_str, appearance));
    rows.add_child(detail_row("大小", &format_size(entry.size), appearance));
    let modified = entry.modified.as_deref().unwrap_or("--");
    rows.add_child(detail_row("修改时间", modified, appearance));
    let permissions = entry.permissions.as_deref().unwrap_or("--");
    rows.add_child(detail_row("权限", permissions, appearance));
    rows.add_child(detail_row(
        "路径",
        &entry.path.display().to_string(),
        appearance,
    ));

    // 关闭按钮
    let close_btn = render_cancel_button(appearance, cancel_btn_state);

    let content = Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_spacing(12.0)
        .with_child(title_bar)
        .with_child(
            ConstrainedBox::new(rows.finish())
                .with_max_height(250.0)
                .finish(),
        )
        .with_child(close_btn)
        .finish();

    let dialog_body = ConstrainedBox::new(dialog_shell(content, appearance))
        .with_max_width(DIALOG_MAX_WIDTH)
        .with_max_height(DIALOG_MAX_HEIGHT)
        .finish();

    wrap_dismiss(dialog_body)
}

/// 渲染移动对话框
fn render_move_dialog(
    source: &PathBuf,
    target_dir: &PathBuf,
    appearance: &Appearance,
    confirm_btn_state: MouseStateHandle,
    cancel_btn_state: MouseStateHandle,
    close_btn_state: MouseStateHandle,
) -> Box<dyn Element> {
    let source_name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let target_display = target_dir.display();
    let desc = format!("将 \"{source_name}\" 移动到 {target_display}");

    render_confirm_dialog(
        "移动文件",
        &desc,
        "移动",
        SftpBrowserAction::ConfirmMove,
        appearance,
        confirm_btn_state,
        cancel_btn_state,
        close_btn_state,
    )
}

/// 渲染覆盖确认对话框
fn render_overwrite_confirm(
    _source: &PathBuf,
    target: &PathBuf,
    _file_size: u64,
    direction: TransferDirection,
    appearance: &Appearance,
    confirm_btn_state: MouseStateHandle,
    cancel_btn_state: MouseStateHandle,
    close_btn_state: MouseStateHandle,
) -> Box<dyn Element> {
    let target_name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let desc = match direction {
        TransferDirection::Upload => format!("远程文件 {target_name} 已存在，是否覆盖？"),
        TransferDirection::Download => format!("目标文件 {target_name} 已存在，是否覆盖？"),
    };

    render_confirm_dialog(
        "确认覆盖",
        &desc,
        "覆盖",
        SftpBrowserAction::ConfirmOverwrite,
        appearance,
        confirm_btn_state,
        cancel_btn_state,
        close_btn_state,
    )
}

/// 渲染对话框（主入口函数）
///
/// 根据对话框类型分发到对应的渲染函数。
pub fn render_dialog(
    dialog: &Dialog,
    rename_editor: &ViewHandle<EditorView>,
    new_folder_editor: &ViewHandle<EditorView>,
    appearance: &Appearance,
    confirm_btn_state: MouseStateHandle,
    cancel_btn_state: MouseStateHandle,
    close_btn_state: MouseStateHandle,
) -> Box<dyn Element> {
    match dialog {
        Dialog::DeleteConfirm { paths, .. } => render_delete_confirm(
            paths,
            appearance,
            confirm_btn_state,
            cancel_btn_state,
            close_btn_state,
        ),
        Dialog::Rename { original_name, .. } => render_rename(
            original_name,
            rename_editor,
            appearance,
            confirm_btn_state,
            cancel_btn_state,
            close_btn_state,
        ),
        Dialog::CreateFolder { .. } => render_create_folder(
            new_folder_editor,
            appearance,
            confirm_btn_state,
            cancel_btn_state,
            close_btn_state,
        ),
        Dialog::FileDetails { entry } => {
            render_file_details(entry, appearance, cancel_btn_state, close_btn_state)
        }
        Dialog::Move { source, target_dir } => render_move_dialog(
            source,
            target_dir,
            appearance,
            confirm_btn_state,
            cancel_btn_state,
            close_btn_state,
        ),
        Dialog::OverwriteConfirm {
            source,
            target,
            file_size,
            direction,
        } => render_overwrite_confirm(
            source,
            target,
            *file_size,
            *direction,
            appearance,
            confirm_btn_state,
            cancel_btn_state,
            close_btn_state,
        ),
        Dialog::CloseTransferPanelConfirm => render_confirm_dialog(
            "关闭传输面板",
            "有正在进行的传输任务，关闭将中断所有传输并清空记录。确定要关闭吗？",
            "关闭",
            SftpBrowserAction::ConfirmCloseTransferPanel,
            appearance,
            confirm_btn_state,
            cancel_btn_state,
            close_btn_state,
        ),
    }
}
