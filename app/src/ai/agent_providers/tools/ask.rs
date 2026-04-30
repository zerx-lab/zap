//! `ask_user_question`:让模型在缺关键信息时主动反问用户(单选/多选/自由补全)。
//!
//! warp 自家是 `AskUserQuestion`,内部全部用 `MultipleChoice` 一种 Question 类型
//! (是否允许 multiselect / 是否允许 "Other" 自由补全靠内部 bool 决定)。
//!
//! ## 使用建议(写到 description 让模型看到)
//!
//! 不要用本工具问"是否继续"/"你确认吗"这类琐碎问题 — 直接走应答策略。
//! 当用户给的指令含多种合理理解、且选错代价高时再用。

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;
use warp_multi_agent_api as api;

use super::OpenAiTool;

#[derive(Debug, Deserialize)]
struct Args {
    questions: Vec<QuestionArg>,
}

#[derive(Debug, Deserialize)]
struct QuestionArg {
    question: String,
    options: Vec<String>,
    /// 0-based,推荐选项的下标。缺省 = 0。
    #[serde(default)]
    recommended_index: i32,
    /// 是否允许多选。
    #[serde(default)]
    multi_select: bool,
    /// 是否允许用户输入"Other"自由文本。
    #[serde(default)]
    supports_other: bool,
}

fn parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "questions": {
                "type": "array",
                "description": "要向用户提的问题列表(通常 1 个就够,确实有多维需澄清才发多个)。",
                "items": {
                    "type": "object",
                    "properties": {
                        "question": {
                            "type": "string",
                            "description": "问题文本(中文,简短具体)。"
                        },
                        "options": {
                            "type": "array",
                            "items": {"type": "string"},
                            "minItems": 2,
                            "maxItems": 4,
                            "description": "可选项标签列表,2-4 个,具体描述每条选项后果。"
                        },
                        "recommended_index": {
                            "type": "integer",
                            "description": "0-based 推荐选项的下标。",
                            "default": 0
                        },
                        "multi_select": {
                            "type": "boolean",
                            "description": "是否允许用户多选。",
                            "default": false
                        },
                        "supports_other": {
                            "type": "boolean",
                            "description": "是否允许用户输入 \"其他\" 自由文本。",
                            "default": false
                        }
                    },
                    "required": ["question", "options"]
                }
            }
        },
        "required": ["questions"],
        "additionalProperties": false
    })
}

fn from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    let parsed: Args = serde_json::from_str(args)?;
    use api::ask_user_question::question::QuestionType;
    use api::ask_user_question::{MultipleChoice, Option as PbOption, Question};

    let questions: Vec<Question> = parsed
        .questions
        .into_iter()
        .map(|q| {
            let options: Vec<PbOption> = q
                .options
                .into_iter()
                .map(|label| PbOption { label })
                .collect();
            Question {
                question_id: Uuid::new_v4().to_string(),
                question: q.question,
                question_type: Some(QuestionType::MultipleChoice(MultipleChoice {
                    options,
                    recommended_option_index: q.recommended_index,
                    is_multiselect: q.multi_select,
                    supports_other: q.supports_other,
                })),
            }
        })
        .collect();

    Ok(api::message::tool_call::Tool::AskUserQuestion(
        api::AskUserQuestion { questions },
    ))
}

fn result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::ask_user_question_result::answer_item::Answer as A;
    use api::ask_user_question_result::Result as AR;
    use api::message::tool_call_result::Result as R;
    let r = match result {
        R::AskUserQuestion(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(AR::Success(s)) => {
            let answers: Vec<Value> = s
                .answers
                .iter()
                .map(|item| match &item.answer {
                    Some(A::MultipleChoice(mc)) => json!({
                        "question_id": item.question_id,
                        "selected": mc.selected_options,
                        "other_text": if mc.other_text.is_empty() {
                            Value::Null
                        } else {
                            Value::String(mc.other_text.clone())
                        },
                    }),
                    Some(A::Skipped(_)) => json!({
                        "question_id": item.question_id,
                        "skipped": true,
                    }),
                    None => json!({ "question_id": item.question_id, "no_answer": true }),
                })
                .collect();
            json!({ "status": "ok", "answers": answers })
        }
        Some(AR::Error(e)) => json!({ "status": "error", "message": e.message }),
        None => json!({ "status": "cancelled" }),
    };
    Some(value)
}

pub static ASK_USER_QUESTION: OpenAiTool = OpenAiTool {
    name: "ask_user_question",
    description: "在用户指令含歧义、选错代价高时,主动向用户反问(单选/多选/可附自由补全)。\
                  避免用于琐碎确认(如「是否继续」之类)。一次最多发 1-2 个 question。",
    parameters,
    from_args,
    result_to_json,
};
