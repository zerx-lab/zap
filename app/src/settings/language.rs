//! 用户界面语言设置(persisted via settings.toml,启动时应用到 i18n loader)。
//!
//! 当前支持英文、简体中文与日语。新增语言只需:
//!   1. `Language` 加 variant
//!   2. `app/i18n/<locale>/warp.ftl` 新建翻译文件
//!   3. `Display` + `to_locale_str` 加 case
//!
//! 切换在重启后完全生效(已渲染 UI 文本不会自动重排,需要 view 重建)。
//! 设置页 dropdown 应附"重启 Warp 后完全生效"提示。

use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};
use warp_core::settings::{macros::define_settings_group, SupportedPlatforms, SyncToCloud};

#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Sequence,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "The language used in Warp's user interface.",
    rename_all = "snake_case"
)]
pub enum Language {
    /// 跟随系统语言;若系统 locale 非已支持语言,fallback 到英文。
    #[default]
    #[schemars(description = "System default")]
    System,
    #[schemars(description = "English")]
    English,
    #[schemars(description = "Simplified Chinese")]
    SimplifiedChinese,
    #[schemars(description = "Japanese")]
    Japanese,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Language::System => "System default",
            Language::English => "English",
            Language::SimplifiedChinese => "简体中文",
            Language::Japanese => "日本語",
        };
        write!(f, "{value}")
    }
}

impl Language {
    /// 转 BCP-47 locale 字符串,`System` 返回 `None` 表示走系统检测。
    pub fn to_locale_str(self) -> Option<&'static str> {
        match self {
            Language::System => None,
            Language::English => Some("en"),
            Language::SimplifiedChinese => Some("zh-CN"),
            Language::Japanese => Some("ja"),
        }
    }
}

define_settings_group!(LanguageSettings, settings: [
    language: LanguageState {
        type: Language,
        default: Language::System,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Never,
        private: false,
        storage_key: "Language",
        toml_path: "appearance.language",
        description: "The language used in Warp's user interface. Falls back to English when the chosen language is not fully translated.",
    },
]);
