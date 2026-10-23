use super::*;

const FIXED_ID: &str = "0b0a0c1e-1111-4222-8333-444455556666";

fn conversation_id() -> AIConversationId {
    AIConversationId::try_from(FIXED_ID.to_owned()).expect("valid uuid")
}

fn headers(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

fn config(
    extra_headers: Vec<(String, String)>,
    conversation_id: Option<AIConversationId>,
) -> OneshotConfig {
    OneshotConfig {
        base_url: "https://example.com/v1/".to_owned(),
        api_key: "test-key".to_owned(),
        model_id: "test-model".to_owned(),
        api_type: AgentProviderApiType::OpenAi,
        reasoning_effort: ReasoningEffortSetting::Auto,
        extra_headers,
        conversation_id,
    }
}

fn header_value(opts: &ChatOptions, name: &str) -> Option<String> {
    opts.extra_headers
        .as_ref()?
        .iter()
        .find(|(key, _)| key.as_str() == name)
        .map(|(_, value)| value.clone())
}

#[test]
fn applies_static_headers() {
    let (_, opts) = build_oneshot_request(
        &config(headers(&[("x-gateway", "static")]), None),
        "system",
        "user",
        &OneshotOptions::default(),
    );
    assert_eq!(header_value(&opts, "x-gateway"), Some("static".to_owned()));
}

#[test]
fn substitutes_conversation_header() {
    let (_, opts) = build_oneshot_request(
        &config(
            headers(&[("x-session-id", "conv-{conversationId}")]),
            Some(conversation_id()),
        ),
        "system",
        "user",
        &OneshotOptions::default(),
    );
    assert_eq!(
        header_value(&opts, "x-session-id"),
        Some(format!("conv-{FIXED_ID}"))
    );
}

#[test]
fn omits_unresolved_conversation_header() {
    let (_, opts) = build_oneshot_request(
        &config(headers(&[("x-session-id", "{conversationId}")]), None),
        "system",
        "user",
        &OneshotOptions::default(),
    );
    assert!(
        opts.extra_headers.is_none(),
        "missing conversation id must omit the header entirely"
    );
}

#[test]
fn keeps_static_headers_when_conversation_is_missing() {
    let (_, opts) = build_oneshot_request(
        &config(
            headers(&[
                ("x-session-id", "{conversationId}"),
                ("x-gateway", "static"),
            ]),
            None,
        ),
        "system",
        "user",
        &OneshotOptions::default(),
    );
    assert_eq!(header_value(&opts, "x-gateway"), Some("static".to_owned()));
    assert_eq!(header_value(&opts, "x-session-id"), None);
}

#[test]
fn omits_empty_header_rows() {
    let (_, opts) = build_oneshot_request(
        &config(headers(&[("", "value")]), None),
        "system",
        "user",
        &OneshotOptions::default(),
    );
    assert!(opts.extra_headers.is_none());
}
