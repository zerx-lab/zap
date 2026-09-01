// On Windows, we don't want to display a console window when the application is running in release
// builds. See https://doc.rust-lang.org/reference/runtime.html#the-windows_subsystem-attribute.
#![cfg_attr(feature = "release_bundle", windows_subsystem = "windows")]

use anyhow::Result;
use warp_core::{
    channel::{Channel, ChannelConfig, ChannelState},
    features::{FeatureFlag, DEBUG_FLAGS},
    AppId,
};

#[cfg(all(
    target_os = "windows",
    feature = "windows_high_performance_gpu_default"
))]
#[allow(non_upper_case_globals)]
#[no_mangle]
#[used]
pub static NvOptimusEnablement: u32 = 1;

#[cfg(all(
    target_os = "windows",
    feature = "windows_high_performance_gpu_default"
))]
#[allow(non_upper_case_globals)]
#[no_mangle]
#[used]
pub static AmdPowerXpressRequestHighPerformance: u32 = 1;

// Zap OSS 构建的入口,简单包一层 warp::run()。
fn main() -> Result<()> {
    let mut state = ChannelState::new(
        Channel::Oss,
        ChannelConfig {
            app_id: AppId::new("dev", "zap", "Zap"),
            logfile_name: "zap.log".into(),
            autoupdate_config: None,
            mcp_static_config: None,
        },
    );
    if cfg!(debug_assertions) {
        state = state.with_additional_features(DEBUG_FLAGS);
    }
    // zap-oss 是本地人工测试入口; 显式启用仍处于 Dogfood 阶段的项目组织功能。
    // 其他发布通道继续只由其各自的 feature flag 列表决定是否开放。
    state = state.with_additional_features(&[
        FeatureFlag::RepositoryWorkspaces,
        FeatureFlag::CliAgentSessionResume,
    ]);
    // 始终启用 IME marked-text 渲染:winit 的 IME 路径在 macOS / Windows 都支持,
    // 但若不在此处显式开启,Zap 会把 preedit / 输入合成更新整体丢弃,只剩 OS 的候选窗
    // 可见 —— 在 Windows 上对日文 / 中文 / 韩文输入都属于实质性损坏。
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        state = state.with_additional_features(&[FeatureFlag::ImeMarkedText]);
    }
    ChannelState::set(state);

    warp::run()
}

// If we're not using an external plist, embed the following as the Info.plist.
#[cfg(all(not(feature = "extern_plist"), target_os = "macos"))]
embed_plist::embed_info_plist_bytes!(r#"
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleDisplayName</key>
    <string>Zap</string>
    <key>CFBundleExecutable</key>
    <string>zap-oss</string>
    <key>CFBundleIdentifier</key>
    <string>dev.zap.Zap</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleLocalizations</key>
    <array>
    <string>en</string>
    <string>ja</string>
    <string>zh-CN</string>
    </array>
    <key>CFBundleName</key>
    <string>Zap</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>UIDesignRequiresCompatibility</key>
    <true/>
    <key>CFBundleURLTypes</key>
    <array><dict><key>CFBundleURLName</key><string>Custom App</string><key>CFBundleURLSchemes</key><array><string>zap</string></array></dict></array>
    <key>NSHumanReadableCopyright</key>
    <string>© 2026, Zap</string>
    </dict>
    </plist>
"#.as_bytes());
