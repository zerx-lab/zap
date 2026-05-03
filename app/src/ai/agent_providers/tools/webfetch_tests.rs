//! `web_runtime::run_webfetch` 单测(mockito,无外网)。

use super::*;
use mockito::{Matcher, Server};

fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .build()
        .expect("reqwest client build")
}

fn args(url: &str) -> FetchArgs {
    FetchArgs {
        url: url.to_owned(),
        format: None,
        timeout: None,
    }
}

// ---------------------------------------------------------------------------
// URL 验证(纯逻辑,无 HTTP)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rejects_non_http_scheme() {
    let client = build_client();
    for bad in [
        "ftp://example.com",
        "file:///etc/passwd",
        "javascript:alert(1)",
        "",
    ] {
        let err = run_webfetch(&client, args(bad)).await.unwrap_err();
        assert!(
            err.to_string().contains("http://") || err.to_string().contains("https://"),
            "bad={bad} err={err}"
        );
    }
}

#[tokio::test]
async fn accepts_http_and_https() {
    // 不真实发送(URL 指向不可达 host),只校验 URL 验证不挡;实际会因 connect 失败报错
    let client = build_client();
    for ok in ["http://127.0.0.1:1/x", "https://127.0.0.1:1/x"] {
        let err = run_webfetch(&client, args(ok)).await.unwrap_err();
        assert!(
            !err.to_string().contains("must start with"),
            "URL 校验不应阻止 {ok}: {err}"
        );
    }
}

// ---------------------------------------------------------------------------
// 内容类型分支
// ---------------------------------------------------------------------------

#[tokio::test]
async fn html_to_markdown() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/page")
        .with_status(200)
        .with_header("content-type", "text/html; charset=utf-8")
        .with_body("<html><body><h1>Hello</h1><p>World</p></body></html>")
        .create_async()
        .await;

    let client = build_client();
    let out = run_webfetch(&client, args(&format!("{}/page", server.url())))
        .await
        .expect("ok");
    assert!(
        out.output.contains("Hello"),
        "missing Hello: {}",
        out.output
    );
    assert!(
        out.output.contains("World"),
        "missing World: {}",
        out.output
    );
    assert!(
        out.output.contains('#') || !out.output.contains("<h1>"),
        "should be markdown not HTML: {}",
        out.output
    );
    assert_eq!(out.format, "markdown");
    assert!(out.attachments.is_empty());
}

#[tokio::test]
async fn text_plain_passthrough() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/text")
        .with_status(200)
        .with_header("content-type", "text/plain")
        .with_body("just some text")
        .create_async()
        .await;

    let client = build_client();
    let out = run_webfetch(&client, args(&format!("{}/text", server.url())))
        .await
        .expect("ok");
    assert_eq!(out.output, "just some text");
}

#[tokio::test]
async fn json_pretty_print() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/api")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"a":1,"b":[2,3]}"#)
        .create_async()
        .await;

    let client = build_client();
    let out = run_webfetch(&client, args(&format!("{}/api", server.url())))
        .await
        .expect("ok");
    assert!(
        out.output.starts_with("```json\n"),
        "missing fence: {}",
        out.output
    );
    assert!(
        out.output.contains("\"a\": 1"),
        "not pretty: {}",
        out.output
    );
    assert!(out.output.ends_with("\n```"));
}

#[tokio::test]
async fn image_attachment_base64() {
    let mut server = Server::new_async().await;
    // 1x1 transparent PNG
    let png_bytes: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    let _m = server
        .mock("GET", "/img.png")
        .with_status(200)
        .with_header("content-type", "image/png")
        .with_body(png_bytes.clone())
        .create_async()
        .await;

    let client = build_client();
    let out = run_webfetch(&client, args(&format!("{}/img.png", server.url())))
        .await
        .expect("ok");
    assert_eq!(out.attachments.len(), 1);
    let att = &out.attachments[0];
    assert_eq!(att.mime, "image/png");
    assert!(att.url.starts_with("data:image/png;base64,"));
    let b64 = att.url.trim_start_matches("data:image/png;base64,");
    let decoded = BASE64.decode(b64).expect("decode");
    assert_eq!(decoded, png_bytes);
}

// ---------------------------------------------------------------------------
// format 参数
// ---------------------------------------------------------------------------

#[tokio::test]
async fn format_html_returns_raw() {
    let mut server = Server::new_async().await;
    let raw = "<html><body><h1>Raw</h1></body></html>";
    let _m = server
        .mock("GET", "/x")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(raw)
        .create_async()
        .await;

    let client = build_client();
    let mut a = args(&format!("{}/x", server.url()));
    a.format = Some(FetchFormat::Html);
    let out = run_webfetch(&client, a).await.expect("ok");
    assert_eq!(out.output, raw);
    assert_eq!(out.format, "html");
}

#[tokio::test]
async fn format_text_strips_html() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/x")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body("<html><body><p>One</p><p>Two</p><script>alert(1)</script></body></html>")
        .create_async()
        .await;

    let client = build_client();
    let mut a = args(&format!("{}/x", server.url()));
    a.format = Some(FetchFormat::Text);
    let out = run_webfetch(&client, a).await.expect("ok");
    assert!(out.output.contains("One"));
    assert!(out.output.contains("Two"));
    assert!(
        !out.output.contains("alert(1)"),
        "script 内容应被剥离: {}",
        out.output
    );
    assert_eq!(out.format, "text");
}

#[tokio::test]
async fn default_format_is_markdown() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/x")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body("<html><body><h2>x</h2></body></html>")
        .create_async()
        .await;
    let client = build_client();
    let out = run_webfetch(&client, args(&format!("{}/x", server.url())))
        .await
        .unwrap();
    assert_eq!(out.format, "markdown");
}

#[tokio::test]
async fn accept_header_negotiation_for_markdown() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/x")
        .match_header(
            "accept",
            Matcher::Regex(r"text/markdown\s*;\s*q=1\.0".into()),
        )
        .with_status(200)
        .with_header("content-type", "text/plain")
        .with_body("ok")
        .create_async()
        .await;

    let client = build_client();
    let out = run_webfetch(&client, args(&format!("{}/x", server.url())))
        .await
        .expect("ok");
    assert_eq!(out.output, "ok");
}

// ---------------------------------------------------------------------------
// 大小 / 状态 / Cloudflare
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rejects_oversized_content_length() {
    // hyper 不允许 Content-Length 与 body 大小不一致(它会 reject 整个响应),
    // 所以用真实 6MB body 来验证 Content-Length 预检 path 触发。预检失败时
    // 错误消息特意带 "Content-Length" 字样,与"实读字节超限"区分。
    let big = vec![b'x'; MAX_RESPONSE_SIZE + 1024];
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/big")
        .with_status(200)
        .with_header("content-type", "text/plain")
        .with_body(big)
        .create_async()
        .await;

    let client = build_client();
    let err = run_webfetch(&client, args(&format!("{}/big", server.url())))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("too large"), "got: {msg}");
    // mockito 自动设置 Content-Length,因此预检 path 应该命中
    assert!(
        msg.contains("Content-Length"),
        "应触发 Content-Length 预检路径: {msg}"
    );
}

#[tokio::test]
async fn rejects_oversized_actual_bytes() {
    let big = vec![b'x'; MAX_RESPONSE_SIZE + 16];
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/big2")
        .with_status(200)
        .with_header("content-type", "text/plain")
        .with_body(big)
        .create_async()
        .await;

    let client = build_client();
    let err = run_webfetch(&client, args(&format!("{}/big2", server.url())))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("too large"), "got: {err}");
}

#[tokio::test]
async fn http_error_status_propagates() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/404")
        .with_status(404)
        .create_async()
        .await;
    let client = build_client();
    let err = run_webfetch(&client, args(&format!("{}/404", server.url())))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("404"), "got: {err}");
}

#[tokio::test]
async fn cloudflare_challenge_triggers_ua_retry() {
    let mut server = Server::new_async().await;
    // 第一轮:Chrome UA 命中 → 403 + cf-mitigated: challenge
    let _m1 = server
        .mock("GET", "/cf")
        .match_header("user-agent", Matcher::Regex(r"Chrome".into()))
        .with_status(403)
        .with_header("cf-mitigated", "challenge")
        .with_body("Just a moment...")
        .create_async()
        .await;
    // 第二轮:OpenWarp UA → 200
    let _m2 = server
        .mock("GET", "/cf")
        .match_header("user-agent", FALLBACK_UA)
        .with_status(200)
        .with_header("content-type", "text/plain")
        .with_body("after retry")
        .create_async()
        .await;

    let client = build_client();
    let out = run_webfetch(&client, args(&format!("{}/cf", server.url())))
        .await
        .expect("retry should succeed");
    assert_eq!(out.output, "after retry");
}

// ---------------------------------------------------------------------------
// timeout(用 reqwest 自身 0.5s 超时,mockito 不延迟也能验证 clamp 不报错)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn timeout_clamped_to_max() {
    // 校验:传 999 秒会被 clamp 到 120s,不会 panic / 报错
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/x")
        .with_status(200)
        .with_header("content-type", "text/plain")
        .with_body("hi")
        .create_async()
        .await;
    let client = build_client();
    let out = run_webfetch(
        &client,
        FetchArgs {
            url: format!("{}/x", server.url()),
            format: None,
            timeout: Some(999),
        },
    )
    .await
    .expect("ok");
    assert_eq!(out.output, "hi");
}

// ---------------------------------------------------------------------------
// FetchOutput 序列化
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 真实端点 smoke 测试(默认开启;CI 网络受限时设 WARP_SKIP_WEB_INTEGRATION=1)
// ---------------------------------------------------------------------------

fn skip_real() -> bool {
    std::env::var("WARP_SKIP_WEB_INTEGRATION").is_ok()
}

#[tokio::test]
async fn real_example_com_markdown() {
    if skip_real() {
        return;
    }
    let client = build_client();
    let out = run_webfetch(&client, args("https://example.com"))
        .await
        .expect("real example.com");
    assert!(
        out.output.to_lowercase().contains("example domain"),
        "got: {}",
        out.output
    );
}

#[tokio::test]
async fn real_httpbin_html_to_markdown() {
    if skip_real() {
        return;
    }
    let client = build_client();
    let out = run_webfetch(&client, args("https://httpbin.org/html"))
        .await
        .expect("real httpbin html");
    assert!(!out.output.trim().is_empty());
    assert_eq!(out.format, "markdown");
}

#[tokio::test]
async fn real_httpbin_json_pretty() {
    if skip_real() {
        return;
    }
    let client = build_client();
    let out = run_webfetch(&client, args("https://httpbin.org/json"))
        .await
        .expect("real httpbin json");
    assert!(out.output.contains("```json"), "got: {}", out.output);
}

#[tokio::test]
async fn real_httpbin_image_attachment() {
    if skip_real() {
        return;
    }
    let client = build_client();
    let out = run_webfetch(&client, args("https://httpbin.org/image/png"))
        .await
        .expect("real png");
    assert_eq!(out.attachments.len(), 1);
    assert_eq!(out.attachments[0].mime, "image/png");
}

#[tokio::test]
async fn real_httpbin_404_errors() {
    if skip_real() {
        return;
    }
    let client = build_client();
    let err = run_webfetch(&client, args("https://httpbin.org/status/404"))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("404"), "got: {err}");
}

// ---------------------------------------------------------------------------
// 描述文档 / opencode 字节级对齐回归
// ---------------------------------------------------------------------------

/// 锁住 webfetch.md 与 opencode `packages/opencode/src/tool/webfetch.txt`
/// 字节级一致。修改时需同步两边。
#[test]
fn webfetch_description_matches_opencode_verbatim() {
    use super::super::webfetch::WEBFETCH;
    let expected = "- Fetches content from a specified URL\n\
                    - Takes a URL and optional format as input\n\
                    - Fetches the URL content, converts to requested format (markdown by default)\n\
                    - Returns the content in the specified format\n\
                    - Use this tool when you need to retrieve and analyze web content\n\
                    \n\
                    Usage notes:\n\
                    \x20\x20- IMPORTANT: if another tool is present that offers better web fetching capabilities, is more targeted to the task, or has fewer restrictions, prefer using that tool instead of this one.\n\
                    \x20\x20- The URL must be a fully-formed valid URL\n\
                    \x20\x20- HTTP URLs will be automatically upgraded to HTTPS\n\
                    \x20\x20- Format options: \"markdown\" (default), \"text\", or \"html\"\n\
                    \x20\x20- This tool is read-only and does not modify any files\n\
                    \x20\x20- Results may be summarized if the content is very large\n";
    assert_eq!(WEBFETCH.description, expected);
}

#[test]
fn fetch_output_omits_empty_attachments_in_json() {
    let out = FetchOutput {
        url: "https://x".into(),
        status: 200,
        content_type: "text/plain".into(),
        format: "markdown".into(),
        output: "hi".into(),
        attachments: vec![],
    };
    let v = fetch_output_to_json(&out);
    assert!(
        v.get("attachments").is_none(),
        "空 attachments 应被 skip: {v}"
    );
    assert_eq!(v["output"], "hi");
}

/// `_byop_intercepted` sentinel 必须存在于所有 web tool result(包括 error)中,
/// 否则 controller (`controller.rs::needs_byop_local_resume`) 不会触发 auto-resume,
/// 模型会卡在等待结果,UI 显示静默失败。
#[test]
fn fetch_output_carries_byop_sentinel() {
    let out = FetchOutput {
        url: "https://x".into(),
        status: 200,
        content_type: "text/plain".into(),
        format: "markdown".into(),
        output: "hi".into(),
        attachments: vec![],
    };
    let v = fetch_output_to_json(&out);
    assert_eq!(v["_byop_intercepted"], true);

    let err = error_to_json("webfetch", &anyhow::anyhow!("boom"));
    assert_eq!(err["_byop_intercepted"], true);
    assert_eq!(err["status"], "error");
}
