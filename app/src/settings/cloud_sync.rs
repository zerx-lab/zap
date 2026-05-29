use serde::{Deserialize, Serialize};
use settings::{macros::define_settings_group, SupportedPlatforms, SyncToCloud};

/// 云同步平台选择
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[serde(rename_all = "snake_case")]
pub enum SyncPlatformSetting {
    #[default]
    GitHub,
    Gitee,
}

impl SyncPlatformSetting {
    /// 转换为 zap_sync::SyncPlatform
    pub fn to_sync_platform(self) -> zap_sync::SyncPlatform {
        match self {
            Self::GitHub => zap_sync::SyncPlatform::GitHub,
            Self::Gitee => zap_sync::SyncPlatform::Gitee,
        }
    }

    /// 获取显示名称
    pub fn label(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::Gitee => "Gitee",
        }
    }
}

impl std::fmt::Display for SyncPlatformSetting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

define_settings_group!(CloudSyncSettings,
    settings: [
        sync_platform: SyncPlatform {
            type: SyncPlatformSetting,
            default: SyncPlatformSetting::GitHub,
            supported_platforms: SupportedPlatforms::ALL,
            sync_to_cloud: SyncToCloud::Never,
            private: false,
            storage_key: "CloudSyncPlatform",
            toml_path: "cloud_sync.sync_platform",
            description: "Cloud sync platform",
        },
        auto_sync: AutoSync {
            type: bool,
            default: false,
            supported_platforms: SupportedPlatforms::ALL,
            sync_to_cloud: SyncToCloud::Never,
            private: false,
            storage_key: "CloudSyncAutoSync",
            toml_path: "cloud_sync.auto_sync",
            description: "Auto sync on config change",
        },
    ]
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_platform_default() {
        assert_eq!(SyncPlatformSetting::default(), SyncPlatformSetting::GitHub);
    }

    #[test]
    fn test_sync_platform_to_sync() {
        assert_eq!(
            SyncPlatformSetting::GitHub.to_sync_platform(),
            zap_sync::SyncPlatform::GitHub
        );
        assert_eq!(
            SyncPlatformSetting::Gitee.to_sync_platform(),
            zap_sync::SyncPlatform::Gitee
        );
    }

    #[test]
    fn test_sync_platform_label() {
        assert_eq!(SyncPlatformSetting::GitHub.label(), "GitHub");
        assert_eq!(SyncPlatformSetting::Gitee.label(), "Gitee");
    }
}
