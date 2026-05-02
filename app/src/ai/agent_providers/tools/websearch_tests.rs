//! `web_runtime::run_websearch` 单测(mockito,无外网)。

use super::*;
use mockito::{Matcher, Server};

fn build_client() -> reqwest::Client {
    reqwest::Client::builder().build().expect("client")
}

fn search_args(query: &str) -> SearchToolArgs {
    SearchToolArgs {
        query: query.to_owned(),
        num_results: None,
        livecrawl: None,
        search_type: None,
        context_max_characters: None,
    }
}

fn sse_body(text: &str) -> String {
    format!(
        "event: message\ndata: {{\"result\":{{\"content\":[{{\"type\":\"text\",\"text\":{}}}]}}}}\n\n",
        serde_json::to_string(text).unwrap()
    )
}

// ---------------------------------------------------------------------------
// 端点路由 / API key 注入
// ---------------------------------------------------------------------------

#[tokio::test]
async fn anonymous_endpoint_no_querystring() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(sse_body("hello"))
        .create_async()
        .await;

    let client = build_client();
    let out = run_websearch(&client, search_args("q"), None, Some(&server.url()))
        .await
        .expect("ok");
    assert_eq!(out.results, "hello");
    assert_eq!(out.query, "q");
}

#[tokio::test]
async fn passes_api_key_via_querystring() {
    // 不直接验证 mockito 的 querystring(因为我们用 endpoint_override,
    // 而 endpoint_override 已经替代了 endpoint_url)。
    // 单独验证 api_key 通过 endpoint_url 拼接。
    let url = exa::endpoint_url(Some("k1+k2"));
    assert!(url.contains("?exaApiKey="));
    assert!(url.contains("k1%2Bk2"), "应 percent-encode: {url}");
}

// ---------------------------------------------------------------------------
// 请求 body shape
// ---------------------------------------------------------------------------

#[tokio::test]
async fn request_body_is_jsonrpc_with_default_args() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .match_body(Matcher::PartialJsonString(
            r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"web_search_exa"}}"#.into(),
        ))
        .match_body(Matcher::PartialJsonString(
            r#"{"params":{"arguments":{"query":"rust","numResults":8,"type":"auto","livecrawl":"fallback"}}}"#.into(),
        ))
        .with_status(200)
        .with_body(sse_body("ok"))
        .create_async()
        .await;

    let client = build_client();
    let out = run_websearch(&client, search_args("rust"), None, Some(&server.url()))
        .await
        .expect("ok");
    assert_eq!(out.results, "ok");
}

#[tokio::test]
async fn all_optional_args_passthrough() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .match_body(Matcher::PartialJsonString(
            r#"{"params":{"arguments":{"query":"deep","numResults":20,"type":"deep","livecrawl":"preferred","contextMaxCharacters":15000}}}"#.into(),
        ))
        .with_status(200)
        .with_body(sse_body("deep result"))
        .create_async()
        .await;

    let args = SearchToolArgs {
        query: "deep".into(),
        num_results: Some(20),
        livecrawl: Some("preferred".into()),
        search_type: Some("deep".into()),
        context_max_characters: Some(15000),
    };
    let out = run_websearch(&build_client(), args, None, Some(&server.url()))
        .await
        .expect("ok");
    assert_eq!(out.results, "deep result");
}

#[tokio::test]
async fn sends_correct_accept_header() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .match_header("accept", Matcher::Regex("text/event-stream".into()))
        .match_header("content-type", Matcher::Regex("application/json".into()))
        .with_status(200)
        .with_body(sse_body("x"))
        .create_async()
        .await;
    run_websearch(&build_client(), search_args("q"), None, Some(&server.url()))
        .await
        .expect("ok");
}

// ---------------------------------------------------------------------------
// SSE 解析 / 错误
// ---------------------------------------------------------------------------

#[tokio::test]
async fn empty_results_returns_fallback() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .with_status(200)
        .with_body("event: message\ndata: {\"result\":{\"content\":[]}}\n\n")
        .create_async()
        .await;
    let out = run_websearch(&build_client(), search_args("q"), None, Some(&server.url()))
        .await
        .expect("ok");
    assert!(
        out.results.contains("No search results found"),
        "got: {}",
        out.results
    );
}

#[tokio::test]
async fn http_error_propagates() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .with_status(500)
        .with_body("internal err")
        .create_async()
        .await;
    let err = run_websearch(&build_client(), search_args("q"), None, Some(&server.url()))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("500"), "got: {err}");
}

#[tokio::test]
async fn invalid_sse_payload_returns_err() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .with_status(200)
        .with_body("data: not_json\n")
        .create_async()
        .await;
    let err = run_websearch(&build_client(), search_args("q"), None, Some(&server.url()))
        .await
        .unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("Exa SSE") || msg.contains("invalid"),
        "got: {msg}"
    );
}

#[tokio::test]
async fn handles_multiple_data_lines() {
    let mut server = Server::new_async().await;
    let body = "data: {\"result\":{\"content\":[]}}\n\
                data: {\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"second\"}]}}\n\n";
    let _m = server
        .mock("POST", "/")
        .with_status(200)
        .with_body(body)
        .create_async()
        .await;
    let out = run_websearch(&build_client(), search_args("q"), None, Some(&server.url()))
        .await
        .expect("ok");
    assert_eq!(out.results, "second");
}

// ---------------------------------------------------------------------------
// SearchToolArgs → SearchArgs 默认填充
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 真实端点 smoke 测试(默认开启;CI 网络受限时设 WARP_SKIP_WEB_INTEGRATION=1)
// ---------------------------------------------------------------------------

fn skip_real() -> bool {
    std::env::var("WARP_SKIP_WEB_INTEGRATION").is_ok()
}

#[tokio::test]
async fn real_exa_anonymous_search() {
    if skip_real() {
        return;
    }
    let client = build_client();
    let out = run_websearch(
        &client,
        search_args("rust async runtime tutorial"),
        None,
        None,
    )
    .await
    .expect("real Exa anonymous");
    assert!(!out.results.trim().is_empty(), "empty Exa output");
    assert_eq!(out.query, "rust async runtime tutorial");
}

// ---------------------------------------------------------------------------
// 描述文档 / opencode 字节级对齐 + {{year}} 占位回归
// ---------------------------------------------------------------------------

/// websearch.md 必须包含 `{{year}}` 占位 — `chat_stream::build_tools_array`
/// 在 build 时会替换成当前年份(对齐 opencode `websearch.ts:30-32`)。删占位
/// 会让模型用训练数据里的旧年份做时间敏感搜索。
#[test]
fn websearch_description_contains_year_placeholder() {
    use super::super::websearch::WEBSEARCH;
    assert!(
        WEBSEARCH.description.contains("{{year}}"),
        "websearch description 必须含 {{{{year}}}} 占位,build 时会替换"
    );
}

/// 锁住 websearch.md 与 opencode `packages/opencode/src/tool/websearch.txt`
/// 字节级一致。修改时需同步两边。
#[test]
fn websearch_description_matches_opencode_verbatim() {
    use super::super::websearch::WEBSEARCH;
    let expected = "- Search the web using Exa AI - performs real-time web searches and can scrape content from specific URLs\n\
                    - Provides up-to-date information for current events and recent data\n\
                    - Supports configurable result counts and returns the content from the most relevant websites\n\
                    - Use this tool for accessing information beyond knowledge cutoff\n\
                    - Searches are performed automatically within a single API call\n\
                    \n\
                    Usage notes:\n\
                    \x20\x20- Supports live crawling modes: 'fallback' (backup if cached unavailable) or 'preferred' (prioritize live crawling)\n\
                    \x20\x20- Search types: 'auto' (balanced), 'fast' (quick results), 'deep' (comprehensive search)\n\
                    \x20\x20- Configurable context length for optimal LLM integration\n\
                    \x20\x20- Domain filtering and advanced search options available\n\
                    \n\
                    The current year is {{year}}. You MUST use this year when searching for recent information or current events\n\
                    - Example: If the current year is 2026 and the user asks for \"latest AI news\", search for \"AI news 2026\", NOT \"AI news 2025\"\n";
    assert_eq!(WEBSEARCH.description, expected);
}

#[test]
fn search_tool_args_into_exa_uses_defaults() {
    let a = SearchToolArgs {
        query: "z".into(),
        num_results: None,
        livecrawl: None,
        search_type: None,
        context_max_characters: None,
    };
    let exa = a.into_exa_args();
    assert_eq!(exa.query, "z");
    assert_eq!(exa.num_results, 8);
    assert_eq!(exa.search_type, "auto");
    assert_eq!(exa.livecrawl, "fallback");
    assert!(exa.context_max_characters.is_none());
}

/// `_byop_intercepted` sentinel 必须存在于 search result 中(同 webfetch),
/// 让 controller 知道触发 auto-resume,否则模型卡死等结果。
#[test]
fn search_output_carries_byop_sentinel() {
    let out = SearchOutput {
        query: "q".into(),
        results: "r".into(),
    };
    let v = search_output_to_json(&out);
    assert_eq!(v["_byop_intercepted"], true);
}

#[test]
fn search_tool_args_overrides_defaults() {
    let a = SearchToolArgs {
        query: "z".into(),
        num_results: Some(2),
        livecrawl: Some("preferred".into()),
        search_type: Some("fast".into()),
        context_max_characters: Some(500),
    };
    let exa = a.into_exa_args();
    assert_eq!(exa.num_results, 2);
    assert_eq!(exa.livecrawl, "preferred");
    assert_eq!(exa.search_type, "fast");
    assert_eq!(exa.context_max_characters, Some(500));
}
