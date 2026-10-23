use super::{HeaderVars, resolve_headers};
use crate::ai::agent::conversation::AIConversationId;

const FIXED_ID: &str = "0b0a0c1e-1111-4222-8333-444455556666";

fn conversation_id() -> AIConversationId {
    AIConversationId::try_from(FIXED_ID.to_owned()).expect("valid uuid")
}

/// Input rows, as the settings model stores them.
fn headers(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

/// A single expected row.
fn expect(name: &str, value: impl Into<String>) -> (String, String) {
    (name.to_owned(), value.into())
}

fn resolve(pairs: &[(&str, &str)], id: Option<AIConversationId>) -> Vec<(String, String)> {
    let vars = HeaderVars::default().with_conversation_id(id);
    resolve_headers(&headers(pairs), &vars)
}

#[test]
fn substitutes_conversation_id_exactly() {
    assert_eq!(
        resolve(
            &[("x-session-id", "conv-{conversationId}")],
            Some(conversation_id())
        ),
        vec![expect("x-session-id", format!("conv-{FIXED_ID}"))]
    );
}

#[test]
fn substitutes_every_occurrence_in_a_value() {
    assert_eq!(
        resolve(
            &[("x-session-id", "{conversationId}-{conversationId}")],
            Some(conversation_id())
        ),
        vec![expect("x-session-id", format!("{FIXED_ID}-{FIXED_ID}"))]
    );
}

#[test]
fn substitutes_next_to_literal_text() {
    assert_eq!(
        resolve(
            &[("x-session-id", "prefix/{conversationId}/suffix")],
            Some(conversation_id())
        ),
        vec![expect("x-session-id", format!("prefix/{FIXED_ID}/suffix"))]
    );
}

#[test]
fn leaves_unknown_tokens_literal() {
    assert_eq!(
        resolve(
            &[("x-gateway", "{modelId}:{conversationId}")],
            Some(conversation_id())
        ),
        vec![expect("x-gateway", format!("{{modelId}}:{FIXED_ID}"))]
    );
}

#[test]
fn leaves_other_casing_and_spacing_literal() {
    assert_eq!(
        resolve(
            &[
                ("a", "{conversationid}"),
                ("b", "{conversation_id}"),
                ("c", "{ conversationId }"),
                ("d", "{conversationID}"),
            ],
            Some(conversation_id())
        ),
        vec![
            expect("a", "{conversationid}"),
            expect("b", "{conversation_id}"),
            expect("c", "{ conversationId }"),
            expect("d", "{conversationID}"),
        ]
    );
}

#[test]
fn never_templates_header_names() {
    assert_eq!(
        resolve(&[("{conversationId}", "value")], Some(conversation_id())),
        vec![expect("{conversationId}", "value")]
    );
}

#[test]
fn omits_row_when_conversation_id_is_missing() {
    assert!(resolve(&[("x-session-id", "conv-{conversationId}")], None).is_empty());
}

#[test]
fn omits_row_when_missing_id_is_embedded_in_text() {
    assert!(resolve(&[("x-session-id", "conv-{conversationId}-suffix")], None).is_empty());
}

#[test]
fn keeps_other_rows_when_one_is_omitted() {
    assert_eq!(
        resolve(
            &[
                ("x-session-id", "conv-{conversationId}"),
                ("x-static", "always"),
            ],
            None
        ),
        vec![expect("x-static", "always")]
    );
}

#[test]
fn skips_empty_header_names() {
    assert!(resolve(&[("", "value"), ("   ", "value")], Some(conversation_id())).is_empty());
}

#[test]
fn keeps_static_rows_unchanged() {
    assert_eq!(
        resolve(&[("x-portkey-provider", "openai")], Some(conversation_id())),
        vec![expect("x-portkey-provider", "openai")]
    );
}

#[test]
fn passes_through_values_without_placeholders_even_without_a_conversation() {
    assert_eq!(
        resolve(&[("authorization", "Bearer token")], None),
        vec![expect("authorization", "Bearer token")]
    );
}

#[test]
fn handles_unbalanced_braces_literally() {
    assert_eq!(
        resolve(
            &[("x-gateway", "a{conversationId")],
            Some(conversation_id())
        ),
        vec![expect("x-gateway", "a{conversationId")]
    );
    assert_eq!(
        resolve(&[("x-gateway", "}not-a-token{")], Some(conversation_id())),
        vec![expect("x-gateway", "}not-a-token{")]
    );
}

#[test]
fn handles_doubled_braces_literally() {
    assert_eq!(
        resolve(
            &[("x-gateway", "{{conversationId}}")],
            Some(conversation_id())
        ),
        vec![expect("x-gateway", "{{conversationId}}")]
    );
}

#[test]
fn preserves_row_order() {
    assert_eq!(
        resolve(
            &[
                ("first", "1"),
                ("second", "{conversationId}"),
                ("third", "3"),
            ],
            Some(conversation_id())
        ),
        vec![
            expect("first", "1"),
            expect("second", FIXED_ID),
            expect("third", "3"),
        ]
    );
}

#[test]
fn multiple_headers_resolve_independently() {
    assert_eq!(
        resolve(
            &[
                ("x-session-id", "{conversationId}"),
                ("x-other", "{conversationId}"),
            ],
            Some(conversation_id())
        ),
        vec![
            expect("x-session-id", FIXED_ID),
            expect("x-other", FIXED_ID),
        ]
    );
}
