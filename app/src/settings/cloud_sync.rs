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

    #[test]
    fn test_sync_platform_display() {
        assert_eq!(format!("{}", SyncPlatformSetting::GitHub), "GitHub");
        assert_eq!(format!("{}", SyncPlatformSetting::Gitee), "Gitee");
    }

    #[test]
    fn test_sync_platform_equality() {
        assert_eq!(SyncPlatformSetting::GitHub, SyncPlatformSetting::GitHub);
        assert_ne!(SyncPlatformSetting::GitHub, SyncPlatformSetting::Gitee);
    }

    #[test]
    fn test_sync_platform_serialization() {
        let github = SyncPlatformSetting::GitHub;
        let json = serde_json::to_string(&github).unwrap();
        assert_eq!(json, r#""git_hub""#);

        let gitee = SyncPlatformSetting::Gitee;
        let json = serde_json::to_string(&gitee).unwrap();
        assert_eq!(json, r#""gitee""#);
    }

    #[test]
    fn test_sync_platform_deserialization() {
        let github: SyncPlatformSetting = serde_json::from_str(r#""git_hub""#).unwrap();
        assert_eq!(github, SyncPlatformSetting::GitHub);

        let gitee: SyncPlatformSetting = serde_json::from_str(r#""gitee""#).unwrap();
        assert_eq!(gitee, SyncPlatformSetting::Gitee);
    }

    #[test]
    fn test_sync_platform_deserialization_invalid() {
        let result: Result<SyncPlatformSetting, _> = serde_json::from_str(r#""invalid""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_sync_platform_roundtrip() {
        for platform in [SyncPlatformSetting::GitHub, SyncPlatformSetting::Gitee] {
            let json = serde_json::to_string(&platform).unwrap();
            let parsed: SyncPlatformSetting = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, platform);
        }
    }

    #[test]
    fn test_sync_platform_copy() {
        let a = SyncPlatformSetting::GitHub;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn test_sync_platform_clone() {
        let a = SyncPlatformSetting::GitHub;
        let b = a.clone();
        assert_eq!(a, b);
    }
}
