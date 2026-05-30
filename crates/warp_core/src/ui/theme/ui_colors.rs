//! UI 颜色覆盖映射结构体。
//! 从独立的 UI 主题文件加载，提供可选的 UI 颜色覆盖。
//! 所有字段为 Option，未设置时回退到 WarpTheme 的程序化派生值。

use serde::{Deserialize, Serialize};
use warpui::color::ColorU;

use crate::ui::color::hex_color_alpha;

/// UI 颜色覆盖映射。所有字段可选，缺失时使用 WarpTheme 的默认派生值。
#[derive(Serialize, Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct UiColors {
    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub surface_1: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub surface_2: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub surface_3: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub border: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub focus_border: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub split_pane_border: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub main_text: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub sub_text: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub hint_text: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub disabled_text: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub selection: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub text_selection: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub hover: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub active: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub warning: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub error: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub success: Option<ColorU>,

    #[serde(default, with = "hex_color_alpha::option", skip_serializing_if = "Option::is_none")]
    pub link: Option<ColorU>,
}

#[cfg(test)]
#[path = "ui_colors_tests.rs"]
mod tests;
