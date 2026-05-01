//! 主动式 AI 模型输出解析。
//!
//! 各子链路要求模型按特定格式回复:
//! - prompt_suggestions:JSON `{"kind","query","should_plan_task"|"files"}`
//! - nld_predict:纯文本单行
//! - relevant_files:JSON `{"paths":[...]}`
//!
//! 模型并不总是干净地遵守格式 — 这里负责剥围栏、容错解析,失败 → `None` / 空。

use serde::Deserialize;

use crate::ai::predict::generate_am_query_suggestions::{
    CodingQuery, GenerateAMQuerySuggestionsResponse, GeneratedFileLocations, SimpleQuery,
    Suggestion,
};

/// 剥 ```` ```json … ``` ```` / ```` ``` … ``` ```` 围栏。
fn strip_code_fence(raw: &str) -> &str {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        // 跳过可能的语言标识(json/JSON/javascript/...)+ 至换行
        let after_lang = match rest.find('\n') {
            Some(idx) => &rest[idx + 1..],
            None => rest,
        };
        if let Some(inner) = after_lang.strip_suffix("```") {
            return inner.trim();
        }
        return after_lang.trim_end_matches('`').trim();
    }
    trimmed
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum SuggestionDto {
    Simple {
        query: String,
        #[serde(default)]
        should_plan_task: bool,
    },
    Coding {
        query: String,
        #[serde(default)]
        files: Vec<String>,
    },
}

/// 解析 prompt_suggestions / nld_generate 的模型输出。
/// 失败 → `None`,调用方映射为 `AgentModePromptSuggestion::Error`。
pub fn parse_suggestion(raw: &str) -> Option<GenerateAMQuerySuggestionsResponse> {
    let cleaned = strip_code_fence(raw);
    let dto: SuggestionDto = serde_json::from_str(cleaned).ok()?;
    let suggestion = match dto {
        SuggestionDto::Simple {
            query,
            should_plan_task,
        } => Suggestion::Simple(SimpleQuery {
            query,
            should_plan_task,
        }),
        SuggestionDto::Coding { query, files } => Suggestion::Coding(CodingQuery {
            query,
            files: files
                .into_iter()
                .map(|file_name| GeneratedFileLocations {
                    file_name,
                    line_numbers: None,
                })
                .collect(),
        }),
    };
    Some(GenerateAMQuerySuggestionsResponse {
        id: String::new(),
        suggestion: Some(suggestion),
    })
}

const PREDICT_MAX_LEN: usize = 200;

/// 解析 nld_predict 的纯文本输出。
/// trim,剥外引号,拒绝多行 / 超长 → `None`。
pub fn sanitize_predict(raw: &str) -> Option<String> {
    let mut s = raw.trim().to_owned();
    if s.is_empty() {
        return None;
    }
    if s.contains('\n') {
        // 取首行(模型有时多嘴解释)
        s = s.lines().next().unwrap_or("").trim().to_owned();
        if s.is_empty() {
            return None;
        }
    }
    let quotes = ['"', '\'', '`', '“', '”', '‘', '’'];
    if let Some(c) = s.chars().next() {
        if quotes.contains(&c) {
            s.remove(0);
        }
    }
    if let Some(c) = s.chars().last() {
        if quotes.contains(&c) {
            let new_len = s.len() - c.len_utf8();
            s.truncate(new_len);
        }
    }
    let s = s.trim().to_owned();
    if s.is_empty() || s.chars().count() > PREDICT_MAX_LEN {
        return None;
    }
    Some(s)
}

#[derive(Debug, Deserialize)]
struct RelevantFilesDto {
    #[serde(default)]
    paths: Vec<String>,
}

/// workflow_metadata 子链路的纯 DTO(避免本模块依赖 drive::workflows 上层类型)。
#[derive(Debug, Clone)]
pub struct WorkflowMetadataDto {
    pub title: String,
    pub description: String,
    pub command: String,
    pub arguments: Vec<WorkflowArgumentDto>,
}

#[derive(Debug, Clone)]
pub struct WorkflowArgumentDto {
    pub name: String,
    pub description: String,
    pub default_value: String,
}

#[derive(Debug, Deserialize)]
struct WorkflowMetadataRaw {
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    command: String,
    #[serde(default)]
    arguments: Vec<WorkflowArgumentRaw>,
}

#[derive(Debug, Deserialize)]
struct WorkflowArgumentRaw {
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    default_value: String,
}

/// 解析 workflow_metadata 的 JSON 输出。
/// 失败 / `command` 为空 → `None`(调用方映射为 BadCommand)。
pub fn parse_workflow_metadata(raw: &str) -> Option<WorkflowMetadataDto> {
    let cleaned = strip_code_fence(raw);
    let parsed: WorkflowMetadataRaw = serde_json::from_str(cleaned).ok()?;
    if parsed.command.trim().is_empty() {
        return None;
    }
    Some(WorkflowMetadataDto {
        title: parsed.title,
        description: parsed.description,
        command: parsed.command,
        arguments: parsed
            .arguments
            .into_iter()
            .filter(|a| !a.name.trim().is_empty())
            .map(|a| WorkflowArgumentDto {
                name: a.name,
                description: a.description,
                default_value: a.default_value,
            })
            .collect(),
    })
}

/// 解析 relevant_files 的 JSON 输出,与输入路径取交集过滤幻觉。
pub fn parse_relevant_files(raw: &str, input_paths: &[String]) -> Vec<String> {
    let cleaned = strip_code_fence(raw);
    let Ok(dto) = serde_json::from_str::<RelevantFilesDto>(cleaned) else {
        return Vec::new();
    };
    let input_set: std::collections::HashSet<&str> =
        input_paths.iter().map(|s| s.as_str()).collect();
    dto.paths
        .into_iter()
        .filter(|p| input_set.contains(p.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_fence_with_lang() {
        assert_eq!(strip_code_fence("```json\n{\"a\":1}\n```"), "{\"a\":1}");
    }

    #[test]
    fn strip_fence_no_lang() {
        assert_eq!(strip_code_fence("```\n{\"a\":1}\n```"), "{\"a\":1}");
    }

    #[test]
    fn parse_simple() {
        let raw = r#"{"kind":"simple","query":"check logs","should_plan_task":false}"#;
        let resp = parse_suggestion(raw).unwrap();
        match resp.suggestion.unwrap() {
            Suggestion::Simple(s) => {
                assert_eq!(s.query, "check logs");
                assert!(!s.should_plan_task);
            }
            _ => panic!("expected Simple"),
        }
    }

    #[test]
    fn parse_coding_with_fence() {
        let raw = "```json\n{\"kind\":\"coding\",\"query\":\"fix bug\",\"files\":[\"a.rs\",\"b.rs\"]}\n```";
        let resp = parse_suggestion(raw).unwrap();
        match resp.suggestion.unwrap() {
            Suggestion::Coding(c) => {
                assert_eq!(c.query, "fix bug");
                assert_eq!(c.files.len(), 2);
            }
            _ => panic!("expected Coding"),
        }
    }

    #[test]
    fn parse_invalid_json() {
        assert!(parse_suggestion("not json").is_none());
    }

    #[test]
    fn sanitize_basic() {
        assert_eq!(
            sanitize_predict("hello world").as_deref(),
            Some("hello world")
        );
    }

    #[test]
    fn sanitize_strip_quotes() {
        assert_eq!(sanitize_predict("\"foo\"").as_deref(), Some("foo"));
    }

    #[test]
    fn sanitize_multiline_takes_first() {
        assert_eq!(
            sanitize_predict("first line\nsecond").as_deref(),
            Some("first line")
        );
    }

    #[test]
    fn sanitize_empty_returns_none() {
        assert!(sanitize_predict("").is_none());
        assert!(sanitize_predict("   ").is_none());
    }

    #[test]
    fn relevant_files_filters_hallucinations() {
        let input = vec!["a.rs".to_owned(), "b.rs".to_owned()];
        let raw = r#"{"paths":["a.rs","fake.rs","b.rs"]}"#;
        let out = parse_relevant_files(raw, &input);
        assert_eq!(out, vec!["a.rs", "b.rs"]);
    }

    #[test]
    fn relevant_files_invalid_returns_empty() {
        assert!(parse_relevant_files("garbage", &[]).is_empty());
    }
}
