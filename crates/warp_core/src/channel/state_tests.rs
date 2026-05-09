use super::{derive_http_origin_from_ws_url, ChannelState};

#[test]
fn wss_becomes_https_and_strips_path() {
    let got = derive_http_origin_from_ws_url("wss://rtc.app.warp.dev/graphql/v2");
    assert_eq!(got.as_deref(), Some("https://rtc.app.warp.dev"));
}

#[test]
fn ws_becomes_http_and_preserves_port() {
    let got = derive_http_origin_from_ws_url("ws://localhost:8080/graphql/v2");
    assert_eq!(got.as_deref(), Some("http://localhost:8080"));
}

#[test]
fn unparseable_input_returns_none() {
    assert!(derive_http_origin_from_ws_url("not a url").is_none());
    assert!(derive_http_origin_from_ws_url("https://app.warp.dev").is_none());
}

/// `ChannelState::init()` (the static default for OSS builds) must satisfy
/// the cloud-disabled predicate; the cloud-removal plan's Phase 5 short-circuit
/// depends on this invariant.
#[test]
fn default_oss_state_is_cloud_disabled() {
    assert!(ChannelState::is_cloud_disabled());
}
