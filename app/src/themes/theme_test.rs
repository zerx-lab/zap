use super::*;
use crate::util::color::OPAQUE;

// --- VS Code 2026 Dark 内置主题测试 ---

/// 验证 vscode_2026_dark 基础颜色正确。
#[test]
fn vscode_2026_dark_base_colors() {
    let theme = vscode_2026_dark();

    // background: #191A1B
    let bg = theme.background().into_solid();
    assert_eq!(bg, ColorU::from_u32(0x191A1BFF));

    // accent: #3994BC
    let accent = theme.accent().into_solid();
    assert_eq!(accent, ColorU::from_u32(0x3994BCFF));

    // name
    assert_eq!(theme.name(), Some("VS Code 2026 Dark".to_string()));
}

/// 验证 vscode_2026_dark 终端 normal 颜色正确。
#[test]
fn vscode_2026_dark_terminal_normal_colors() {
    let theme = vscode_2026_dark();
    let colors = theme.terminal_colors();

    assert_eq!(colors.normal.black, AnsiColor::from_u32(0x000000FF));
    assert_eq!(colors.normal.red, AnsiColor::from_u32(0xCD3131FF));
    assert_eq!(colors.normal.green, AnsiColor::from_u32(0x0DBC79FF));
    assert_eq!(colors.normal.yellow, AnsiColor::from_u32(0xE5E510FF));
    assert_eq!(colors.normal.blue, AnsiColor::from_u32(0x2472C8FF));
    assert_eq!(colors.normal.magenta, AnsiColor::from_u32(0xBC3FBCFF));
    assert_eq!(colors.normal.cyan, AnsiColor::from_u32(0x11A8CDFF));
    assert_eq!(colors.normal.white, AnsiColor::from_u32(0xE5E5E5FF));
}

/// 验证 vscode_2026_dark 终端 bright 颜色正确。
#[test]
fn vscode_2026_dark_terminal_bright_colors() {
    let theme = vscode_2026_dark();
    let colors = theme.terminal_colors();

    assert_eq!(colors.bright.black, AnsiColor::from_u32(0x666666FF));
    assert_eq!(colors.bright.red, AnsiColor::from_u32(0xF14C4CFF));
    assert_eq!(colors.bright.green, AnsiColor::from_u32(0x23D18BFF));
    assert_eq!(colors.bright.yellow, AnsiColor::from_u32(0xF5F543FF));
    assert_eq!(colors.bright.blue, AnsiColor::from_u32(0x3B8EEAFF));
    assert_eq!(colors.bright.magenta, AnsiColor::from_u32(0xD670D6FF));
    assert_eq!(colors.bright.cyan, AnsiColor::from_u32(0x29B8DBFF));
    assert_eq!(colors.bright.white, AnsiColor::from_u32(0xE5E5E5FF));
}

/// 验证 vscode_2026_dark 包含 UiColors 覆盖且值正确。
#[test]
fn vscode_2026_dark_has_ui_colors_override() {
    let theme = vscode_2026_dark();

    let ui = theme.ui_colors().expect("应有 ui_colors 覆盖");

    // surface 层级
    assert_eq!(ui.surface_1, Some(ColorU { r: 0x20, g: 0x21, b: 0x22, a: 255 }));
    assert_eq!(ui.surface_2, Some(ColorU { r: 0x24, g: 0x25, b: 0x26, a: 255 }));
    assert_eq!(ui.surface_3, Some(ColorU { r: 0x2A, g: 0x2B, b: 0x2C, a: 255 }));

    // 边框
    assert_eq!(ui.border, Some(ColorU { r: 0x33, g: 0x35, b: 0x36, a: 255 }));
    assert_eq!(ui.focus_border, Some(ColorU { r: 0x39, g: 0x94, b: 0xBC, a: 0xB3 }));
    assert_eq!(ui.split_pane_border, Some(ColorU { r: 0x2A, g: 0x2B, b: 0x2C, a: 255 }));

    // 文字颜色
    assert_eq!(ui.main_text, Some(ColorU { r: 0xED, g: 0xED, b: 0xED, a: 255 }));
    assert_eq!(ui.sub_text, Some(ColorU { r: 0x8C, g: 0x8C, b: 0x8C, a: 255 }));
    assert_eq!(ui.hint_text, Some(ColorU { r: 0x55, g: 0x55, b: 0x55, a: 255 }));
    assert_eq!(ui.disabled_text, Some(ColorU { r: 0x55, g: 0x55, b: 0x55, a: 255 }));

    // 交互状态
    assert_eq!(ui.selection, Some(ColorU { r: 0x39, g: 0x94, b: 0xBC, a: 0x33 }));
    assert_eq!(ui.hover, Some(ColorU { r: 0xFF, g: 0xFF, b: 0xFF, a: 0x0D }));
    assert_eq!(ui.active, Some(ColorU { r: 0x39, g: 0x94, b: 0xBC, a: 255 }));

    // 功能色
    assert_eq!(ui.warning, Some(ColorU { r: 0xE5, g: 0xBA, b: 0x7D, a: 255 }));
    assert_eq!(ui.error, Some(ColorU { r: 0xF4, g: 0x87, b: 0x71, a: 255 }));
    assert_eq!(ui.success, Some(ColorU { r: 0x72, g: 0xC8, b: 0x92, a: 255 }));
    assert_eq!(ui.link, Some(ColorU { r: 0x48, g: 0xA0, b: 0xC7, a: 255 }));
}

/// 验证 UiColors 覆盖实际生效（surface_1 返回覆盖值而非派生值）。
#[test]
fn vscode_2026_dark_ui_colors_override_works() {
    let theme = vscode_2026_dark();

    // surface_1 应返回 UiColors 中定义的 #1E1F20 而非派生值
    let s1 = theme.surface_1().into_solid();
    assert_eq!(s1, ColorU { r: 0x20, g: 0x21, b: 0x22, a: 255 });

    // outline 应返回 UiColors 中定义的 border #333536
    let ol = theme.outline().into_solid();
    assert_eq!(ol, ColorU { r: 0x33, g: 0x35, b: 0x36, a: 255 });

    // text_selection_color 应返回 UiColors 中定义的 selection
    let sel = theme.text_selection_color().into_solid();
    assert_eq!(sel, ColorU { r: 0x39, g: 0x94, b: 0xBC, a: 0x33 });
}

/// 验证 ThemeKind::VsCode2026Dark 已注册到默认配置中。
#[test]
fn vscode_2026_dark_registered_in_default_config() {
    let config = WarpThemeConfig::default();
    let theme = config.theme_map.get(&ThemeKind::VsCode2026Dark);
    assert!(theme.is_some(), "VsCode2026Dark 应存在于默认主题配置中");
    assert_eq!(theme.unwrap().name(), Some("VS Code 2026 Dark".to_string()));
}

/// 验证 ThemeKind::VsCode2026Dark 的 Display 输出。
#[test]
fn vscode_2026_dark_display_name() {
    assert_eq!(format!("{}", ThemeKind::VsCode2026Dark), "VS Code 2026 Dark");
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn in_memory_theme_generation_test() {
    let mountains_bg_path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "assets",
        "async",
        "jpg",
        "mountains.jpg",
    ]
    .iter()
    .collect();

    let mut in_memory_theme = warpui::r#async::block_on(InMemoryThemeOptions::new(
        "mountains".to_string(),
        mountains_bg_path.clone(),
    ))
    .unwrap();

    let mountains_bg_path_string = mountains_bg_path.to_str().unwrap_or_default().to_owned();
    assert_eq!(
        in_memory_theme.theme(),
        WarpTheme::new(
            // the theme defaults to the 0th bg color
            ColorU::new(35, 31, 44, OPAQUE).into(),
            // this background color makes it a "dark" theme, so the foreground is white
            ColorU::white(),
            // the most distinct accent color is 3rd one
            ColorU::new(238, 203, 111, OPAQUE).into(),
            None,
            Some(Details::Darker),
            dark_mode_colors(),
            Some(Image {
                source: AssetSource::LocalFile {
                    path: mountains_bg_path_string.clone()
                },
                opacity: 30,
            }),
            Some("mountains".to_string()),
            None,
        )
    );

    in_memory_theme.chosen_bg_color_index = 2;

    assert_eq!(
        in_memory_theme.theme(),
        WarpTheme::new(
            // now the background is the 2nd one
            ColorU::new(229, 142, 113, OPAQUE).into(),
            // changing the background color made this a light theme
            ColorU::black(),
            // now the 4th color is the most distinct color
            ColorU::new(193, 217, 212, OPAQUE).into(),
            None,
            Some(Details::Lighter),
            light_mode_colors(),
            Some(Image {
                source: AssetSource::LocalFile {
                    path: mountains_bg_path_string
                },
                opacity: 30,
            }),
            Some("mountains".to_string()),
            None,
        )
    );
}
