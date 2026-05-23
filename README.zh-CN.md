<div align="center">

<img src="assets/zap-logo.svg" alt="Zap" width="128" />

# Zap

[English](./README.md) · [日本語](./README.ja.md)

<sub><i>目前基于 <a href="https://github.com/warpdotdev/warp">Warp</a>,后续将独立演进。</i></sub>

</div>

Zap 是一个开放、本地优先的终端,带一等公民的 AI 与 Agent 体验。接入任意 AI 提供商、接入任意 CLI Agent、在终端内管理 SSH 主机 —— 密钥、历史与 Agent 状态默认留在本地。

## 相比官方 Warp 多出的功能

- **无强制云端** —— 不需要账号、登录、Drive 同步或云端 Agent 历史。
- **BYOP 自定义 AI 提供商** —— 任意 OpenAI 兼容端点,以及 OpenAI / Anthropic / Gemini / DeepSeek / Ollama 等原生协议,密钥仅存本地。
- **第三方 CLI Agent 接入** —— DeepSeek-TUI / Codex CLI / Claude Code / Google Antigravity(`agy`)接入 Block 与通知中心。
- **内置 SSH 主机管理器** —— 在终端内管理主机、配置与会话,集成 tmux。
- **可编辑系统提示词** —— 基于 minijinja 模板,客户端实时渲染。
- **渲染优化** —— Markdown 管线优化;CJK 软换行 caret 与加粗子像素修复。
- **多语言界面** —— 原生英文 / 简体中文 / 日语,社区可扩展。
- **隐私默认值** —— Cloud Agent / Computer Use / Referral / 遥测默认关闭。

## 从 OpenWarp 或 Warp 迁移过来

如果你在项目改名 Zap 之前就一直在用(那时还叫 **OpenWarp**),
或者你是从上游 **Warp** 切过来的,参见
[docs/migrate-from-warp.zh-CN.md](docs/migrate-from-warp.zh-CN.md) 把设置带过来。

## 后续计划

见 [docs/roadmap.zh-CN.md](docs/roadmap.zh-CN.md)。

## 鸣谢

- [Warp](https://github.com/warpdotdev/warp) —— Zap 所基于的上游终端。
- [DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI) —— 深度适配的 CLI Agent 合作伙伴。
