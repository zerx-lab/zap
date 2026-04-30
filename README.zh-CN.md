<div align="center">

# OpenWarp

**把任意 AI 模型,装进你的终端**

OpenWarp 是基于 [Warp](https://github.com/warpdotdev/warp) 开源代码的社区分支 ——
在保留 Warp 全部终端体验的同时,加入 **BYOP(Bring Your Own Provider)** 能力,
让 AI 层完全开放:密钥、模型、提示词全部由你掌控。

[English](./README.md) · [文档](https://docs.warp.dev) · [上游 Warp](https://www.warp.dev)

> ⚠️ 当前项目处于早期开发,尚未发布正式版本。与 Warp 官方公司**无任何附属关系**。

</div>

---

## ✨ 为什么选 OpenWarp

Warp 官方客户端依赖云端 Agent 服务,AI 能力默认必须经过 Warp 后端。
OpenWarp 把这层完全打开:

| 维度 | Warp 上游 | OpenWarp |
| --- | --- | --- |
| AI 提供商 | Warp 官方网关 | **任意 OpenAI 兼容端点** |
| 凭证存储 | 云端账户 | **本地配置文件,零外发** |
| 系统提示词 | 后端组装,客户端不可见 | **minijinja 模板,完全可改** |
| 界面语言 | 英文 | **原生中英文,可扩展** |
| Cloud Agent / Computer Use | 默认开启 | **默认关闭,纯本地** |
| 体验(Blocks / Workflows / 键位) | ✓ | ✓ 完整保留 |
| 协议 | AGPL-3.0 / MIT 双许可 | 与上游一致 |

## 🚀 三步,把 AI 完全握在自己手里

**01 · 接入任意提供商**
设置中粘贴 Base URL 与 API Key —— 任何 OpenAI Chat Completions 兼容端点都即插即用,
凭证仅保存在本地。

**02 · 编写动态提示词**
基于 minijinja 模板引擎,根据当前工作目录、语言、角色实时渲染系统提示词。

**03 · 在终端立即启用**
一键切换模型、对话、命令补全 —— 体验与 Warp 一致,但完全由你掌控。

## 🧩 已验证的提供商

| 提供商 | Base URL | 备注 |
| --- | --- | --- |
| **OpenAI** | `https://api.openai.com/v1` | 官方协议 |
| **Anthropic** | 通过 genai 原生协议 | Claude 4.x 全系列 |
| **DeepSeek** | `https://api.deepseek.com/v1` | thinking + tool calling |
| **Gemini** | 通过 genai 原生协议 | Google AI Studio |
| **Ollama** | `http://localhost:11434/v1` | 本地推理,无需 Key |
| **OpenRouter** | `https://openrouter.ai/api/v1` | 聚合网关 |
| **Qwen / Groq / Together / LM Studio / 任意 OpenAI 兼容代理** | — | 配置即用 |

## 🔧 核心特性

- **BYOP 自定义提供商** — 5 种原生协议(OpenAI / OpenAIResp / Anthropic / Gemini / Ollama / DeepSeek)显式绑定,基于 [genai](https://github.com/jeremychone/rust-genai) 0.6
- **流式打字机** — SSE 增量渲染,与 Warp 自家路径一致的 Block 体验
- **18 个本地工具** — shell / read / edit / search / mcp / drive 文档 / skills / ask 等,全部本地执行
- **系统提示词模板** — 移植 opencode 的 8 份模型族 prompt(default / anthropic / gpt / beast / gemini / kimi / codex / trinity)
- **models.dev 数据源** — Providers 子页搜索框快速添加,内置数千模型元数据
- **隐私优先** — 关闭 Cloud Agent / Computer Use / Referral,默认不上传遥测
- **保留 Warp 体验** — 持续合并上游,Blocks / Workflows / AI 命令 / Keymaps / 主题完整保留
- **多语言界面** — 简体中文 + English,后续社区可扩展

## 📦 本地构建

```bash
git clone https://github.com/zerx-lab/openwarp
cd openwarp
./script/bootstrap   # 平台依赖
./script/run         # 构建并运行
./script/presubmit   # fmt / clippy / tests
```

详见 [WARP.md](WARP.md) 获取完整工程指南(代码风格、测试、平台说明)。

## 📜 协议

与 Warp 上游一致:

- `warpui_core` / `warpui` crates 采用 [MIT](LICENSE-MIT)
- 其余代码采用 [AGPL-3.0](LICENSE-AGPL)

## 🤝 贡献

欢迎社区贡献。完整流程见 [CONTRIBUTING.md](CONTRIBUTING.md)。

提交 Issue 前,请先 [搜索现有 Issues](https://github.com/zerx-lab/openwarp/issues)。
安全漏洞请按 [CONTRIBUTING.md#reporting-security-issues](CONTRIBUTING.md#reporting-security-issues) 私下上报。

## 🙏 致谢

OpenWarp 站在 Warp 团队和众多开源项目的肩膀上:

[Warp](https://github.com/warpdotdev/warp) · [genai](https://github.com/jeremychone/rust-genai) · [opencode](https://github.com/opencode-ai/opencode) · [models.dev](https://models.dev) · [Tokio](https://github.com/tokio-rs/tokio) · [NuShell](https://github.com/nushell/nushell) · [Alacritty](https://github.com/alacritty/alacritty) · [Hyper](https://github.com/hyperium/hyper) · [minijinja](https://github.com/mitsuhiko/minijinja)
