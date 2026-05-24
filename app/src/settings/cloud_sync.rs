use settings::{
    macros::define_settings_group, SupportedPlatforms, SyncToCloud,
};

define_settings_group!(CloudSyncSettings,
    settings: [
        github_token: GithubToken {
            type: String,
            default: "".to_string(),
            supported_platforms: SupportedPlatforms::ALL,
            sync_to_cloud: SyncToCloud::Never,
            private: true,
            storage_key: "CloudSyncGithubToken",
            toml_path: "cloud_sync.github_token",
            description: "GitHub Gist API Token",
        },
        gitee_token: GiteeToken {
            type: String,
            default: "".to_string(),
            supported_platforms: SupportedPlatforms::ALL,
            sync_to_cloud: SyncToCloud::Never,
            private: true,
            storage_key: "CloudSyncGiteeToken",
            toml_path: "cloud_sync.gitee_token",
            description: "Gitee Gist API Token",
        },
    ]
);
