use super::*;
use crate::ui::theme::mock_terminal_colors;
use warpui::color::ColorU;

fn mock_appearance() -> Appearance {
    use super::super::theme::Fill;
    use crate::ui::theme::Details;

    let theme = WarpTheme::new(
        Fill::Solid(ColorU::from_u32(0x000000ff)),
        ColorU::from_u32(0xffffffff),
        Fill::Solid(ColorU::new(18, 123, 156, 255)),
        None,
        Some(Details::Darker),
        mock_terminal_colors(),
        None,
        Some("Dark".to_string()),
        None,
    );
    Appearance::new(
        theme,
        FamilyId(0),
        13.0,
        Weight::Normal,
        FamilyId(1),
        1.4,
        FamilyId(0),
        None,
        FamilyId(0),
        DEFAULT_UI_FONT_SIZE,
        Default::default(),
    )
}

/// 作者: logic
/// UI 字体大小语义化方法测试
#[test]
fn test_semantic_font_sizes_at_default() {
    let appearance = mock_appearance();
    assert_eq!(appearance.ui_font_size(), 12.0);

    assert_eq!(appearance.ui_font_overline(), 10.0);
    assert_eq!(appearance.ui_font_footnote(), 11.0);
    assert_eq!(appearance.ui_font_body(), 12.0);
    assert_eq!(appearance.ui_font_body_large(), 13.0);
    assert_eq!(appearance.ui_font_subheading(), 14.0);
    assert_eq!(appearance.ui_font_heading_3(), 16.0);
    assert_eq!(appearance.ui_font_heading_2(), 18.0);
    assert_eq!(appearance.ui_font_heading_1(), 20.0);
    assert_eq!(appearance.ui_font_display(), 24.0);
    assert_eq!(appearance.ui_font_hero(), 36.0);
}

/// 作者: logic
/// 验证 header_font_size 和 overline_font_size 也随 ui_font_size 缩放
#[test]
fn test_header_and_overline_font_size_scaling() {
    let appearance = mock_appearance();
    assert_eq!(appearance.header_font_size(), 18.0);
    assert_eq!(appearance.overline_font_size(), 10.0);
}

/// 作者: logic
/// 设置 ui_font_size 为最小值后验证比例缩放
#[test]
fn test_semantic_font_sizes_at_minimum() {
    let mut appearance = mock_appearance();
    appearance.set_ui_font_size_test(8.0);

    assert_eq!(appearance.ui_font_size(), 8.0);
    assert_eq!(appearance.ui_font_overline(), 8.0 * 10.0 / 12.0);
    assert_eq!(appearance.ui_font_footnote(), 8.0 * 11.0 / 12.0);
    assert_eq!(appearance.ui_font_body(), 8.0);
    assert_eq!(appearance.ui_font_body_large(), 8.0 * 13.0 / 12.0);
    assert_eq!(appearance.ui_font_subheading(), 8.0 * 14.0 / 12.0);
    assert_eq!(appearance.ui_font_heading_3(), 8.0 * 16.0 / 12.0);
    assert_eq!(appearance.ui_font_heading_2(), 8.0 * 18.0 / 12.0);
    assert_eq!(appearance.ui_font_heading_1(), 8.0 * 20.0 / 12.0);
    assert_eq!(appearance.ui_font_display(), 8.0 * 24.0 / 12.0);
    assert_eq!(appearance.ui_font_hero(), 8.0 * 36.0 / 12.0);
    assert_eq!(appearance.header_font_size(), 8.0 * 18.0 / 12.0);
    assert_eq!(appearance.overline_font_size(), 8.0 * 10.0 / 12.0);
}

/// 作者: logic
/// 验证最大值 (20.0) 时的缩放
#[test]
fn test_semantic_font_sizes_at_maximum() {
    let mut appearance = mock_appearance();
    appearance.set_ui_font_size_test(20.0);

    assert_eq!(appearance.ui_font_size(), 20.0);
    assert_eq!(appearance.ui_font_overline(), 20.0 * 10.0 / 12.0);
    assert_eq!(appearance.ui_font_body(), 20.0);
    assert_eq!(appearance.ui_font_subheading(), 20.0 * 14.0 / 12.0);
    assert_eq!(appearance.ui_font_heading_3(), 20.0 * 16.0 / 12.0);
    assert_eq!(appearance.ui_font_display(), 20.0 * 24.0 / 12.0);
}

/// 作者: logic
/// 验证默认值常量
#[test]
fn test_default_constants() {
    assert_eq!(DEFAULT_UI_FONT_SIZE, 12.0);
}

/// 作者: logic
/// 验证 ui_font_body 等于 ui_font_size（1:1 比例）
#[test]
fn test_ui_font_body_equals_base() {
    let mut appearance = mock_appearance();
    for size in [8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0] {
        appearance.set_ui_font_size_test(size);
        assert_eq!(
            appearance.ui_font_body(),
            appearance.ui_font_size(),
            "ui_font_body should equal ui_font_size at base={}",
            size
        );
    }
}

/// 作者: logic
/// 验证各语义化方法保持严格的大小层级关系
#[test]
fn test_semantic_font_size_ordering() {
    let mut appearance = mock_appearance();
    for size in [8.0, 12.0, 20.0] {
        appearance.set_ui_font_size_test(size);

        let overline = appearance.ui_font_overline();
        let footnote = appearance.ui_font_footnote();
        let body = appearance.ui_font_body();
        let body_large = appearance.ui_font_body_large();
        let subheading = appearance.ui_font_subheading();
        let h3 = appearance.ui_font_heading_3();
        let h2 = appearance.ui_font_heading_2();
        let h1 = appearance.ui_font_heading_1();
        let display = appearance.ui_font_display();
        let hero = appearance.ui_font_hero();

        assert!(
            overline <= footnote,
            "overline <= footnote at base={}",
            size
        );
        assert!(footnote <= body, "footnote <= body at base={}", size);
        assert!(body <= body_large, "body <= body_large at base={}", size);
        assert!(
            body_large <= subheading,
            "body_large <= subheading at base={}",
            size
        );
        assert!(subheading <= h3, "subheading <= h3 at base={}", size);
        assert!(h3 <= h2, "h3 <= h2 at base={}", size);
        assert!(h2 <= h1, "h2 <= h1 at base={}", size);
        assert!(h1 <= display, "h1 <= display at base={}", size);
        assert!(display <= hero, "display <= hero at base={}", size);
    }
}

/// 作者: logic
/// 验证 dropdown 顶栏高度公式在默认字号 (12) 时为 30.0
#[test]
fn test_dropdown_top_bar_height_at_default() {
    let appearance = mock_appearance();
    assert_eq!(appearance.dropdown_top_bar_height(), 30.0);
}

/// 作者: logic
/// 验证 dropdown 顶栏高度随字号线性缩放且不低于 30.0
#[test]
fn test_dropdown_top_bar_height_scaling() {
    let mut appearance = mock_appearance();

    appearance.set_ui_font_size_test(8.0);
    assert_eq!(
        appearance.dropdown_top_bar_height(),
        30.0,
        "min size should clamp to 30.0"
    );

    appearance.set_ui_font_size_test(10.0);
    assert_eq!(
        appearance.dropdown_top_bar_height(),
        30.0,
        "size 10: 10*2.5=25, clamped to 30.0"
    );

    appearance.set_ui_font_size_test(12.0);
    assert_eq!(
        appearance.dropdown_top_bar_height(),
        30.0,
        "size 12: 12*2.5=30, exactly 30.0"
    );

    appearance.set_ui_font_size_test(16.0);
    assert_eq!(
        appearance.dropdown_top_bar_height(),
        40.0,
        "size 16: 16*2.5=40"
    );

    appearance.set_ui_font_size_test(20.0);
    assert_eq!(
        appearance.dropdown_top_bar_height(),
        50.0,
        "size 20: 20*2.5=50"
    );
}

/// 作者: logic
/// 验证 dropdown 顶栏高度永远不低于 30.0
#[test]
fn test_dropdown_top_bar_height_never_below_minimum() {
    let mut appearance = mock_appearance();
    for size in [8.0, 9.0, 10.0, 11.0, 12.0] {
        appearance.set_ui_font_size_test(size);
        assert!(
            appearance.dropdown_top_bar_height() >= 30.0,
            "height should be >= 30.0 at size={}",
            size
        );
    }
}

/// 作者: logic
/// 验证 dropdown 顶栏高度在边界字号 (8.0 和 20.0) 下的值
#[test]
fn test_dropdown_top_bar_height_at_boundaries() {
    let mut appearance = mock_appearance();

    appearance.set_ui_font_size_test(8.0);
    assert_eq!(appearance.dropdown_top_bar_height(), 30.0);

    appearance.set_ui_font_size_test(20.0);
    assert_eq!(appearance.dropdown_top_bar_height(), 50.0);
}

#[test]
fn test_terminal_fallback_font_family_can_be_updated() {
    let mut appearance = mock_appearance();

    assert_eq!(appearance.terminal_fallback_font_family(), None);

    appearance.set_terminal_fallback_font_family_test(Some(FamilyId(42)));
    assert_eq!(
        appearance.terminal_fallback_font_family(),
        Some(FamilyId(42))
    );

    appearance.set_terminal_fallback_font_family_test(None);
    assert_eq!(appearance.terminal_fallback_font_family(), None);
}

/// Per-window theme override resolution: `theme()`/`ui_builder()` return the
/// override only for the ambient render window, and the global value otherwise.
/// Also exercises the `theme_overrides.is_empty()` zero-cost fast path.
#[test]
fn test_per_window_theme_override_resolution() {
    use warpui::{current_render_window, set_current_render_window, WindowId};

    let mut appearance = mock_appearance();
    let window_a = WindowId::from_usize(1);
    let window_b = WindowId::from_usize(2);

    // Fast path: with no overrides, the ambient window is irrelevant and the
    // global theme/ui_builder is always returned.
    set_current_render_window(Some(window_a));
    assert!(std::ptr::eq(appearance.theme(), &appearance.theme));
    assert!(std::ptr::eq(
        appearance.ui_builder(),
        &appearance.ui_builder
    ));
    set_current_render_window(None);

    // Insert an override for window A directly (bypassing `ModelContext`, which
    // the unit-test harness doesn't provide). A distinct instance differs by
    // address from the global theme even though the values are equal.
    appearance
        .theme_overrides
        .insert(window_a, appearance.theme.clone());
    let override_ui_builder = appearance.ui_builder.clone();
    appearance
        .ui_builder_overrides
        .insert(window_a, override_ui_builder);

    // Ambient == A → the override is returned.
    set_current_render_window(Some(window_a));
    assert!(std::ptr::eq(
        appearance.theme(),
        appearance.theme_overrides.get(&window_a).unwrap()
    ));
    assert!(std::ptr::eq(
        appearance.ui_builder(),
        appearance.ui_builder_overrides.get(&window_a).unwrap()
    ));

    // Ambient == B (no override) → the global theme is returned.
    set_current_render_window(Some(window_b));
    assert!(std::ptr::eq(appearance.theme(), &appearance.theme));
    assert!(std::ptr::eq(
        appearance.ui_builder(),
        &appearance.ui_builder
    ));

    // Ambient == None (e.g. non-render reads) → the global theme is returned.
    set_current_render_window(None);
    assert_eq!(current_render_window(), None);
    assert!(std::ptr::eq(appearance.theme(), &appearance.theme));

    // Clearing the override returns to the fast path.
    appearance.theme_overrides.remove(&window_a);
    appearance.ui_builder_overrides.remove(&window_a);
    set_current_render_window(Some(window_a));
    assert!(std::ptr::eq(appearance.theme(), &appearance.theme));

    // Reset the thread-local ambient so it doesn't leak into other tests sharing
    // this test thread.
    set_current_render_window(None);
}
