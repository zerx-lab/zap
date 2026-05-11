//! `manager.rs` 的纯函数级单元测试。
//!
//! 这里只覆盖纯函数 helper —— 不触碰 `RemoteServerManager` 本体,
//! 因为后者依赖 `warpui::Entity` / `ModelContext`,要起一整套 App
//! 上下文,放到 integration testing 框架更合适。

use super::*;

// ---------------------------------------------------------------------------
// version_is_compatible
// ---------------------------------------------------------------------------

#[test]
fn version_compat_both_tagged_and_equal() {
    assert!(version_is_compatible(
        Some("v0.2026.05.10.stable"),
        "v0.2026.05.10.stable",
    ));
}

#[test]
fn version_compat_both_tagged_and_different() {
    assert!(!version_is_compatible(
        Some("v0.2026.05.10.stable"),
        "v0.2026.05.10.preview",
    ));
}

#[test]
fn version_compat_both_untagged() {
    // 客户端没有 GIT_RELEASE_TAG(cargo run),服务器也报空串
    // (`script/deploy_remote_server` dev 部署):视为兼容,保留
    // 本地开发循环不受影响。
    assert!(version_is_compatible(None, ""));
}

#[test]
fn version_compat_client_tagged_server_untagged() {
    // 客户端是 release,服务器是 dev 部署 → 视为不兼容,正常
    // 触发 reinstall 流程。
    assert!(!version_is_compatible(Some("v0.2026.05.10.stable"), ""));
}

#[test]
fn version_compat_client_untagged_server_tagged() {
    // **关键场景**:OpenWarp 客户端无 tag(cargo build),
    // 服务器是从官方 CDN 下来的 release(带 tag)。原 helper 判定
    // 不兼容,会触发 `remove_remote_server_binary` → 死循环。
    // 这个 test 仅记录 `version_is_compatible` 自身的行为不变,
    // 真正"跳过校验"由 [`should_enforce_remote_version_check`] 负责。
    assert!(!version_is_compatible(None, "v0.2026.05.10.stable"));
}

// ---------------------------------------------------------------------------
// should_enforce_remote_version_check
// ---------------------------------------------------------------------------

#[test]
fn enforce_version_check_skipped_on_oss() {
    // OpenWarp 临时复用官方 release 二进制时,客户端与服务端版本
    // 永远不一致,必须跳过严格校验。
    assert!(!should_enforce_remote_version_check(Channel::Oss));
}

#[test]
fn enforce_version_check_kept_on_official_channels() {
    // 官方 channel 上客户端和服务端要么都来自同一次 release CI,
    // 要么都来自 `script/deploy_remote_server` 的本地部署,严格
    // 校验仍然必要 —— 保留原有 stale binary 自愈路径。
    for channel in [
        Channel::Stable,
        Channel::Preview,
        Channel::Dev,
        Channel::Local,
        Channel::Integration,
    ] {
        assert!(
            should_enforce_remote_version_check(channel),
            "channel {channel:?} should still enforce version check"
        );
    }
}
