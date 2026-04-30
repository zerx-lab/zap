<div align="center">

# OpenWarp

**Bring any AI model into your terminal**

OpenWarp is a community fork of [Warp](https://github.com/warpdotdev/warp) that opens up the AI layer.
Keep the full Warp terminal experience — blocks, workflows, keymaps — while plugging in
**any OpenAI-compatible provider**, customizing system prompts with minijinja templates,
and keeping every credential local.

[简体中文](./README.zh-CN.md) · [Docs](https://docs.warp.dev) · [Upstream Warp](https://www.warp.dev)

> ⚠️ Early development. No official release yet. **Not affiliated with Warp, Inc.**

</div>

---

## ✨ Why OpenWarp

The official Warp client routes AI through Warp's cloud agent service.
OpenWarp opens that layer entirely:

| | Upstream Warp | OpenWarp |
| --- | --- | --- |
| AI provider | Warp gateway | **Any OpenAI-compatible endpoint** |
| Credentials | Cloud account | **Local config file, never leaves device** |
| System prompt | Server-assembled, opaque | **minijinja templates, fully editable** |
| UI language | English | **Native English + Simplified Chinese, extensible** |
| Cloud Agent / Computer Use | On by default | **Off by default, fully local** |
| Blocks / Workflows / Keymaps | ✓ | ✓ Fully preserved |
| License | AGPL-3.0 / MIT dual | Same as upstream |

## 🚀 Three steps to take AI fully into your own hands

**01 · Plug in any provider**
Paste a Base URL and API key in settings — any OpenAI Chat Completions–compatible
endpoint works out of the box. Credentials are stored locally only.

**02 · Author dynamic prompts**
A minijinja-powered template engine renders the system prompt in real time
based on the current working directory, language, and role.

**03 · Use it in the terminal immediately**
Switch models, conversations, and command suggestions with one click —
the experience is identical to Warp, but every layer is yours.

## 🧩 Verified providers

| Provider | Base URL | Notes |
| --- | --- | --- |
| **OpenAI** | `https://api.openai.com/v1` | Native protocol |
| **Anthropic** | via genai native | Claude 4.x family |
| **DeepSeek** | `https://api.deepseek.com/v1` | thinking + tool calling |
| **Gemini** | via genai native | Google AI Studio |
| **Ollama** | `http://localhost:11434/v1` | Local inference, no key |
| **OpenRouter** | `https://openrouter.ai/api/v1` | Aggregator gateway |
| **Qwen / Groq / Together / LM Studio / any OpenAI-compatible proxy** | — | Configure and go |

## 🔧 Core features

- **BYOP custom providers** — five native protocols (OpenAI / OpenAIResp / Anthropic / Gemini / Ollama / DeepSeek) explicitly bound on top of [genai](https://github.com/jeremychone/rust-genai) 0.6
- **SSE streaming** — incremental block rendering identical to Warp's first-party path
- **18 local tools** — shell / read / edit / search / mcp / drive docs / skills / ask, all executed locally
- **System prompt templates** — eight model-family prompts ported from opencode (default / anthropic / gpt / beast / gemini / kimi / codex / trinity)
- **models.dev integration** — searchable Providers subpage with thousands of preloaded model entries
- **Privacy first** — Cloud Agent / Computer Use / Referral disabled by default; no telemetry
- **Warp experience preserved** — continuously merged with upstream; Blocks, Workflows, AI commands, Keymaps and themes all kept
- **Localized UI** — Simplified Chinese + English, community-extensible

## 📦 Build from source

```bash
git clone https://github.com/zerx-lab/openwarp
cd openwarp
./script/bootstrap   # platform-specific deps
./script/run         # build & run
./script/presubmit   # fmt / clippy / tests
```

See [WARP.md](WARP.md) for the full engineering guide (style, testing, platform notes).

## 📜 License

Same as upstream Warp:

- `warpui_core` / `warpui` crates — [MIT](LICENSE-MIT)
- Everything else — [AGPL-3.0](LICENSE-AGPL)

## 🤝 Contributing

Community contributions welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for the full flow.

Before filing, please [search existing issues](https://github.com/zerx-lab/openwarp/issues).
Security vulnerabilities should be reported privately per
[CONTRIBUTING.md#reporting-security-issues](CONTRIBUTING.md#reporting-security-issues).

## 🙏 Acknowledgements

OpenWarp stands on the shoulders of the Warp team and many open-source projects:

[Warp](https://github.com/warpdotdev/warp) · [genai](https://github.com/jeremychone/rust-genai) · [opencode](https://github.com/opencode-ai/opencode) · [models.dev](https://models.dev) · [Tokio](https://github.com/tokio-rs/tokio) · [NuShell](https://github.com/nushell/nushell) · [Alacritty](https://github.com/alacritty/alacritty) · [Hyper](https://github.com/hyperium/hyper) · [minijinja](https://github.com/mitsuhiko/minijinja)
