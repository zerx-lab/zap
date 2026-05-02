//! 把单条 `AIAgentInput::UserQuery` 自带的附件类 `AIAgentContext` 渲染为
//! 发往上游模型的 user message 内容(text 前缀 + binary 多模态部件)。
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
//! | `File`(text)    | file_name / content / line_range                            |
//! | `File`(binary)  | file_name / data / mime_type(P1 binary 通道)              |
//! | `Image`          | mime_type / file_name / data(P1 binary 通道)              |
//!
//! ## 作用域:per-input,不影响 system prompt
//!
//! - 这些附件只注入**当前 UserQuery** 对应的 user message,不进 system prompt
//! - 历史轮的附件不重发(warp 自家也不重发,InputContext 是单 Request 级 payload)
//! - env / git / skills / project_rules / codebase / current_time 这些**环境型** context
//!   仍由 `prompt_renderer` 渲染进 system,与本模块互不重叠

use base64::Engine;

use crate::ai::agent::{AIAgentContext, ImageContext};
use crate::ai::block_context::BlockContext;
use ai::agent::action_result::{AnyFileContent, FileContext};

/// `collect_user_attachments` 返回的双通道结果。
///
/// - `prefix`: 文本前缀块,prepend 到 user message text。包含 block / selected_text /
///   text-like file 的内联 XML,以及 binary 附件的占位提示(让 LLM 能引用文件名)。
/// - `binaries`: 需要作为 `ContentPart::Binary` 注入到多模态 message 的附件
///   (image / PDF / audio)。caller(chat_stream.rs)会按 model capability 过滤后
///   决定是否切到 `MessageContent::Parts`。
#[derive(Debug, Default, Clone)]
pub struct UserAttachments {
    pub prefix: Option<String>,
    pub binaries: Vec<UserBinary>,
}

/// 一条 binary 附件,等价于 genai `Binary::from_base64` 的输入三元组。
#[derive(Debug, Clone)]
pub struct UserBinary {
    pub name: String,
    pub content_type: String,
    /// base64 编码后的数据(无 `data:` 前缀)。
    pub data: String,
}

impl UserAttachments {
    pub fn is_empty(&self) -> bool {
        self.prefix.is_none() && self.binaries.is_empty()
    }
}

/// 渲染单条 UserQuery 的附件 context 为「文本前缀 + binary 部件」。
///
/// 调用方应:
/// 1. 把 `prefix` prepend 到 user query 文本前(中间留空行)
/// 2. 按 model capability 过滤 `binaries`,有保留时切 `MessageContent::Parts`
pub fn collect_user_attachments(ctx: &[AIAgentContext]) -> UserAttachments {
    let mut blocks: Vec<&BlockContext> = Vec::new();
    let mut selected_texts: Vec<&str> = Vec::new();
    let mut text_files: Vec<&FileContext> = Vec::new();
    let mut binary_files: Vec<&FileContext> = Vec::new();
    let mut images: Vec<&ImageContext> = Vec::new();

    for c in ctx {
        match c {
            AIAgentContext::Block(b) => blocks.push(b),
            AIAgentContext::SelectedText(t) => selected_texts.push(t),
            AIAgentContext::File(f) => match &f.content {
                AnyFileContent::StringContent(_) => text_files.push(f),
                AnyFileContent::BinaryContent(_) => binary_files.push(f),
            },
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

    let mut out = UserAttachments::default();

    // ----- prefix -----
    let has_any_prefix_content = !blocks.is_empty()
        || !selected_texts.is_empty()
        || !text_files.is_empty()
        || !binary_files.is_empty()
        || !images.is_empty();
    if has_any_prefix_content {
        let mut prefix = String::with_capacity(256);
        prefix.push_str("<attached_context>\n");
        for b in &blocks {
            render_block(&mut prefix, b);
        }
        for t in &selected_texts {
            render_selected_text(&mut prefix, t);
        }
        for f in &text_files {
            render_file_text(&mut prefix, f);
        }
        for f in &binary_files {
            render_file_binary_placeholder(&mut prefix, f);
        }
        for img in &images {
            render_image_placeholder(&mut prefix, img);
        }
        prefix.push_str("</attached_context>");
        out.prefix = Some(prefix);
    }

    // ----- binaries(供 caller 按 capability 过滤后注入 ContentPart::Binary) -----
    for img in &images {
        out.binaries.push(UserBinary {
            name: img.file_name.clone(),
            content_type: img.mime_type.clone(),
            // ImageContext.data 已经是 base64 字符串(`process_non_image_files` 兄弟路径
            // `read_and_process_images_async` 在 PendingAttachment::Image 入队时就完成了 encoding)
            data: img.data.to_string(),
        });
    }
    for f in &binary_files {
        if let AnyFileContent::BinaryContent(bytes) = &f.content {
            let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
            // 用 file_name 上的扩展名猜 mime;`mime_guess` 与 process_non_image_files
            // 走的是同一套规则,这里再算一遍是因为 FileContext 不保存 mime。
            let mime = mime_guess::from_path(&f.file_name)
                .first_or_octet_stream()
                .to_string();
            out.binaries.push(UserBinary {
                name: f.file_name.clone(),
                content_type: mime,
                data: b64,
            });
        }
    }

    out
}

/// 兼容旧调用方:仅取 prefix 文本。新代码请用 `collect_user_attachments`。
#[cfg(test)]
pub fn render_user_attachments(ctx: &[AIAgentContext]) -> Option<String> {
    collect_user_attachments(ctx).prefix
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

fn render_file_text(out: &mut String, f: &FileContext) {
    use std::fmt::Write;
    let path = xml_attr(&f.file_name);
    let content = match &f.content {
        AnyFileContent::StringContent(content) => content.as_str(),
        AnyFileContent::BinaryContent(_) => return, // shouldn't happen, dispatched away above
    };
    let _ = write!(out, "  <file path=\"{path}\"");
    if let Some(range) = &f.line_range {
        let _ = write!(
            out,
            " line_start=\"{}\" line_end=\"{}\"",
            range.start, range.end
        );
    }
    out.push_str(">\n");
    out.push_str(&xml_text(content));
    if !content.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("  </file>\n");
}

/// Binary 文件 prefix 占位:让 LLM 知道有这个文件可以按 file_name 引用。
/// 实际 bytes 通过 caller 端 `MessageContent::Parts` 走 ContentPart::Binary,
/// 这里**不**重复贴 base64(避免双倍 token + 不少模型对超长 base64 解析慢)。
fn render_file_binary_placeholder(out: &mut String, f: &FileContext) {
    use std::fmt::Write;
    let path = xml_attr(&f.file_name);
    let size = match &f.content {
        AnyFileContent::BinaryContent(bytes) => bytes.len(),
        AnyFileContent::StringContent(_) => 0,
    };
    let _ = writeln!(
        out,
        "  <file path=\"{path}\" binary=\"true\" size=\"{size}\" />"
    );
}

/// Image prefix 占位:与 binary file 同语义,实际数据通过 ContentPart::Binary 进多模态。
fn render_image_placeholder(out: &mut String, img: &ImageContext) {
    use std::fmt::Write;
    let _ = writeln!(
        out,
        "  <image file_name=\"{}\" mime_type=\"{}\" />",
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
