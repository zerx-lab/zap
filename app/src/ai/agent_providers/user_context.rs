//! 把单条 `AIAgentInput::UserQuery` 自带的附件类 `AIAgentContext` 渲染为
//! 发往上游模型的 user message 前置文本块。
//!
//! ## 与 warp 自家路径的对齐
//!
//! warp 自家协议下,这些附件走 `api::InputContext` 的 `executed_shell_commands /
//! selected_text / files / images` 字段(见 `app/src/ai/agent/api/convert_to.rs`
//! `convert_context`)。BYOP 直接对接 OpenAI / Anthropic / Gemini / Ollama 兼容
//! `/chat/completions`,没有 `InputContext` 这层结构,只能把数据嵌进 user message。
//!
//! 字段严格对齐 warp protobuf,不引入协议外字段:
//!
//! | 类型             | warp protobuf 字段                                          |
//! |------------------|-------------------------------------------------------------|
//! | `Block`          | command / output / exit_code / command_id / is_auto_attached / started_ts / finished_ts |
//! | `SelectedText`   | text                                                        |
//! | `File`           | file_name / content / line_range                            |
//! | `Image`          | mime_type / file_name(数据走 base64 暂仅在 multipart 模式) |
//!
//! ## 作用域:per-input,不影响 system prompt
//!
//! - 这些附件只注入**当前 UserQuery** 对应的 user message,不进 system prompt
//! - 历史轮的附件不重发(warp 自家也不重发,InputContext 是单 Request 级 payload)
//! - env / git / skills / project_rules / codebase / current_time 这些**环境型** context
//!   仍由 `prompt_renderer` 渲染进 system,与本模块互不重叠

use crate::ai::agent::{AIAgentContext, ImageContext};
use crate::ai::block_context::BlockContext;
use ai::agent::action_result::{AnyFileContent, FileContext};

/// 渲染单条 UserQuery 的附件 context 为 user message 前置文本块。
///
/// 返回 `None` 表示这条 input 没有任何附件类 context(env/git/skills 等环境型不算附件,
/// 它们由 system prompt 路径处理)。返回 `Some(s)` 时,调用方应把 `s` prepend 到 user
/// query 文本前(中间留空行)。
pub fn render_user_attachments(ctx: &[AIAgentContext]) -> Option<String> {
    let mut blocks: Vec<&BlockContext> = Vec::new();
    let mut selected_texts: Vec<&str> = Vec::new();
    let mut files: Vec<&FileContext> = Vec::new();
    let mut images: Vec<&ImageContext> = Vec::new();

    for c in ctx {
        match c {
            AIAgentContext::Block(b) => blocks.push(b),
            AIAgentContext::SelectedText(t) => selected_texts.push(t),
            AIAgentContext::File(f) => files.push(f),
            AIAgentContext::Image(img) => images.push(img),
            // 环境型 context 由 prompt_renderer 处理,不进 user message。
            AIAgentContext::Directory { .. }
            | AIAgentContext::ExecutionEnvironment(_)
            | AIAgentContext::CurrentTime { .. }
            | AIAgentContext::Codebase { .. }
            | AIAgentContext::ProjectRules { .. }
            | AIAgentContext::Git { .. }
            | AIAgentContext::Skills { .. } => {}
        }
    }

    if blocks.is_empty() && selected_texts.is_empty() && files.is_empty() && images.is_empty() {
        return None;
    }

    let mut out = String::with_capacity(256);
    out.push_str("<attached_context>\n");
    for b in &blocks {
        render_block(&mut out, b);
    }
    for t in &selected_texts {
        render_selected_text(&mut out, t);
    }
    for f in &files {
        render_file(&mut out, f);
    }
    for img in &images {
        render_image_placeholder(&mut out, img);
    }
    out.push_str("</attached_context>");
    Some(out)
}

// ---------------------------------------------------------------------------
// 子渲染器
// ---------------------------------------------------------------------------

fn render_block(out: &mut String, b: &BlockContext) {
    use std::fmt::Write;
    let _ = write!(
        out,
        "  <executed_shell_command command_id=\"{}\" exit_code=\"{}\" auto_attached=\"{}\"",
        xml_attr(&String::from(b.id.clone())),
        b.exit_code.value(),
        b.is_auto_attached
    );
    if let Some(ts) = b.started_ts {
        let _ = write!(out, " started_ts=\"{}\"", ts.to_rfc3339());
    }
    if let Some(ts) = b.finished_ts {
        let _ = write!(out, " finished_ts=\"{}\"", ts.to_rfc3339());
    }
    out.push_str(">\n");
    out.push_str("    <command>");
    out.push_str(&xml_text(&b.command));
    out.push_str("</command>\n");
    out.push_str("    <output>");
    out.push_str(&xml_text(&b.output));
    out.push_str("</output>\n");
    out.push_str("  </executed_shell_command>\n");
}

fn render_selected_text(out: &mut String, t: &str) {
    out.push_str("  <selected_text>");
    out.push_str(&xml_text(t));
    out.push_str("</selected_text>\n");
}

fn render_file(out: &mut String, f: &FileContext) {
    use std::fmt::Write;
    let path = xml_attr(&f.file_name);
    match &f.content {
        AnyFileContent::StringContent(content) => {
            let _ = write!(out, "  <file path=\"{path}\"");
            if let Some(range) = &f.line_range {
                let _ = write!(out, " line_start=\"{}\" line_end=\"{}\"", range.start, range.end);
            }
            out.push_str(">\n");
            out.push_str(&xml_text(content));
            if !content.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("  </file>\n");
        }
        AnyFileContent::BinaryContent(data) => {
            let _ = write!(
                out,
                "  <file path=\"{path}\" binary=\"true\" size=\"{}\" />\n",
                data.len()
            );
        }
    }
}

/// 首版纯 text 模式:图片仅作占位提示,不嵌 base64。
/// 后续按 provider adapter 升级到 multipart(genai `MessageContent::Parts`)。
fn render_image_placeholder(out: &mut String, img: &ImageContext) {
    use std::fmt::Write;
    let _ = write!(
        out,
        "  <image file_name=\"{}\" mime_type=\"{}\" />\n",
        xml_attr(&img.file_name),
        xml_attr(&img.mime_type),
    );
}

// ---------------------------------------------------------------------------
// XML 转义
// ---------------------------------------------------------------------------

fn xml_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn xml_attr(s: &str) -> String {
    xml_text(s).replace('"', "&quot;")
}

#[cfg(test)]
#[path = "user_context_tests.rs"]
mod tests;
