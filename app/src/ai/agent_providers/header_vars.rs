//! Placeholder expansion for provider header values.
//!
//! `AgentProvider::extra_headers` stores literal name/value pairs. Values may contain
//! `{conversationId}`, which is replaced at request time with the local ID of the agent
//! conversation the request belongs to. Resolution is deliberately conservative: only exact,
//! case-sensitive tokens are substituted, and anything else between braces is left untouched so
//! literal braces keep working.

use crate::ai::agent::conversation::AIConversationId;

const CONVERSATION_ID_TOKEN: &str = "conversationId";

/// Values available to `{...}` placeholders in provider header values.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct HeaderVars {
    pub(crate) conversation_id: Option<AIConversationId>,
}

impl HeaderVars {
    pub(crate) fn with_conversation_id(
        mut self,
        conversation_id: Option<AIConversationId>,
    ) -> Self {
        self.conversation_id = conversation_id;
        self
    }
}

/// Expands supported placeholders in header values and drops rows that must not be sent.
///
/// A row is dropped when its name is empty, when a known placeholder has no value for this
/// request, or when the resolved value is empty. Unknown `{...}` sequences are preserved
/// verbatim.
pub(crate) fn resolve_headers(
    headers: &[(String, String)],
    vars: &HeaderVars,
) -> Vec<(String, String)> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            if name.trim().is_empty() {
                return None;
            }
            let value = resolve_value(value, vars)?;
            if value.is_empty() {
                return None;
            }
            Some((name.clone(), value))
        })
        .collect()
}

/// Returns `None` when the value references a known placeholder that has no value; the caller
/// then omits the whole header instead of sending an empty or partial value.
fn resolve_value(value: &str, vars: &HeaderVars) -> Option<String> {
    let mut resolved = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(open) = rest.find('{') {
        resolved.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            // Unbalanced brace: keep the remainder, including the `{`, as written.
            resolved.push_str(&rest[open..]);
            return Some(resolved);
        };
        let token = &after_open[..close];
        if token == CONVERSATION_ID_TOKEN {
            resolved.push_str(&vars.conversation_id?.to_string());
        } else {
            resolved.push('{');
            resolved.push_str(token);
            resolved.push('}');
        }
        rest = &after_open[close + 1..];
    }
    resolved.push_str(rest);
    Some(resolved)
}

#[cfg(test)]
#[path = "header_vars_tests.rs"]
mod tests;
