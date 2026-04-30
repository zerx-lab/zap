//! MCP(Model Context Protocol)服务器的 tool 注入与双向翻译。
//!
//! 与 `shell.rs` / `files.rs` 等静态 tool 不同,MCP tool 是**动态**的:
//! 用户配置的每个 MCP server 暴露自己的 tool 列表(name + description +
//! JSON Schema),需要在每次请求构造时按 `RequestParams.mcp_context` 即时
//! 注入到 OpenAI tools 数组。
//!
//! ## 命名约定
//!
//! OpenAI function 名: `mcp__<server_name_safe>__<tool_name>`
//! - 双下划线分隔,避免与内置 tool 名(下划线分词)冲突
//! - server_name_safe = server.name 中所有非 `[a-zA-Z0-9_-]` 字符替换为 `_`
//!
//! ## 反向解析
//!
//! 看到 `mcp__` 前缀名时:
//! 1. 拆出 `server_name_safe` 和 `tool_name`
//! 2. 在 `params.mcp_context.servers` 中按 sanitize 后的 name 匹配,拿 server.id
//! 3. 构造 `Message::ToolCall::CallMcpTool { name: tool_name, args, server_id }`
//!
//! ## Result 序列化
//!
//! `ToolCallResultType::CallMcpTool(CallMcpToolResult)` 中的 result 是结构化
//! 的 MCP content,转成 JSON 给上游模型。

use anyhow::{anyhow, Result};
use prost_types::value::Kind as ProstKind;
use serde_json::{json, Map, Value};
use warp_multi_agent_api as api;

use crate::ai::agent::{MCPContext, MCPServer};

const PREFIX: &str = "mcp__";
const SEP: &str = "__";
/// 读 MCP resource 的统一函数名(uri 跨 server,语义上是单一 tool)。
const READ_RESOURCE_NAME: &str = "mcp_read_resource";

/// 把 server.name 转成可作为 OpenAI function name 一部分的安全字符串。
fn sanitize_server_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// 给一条 MCP tool 生成 OpenAI function 名。
pub fn function_name(server: &MCPServer, tool_name: &str) -> String {
    format!("{}{}{}{}", PREFIX, sanitize_server_name(&server.name), SEP, tool_name)
}

/// 判断给定 OpenAI function name 是否是 MCP 调用(含动态 mcp__ 前缀工具调用
/// 与统一的 mcp_read_resource 资源读取)。
pub fn is_mcp_function(name: &str) -> bool {
    name == READ_RESOURCE_NAME || name.starts_with(PREFIX)
}

/// 把 mcp_context 中所有 server 的 tool 转成 OpenAI tool 定义(name/description/parameters)。
/// 同时,如果至少有一个 server 暴露了 resources,会附加一个统一的 `mcp_read_resource`
/// tool 定义,供模型读资源用。
/// 返回三元组 `(name, description, parameters_value)` — 调用方包成 ToolDef。
pub fn build_mcp_tool_defs(ctx: &MCPContext) -> Vec<(String, String, Value)> {
    let mut out = Vec::new();
    for server in &ctx.servers {
        for tool in &server.tools {
            // rmcp::Tool.input_schema 是 Arc<Map<String,Value>>,克隆后 wrap 成 Value::Object。
            let schema = Value::Object((*tool.input_schema).clone());
            let desc = tool
                .description
                .as_ref()
                .map(|d| d.to_string())
                .unwrap_or_default();
            let prefixed_desc = if desc.is_empty() {
                format!("MCP server `{}` 的工具 {}", server.name, tool.name)
            } else {
                format!("[MCP/{}] {}", server.name, desc)
            };
            out.push((function_name(server, &tool.name), prefixed_desc, schema));
        }
    }

    // 仅在有任意 server 暴露 resources 时才注入 read_resource tool,避免
    // 模型空发(可读列表是 server 决定的)。
    let any_resources = ctx.servers.iter().any(|s| !s.resources.is_empty());
    if any_resources {
        let mut available_uris: Vec<String> = Vec::new();
        for s in &ctx.servers {
            for r in &s.resources {
                available_uris.push(format!("[{}] {} ({})", s.name, r.name, r.uri));
            }
        }
        let desc = format!(
            "读取 MCP server 暴露的资源(文件 / 数据库 / API 等)。\
             可用资源:\n- {}",
            available_uris.join("\n- ")
        );
        let schema = json!({
            "type": "object",
            "properties": {
                "uri": {
                    "type": "string",
                    "description": "资源 URI(从可用资源列表中选)。"
                },
                "server": {
                    "type": "string",
                    "description": "可选: 资源所属 MCP server 的 name(会按 sanitize 规则匹配)。当多个 server 暴露同名 uri 时必填。"
                }
            },
            "required": ["uri"],
            "additionalProperties": false
        });
        out.push((READ_RESOURCE_NAME.to_owned(), desc, schema));
    }

    out
}

/// 反向解析:把上游模型回的 `mcp__server__tool` 或 `mcp_read_resource` 调用
/// 翻译成 warp `Tool::CallMcpTool` 或 `Tool::ReadMcpResource`。
/// 失败原因: name 格式错误 / server 找不到 / args 解析失败。
pub fn parse_mcp_tool_call(
    function_name: &str,
    arguments_json: &str,
    ctx: Option<&MCPContext>,
) -> Result<api::message::tool_call::Tool> {
    if function_name == READ_RESOURCE_NAME {
        return parse_read_resource(arguments_json, ctx);
    }
    let body = function_name
        .strip_prefix(PREFIX)
        .ok_or_else(|| anyhow!("not an MCP function name"))?;
    let (server_name_safe, tool_name) = body
        .split_once(SEP)
        .ok_or_else(|| anyhow!("malformed MCP function name (missing __): {function_name}"))?;

    let ctx = ctx.ok_or_else(|| anyhow!("MCP function called but no mcp_context present"))?;
    let server = ctx
        .servers
        .iter()
        .find(|s| sanitize_server_name(&s.name) == server_name_safe)
        .ok_or_else(|| anyhow!("MCP server `{server_name_safe}` not in current mcp_context"))?;

    // args: JSON object → prost_types::Struct
    let parsed: Value = if arguments_json.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(arguments_json)?
    };
    let obj = parsed
        .as_object()
        .ok_or_else(|| anyhow!("MCP tool args must be a JSON object"))?;
    let args_struct = json_object_to_prost_struct(obj);

    Ok(api::message::tool_call::Tool::CallMcpTool(
        api::message::tool_call::CallMcpTool {
            name: tool_name.to_owned(),
            args: Some(args_struct),
            server_id: server.id.clone(),
        },
    ))
}

fn json_object_to_prost_struct(obj: &Map<String, Value>) -> prost_types::Struct {
    let mut fields = std::collections::BTreeMap::new();
    for (k, v) in obj {
        fields.insert(k.clone(), json_value_to_prost(v));
    }
    prost_types::Struct {
        fields: fields.into_iter().collect(),
    }
}

fn json_value_to_prost(v: &Value) -> prost_types::Value {
    let kind = match v {
        Value::Null => ProstKind::NullValue(0),
        Value::Bool(b) => ProstKind::BoolValue(*b),
        Value::Number(n) => ProstKind::NumberValue(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => ProstKind::StringValue(s.clone()),
        Value::Array(arr) => ProstKind::ListValue(prost_types::ListValue {
            values: arr.iter().map(json_value_to_prost).collect(),
        }),
        Value::Object(o) => ProstKind::StructValue(json_object_to_prost_struct(o)),
    };
    prost_types::Value { kind: Some(kind) }
}

#[derive(Debug, serde::Deserialize)]
struct ReadResourceArgs {
    uri: String,
    #[serde(default)]
    server: Option<String>,
}

fn parse_read_resource(
    arguments_json: &str,
    ctx: Option<&MCPContext>,
) -> Result<api::message::tool_call::Tool> {
    let parsed: ReadResourceArgs = serde_json::from_str(arguments_json)?;
    // 解析 server_id:
    // 1) 若给了 server 名,按 sanitize 后匹配
    // 2) 否则在所有 server 中找含此 uri 的 resource(命中第一个)
    // 3) 兜底 server_id 为空(server 端按 uri 自己定位)
    let server_id = if let Some(ctx) = ctx {
        match parsed.server.as_deref() {
            Some(name) => ctx
                .servers
                .iter()
                .find(|s| sanitize_server_name(&s.name) == sanitize_server_name(name))
                .map(|s| s.id.clone())
                .unwrap_or_default(),
            None => ctx
                .servers
                .iter()
                .find(|s| s.resources.iter().any(|r| r.uri.as_str() == parsed.uri.as_str()))
                .map(|s| s.id.clone())
                .unwrap_or_default(),
        }
    } else {
        String::new()
    };
    Ok(api::message::tool_call::Tool::ReadMcpResource(
        api::message::tool_call::ReadMcpResource {
            uri: parsed.uri,
            server_id,
        },
    ))
}

/// 给历史里的 `Tool::ReadMcpResource` 序列化为 OpenAI tool_calls 中的 (name, args_json)。
pub fn serialize_outgoing_read_resource(
    tc: &api::message::tool_call::ReadMcpResource,
    ctx: Option<&MCPContext>,
) -> (String, String) {
    let server_name = ctx
        .and_then(|c| c.servers.iter().find(|s| s.id == tc.server_id))
        .map(|s| s.name.clone());
    let mut args = json!({ "uri": tc.uri });
    if let Some(name) = server_name {
        args["server"] = json!(name);
    }
    (READ_RESOURCE_NAME.to_owned(), args.to_string())
}

/// 给历史里的 `Tool::CallMcpTool` 序列化为 OpenAI tool_calls 中的 (name, args_json) 对。
pub fn serialize_outgoing_call(
    tc: &api::message::tool_call::CallMcpTool,
    ctx: Option<&MCPContext>,
) -> (String, String) {
    // 找回对应 server.name(若 mcp_context 已变,fallback 到 server_id)
    let server_name = ctx
        .and_then(|c| c.servers.iter().find(|s| s.id == tc.server_id))
        .map(|s| sanitize_server_name(&s.name))
        .unwrap_or_else(|| tc.server_id.clone());
    let name = format!("{PREFIX}{server_name}{SEP}{}", tc.name);
    // args (Option<prost_types::Struct>) → serde_json
    let args_value = tc
        .args
        .as_ref()
        .map(|s| Value::Object(prost_struct_to_json(s)))
        .unwrap_or_else(|| json!({}));
    (name, args_value.to_string())
}

fn prost_struct_to_json(s: &prost_types::Struct) -> Map<String, Value> {
    let mut out = Map::new();
    for (k, v) in &s.fields {
        out.insert(k.clone(), prost_value_to_json(v));
    }
    out
}

fn prost_value_to_json(v: &prost_types::Value) -> Value {
    match &v.kind {
        Some(ProstKind::NullValue(_)) | None => Value::Null,
        Some(ProstKind::BoolValue(b)) => Value::Bool(*b),
        Some(ProstKind::NumberValue(n)) => serde_json::Number::from_f64(*n)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        Some(ProstKind::StringValue(s)) => Value::String(s.clone()),
        Some(ProstKind::ListValue(l)) => {
            Value::Array(l.values.iter().map(prost_value_to_json).collect())
        }
        Some(ProstKind::StructValue(o)) => Value::Object(prost_struct_to_json(o)),
    }
}

/// 序列化 ToolCallResult 中 CallMcpTool 或 ReadMcpResource 的 result 给上游模型。
pub fn serialize_result(result: &api::message::tool_call_result::Result) -> Option<Value> {
    use api::call_mcp_tool_result::Result as McpR;
    use api::message::tool_call_result::Result as R;
    use api::read_mcp_resource_result::Result as ReadR;

    if let R::CallMcpTool(r) = result {
        let value = match &r.result {
            Some(McpR::Success(s)) => json!({
                "status": "ok",
                // s.content 是 Vec<rmcp Content> 类型,此处简化为 debug 字符串。
                "content": format!("{:?}", s),
            }),
            Some(McpR::Error(e)) => json!({ "status": "error", "message": e.message }),
            None => json!({ "status": "cancelled" }),
        };
        return Some(value);
    }
    if let R::ReadMcpResource(r) = result {
        let value = match &r.result {
            Some(ReadR::Success(s)) => json!({
                "status": "ok",
                // contents 是 Vec<rmcp ResourceContents>,debug 序列化保留所有信息
                "contents": format!("{:?}", s.contents),
            }),
            Some(ReadR::Error(e)) => json!({ "status": "error", "message": e.message }),
            None => json!({ "status": "cancelled" }),
        };
        return Some(value);
    }
    None
}
