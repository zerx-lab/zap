pub mod fonts;
pub mod platform;
pub mod rendering;
pub mod windowing;

// Re-export everything from the core crate.
pub use warpui_core::*;

/// UI locale used to bias DirectWrite / CoreText / fontconfig glyph fallback for CJK Han characters.
/// Set by `app::i18n::init` / `set_locale` so that font fallback for Japanese UI prefers Japanese
/// glyphs (e.g. Yu Gothic / Meiryo) over Simplified Chinese (Microsoft YaHei) on Windows.
mod ui_locale {
    use std::sync::{Arc, Mutex, OnceLock, RwLock};

    static UI_LOCALE: OnceLock<RwLock<String>> = OnceLock::new();

    type LocaleListener = Arc<dyn Fn(&str) + Send + Sync>;
    static LISTENERS: OnceLock<Mutex<Vec<LocaleListener>>> = OnceLock::new();

    fn cell() -> &'static RwLock<String> {
        UI_LOCALE.get_or_init(|| RwLock::new("en-US".to_string()))
    }

    fn listeners() -> &'static Mutex<Vec<LocaleListener>> {
        LISTENERS.get_or_init(|| Mutex::new(Vec::new()))
    }

    pub fn set_ui_locale(locale: impl Into<String>) {
        let s = locale.into();
        if s.is_empty() {
            return;
        }
        {
            let mut guard = cell().write().unwrap();
            if *guard == s {
                return;
            }
            *guard = s.clone();
        }
        let snapshot: Vec<LocaleListener> = listeners().lock().unwrap().iter().cloned().collect();
        for cb in snapshot {
            cb(&s);
        }
    }

    pub fn current_ui_locale() -> String {
        cell().read().unwrap().clone()
    }

    /// Register a callback fired whenever `set_ui_locale` actually changes the stored value.
    /// Used by `TextLayoutSystem` to rebuild cosmic-text's `FontSystem` with the new locale
    /// (it has no public `set_locale`). Subscribers are kept alive by this registry; capture
    /// `Weak` references inside the closure if you want the underlying object to be droppable.
    pub fn on_ui_locale_changed(cb: LocaleListener) {
        listeners().lock().unwrap().push(cb);
    }
}

pub use ui_locale::{current_ui_locale, on_ui_locale_changed, set_ui_locale};

/// 共享 CJK Han 码点判定:这些字符在 ja / zh-Hans / zh-Hant / ko 之间字形不同,
/// 调用方据此把 DirectWrite / cosmic-text 的 Han 回退偏向当前 UI locale 的本地字体
/// (例如 ja-* → Yu Gothic UI、ko-* → Malgun Gothic、zh-Hant → Microsoft JhengHei UI)。
///
/// 覆盖 Unicode 15.0 之前已分配的所有 Unified Ideographs 区段(到 Extension G)。
/// 后续 Unicode 再扩展时,按需在此处统一追加 —— 这是仓库内 CJK Han 范围的**单一来源**,
/// 调用方(`crates/warpui/src/windowing/winit/fonts/windows.rs` 与
/// `app/src/font_fallback.rs`)不应自己 fork 范围。
pub fn is_shared_cjk_han(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF       // CJK Unified Ideographs Extension A
            | 0x4E00..=0x9FFF // CJK Unified Ideographs
            | 0xF900..=0xFAFF // CJK Compatibility Ideographs
            | 0x20000..=0x2A6DF // Extension B
            | 0x2A700..=0x2B73F // Extension C
            | 0x2B740..=0x2B81F // Extension D
            | 0x2B820..=0x2CEAF // Extension E
            | 0x2CEB0..=0x2EBEF // Extension F
            | 0x30000..=0x3134F // Extension G
    )
}
