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
