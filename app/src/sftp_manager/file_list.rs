//! 文件列表渲染组件
//!
//! 提供文件列表表头和文件行的渲染功能。
//! author: logic
//! date: 2026-05-26

use std::collections::HashSet;

use warp_core::ui::appearance::Appearance;
use warp_core::ui::theme::color::internal_colors;
use warpui::elements::{
    ConstrainedBox, Container, CrossAxisAlignment, Fill, Flex, Hoverable, MouseStateHandle,
    ParentElement, SavePosition, Shrinkable, Text,
};
use warpui::platform::Cursor;
use warpui::Element;

use crate::sftp_manager::browser::SftpBrowserAction;
use crate::sftp_manager::types::{format_size, FileEntry, FileEntryType};
use crate::ui_components::icons::Icon;

/// 文件大小列宽度
const FILE_SIZE_WIDTH: f32 = 80.0;
/// 文件日期列宽度
const FILE_DATE_WIDTH: f32 = 120.0;

/// 根据文件条目类型返回对应图标
pub fn file_icon(entry_type: &FileEntryType) -> Icon {
    match entry_type {
        FileEntryType::Directory | FileEntryType::Symlink => Icon::Folder,
        FileEntryType::File | FileEntryType::Other => Icon::File,
    }
}

/// 渲染单个文件行
pub fn render_file_row(
    entry: &FileEntry,
    index: usize,
    is_selected: bool,
    mouse_handle: MouseStateHandle,
    appearance: &Appearance,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let bg_color = if is_selected {
        internal_colors::neutral_3(theme)
    } else {
        theme.background().into_solid()
    };
    let icon_color = if matches!(
        entry.file_type,
        FileEntryType::Directory | FileEntryType::Symlink
    ) {
        theme.accent().into_solid()
    } else {
        theme.sub_text_color(theme.background()).into_solid()
    };
    let text_color = theme.active_ui_text_color();
    let sub_color = theme.sub_text_color(theme.background());

    let name = entry.name.clone();
    let file_type = entry.file_type;
    let size = entry.size;
    let modified = entry.modified.clone();
    let ui_font = appearance.ui_font_family();
    let ui_font_size = appearance.ui_font_size();
    let bg_fill: Fill = bg_color.into();

    Hoverable::new(mouse_handle, move |_| {
        // 图标
        let icon_el = ConstrainedBox::new(
            file_icon(&file_type)
                .to_warpui_icon(icon_color.into())
                .finish(),
        )
        .with_width(16.0)
        .with_height(16.0)
        .finish();

        // 名称
        let name_el = Shrinkable::new(
            1.0,
            Text::new_inline(name.clone(), ui_font, ui_font_size)
                .with_color(text_color.into())
                .finish(),
        )
        .finish();

        // 大小
        let size_text = if matches!(file_type, FileEntryType::Directory | FileEntryType::Symlink) {
            String::from("--")
        } else {
            format_size(size)
        };
        let size_el = ConstrainedBox::new(
            Text::new_inline(size_text, ui_font, ui_font_size)
                .with_color(sub_color.into())
                .finish(),
        )
        .with_width(FILE_SIZE_WIDTH)
        .finish();

        // 修改日期
        let date_text = modified.clone().unwrap_or_else(|| String::from("--"));
        let date_el = ConstrainedBox::new(
            Text::new_inline(date_text, ui_font, ui_font_size)
                .with_color(sub_color.into())
                .finish(),
        )
        .with_width(FILE_DATE_WIDTH)
        .finish();

        // 组装行内容
        let row_content = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.0)
            .with_child(icon_el)
            .with_child(name_el)
            .with_child(size_el)
            .with_child(date_el)
            .finish();

        Container::new(row_content)
            .with_background(bg_fill)
            .with_padding_left(8.0)
            .with_padding_right(8.0)
            .with_padding_top(4.0)
            .with_padding_bottom(4.0)
            .finish()
    })
    .with_cursor(Cursor::PointingHand)
    .on_click(move |ctx, _, _| {
        ctx.dispatch_typed_action(SftpBrowserAction::SelectEntry(index));
    })
    .on_double_click(move |ctx, _, _| {
        ctx.dispatch_typed_action(SftpBrowserAction::OpenEntry(index));
    })
    .on_right_click(move |ctx, _, position| {
        use super::browser::SFTP_PANEL_POSITION_ID;
        let offset = match ctx.element_position_by_id(SFTP_PANEL_POSITION_ID) {
            Some(bounds) => position - bounds.origin(),
            None => position,
        };
        ctx.dispatch_typed_action(SftpBrowserAction::ContextMenu {
            index,
            position: offset,
        });
    })
    .finish()
}

/// 渲染文件列表头部
pub fn render_header(appearance: &Appearance) -> Box<dyn Element> {
    let theme = appearance.theme();
    let header_color = theme.sub_text_color(theme.background());

    let name_el = Shrinkable::new(
        1.0,
        Text::new_inline(
            String::from("名称"),
            appearance.ui_font_family(),
            appearance.ui_font_size(),
        )
        .with_color(header_color.into())
        .finish(),
    )
    .finish();

    let size_el = ConstrainedBox::new(
        Text::new_inline(
            String::from("大小"),
            appearance.ui_font_family(),
            appearance.ui_font_size(),
        )
        .with_color(header_color.into())
        .finish(),
    )
    .with_width(FILE_SIZE_WIDTH)
    .finish();

    let date_el = ConstrainedBox::new(
        Text::new_inline(
            String::from("修改时间"),
            appearance.ui_font_family(),
            appearance.ui_font_size(),
        )
        .with_color(header_color.into())
        .finish(),
    )
    .with_width(FILE_DATE_WIDTH)
    .finish();

    let header_row = Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_spacing(24.0) // 图标16 + 间距8
        .with_child(name_el)
        .with_child(size_el)
        .with_child(date_el)
        .finish();

    Container::new(header_row)
        .with_padding_left(8.0)
        .with_padding_right(8.0)
        .with_padding_top(4.0)
        .with_padding_bottom(4.0)
        .finish()
}

/// 渲染文件行列表
pub fn render_file_rows(
    entries: &[FileEntry],
    filtered_indices: &[usize],
    selected: &HashSet<usize>,
    mouse_handles: &[MouseStateHandle],
    appearance: &Appearance,
) -> Box<dyn Element> {
    let mut col = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);

    for &index in filtered_indices {
        let entry = &entries[index];
        let is_selected = selected.contains(&index);
        let mouse_handle = mouse_handles.get(index).cloned().unwrap_or_default();
        let row = render_file_row(entry, index, is_selected, mouse_handle, appearance);
        let position_id = format!("sftp_row:{index}");
        let positioned = SavePosition::new(row, &position_id).finish();
        col.add_child(positioned);
    }

    col.finish()
}
