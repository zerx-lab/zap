//! `read_skill`:读取 Zap 的 Skill markdown 模板。
//!
//! Skill 是用户/项目预定义的可复用工作流(`SKILL.md` 文件 + 可选元数据)。
//! 模型读 skill 后能按用户期望的步骤推进任务。warp 自家维护一个 `SkillManager`
//! 索引所有可用 skill,既可以用 name(frontmatter `name` 字段)也可以用绝对路径或
//! bundled id 引用。
//!
//! ## 入参契约
//!
//! BYOP 路径暴露 `name` 字段,值取自 system prompt `<available_skills><skill><name>`。
//! `from_args` 把 name 装入 proto 的 `SkillReference::SkillPath` 槽位(不改 proto),
//! 由 `read_skill` executor 在 cache miss 时先按 name 反查到真实 SKILL.md 绝对路径
//! 再读盘。这条 fallback 也兼容模型万一直接传绝对路径或 bundled 形式
//! `@warp-skill:<id>` 的旧写法。
//!
//! ## 使用建议(写到 description)
//!
//! 模型可在以下场景主动调:
//! - 用户提到 skill 名 / 文件名 / 路径
//! - 任务匹配某 skill 描述(如"做 PR review" 触发 `review` skill)

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use warp_multi_agent_api as api;

use super::OpenAiTool;

#[derive(Debug, Deserialize)]
struct Args {
    name: String,
}

fn parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Skill 名称(与 system prompt 内 <available_skills><skill><name> 字段完全一致)。"
            }
        },
        "required": ["name"],
        "additionalProperties": false
    })
}

fn from_args(args: &str) -> Result<api::message::tool_call::Tool> {
    use api::message::tool_call::read_skill::SkillReference;
    let parsed: Args = serde_json::from_str(args)?;
    // 复用 proto 的 `SkillPath` 槽位携带 name(避免 proto schema 变更);
    // executor 端在 cache miss 时按 name 反查真实 SKILL.md 路径。
    Ok(api::message::tool_call::Tool::ReadSkill(
        api::message::tool_call::ReadSkill {
            skill_reference: Some(SkillReference::SkillPath(parsed.name)),
            name: String::new(),
        },
    ))
}

fn result_to_json(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::message::tool_call_result::Result as R;
    use api::read_skill_result::Result as SR;
    let r = match result {
        R::ReadSkill(r) => r,
        _ => return None,
    };
    let value = match &r.result {
        Some(SR::Success(s)) => {
            // FileContent { file_path, content, line_range } 直接是单个 message
            // 不是 oneof,无须解包 inner content。
            let (path, content) = s
                .content
                .as_ref()
                .map(|c| (c.file_path.clone(), c.content.clone()))
                .unwrap_or_default();
            json!({ "status": "ok", "path": path, "content": content })
        }
        Some(SR::Error(e)) => json!({ "status": "error", "message": e.message }),
        None => json!({ "status": "cancelled" }),
    };
    Some(value)
}

pub static READ_SKILL: OpenAiTool = OpenAiTool {
    name: "read_skill",
    description: include_str!("../prompts/tool_descriptions/read_skill.md"),
    parameters,
    from_args,
    result_to_json,
};
