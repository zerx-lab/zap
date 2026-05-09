<div align="center">

# OpenWarp

**A fully decentralized terminal — your AI, your agents, your keys, your machine.**

OpenWarp is a community fork of [Warp](https://github.com/warpdotdev/warp) that
**strips Warp's mandatory cloud dependency** while preserving the full Warp
terminal experience. It opens up the AI provider layer, lets you plug in any
third-party CLI agent, ships a built-in SSH host manager, and fixes a number of
upstream rendering issues — all while keeping every credential, conversation,
and agent history on your own machine.

[简体中文](./README.zh-CN.md) · [Upstream Warp](https://www.warp.dev) · [Upstream sync notes](docs/openwarp-upstream-sync.md)

> Early development. No official release yet. **Not affiliated with Warp, Inc.**

</div>

---

## Why OpenWarp

Upstream Warp ties AI, accounts, sync, and agent history to Warp's cloud.
OpenWarp opens that layer entirely and **adds capabilities the upstream client
does not provide**:

| | Upstream Warp | OpenWarp |
| --- | --- | --- |
| Cloud dependency | Hard dependency on Warp backend (auth / Drive / history / Agent) | **Fully decentralized, no mandatory cloud calls** |
| AI provider | Warp gateway only | **Any OpenAI-compatible endpoint + 6 native protocols** |
| Third-party agents | Built-in Warp Agent only | **Any CLI agent — DeepSeek-TUI / Codex / Claude Code wired in** |
| SSH management | Not built-in | **Built-in SSH host manager (connect / config / tmux)** |
| Markdown rendering | Upstream baseline | **Tuned MD pipeline — code blocks, tables, mixed CJK** |
| Font rendering | Upstream cosmic_text default | **CJK soft-wrap caret + bold subpixel fixes** |
| Credentials | Cloud account | **Local config file, never leaves device** |
| System prompt | Server-assembled, opaque | **minijinja templates, fully editable** |
| UI language | English | **Native English + Simplified Chinese, extensible** |
| Cloud Agent / Computer Use | On by default | **Off by default (and being physically removed)** |
| Blocks / Workflows / Keymaps | Kept | Fully preserved, continuously synced |
| License | AGPL-3.0 / MIT dual | Same as upstream |

## Things upstream Warp does NOT support, but OpenWarp does

These are net-new capabilities OpenWarp adds on top of the fork:

- **SSH host manager** — connect, configure and manage SSH hosts and sessions
  directly inside the terminal (with tmux integration). No external switcher needed.
- **Third-party CLI agents** — bring any CLI agent into the Warp Block model.
  First-class adapters for:
  - **DeepSeek-TUI** (completion notifications, text-notification mapping,
    input-restore plumbing all wired up)
  - **Codex CLI**, **Claude Code**, and other mainstream CLI agents
  - Unified routing through OSC9 / OSC777 into Warp's notification center
- **BYOP across many providers** — 6 native protocols (OpenAI / OpenAIResp /
  Anthropic / Gemini / Ollama / DeepSeek) explicitly bound; any OpenAI-compatible
  proxy works out of the box. Credentials stay local.
- **Fully decentralized** — no Warp account, no forced login, no cloud Drive /
  Notebook sync, no cloud agent history. Cloud code paths are being physically
  removed in stages.
- **Markdown rendering improvements** — better stability for code blocks,
  tables, lists and mixed Chinese/English text inside AI blocks.
- **Font rendering algorithm fixes** — CJK soft-wrap caret offset, bold-on-small
  Chinese characters, and other long-standing upstream rendering papercuts.

## Three steps to take your terminal fully into your own hands

**01 · Plug in any provider**
Paste a Base URL and API key in settings — any OpenAI Chat Completions–compatible
endpoint works out of the box. Credentials are stored locally only.

**02 · Author dynamic prompts**
A minijinja-powered template engine renders the system prompt in real time
based on the current working directory, language, and role.

**03 · Use it immediately**
Switch models, conversations, command suggestions, and third-party agents with
one click — the experience is identical to Warp, but every layer is yours.

## Verified AI providers

| Provider | Base URL | Notes |
| --- | --- | --- |
| **OpenAI** | `https://api.openai.com/v1` | Native protocol |
| **Anthropic** | via genai native | Claude 4.x family |
| **DeepSeek** | `https://api.deepseek.com/v1` | thinking + tool calling |
| **Gemini** | via genai native | Google AI Studio |
| **Ollama** | `http://localhost:11434/v1` | Local inference, no key |
| **OpenRouter** | `https://openrouter.ai/api/v1` | Aggregator gateway |
| **Qwen / Groq / Together / LM Studio / any OpenAI-compatible proxy** | — | Configure and go |

## Core features

- **BYOP custom providers** — 6 native protocols explicitly bound on top of
  [genai](https://github.com/jeremychone/rust-genai) 0.6
- **Third-party CLI agents** — DeepSeek-TUI / Codex CLI / Claude Code routed
  through OSC9 into Blocks and the notification center
- **SSH host manager** — manage SSH hosts and sessions inside the terminal,
  with tmux integration
- **SSE streaming** — incremental block rendering identical to Warp's first-party path
- **18 local tools** — shell / read / edit / search / mcp / drive docs / skills / ask,
  all executed locally
- **System prompt templates** — eight model-family prompts ported from opencode
  (default / anthropic / gpt / beast / gemini / kimi / codex / trinity)
- **models.dev integration** — searchable Providers subpage with thousands of
  preloaded model entries
- **Rendering improvements** — tuned Markdown pipeline + CJK soft-wrap / bold fixes
- **Privacy first** — Cloud Agent / Computer Use / Referral / telemetry all
  disabled by default
- **Warp experience preserved** — continuously merged with upstream; Blocks,
  Workflows, AI commands, Keymaps and themes all kept
- **Localized UI** — Simplified Chinese + English, community-extensible

## What we are aiming for

OpenWarp wants to be the kind of terminal that:

1. **Runs fully without any centralized service** — no account, no forced login,
   no feature that "only works when the cloud is reachable".
2. **Treats AI and agents as an open ecosystem**, not a single vendor — every
   mainstream LLM provider and CLI agent is a first-class citizen.
3. **Makes remote work native** — SSH / tmux / remote sessions are built-in,
   not bolted on.
4. **Earns the right to be used all day** — mixed CJK, Markdown, code blocks
   and font rendering should never be the weak link.
5. **Stays in sync with upstream Warp** — benefit from Warp's engineering work
   while keeping fork-level autonomy on direction.

If you share these goals, come help us finish it.

## Build from source

```bash
git clone https://github.com/zerx-lab/openwarp
cd openwarp
./script/bootstrap   # platform-specific deps
./script/run         # build & run
./script/presubmit   # fmt / clippy / tests
```

If you prefer raw `cargo`, **always target the OSS binary explicitly**:

```bash
cargo build --release --bin warp-oss
cargo run   --release --bin warp-oss
```

> Do not run `cargo build --release` / `cargo run --release --bin {warp,stable,dev,preview}`
> without a filter — those entry points (`local.rs` / `stable.rs` / `dev.rs` / `preview.rs`) load
> their channel config through Warp's private `warp-channel-config` binary, which lives in a
> closed-source repo. Compilation succeeds, but the resulting executables panic at startup
> asking you to run `./script/install_channel_config`. That script clones an SSH repo only
> Warp employees can access. OpenWarp users only need the `warp-oss` binary.

See [WARP.md](WARP.md) for the full engineering guide (style, testing, platform notes).

## License

Same as upstream Warp:

- `warpui_core` / `warpui` crates — [MIT](LICENSE-MIT)
- Everything else — [AGPL-3.0](LICENSE-AGPL)

## Branches & upstream sync

`zerx-lab/warp` keeps two long-lived branches:

| Branch | Tracks | Purpose |
| --- | --- | --- |
| `main` | `zerx-lab/warp:main` (default) | OpenWarp's main development line. **All PRs target this.** |
| `warp-upstream` | `warpdotdev/warp:master` | Pristine mirror of upstream Warp, used to pull in new commits. **No fork-local changes.** |

**For contributors**

Open PRs against **`main`**. Never against `warp-upstream`.

**For maintainers (write access)**

**Do not click the "Sync fork" button** on `main` in the GitHub web UI. It would merge the entire upstream history straight into OpenWarp's main line and trigger large-scale conflicts. Pull upstream changes through the mirror branch, following the blacklist and flow in [`docs/openwarp-upstream-sync.md`](docs/openwarp-upstream-sync.md):

```bash
# one-time setup
git remote add upstream https://github.com/warpdotdev/warp.git

# refresh the mirror
git checkout warp-upstream
git pull                          # fast-forwards from upstream/master
git push origin warp-upstream

# bring selected commits into main
git checkout main
git cherry-pick <sha>             # or merge warp-upstream when a full sync makes sense
```

## Featured Partner

<a href="https://github.com/Hmbown/DeepSeek-TUI">
  <img src="assets/DeepSeek-TUI.png" alt="DeepSeek-TUI" width="96" align="left" />
</a>

**[DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI)** — a terminal UI for the
DeepSeek model family. OpenWarp ships first-class integration: completion
notifications, OSC9 text-notification mapping, and input-restore plumbing are
all wired up so DeepSeek-TUI runs as a native Block inside OpenWarp.

Launch it with `deepseek` from any OpenWarp terminal — Block lifecycle, footer
status and notification center all work out of the box.

<br clear="left" />

> **Windows note** — DeepSeek-TUI's `[notifications].method` defaults to `auto`,
> which resolves to `Off` on Windows for any `TERM_PROGRAM` outside its
> built-in allowlist (iTerm.app / Ghostty / WezTerm). OpenWarp identifies as
> `WarpTerminal`, so to receive turn-completion notifications inside OpenWarp
> on Windows, add the following to `~/.deepseek/config.toml`:
>
> ```toml
> [notifications]
> method = "osc9"
>
> [tui]
> notification_condition = "always"  # optional: notify on every turn
> ```

If you maintain a CLI agent or a terminal-adjacent tool and want similar
first-class integration, open an issue — we are happy to wire more partners in.

## Contributing

Community contributions welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for the full flow.

Before filing, please [search existing issues](https://github.com/zerx-lab/warp/issues).
Security vulnerabilities should be reported privately per
[CONTRIBUTING.md#reporting-security-issues](CONTRIBUTING.md#reporting-security-issues).

## Acknowledgements

OpenWarp stands on the shoulders of the Warp team and many open-source projects:

[Warp](https://github.com/warpdotdev/warp) · [genai](https://github.com/jeremychone/rust-genai) · [opencode](https://github.com/opencode-ai/opencode) · [models.dev](https://models.dev) · [DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI) · [Codex CLI](https://github.com/openai/codex) · [Tokio](https://github.com/tokio-rs/tokio) · [NuShell](https://github.com/nushell/nushell) · [Alacritty](https://github.com/alacritty/alacritty) · [Hyper](https://github.com/hyperium/hyper) · [minijinja](https://github.com/mitsuhiko/minijinja) · [cosmic-text](https://github.com/pop-os/cosmic-text)
