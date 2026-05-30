use super::*;

/// 验证 UiColors 全部为 Option::None 时能正确反序列化（空 ui_colors 块）。
#[test]
fn deserialize_empty_ui_colors() {
    let yaml = r##"---
{}
"##;
    let colors: UiColors = serde_yaml::from_str(yaml).expect("反序列化失败");
    assert!(colors.surface_1.is_none());
    assert!(colors.border.is_none());
    assert!(colors.main_text.is_none());
}

/// 验证 UiColors 能正确反序列化带 alpha 的颜色。
#[test]
fn deserialize_ui_colors_with_alpha() {
    let yaml = r##"---
surface_1: "#202122"
surface_2: "#242526"
surface_3: "#2A2B2C"
border: "#333536"
focus_border: "#3994BCB3"
selection: "#3994BC33"
hover: "#FFFFFF0D"
"##;
    let colors: UiColors = serde_yaml::from_str(yaml).expect("反序列化失败");

    assert_eq!(colors.surface_1.unwrap(), ColorU { r: 0x20, g: 0x21, b: 0x22, a: 255 });
    assert_eq!(colors.surface_2.unwrap(), ColorU { r: 0x24, g: 0x25, b: 0x26, a: 255 });
    assert_eq!(colors.focus_border.unwrap(), ColorU { r: 0x39, g: 0x94, b: 0xBC, a: 0xB3 });
    assert_eq!(colors.selection.unwrap(), ColorU { r: 0x39, g: 0x94, b: 0xBC, a: 0x33 });
    assert_eq!(colors.hover.unwrap(), ColorU { r: 0xFF, g: 0xFF, b: 0xFF, a: 0x0D });
    // 未设置的字段应为 None
    assert!(colors.main_text.is_none());
}

/// 验证 UiColors 序列化时跳过 None 字段。
#[test]
fn serialize_ui_colors_skips_none() {
    let colors = UiColors {
        surface_1: Some(ColorU { r: 0x20, g: 0x21, b: 0x22, a: 255 }),
        surface_2: None,
        border: Some(ColorU { r: 0x33, g: 0x35, b: 0x36, a: 255 }),
        surface_3: None,
        focus_border: None,
        split_pane_border: None,
        main_text: None,
        sub_text: None,
        hint_text: None,
        disabled_text: None,
        selection: None,
        text_selection: None,
        hover: None,
        active: None,
        warning: None,
        error: None,
        success: None,
        link: None,
    };
    let yaml = serde_yaml::to_string(&colors).expect("序列化失败");
    assert!(yaml.contains("surface_1"));
    assert!(yaml.contains("border"));
    assert!(!yaml.contains("surface_2"));
    assert!(!yaml.contains("main_text"));
}
