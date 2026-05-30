use super::*;

#[test]
fn serialize_test() {
    let theme = WarpTheme::new(
        Fill::Solid(ColorU::from_u32(0x20A5BAFF)),
        ColorU::from_u32(0x20A5BAFF),
        Fill::Solid(ColorU::from_u32(0x20A5BAFF)),
        None,
        Some(Details::Darker),
        mock_terminal_colors(),
        None,
        Some("test_theme".to_string()),
        None,
    );
    assert_eq!(
        r##"---
background: "#20a5ba"
accent: "#20a5ba"
foreground: "#20a5ba"
details: darker
terminal_colors:
  normal:
    black: "#616161"
    red: "#ff8272"
    green: "#b4fa72"
    yellow: "#fefdc2"
    blue: "#a5d5fe"
    magenta: "#ff8ffd"
    cyan: "#d0d1fe"
    white: "#f1f1f1"
  bright:
    black: "#8e8e8e"
    red: "#ffc4bd"
    green: "#d6fcb9"
    yellow: "#fefdd5"
    blue: "#c1e3fe"
    magenta: "#ffb1fe"
    cyan: "#e5e6fe"
    white: "#feffff"
name: test_theme
"##,
        serde_yaml::to_string(&theme).expect("Couldn't serialize")
    );
}

#[test]
fn deserialize_with_name_test() {
    let theme = serde_yaml::from_str::<WarpTheme>(
        r##"---
background: "#20a5ba"
accent: "#20a5ba"
foreground: "#20a5ba"
details: darker
terminal_colors:
  normal:
    black: "#616161"
    red: "#ff8272"
    green: "#b4fa72"
    yellow: "#fefdc2"
    blue: "#a5d5fe"
    magenta: "#ff8ffd"
    cyan: "#d0d1fe"
    white: "#f1f1f1"
  bright:
    black: "#8e8e8e"
    red: "#ffc4bd"
    green: "#d6fcb9"
    yellow: "#fefdd5"
    blue: "#c1e3fe"
    magenta: "#ffb1fe"
    cyan: "#e5e6fe"
    white: "#feffff"
name: test_theme
"##,
    )
    .expect("Couldn't deserialize");

    let expected_theme = WarpTheme::new(
        Fill::Solid(ColorU::from_u32(0x20A5BAFF)),
        ColorU::from_u32(0x20A5BAFF),
        Fill::Solid(ColorU::from_u32(0x20A5BAFF)),
        None,
        Some(Details::Darker),
        mock_terminal_colors(),
        None,
        Some("test_theme".to_string()),
        None,
    );

    assert_eq!(expected_theme, theme);
}

#[test]
fn deserialize_without_name_test() {
    let theme = serde_yaml::from_str::<WarpTheme>(
        r##"---
background: "#20a5ba"
accent: "#20a5ba"
foreground: "#20a5ba"
details: darker
terminal_colors:
  normal:
    black: "#616161"
    red: "#ff8272"
    green: "#b4fa72"
    yellow: "#fefdc2"
    blue: "#a5d5fe"
    magenta: "#ff8ffd"
    cyan: "#d0d1fe"
    white: "#f1f1f1"
  bright:
    black: "#8e8e8e"
    red: "#ffc4bd"
    green: "#d6fcb9"
    yellow: "#fefdd5"
    blue: "#c1e3fe"
    magenta: "#ffb1fe"
    cyan: "#e5e6fe"
    white: "#feffff"
"##,
    )
    .expect("Couldn't deserialize");

    let expected_theme = WarpTheme::new(
        Fill::Solid(ColorU::from_u32(0x20A5BAFF)),
        ColorU::from_u32(0x20A5BAFF),
        Fill::Solid(ColorU::from_u32(0x20A5BAFF)),
        None,
        Some(Details::Darker),
        mock_terminal_colors(),
        None,
        None,
        None,
    );

    assert_eq!(expected_theme, theme);
}

#[test]
fn blend_gradient_test() {
    let (c1, c2, c3, c4) = (
        ColorU::from_u32(0x002b36ff),
        ColorU::from_u32(0xcb4b16ff),
        ColorU::from_u32(0xffffff19),
        ColorU::from_u32(0xffffff19),
    );
    let g1 = VerticalGradient::new(c1, c2);
    let g2 = VerticalGradient::new(c3, c4);

    assert_eq!(
        g1.blend(&g2),
        VerticalGradient::new(c1.blend(&c3), c2.blend(&c4))
    );
}

#[test]
fn blend_coloru_test() {
    let c1 = ColorU::from_u32(0x002b36ff);
    let c2 = ColorU::from_u32(0xF8F8F2FF);
    assert_eq!(
        c1.blend(&coloru_with_opacity(c2, 10)),
        ColorU::from_u32(0x183f48ff)
    );
    assert_eq!(
        ColorU::from_u32(0x000000ff).blend(&coloru_with_opacity(c2, 10)),
        ColorU::from_u32(0x181818ff)
    );
}

/// TODO(CORE-3626): write an equivalent test with Windows paths.
#[cfg(not(windows))]
#[test]
fn test_deserialize_image() {
    // Paths that start with `~` should expand to include the home dir.
    let a = "
    path: ~/warp.jpg
    opacity: 60
    ";
    let image: Image = serde_yaml::from_str(a).unwrap();
    assert_eq!(image.opacity, 60);
    assert_eq!(
        image.source,
        AssetSource::LocalFile {
            path: home_dir()
                .unwrap()
                .join("warp.jpg")
                .to_str()
                .unwrap_or_default()
                .to_owned()
        }
    );

    // Absolute paths should be unchanged.
    let b = "
    path: /warp.jpg
    opacity: 60
    ";
    let image: Image = serde_yaml::from_str(b).unwrap();
    assert_eq!(image.opacity, 60);
    assert_eq!(
        image.source,
        AssetSource::LocalFile {
            path: "/warp.jpg".to_owned()
        }
    );

    // Relative paths should expand to include the theme dir.
    let c = "
    path: warp.jpg
    opacity: 60
    ";
    let image: Image = serde_yaml::from_str(c).unwrap();
    assert_eq!(image.opacity, 60);
    assert_eq!(
        image.source,
        AssetSource::LocalFile {
            path: themes_dir()
                .join("warp.jpg")
                .to_str()
                .unwrap_or_default()
                .to_owned()
        }
    );

    // No opacity should become the default
    let d = "
    path: warp.jpg
    ";
    let image: Image = serde_yaml::from_str(d).unwrap();
    assert_eq!(image.opacity, default_image_opacity());
}

#[test]
fn ansi_color_deserializing_test() {
    let raw = r##"
        black: "#000000"
        red: "#ff0000"
        green: "#00ff00"
        yellow: "#00ffff"
        blue: "#0000ff"
        magenta: "#ff0000"
        cyan: "#0000ff"
        white: "#ffffff"
        "##;
    let ansi_colors: AnsiColors = serde_yaml::from_str(raw).expect("Couldn't deserialize");
    assert_eq!(ansi_colors.black, AnsiColor::from_u32(0x000000ff));
    assert_eq!(ansi_colors.red, AnsiColor::from_u32(0xff0000ff));
    assert_eq!(ansi_colors.green, AnsiColor::from_u32(0x00ff00ff));
    assert_eq!(ansi_colors.yellow, AnsiColor::from_u32(0x00ffffff));
    assert_eq!(ansi_colors.blue, AnsiColor::from_u32(0x0000ffff));
    assert_eq!(ansi_colors.magenta, AnsiColor::from_u32(0xff0000ff));
    assert_eq!(ansi_colors.cyan, AnsiColor::from_u32(0x0000ffff));
    assert_eq!(ansi_colors.white, AnsiColor::from_u32(0xffffffff));
}

#[test]
fn ansi_color_serializing_test() {
    let ansi_colors = AnsiColors::new(
        AnsiColor::from_u32(0x000000ff),
        AnsiColor::from_u32(0xff0000ff),
        AnsiColor::from_u32(0x00ff00ff),
        AnsiColor::from_u32(0x00ffffff),
        AnsiColor::from_u32(0x0000ffff),
        AnsiColor::from_u32(0xff0000ff),
        AnsiColor::from_u32(0x0000ffff),
        AnsiColor::from_u32(0xffffffff),
    );
    let serialized = serde_yaml::to_string(&ansi_colors).expect("Couldn't serialize");
    let raw = r##"---
black: "#000000"
red: "#ff0000"
green: "#00ff00"
yellow: "#00ffff"
blue: "#0000ff"
magenta: "#ff0000"
cyan: "#0000ff"
white: "#ffffff"
"##;
    assert_eq!(serialized, raw);

    let ansi_colors2: AnsiColors = serde_yaml::from_str(&serialized).expect("Couldn't deserialize");
    assert_eq!(ansi_colors2, ansi_colors);
}

#[test]
fn from_hex_negative_test() {
    assert_eq!(
        hex_color::coloru_from_hex_string("#0").unwrap_err(),
        hex_color::HexColorError::InvalidLength
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("#00").unwrap_err(),
        hex_color::HexColorError::InvalidLength
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("#00000").unwrap_err(),
        hex_color::HexColorError::InvalidLength
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("#0000000").unwrap_err(),
        hex_color::HexColorError::InvalidLength
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("0000").unwrap_err(),
        hex_color::HexColorError::HashPrefix
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("#ZXD").unwrap_err(),
        hex_color::HexColorError::InvalidValue
    );
}

#[test]
fn from_hex_positive_test() {
    assert_eq!(
        hex_color::coloru_from_hex_string("#000").unwrap(),
        ColorU::from_u32(0x000000ff)
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("#000000").unwrap(),
        ColorU::from_u32(0x000000ff)
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("#123").unwrap(),
        ColorU::from_u32(0x112233ff)
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("#112233").unwrap(),
        ColorU::from_u32(0x112233ff)
    );
}

#[test]
fn infer_from_foreground_color_test() {
    assert_eq!(
        ColorScheme::infer_from_foreground_color(ColorU::white()),
        ColorScheme::LightOnDark
    );
    assert_eq!(
        ColorScheme::infer_from_foreground_color(ColorU::black()),
        ColorScheme::DarkOnLight
    );
}

#[test]
fn deserialize_with_ui_colors_test() {
    let theme = serde_yaml::from_str::<WarpTheme>(
        r##"---
background: "#191A1B"
accent: "#3994BC"
foreground: "#bfbfbf"
details: darker
terminal_colors:
  normal:
    black: "#191A1B"
    red: "#F48771"
    green: "#2EA043"
    yellow: "#E5BA7D"
    blue: "#3994BC"
    magenta: "#B180D7"
    cyan: "#48A0C7"
    white: "#EDEDED"
  bright:
    black: "#555555"
    red: "#F48771"
    green: "#369432"
    yellow: "#B89500"
    blue: "#53A5CA"
    magenta: "#FF8FFD"
    cyan: "#56D4DD"
    white: "#FFFFFF"
name: VS Code 2026 Dark
ui_colors:
  surface_1: "#1E1F20"
  border: "#333536"
  main_text: "#EDEDED"
"##,
    )
    .expect("Couldn't deserialize");

    assert_eq!(theme.name(), Some("VS Code 2026 Dark".to_string()));
    assert!(theme.ui_colors().is_some());
    let colors = theme.ui_colors().unwrap();
    assert_eq!(
        colors.surface_1.unwrap(),
        ColorU { r: 0x1E, g: 0x1F, b: 0x20, a: 255 }
    );
    assert_eq!(
        colors.border.unwrap(),
        ColorU { r: 0x33, g: 0x35, b: 0x36, a: 255 }
    );
    assert_eq!(
        colors.main_text.unwrap(),
        ColorU { r: 0xED, g: 0xED, b: 0xED, a: 255 }
    );
    assert!(colors.surface_2.is_none());
}

#[test]
fn deserialize_without_ui_colors_test() {
    let theme = serde_yaml::from_str::<WarpTheme>(
        r##"---
background: "#191A1B"
accent: "#3994BC"
foreground: "#bfbfbf"
details: darker
terminal_colors:
  normal:
    black: "#191A1B"
    red: "#F48771"
    green: "#2EA043"
    yellow: "#E5BA7D"
    blue: "#3994BC"
    magenta: "#B180D7"
    cyan: "#48A0C7"
    white: "#EDEDED"
  bright:
    black: "#555555"
    red: "#F48771"
    green: "#369432"
    yellow: "#B89500"
    blue: "#53A5CA"
    magenta: "#FF8FFD"
    cyan: "#56D4DD"
    white: "#FFFFFF"
name: VS Code 2026 Dark
"##,
    )
    .expect("Couldn't deserialize");

    assert!(theme.ui_colors().is_none());
}

/// 构建 UiColors 测试实例的工具函数。
fn test_ui_colors() -> super::ui_colors::UiColors {
    use super::ui_colors::UiColors;
    UiColors {
        surface_1: Some(ColorU { r: 0x11, g: 0x11, b: 0x11, a: 255 }),
        surface_2: Some(ColorU { r: 0x22, g: 0x22, b: 0x22, a: 255 }),
        surface_3: Some(ColorU { r: 0x33, g: 0x33, b: 0x33, a: 255 }),
        border: Some(ColorU { r: 0x44, g: 0x44, b: 0x44, a: 255 }),
        focus_border: Some(ColorU { r: 0x55, g: 0x55, b: 0x55, a: 128 }),
        split_pane_border: Some(ColorU { r: 0x66, g: 0x66, b: 0x66, a: 255 }),
        main_text: Some(ColorU { r: 0x77, g: 0x77, b: 0x77, a: 255 }),
        sub_text: Some(ColorU { r: 0x88, g: 0x88, b: 0x88, a: 255 }),
        hint_text: Some(ColorU { r: 0x99, g: 0x99, b: 0x99, a: 255 }),
        disabled_text: Some(ColorU { r: 0xAA, g: 0xAA, b: 0xAA, a: 255 }),
        selection: Some(ColorU { r: 0xBB, g: 0xBB, b: 0xBB, a: 128 }),
        hover: Some(ColorU { r: 0xCC, g: 0xCC, b: 0xCC, a: 128 }),
        active: Some(ColorU { r: 0xDD, g: 0xDD, b: 0xDD, a: 255 }),
        warning: Some(ColorU { r: 0xEE, g: 0x00, b: 0x00, a: 255 }),
        error: Some(ColorU { r: 0x00, g: 0xEE, b: 0x00, a: 255 }),
        success: Some(ColorU { r: 0x00, g: 0x00, b: 0xEE, a: 255 }),
        link: Some(ColorU { r: 0xFF, g: 0xFF, b: 0x00, a: 255 }),
    }
}

/// 构建 WarpTheme 的工具函数，可选注入 UiColors。
fn build_theme(ui_colors: Option<super::ui_colors::UiColors>) -> WarpTheme {
    WarpTheme::new(
        Fill::Solid(ColorU::from_u32(0x1E1E1EFF)),
        ColorU::from_u32(0xD4D4D4FF),
        Fill::Solid(ColorU::from_u32(0x007ACCFF)),
        None,
        Some(Details::Darker),
        mock_terminal_colors(),
        None,
        Some("test".to_string()),
        ui_colors,
    )
}

// --- surface_1 回退逻辑测试 ---

#[test]
fn surface_1_without_ui_colors_returns_derived() {
    let theme = build_theme(None);
    let derived = Fill::Solid(super::color::internal_colors::neutral_1(&theme));
    assert_eq!(theme.surface_1(), derived);
}

#[test]
fn surface_1_with_ui_colors_returns_override() {
    let ui = test_ui_colors();
    let expected = Fill::Solid(ui.surface_1.unwrap());
    let theme = build_theme(Some(ui));
    assert_eq!(theme.surface_1(), expected);
}

#[test]
fn surface_1_with_ui_colors_but_none_field_returns_derived() {
    let mut ui = test_ui_colors();
    ui.surface_1 = None;
    let theme = build_theme(Some(ui));
    let derived = Fill::Solid(super::color::internal_colors::neutral_1(&theme));
    assert_eq!(theme.surface_1(), derived);
}

// --- surface_2 回退逻辑测试 ---

#[test]
fn surface_2_with_ui_colors_returns_override() {
    let ui = test_ui_colors();
    let expected = Fill::Solid(ui.surface_2.unwrap());
    let theme = build_theme(Some(ui));
    assert_eq!(theme.surface_2(), expected);
}

#[test]
fn surface_2_without_ui_colors_returns_derived() {
    let theme = build_theme(None);
    let derived = Fill::Solid(super::color::internal_colors::neutral_2(&theme));
    assert_eq!(theme.surface_2(), derived);
}

// --- surface_3 回退逻辑测试 ---

#[test]
fn surface_3_with_ui_colors_returns_override() {
    let ui = test_ui_colors();
    let expected = Fill::Solid(ui.surface_3.unwrap());
    let theme = build_theme(Some(ui));
    assert_eq!(theme.surface_3(), expected);
}

#[test]
fn surface_3_without_ui_colors_returns_derived() {
    let theme = build_theme(None);
    let derived = Fill::Solid(super::color::internal_colors::neutral_3(&theme));
    assert_eq!(theme.surface_3(), derived);
}

// --- outline (border) 回退逻辑测试 ---

#[test]
fn outline_with_ui_colors_returns_border_override() {
    let ui = test_ui_colors();
    let expected = Fill::Solid(ui.border.unwrap());
    let theme = build_theme(Some(ui));
    assert_eq!(theme.outline(), expected);
}

#[test]
fn outline_without_ui_colors_returns_derived() {
    let theme = build_theme(None);
    let derived = super::color::internal_colors::fg_overlay_2(&theme);
    assert_eq!(theme.outline(), derived);
}

// --- split_pane_border_color 回退逻辑测试 ---

#[test]
fn split_pane_border_with_ui_colors_returns_override() {
    let ui = test_ui_colors();
    let expected = Fill::Solid(ui.split_pane_border.unwrap());
    let theme = build_theme(Some(ui));
    assert_eq!(theme.split_pane_border_color(), expected);
}

#[test]
fn split_pane_border_without_ui_colors_returns_derived() {
    let theme = build_theme(None);
    let derived = super::color::internal_colors::fg_overlay_3(&theme);
    assert_eq!(theme.split_pane_border_color(), derived);
}

// --- text_selection_color 回退逻辑测试 ---

#[test]
fn text_selection_color_with_ui_colors_returns_override() {
    let ui = test_ui_colors();
    let expected = Fill::Solid(ui.selection.unwrap());
    let theme = build_theme(Some(ui));
    assert_eq!(theme.text_selection_color(), expected);
}

#[test]
fn text_selection_color_without_ui_colors_returns_default() {
    let theme = build_theme(None);
    let expected = Fill::Solid(ColorU::new(118, 167, 250, (0.4 * 255.) as u8));
    assert_eq!(theme.text_selection_color(), expected);
}

// --- block_selection_color 回退逻辑测试 ---

#[test]
fn block_selection_color_with_ui_colors_returns_override() {
    let ui = test_ui_colors();
    let expected = Fill::Solid(ui.selection.unwrap());
    let theme = build_theme(Some(ui));
    assert_eq!(theme.block_selection_color(), expected);
}

#[test]
fn block_selection_color_without_ui_colors_returns_derived() {
    let theme = build_theme(None);
    let derived = super::color::internal_colors::accent_overlay_2(&theme);
    assert_eq!(theme.block_selection_color(), derived);
}

// --- UiColors 往返序列化测试 ---

#[test]
fn ui_colors_roundtrip_serialization() {
    use super::ui_colors::UiColors;
    let original = test_ui_colors();
    let yaml = serde_yaml::to_string(&original).expect("序列化失败");
    let restored: UiColors = serde_yaml::from_str(&yaml).expect("反序列化失败");
    assert_eq!(original, restored);
}

// --- hex_color_alpha 边界条件测试（通过 UiColors 间接测试） ---

#[test]
fn hex_alpha_rejects_no_hash_prefix_via_ui_colors() {
    let yaml = r##"---
surface_1: "3994BC"
"##;
    let result = serde_yaml::from_str::<super::ui_colors::UiColors>(yaml);
    assert!(result.is_err());
}

#[test]
fn hex_alpha_rejects_invalid_length_via_ui_colors() {
    let yaml = r##"---
surface_1: "#1234"
"##;
    let result = serde_yaml::from_str::<super::ui_colors::UiColors>(yaml);
    assert!(result.is_err());
}

#[test]
fn hex_alpha_rejects_invalid_chars_via_ui_colors() {
    let yaml = r##"---
surface_1: "#GHIJKL"
"##;
    let result = serde_yaml::from_str::<super::ui_colors::UiColors>(yaml);
    assert!(result.is_err());
}

#[test]
fn hex_alpha_roundtrip_with_alpha_via_ui_colors() {
    let mut colors = test_ui_colors();
    // 确保 surface_1 有特定 alpha 值
    colors.surface_1 = Some(ColorU { r: 0x39, g: 0x94, b: 0xBC, a: 0x26 });
    colors.surface_2 = None;
    colors.surface_3 = None;
    let yaml = serde_yaml::to_string(&colors).expect("序列化失败");
    let restored: super::ui_colors::UiColors =
        serde_yaml::from_str(&yaml).expect("反序列化失败");
    assert_eq!(restored.surface_1, colors.surface_1);
}

#[test]
fn hex_alpha_roundtrip_opaque_via_ui_colors() {
    let mut colors = test_ui_colors();
    colors.surface_1 = Some(ColorU { r: 0xFF, g: 0x00, b: 0x80, a: 255 });
    colors.surface_2 = None;
    colors.surface_3 = None;
    let yaml = serde_yaml::to_string(&colors).expect("序列化失败");
    let restored: super::ui_colors::UiColors =
        serde_yaml::from_str(&yaml).expect("反序列化失败");
    assert_eq!(restored.surface_1, colors.surface_1);
}
