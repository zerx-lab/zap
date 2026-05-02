//! BYOP system prompt 模板渲染。
//!
//! 把 warp 客户端已经收集好的 `AIAgentContext`(env / git / skills / project_rules / current_time)
//! 渲染为 OpenAI 兼容 endpoint 的 `system` message 字符串。
//!
//! ## 工作流
//!
//! 1. 从 `params.input` 抽出最近一条 `UserQuery.context: Arc<[AIAgentContext]>`
//!    (warp `convert_to.rs::convert_input` 取的也是同一份)
//! 2. `collect_prompt_context` 把每个 enum variant 拍成扁平 `PromptContext` struct
//! 3. `pick_template` 按 model id 子串匹配选 `system/{anthropic,gpt,beast,codex,
//!    gemini,kimi,trinity,default}.j2`(对齐 opencode
//!    `packages/opencode/src/session/system.ts::provider`)
//! 4. minijinja 渲染
//!
//! ## 模板加载
//!
//! 全部模板 `include_str!` 编进二进制(零运行时 IO),改模板需重编。

use std::sync::OnceLock;

use ai::LLMId;
use chrono::Local;
use minijinja::{Environment, Value};
use serde::Serialize;

use crate::ai::agent::AIAgentContext;

// ---------------------------------------------------------------------------
// Template environment
// ---------------------------------------------------------------------------

static ENV: OnceLock<Environment<'static>> = OnceLock::new();

fn build_env() -> Environment<'static> {
    let mut env = Environment::new();

    // Partials
    env.add_template("partials/env.j2", include_str!("prompts/partials/env.j2"))
        .expect("env partial parses");
    env.add_template(
        "partials/skills.j2",
        include_str!("prompts/partials/skills.j2"),
    )
    .expect("skills partial parses");
    env.add_template(
        "partials/project_rules.j2",
        include_str!("prompts/partials/project_rules.j2"),
    )
    .expect("project_rules partial parses");
    env.add_template(
        "partials/tool_aliases.j2",
        include_str!("prompts/partials/tool_aliases.j2"),
    )
    .expect("tool_aliases partial parses");
    env.add_template(
        "partials/footer.j2",
        include_str!("prompts/partials/footer.j2"),
    )
    .expect("footer partial parses");

    // 按 model id 子串匹配分发 system prompt(对齐 opencode
    // `packages/opencode/src/session/system.ts::provider`)。OpenRouter 路径形如
    // `anthropic/claude-3.5-sonnet` / `google/gemini-2.5-flash` / `openai/gpt-4o`
    // 也能正确命中。识别不到家族就走 default.j2 兜底,所以自定义 model id 安全。
    for (name, src) in [
        (
            "system/default.j2",
            include_str!("prompts/system/default.j2") as &str,
        ),
        ("system/anthropic.j2", include_str!("prompts/system/anthropic.j2")),
        ("system/gpt.j2", include_str!("prompts/system/gpt.j2")),
        ("system/beast.j2", include_str!("prompts/system/beast.j2")),
        ("system/codex.j2", include_str!("prompts/system/codex.j2")),
        ("system/gemini.j2", include_str!("prompts/system/gemini.j2")),
        ("system/kimi.j2", include_str!("prompts/system/kimi.j2")),
        ("system/trinity.j2", include_str!("prompts/system/trinity.j2")),
    ] {
        env.add_template(name, src)
            .unwrap_or_else(|e| panic!("template {name} parses: {e}"));
    }

    env
}

fn env() -> &'static Environment<'static> {
    ENV.get_or_init(build_env)
}

// ---------------------------------------------------------------------------
// 模板选择
// ---------------------------------------------------------------------------

/// 按 model id 子串匹配选模板(对齐 opencode
/// `packages/opencode/src/session/system.ts::provider`)。
///
/// 匹配规则(顺序敏感,先到先得):
/// - `gpt-4` / `o1` / `o3` / `o4` → beast(强自治 + sequential thinking)
/// - 其他 `gpt` 中含 `codex` → codex(apply_file_diffs + 严格 final answer formatting)
/// - 其他 `gpt` → gpt(pragmatic engineer + commentary/final 双通道)
/// - `gemini-` → gemini(Core Mandates + Workflows + 大量 examples)
/// - `claude` / `sonnet` / `opus` / `haiku` → anthropic(Claude Code 风格)
/// - `trinity` → trinity(一 tool 一 message 风格)
/// - `kimi` → kimi(SAME language + AGENTS.md)
/// - 其他 → default.j2(兜底)
///
/// 全程 lowercase 后匹配,兼容 `GPT-4o` / `OPENAI/gpt-4o` / `Anthropic/Claude-3.5`
/// 这种用户大小写写法。OpenRouter 形式 `provider/model` 也能正确命中。
pub fn pick_template(model_id: &str) -> &'static str {
    let id = model_id.to_ascii_lowercase();

    if id.contains("gpt-4") || id.contains("o1") || id.contains("o3") || id.contains("o4") {
        return "system/beast.j2";
    }
    if id.contains("gpt") {
        if id.contains("codex") {
            return "system/codex.j2";
        }
        return "system/gpt.j2";
    }
    if id.contains("gemini-") {
        return "system/gemini.j2";
    }
    if id.contains("claude")
        || id.contains("sonnet")
        || id.contains("opus")
        || id.contains("haiku")
    {
        return "system/anthropic.j2";
    }
    if id.contains("trinity") {
        return "system/trinity.j2";
    }
    if id.contains("kimi") {
        return "system/kimi.j2";
    }
    "system/default.j2"
}

/// 从 `LLMId` 中抽取模型 id 字串。BYOP 编码会取 model 部分,
/// 否则原样返回(理论上 BYOP 路径只会传 BYOP id,但兜底一下)。
fn model_id_from_llm_id(id: &LLMId) -> String {
    if let Some((_pid, mid)) = super::llm_id::decode(id) {
        mid
    } else {
        id.as_str().to_owned()
    }
}

// ---------------------------------------------------------------------------
// AIAgentContext → 扁平模板上下文
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Serialize)]
struct ShellCtx {
    name: String,
    version: Option<String>,
}

#[derive(Debug, Default, Serialize)]
struct OsCtx {
    platform: String,
    distribution: Option<String>,
}

#[derive(Debug, Default, Serialize)]
struct GitCtx {
    head: String,
    branch: Option<String>,
}

#[derive(Debug, Serialize)]
struct SkillCtx {
    name: String,
    description: String,
}

#[derive(Debug, Serialize)]
struct ProjectRuleCtx {
    path: String,
    content: String,
}

#[derive(Debug, Default, Serialize)]
struct PromptContext {
    cwd: Option<String>,
    shell: Option<ShellCtx>,
    os: Option<OsCtx>,
    git: Option<GitCtx>,
    skills: Vec<SkillCtx>,
    project_rules: Vec<ProjectRuleCtx>,
    current_time: String,
    model_id: String,
}

fn collect_prompt_context(model_id: &str, ctx: &[AIAgentContext]) -> PromptContext {
    let mut out = PromptContext {
        current_time: Local::now().format("%Y-%m-%d %H:%M:%S %:z").to_string(),
        model_id: model_id.to_owned(),
        ..Default::default()
    };

    for c in ctx {
        match c {
            AIAgentContext::Directory { pwd, .. } => {
                if out.cwd.is_none() {
                    out.cwd = pwd.clone();
                }
            }
            AIAgentContext::ExecutionEnvironment(exec) => {
                out.shell = Some(ShellCtx {
                    name: exec.shell_name.clone(),
                    version: exec.shell_version.clone(),
                });
                let has_os = exec.os.category.is_some() || exec.os.distribution.is_some();
                if has_os {
                    out.os = Some(OsCtx {
                        platform: exec.os.category.clone().unwrap_or_default(),
                        distribution: exec.os.distribution.clone(),
                    });
                }
            }
            AIAgentContext::CurrentTime { current_time } => {
                out.current_time = current_time.format("%Y-%m-%d %H:%M:%S %:z").to_string();
            }
            // 代码索引功能未实现,Codebase 上下文不进 system prompt。
            AIAgentContext::Codebase { .. } => {}
            AIAgentContext::Git { head, branch } => {
                out.git = Some(GitCtx {
                    head: head.clone(),
                    branch: branch.clone(),
                });
            }
            AIAgentContext::Skills { skills } => {
                for s in skills {
                    out.skills.push(SkillCtx {
                        name: s.name.clone(),
                        description: s.description.clone(),
                    });
                }
            }
            AIAgentContext::ProjectRules {
                root_path,
                active_rules,
                ..
            } => {
                use ai::agent::action_result::AnyFileContent;
                for rule in active_rules {
                    let content = match &rule.content {
                        AnyFileContent::StringContent(s) => s.clone(),
                        AnyFileContent::BinaryContent(_) => continue,
                    };
                    let path = if rule.file_name.starts_with('/') {
                        rule.file_name.clone()
                    } else {
                        format!("{root_path}/{}", rule.file_name)
                    };
                    out.project_rules.push(ProjectRuleCtx { path, content });
                }
            }
            // 用户附件类 context(File / Image / SelectedText / Block)不进 system prompt,
            // 由 `user_context::render_user_attachments` 在 chat_stream 的 UserQuery 分支
            // 注入到当前轮 user message。这跟 warp 自家路径分两类的语义对齐:
            // - 环境型 → InputContext.{directory,shell,git,...} → 后端注入 system 区
            // - 附件型 → InputContext.{executed_shell_commands,selected_text,files,images}
            //            → 后端注入 user 区
            AIAgentContext::File(_)
            | AIAgentContext::Image(_)
            | AIAgentContext::SelectedText(_)
            | AIAgentContext::Block(_) => {}
        }
    }

    out
}

// ---------------------------------------------------------------------------
// 公共 API
// ---------------------------------------------------------------------------

/// 渲染最终发给上游模型的 system message 字符串。
///
/// `ctx` 一般来自 `params.input` 中最近一条 `AIAgentInput::UserQuery.context`。
/// 拿不到 context(空数组)也 OK — 模板会用 default 占位渲染。
pub fn render_system(model: &LLMId, ctx: &[AIAgentContext]) -> String {
    let model_id = model_id_from_llm_id(model);
    let template_name = pick_template(&model_id);
    let prompt_ctx = collect_prompt_context(&model_id, ctx);

    let env = env();
    let tmpl = match env.get_template(template_name) {
        Ok(t) => t,
        Err(e) => {
            log::error!("[byop prompt] failed to get template {template_name}: {e}");
            return fallback_system(&model_id);
        }
    };
    match tmpl.render(Value::from_serialize(&prompt_ctx)) {
        Ok(s) => s,
        Err(e) => {
            log::error!("[byop prompt] render {template_name} failed: {e}");
            fallback_system(&model_id)
        }
    }
}

/// 渲染兜底 system(只在模板加载/渲染失败时用,不应在正常路径触发)。
fn fallback_system(model_id: &str) -> String {
    format!(
        "You are the AI coding agent inside OpenWarp, an AI Development Environment (ADE). \
         Model: {model_id}. \
         Use the registered tools (run_shell_command / read_files / apply_file_diffs / grep / file_glob / ...) \
         to take actions on the user's behalf. Be concise."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::agent::AIAgentContext;
    use crate::ai_assistant::execution_context::{WarpAiExecutionContext, WarpAiOsContext};

    #[test]
    fn pick_template_dispatches_by_model_family() {
        // 直连形式
        for (id, want) in [
            ("claude-sonnet-4-5", "system/anthropic.j2"),
            ("claude-opus-4-1", "system/anthropic.j2"),
            ("haiku-3-5", "system/anthropic.j2"),
            ("gpt-4o", "system/beast.j2"),
            ("gpt-4-turbo", "system/beast.j2"),
            ("o1-preview", "system/beast.j2"),
            ("o3-mini", "system/beast.j2"),
            ("o4-mini", "system/beast.j2"),
            ("gpt-5-codex", "system/codex.j2"),
            ("gpt-3.5-turbo", "system/gpt.j2"),
            ("gemini-2.0-flash", "system/gemini.j2"),
            ("gemini-2.5-pro", "system/gemini.j2"),
            ("kimi-k2", "system/kimi.j2"),
            ("trinity-v1", "system/trinity.j2"),
            // 兜底
            ("deepseek-chat", "system/default.j2"),
            ("qwen2.5-coder", "system/default.j2"),
            ("glm-4", "system/default.j2"),
            ("my-custom-model", "system/default.j2"),
            ("", "system/default.j2"),
        ] {
            assert_eq!(pick_template(id), want, "id={id}");
        }
    }

    #[test]
    fn pick_template_handles_openrouter_path_form() {
        // OpenRouter 形式 `provider/model`,子串匹配仍命中正确家族
        for (id, want) in [
            ("anthropic/claude-3.5-sonnet", "system/anthropic.j2"),
            ("anthropic/claude-opus-4", "system/anthropic.j2"),
            ("openai/gpt-4o", "system/beast.j2"),
            ("openai/gpt-5-codex", "system/codex.j2"),
            ("openai/o1-preview", "system/beast.j2"),
            ("google/gemini-2.5-flash", "system/gemini.j2"),
            ("moonshot/kimi-k2", "system/kimi.j2"),
        ] {
            assert_eq!(pick_template(id), want, "id={id}");
        }
    }

    #[test]
    fn pick_template_is_case_insensitive() {
        for (id, want) in [
            ("Claude-Sonnet-4", "system/anthropic.j2"),
            ("GPT-4o", "system/beast.j2"),
            ("Gemini-2.5-Pro", "system/gemini.j2"),
            ("KIMI-K2", "system/kimi.j2"),
            ("Anthropic/Claude-3.5", "system/anthropic.j2"),
        ] {
            assert_eq!(pick_template(id), want, "id={id}");
        }
    }

    #[test]
    fn render_includes_env_block_with_cwd_and_shell() {
        let ctx = vec![
            AIAgentContext::Directory {
                pwd: Some("/home/user/project".into()),
                home_dir: Some("/home/user".into()),
                are_file_symbols_indexed: false,
            },
            AIAgentContext::ExecutionEnvironment(WarpAiExecutionContext {
                os: WarpAiOsContext {
                    category: Some("linux".into()),
                    distribution: Some("Ubuntu 22.04".into()),
                },
                shell_name: "bash".into(),
                shell_version: Some("5.1".into()),
            }),
        ];
        let out = render_system(&LLMId::from("byop:p:deepseek-chat"), &ctx);
        assert!(
            out.contains("Working directory: /home/user/project"),
            "{out}"
        );
        assert!(out.contains("Shell: bash 5.1"), "{out}");
        assert!(out.contains("linux (Ubuntu 22.04)"), "{out}");
        // home 字段已对齐 opencode 砍掉,不再渲染
        assert!(!out.contains("Home directory:"), "{out}");
    }

    #[test]
    fn render_produces_non_empty_for_all_families() {
        // 任意 model id 都能渲染出非空字符串(包含 OpenWarp 自我标识)。
        for id in [
            "claude-sonnet-4-5",
            "gpt-4o",
            "gpt-5-codex",
            "gemini-2.5-pro",
            "kimi-k2",
            "trinity-v1",
            "deepseek-chat",
            "weird-model",
        ] {
            let out = render_system(&LLMId::from(format!("byop:p:{id}").as_str()), &[]);
            assert!(
                out.contains("OpenWarp"),
                "id={id} should mention OpenWarp, got: {out}"
            );
        }
    }

    #[test]
    fn render_omits_skills_block_when_empty() {
        let out = render_system(&LLMId::from("byop:p:deepseek-chat"), &[]);
        // 没 skills 时 skills 区块不应出现
        assert!(
            !out.contains("Skills provide specialized instructions"),
            "{out}"
        );
    }

    #[test]
    fn fallback_does_not_panic() {
        // render_system 永远不会 panic,失败也走 fallback_system
        let out = render_system(&LLMId::from("byop:p:any"), &[]);
        assert!(!out.is_empty());
    }
}
