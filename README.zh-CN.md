<div align="center">

# OpenWarp

**完全去中心化的终端,把 AI 与 Agent 的控制权全部交还给你**

OpenWarp 是基于 [Warp](https://github.com/warpdotdev/warp) 开源代码的社区分支 ——
保留 Warp 全部终端体验,同时**剥离对 Warp 云端的依赖**,
开放 AI 接入层、第三方 Agent 接入、SSH 管理与渲染细节,
让密钥、模型、提示词、Agent、会话历史 **全部留在本地**。

[English](./README.md) · [上游 Warp](https://www.warp.dev) · [上游同步说明](docs/openwarp-upstream-sync.md)

> 当前项目处于早期开发,尚未发布正式版本。与 Warp 官方公司**无任何附属关系**。

</div>

---

## 为什么选 OpenWarp

Warp 官方客户端的 AI、账号、同步、Agent 历史全部依赖 Warp 云端服务。
OpenWarp 把这层完全打开,并在上游基础上**追加**了一批上游不支持的能力:

| 维度 | Warp 上游 | OpenWarp |
| --- | --- | --- |
| 云端依赖 | 强依赖 Warp 后端(账号 / Drive / 历史 / Agent) | **完全去中心化,无任何强制云端调用** |
| AI 提供商 | 仅 Warp 官方网关 | **任意 OpenAI 兼容端点 + 6 种原生协议** |
| 第三方 Agent | 仅内置 Warp Agent | **可接入任意 CLI Agent,内置 DeepSeek-TUI / Codex / Claude Code 等** |
| SSH 管理 | 无内置 SSH 管理器 | **内置 SSH 主机管理器(连接 / 配置 / tmux 集成)** |
| Markdown 渲染 | 上游基础渲染 | **优化 MD 渲染管线,代码块 / 表格 / 中文混排更稳** |
| 字体渲染 | 上游 cosmic_text 默认路径 | **CJK 软换行 caret / 中文加粗子像素优化** |
| 凭证存储 | 云端账户 | **本地配置文件,零外发** |
| 系统提示词 | 后端组装,客户端不可见 | **minijinja 模板,完全可改** |
| 界面语言 | 英文 | **原生中英文,可扩展** |
| Cloud Agent / Computer Use | 默认开启 | **默认关闭(并在持续物理剥离中)** |
| 体验(Blocks / Workflows / 键位) | 保留 | 完整保留并持续合并上游 |
| 协议 | AGPL-3.0 / MIT 双许可 | 与上游一致 |

## 上游不支持、OpenWarp 独有的能力

以下能力上游 Warp **不提供**,是 OpenWarp 在 fork 之上新增的差异化功能:

- **SSH 主机管理器** —— 内置 SSH 连接、配置与会话管理(支持 tmux 集成),不再依赖外部工具切换远端。
- **第三方 CLI Agent 接入** —— 可把任意 CLI Agent 接入终端 Block 体系,内置适配:
  - **DeepSeek-TUI**(完成通知 / 文本通知映射 / 输入框恢复链路均已打通)
  - **Codex CLI**、**Claude Code** 等主流 CLI Agent
  - 通过 OSC9 / OSC777 通知机制统一进入 Warp 通知中心
- **多 AI 提供商 BYOP** —— 6 种原生协议(OpenAI / OpenAIResp / Anthropic / Gemini / Ollama / DeepSeek)显式绑定,任意 OpenAI 兼容代理即插即用,凭证仅存本地。
- **完全去中心化** —— 无 Warp 账号、无强制登录、无云端 Drive / Notebook 同步、无 Agent 云端历史;按计划逐步物理删除 Warp 云端代码路径。
- **Markdown 渲染优化** —— 改进 AI Block 中代码块、表格、列表与中英文混排的渲染稳定性。
- **字体渲染算法优化** —— 修复 CJK 软换行后 caret 索引偏移、设置页中文小字号加粗发糊等上游遗留问题。

## 三步,把终端完全握在自己手里

**01 · 接入任意提供商**
设置中粘贴 Base URL 与 API Key —— 任何 OpenAI Chat Completions 兼容端点都即插即用,
凭证仅保存在本地。

**02 · 编写动态提示词**
基于 minijinja 模板引擎,根据当前工作目录、语言、角色实时渲染系统提示词。

**03 · 在终端立即启用**
一键切换模型、对话、命令补全、第三方 Agent —— 体验与 Warp 一致,但每一层都由你掌控。

## 已验证的 AI 提供商

| 提供商 | Base URL | 备注 |
| --- | --- | --- |
| **OpenAI** | `https://api.openai.com/v1` | 官方协议 |
| **Anthropic** | 通过 genai 原生协议 | Claude 4.x 全系列 |
| **DeepSeek** | `https://api.deepseek.com/v1` | thinking + tool calling |
| **Gemini** | 通过 genai 原生协议 | Google AI Studio |
| **Ollama** | `http://localhost:11434/v1` | 本地推理,无需 Key |
| **OpenRouter** | `https://openrouter.ai/api/v1` | 聚合网关 |
| **Qwen / Groq / Together / LM Studio / 任意 OpenAI 兼容代理** | — | 配置即用 |

## 核心特性

- **BYOP 自定义提供商** — 6 种原生协议显式绑定,基于 [genai](https://github.com/jeremychone/rust-genai) 0.6
- **第三方 Agent 接入** — DeepSeek-TUI / Codex CLI / Claude Code 等通过 OSC9 通知统一进入 Block 与通知中心
- **SSH 主机管理器** — 终端内直接管理 SSH 主机与会话,集成 tmux
- **流式打字机** — SSE 增量渲染,与 Warp 自家路径一致的 Block 体验
- **18 个本地工具** — shell / read / edit / search / mcp / drive 文档 / skills / ask 等,全部本地执行
- **系统提示词模板** — 移植 opencode 的 8 份模型族 prompt(default / anthropic / gpt / beast / gemini / kimi / codex / trinity)
- **models.dev 数据源** — Providers 子页搜索框快速添加,内置数千模型元数据
- **渲染优化** — Markdown 渲染管线 + CJK 字体软换行 / 加粗算法修复
- **隐私优先** — Cloud Agent / Computer Use / Referral / 遥测全部默认关闭
- **保留 Warp 体验** — 持续合并上游,Blocks / Workflows / AI 命令 / Keymaps / 主题完整保留
- **多语言界面** — 简体中文 + English,后续社区可扩展

## 我们的期望

OpenWarp 想成为这样一个终端:

1. **不依赖任何中心化服务也能完整运行** —— 没有账号、没有强制登录、没有"必须连云端才能用"的功能。
2. **AI 与 Agent 是开放生态而非单一供应商** —— 主流 LLM Provider 与 CLI Agent 都能平等接入。
3. **远端工作流原生融入终端** —— SSH / tmux / 远端会话作为一等公民,而不是外挂。
4. **渲染细节配得上长时间使用** —— 中英混排、Markdown、代码块、字体渲染都不应是体验的短板。
5. **持续与上游 Warp 同步** —— 享受 Warp 团队的工程红利,但保留 fork 的方向自主权。

如果你认同这些目标,欢迎一起把它做完。

## 本地构建

```bash
git clone https://github.com/zerx-lab/openwarp
cd openwarp
./script/bootstrap   # 平台依赖
./script/run         # 构建并运行
./script/presubmit   # fmt / clippy / tests
```

若要直接用 `cargo`,**必须显式指定 OSS 二进制**:

```bash
cargo build --release --bin warp-oss
cargo run   --release --bin warp-oss
```

> 不要不带过滤地跑 `cargo build --release`,也不要 `--bin {warp,stable,dev,preview}` ——
> 这些入口(`local.rs` / `stable.rs` / `dev.rs` / `preview.rs`)通过 Warp 私有的
> `warp-channel-config` 二进制加载 channel 配置,而该二进制位于闭源私仓。编译能过,
> 但运行时会 panic 提示 `./script/install_channel_config`,而那个脚本会去 clone 只有 Warp
> 员工才有 SSH 权限的私仓。OpenWarp 用户只需要 `warp-oss` 这一个 bin。

详见 [WARP.md](WARP.md) 获取完整工程指南(代码风格、测试、平台说明)。

## 协议

与 Warp 上游一致:

- `warpui_core` / `warpui` crates 采用 [MIT](LICENSE-MIT)
- 其余代码采用 [AGPL-3.0](LICENSE-AGPL)

## 分支与上游同步

`zerx-lab/warp` 维护两条长期分支:

| 分支 | 跟踪 | 用途 |
| --- | --- | --- |
| `main` | `zerx-lab/warp:main`(默认分支) | OpenWarp 主开发线,**所有 PR 都提到这条分支**。 |
| `warp-upstream` | `warpdotdev/warp:master` | 上游 Warp 的纯净镜像,用于拉取上游更新,**不在此分支做 fork 自有改动**。 |

**贡献者须知**

PR 请提到 **`main`**,不要提到 `warp-upstream`。

**维护者须知(具有写权限)**

**不要在 GitHub 网页上点 `main` 分支的 "Sync fork" 按钮**。这会把上游整段历史直接合并进 OpenWarp 主线,造成大规模冲突。拉取上游更新请通过镜像分支,并参考 [`docs/openwarp-upstream-sync.md`](docs/openwarp-upstream-sync.md) 中的黑名单与流程:

```bash
# 一次性配置
git remote add upstream https://github.com/warpdotdev/warp.git

# 刷新镜像分支
git checkout warp-upstream
git pull                          # 从 upstream/master 快进
git push origin warp-upstream

# 把需要的上游 commit 引入 main
git checkout main
git cherry-pick <sha>             # 或在需要整体同步时 merge warp-upstream
```

## 合作伙伴

<a href="https://github.com/Hmbown/DeepSeek-TUI">
  <img src="assets/DeepSeek-TUI.png" alt="DeepSeek-TUI" width="96" align="left" />
</a>

**[DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI)** —— DeepSeek 模型家族的
终端 UI。OpenWarp 提供一流的接入支持:完成通知、OSC9 文本通知映射、输入框恢复
链路均已打通,DeepSeek-TUI 可作为原生 Block 在 OpenWarp 中运行。

在任意 OpenWarp 终端中执行 `deepseek` 即可启动,Block 生命周期、底部状态条与
通知中心全部开箱即用。

<br clear="left" />

> **Windows 用户注意** —— DeepSeek-TUI 的 `[notifications].method` 默认值是
> `auto`,在 Windows 上,只有当 `TERM_PROGRAM` 命中其内置白名单
> (iTerm.app / Ghostty / WezTerm)时才会走 OSC9,否则 fallback 为 `Off`。
> OpenWarp 将自身标记为 `WarpTerminal`,因此 Windows 下要让 DeepSeek-TUI 的
> 完成通知进入 OpenWarp 通知中心,需要在 `~/.deepseek/config.toml` 中显式指定:
>
> ```toml
> [notifications]
> method = "osc9"
>
> [tui]
> notification_condition = "always"  # 可选:每轮都通知,不受时长阈值限制
> ```

如果你在维护 CLI Agent 或与终端深度结合的工具,希望获得同等级别的接入支持,
欢迎开 issue —— 我们乐于继续接入更多合作伙伴。

## 贡献

欢迎社区贡献。完整流程见 [CONTRIBUTING.md](CONTRIBUTING.md)。

提交 Issue 前,请先 [搜索现有 Issues](https://github.com/zerx-lab/warp/issues)。
安全漏洞请按 [CONTRIBUTING.md#reporting-security-issues](CONTRIBUTING.md#reporting-security-issues) 私下上报。

## 致谢

OpenWarp 站在 Warp 团队和众多开源项目的肩膀上:

[Warp](https://github.com/warpdotdev/warp) · [genai](https://github.com/jeremychone/rust-genai) · [opencode](https://github.com/opencode-ai/opencode) · [models.dev](https://models.dev) · [DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI) · [Codex CLI](https://github.com/openai/codex) · [Tokio](https://github.com/tokio-rs/tokio) · [NuShell](https://github.com/nushell/nushell) · [Alacritty](https://github.com/alacritty/alacritty) · [Hyper](https://github.com/hyperium/hyper) · [minijinja](https://github.com/mitsuhiko/minijinja) · [cosmic-text](https://github.com/pop-os/cosmic-text)
