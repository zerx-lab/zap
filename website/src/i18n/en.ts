import type { Dict } from './zh-CN';

export const en: Dict = {
  meta: {
    title: 'OpenWarp — Unlock custom AI providers for Warp',
    description:
      'OpenWarp is the open enhancement layer for Warp. Plug in any OpenAI-compatible model, craft your own prompts, and own your terminal AI.',
  },
  nav: {
    how: 'How it works',
    features: 'Features',
    providers: 'Custom Providers',
    faq: 'FAQ',
    docs: 'Docs',
    github: 'GitHub',
  },
  hero: {
    badge: 'Community · Work in progress',
    title_1: 'Bring',
    title_em: 'any AI model',
    title_2: 'into your terminal',
    subtitle:
      'OpenWarp adds BYOP (Bring Your Own Provider) on top of Warp — wire up any OpenAI-compatible endpoint, customize models and system prompts, with first-class i18n.',
    cta_primary: 'View on GitHub',
    cta_secondary: 'Read docs',
    note: 'Early development — no public release yet',
    trust_lead: 'Compatible with major providers',
  },
  terminal: {
    tabs: ['zsh', 'openwarp', 'agent'],
    breadcrumb: '~/projects/openwarp',
    scenarios: [
      {
        model: 'deepseek-r1',
        tag: 'reasoning',
        user: 'Refactor this Rust trait',
        reply:
          'This trait carries three responsibilities. Splitting it into Reader and Writer decouples I/O and makes both sides easier to test.',
        suggest: [
          'pub trait Reader { fn read(&self) -> Bytes; }',
          'pub trait Writer { fn write(&mut self, b: Bytes); }',
        ],
      },
      {
        model: 'gpt-4o',
        tag: 'OpenAI',
        user: 'Generate a migration for users',
        reply:
          'Add last_login_at and a composite index on (email, last_login_at) to speed up session-aware lookups.',
        suggest: [
          'ALTER TABLE users ADD COLUMN last_login_at TIMESTAMPTZ;',
          'CREATE INDEX idx_users_email_login ON users(email, last_login_at);',
        ],
      },
      {
        model: 'qwen-2.5-coder',
        tag: 'local',
        user: 'Explain this unsafe block',
        reply:
          'It writes through a raw pointer, bypassing borrow checking. Safe only when layout and lifetime are statically guaranteed.',
        suggest: [
          '// safe only when layout & lifetime are provable',
          'unsafe { *ptr = value; }',
        ],
      },
    ],
    status: {
      tokens: 'tokens',
      latency: 'latency',
      local: 'local',
      streaming: 'streaming',
      ready: 'ready',
    },
  },
  stats: {
    items: [
      { value: '∞',    label: 'Pluggable providers' },
      { value: '2',    label: 'Built-in locales' },
      { value: 'AGPL', label: 'Open source license' },
      { value: '100%', label: 'Local credentials' },
    ],
  },
  how: {
    eyebrow: 'How it works',
    title: 'Three steps. Your terminal. Your AI.',
    subtitle:
      'Keep the full Warp experience — only the AI layer is opened up. Your keys, your models, your prompts.',
    steps: [
      {
        num: '01',
        title: 'Plug in any provider',
        desc: 'Paste a Base URL and API key in settings. Any OpenAI-compatible endpoint works, credentials stay on-device.',
      },
      {
        num: '02',
        title: 'Author dynamic prompts',
        desc: 'A minijinja template engine renders your system prompt from the cwd, locale, and role — context-aware on every call.',
      },
      {
        num: '03',
        title: 'Use it in the terminal',
        desc: 'Switch models, chat, and complete commands the same way you use Warp — except now you own the stack end-to-end.',
      },
    ],
  },
  providers: {
    eyebrow: 'Custom Providers',
    title: 'Configure once, every model unlocked',
    subtitle:
      'Any endpoint that speaks the OpenAI Chat Completions protocol works — OpenAI, Anthropic gateways, DeepSeek, Qwen, local Ollama, your call.',
    fields: {
      name: 'Provider name',
      base_url: 'Base URL',
      api_key: 'API key',
      model: 'Default model',
      prompt: 'System prompt template',
    },
    bullets: [
      '✓ OpenAI-compatible streaming',
      '✓ minijinja-powered prompt templates',
      '✓ Multi-account, one-click model switch',
      '✓ Stored locally, credentials never leave your device',
    ],
    tabs: [
      {
        id: 'deepseek',
        name: 'DeepSeek',
        tag: 'reasoning',
        baseUrl: 'https://api.deepseek.com/v1',
        apiKey: 'sk-•••••••••••••••••••••',
        model: 'deepseek-r1',
      },
      {
        id: 'ollama',
        name: 'Ollama',
        tag: 'local',
        baseUrl: 'http://localhost:11434/v1',
        apiKey: '— not required —',
        model: 'qwen2.5-coder:7b',
      },
      {
        id: 'openrouter',
        name: 'OpenRouter',
        tag: 'gateway',
        baseUrl: 'https://openrouter.ai/api/v1',
        apiKey: 'sk-or-•••••••••••••••••••',
        model: 'anthropic/claude-3.5-sonnet',
      },
    ],
  },
  features: {
    eyebrow: 'Core features',
    title: 'Everything you expected — opened up',
    items: [
      { title: 'BYOP custom providers', desc: 'Built-in OpenAI-compatible client. Mix any Base URL / API Key / Model.' },
      { title: 'Prompt templates',      desc: 'Powered by minijinja — render instructions dynamically from context.' },
      { title: 'Multilingual UI',       desc: 'First-class Chinese & English. Community can grow the locale set.' },
      { title: 'Privacy first',         desc: 'Cloud Agent / Computer Use disabled by default. Credentials stay local.' },
      { title: 'Warp-native experience',desc: 'Continuously merging upstream — keep blocks, AI commands, workflows, keymaps.' },
      { title: 'Open source',           desc: 'AGPL / MIT dual license, mirroring upstream Warp. All code is public.' },
    ],
  },
  faq: {
    eyebrow: 'FAQ',
    title: 'About OpenWarp',
    items: [
      {
        q: 'How is OpenWarp related to Warp the company?',
        a: "OpenWarp is a community fork of Warp's open-source code. It is not affiliated with Warp Inc. and follows the upstream AGPL / MIT dual license.",
      },
      {
        q: 'Will my API keys be uploaded?',
        a: 'No. Custom provider credentials live only in your local config file. OpenWarp talks directly to the Base URL you configure, with no relay.',
      },
      {
        q: 'Which providers are supported?',
        a: 'Anything that speaks OpenAI Chat Completions streaming: OpenAI, DeepSeek, Qwen, Groq, Together, local Ollama / LM Studio, plus most proxy gateways.',
      },
      {
        q: 'Will I keep getting upstream Warp updates?',
        a: 'Yes — we continuously merge from upstream Warp while layering BYOP and i18n on top.',
      },
    ],
  },
  cta: {
    title: 'Want early access?',
    desc: 'Clone the repo and build locally, or watch GitHub for the next release.',
    button: 'Go to GitHub',
    command_label: 'Clone the repo',
    command: 'git clone https://github.com/zerx-lab/openwarp',
    copy: 'Copy',
    copied: 'Copied',
    steps: [
      'cargo build --release',
      './target/release/openwarp',
      'Add a custom provider in Settings',
    ],
  },
  bento: {
    byop: {
      tag: 'Live · Click to switch',
      hint: 'Click to switch provider',
    },
    privacy: {
      tag: 'On-device',
      bullets: ['No cloud upload', 'No telemetry', 'Zero credential leak'],
    },
    i18n: {
      tag: 'Extensible',
      pills: ['English', '简体中文', '日本語', 'Español'],
    },
    templates: {
      tag: 'minijinja',
      preview: 'rendered output →',
    },
    warp: {
      tag: 'Preserved',
      chips: ['Blocks', 'Workflows', 'AI commands', 'Keymaps', 'Themes'],
    },
    opensource: {
      tag: 'Open',
      license: ['AGPL-3.0', 'MIT'],
      links: ['View source', 'Read LICENSE', 'Open an issue'],
    },
  },
  roadmap: {
    meta_title: 'OpenWarp Roadmap — Shipped & in-flight enhancements',
    meta_description:
      'The OpenWarp roadmap on top of upstream Warp: i18n, multilingual client tokenizer, expanded provider support.',
    eyebrow: 'Roadmap',
    title: 'Opening Warp up, one merge at a time',
    subtitle:
      'Every status below maps to merged code or commits — no marketing language. Green = shipped, blue = in flight, gray = planned.',
    legend: {
      shipped: 'Shipped',
      in_progress: 'In flight',
      planned: 'Planned',
    },
    progress_label: 'progress',
    tracks: [
      {
        id: 'i18n',
        eyebrow: '01 · Internationalization',
        title: 'First-class multilingual UI',
        summary:
          'A Fluent (.ftl) infrastructure is in place. English and Simplified Chinese ship in lockstep; the community can extend the locale set from there.',
        progress: 80,
        items: [
          { status: 'shipped', text: 'Fluent infrastructure + ANCHOR protocol (en + zh-CN updated together)' },
          { status: 'shipped', text: 'AI / Features / Teams / Code / MCP Servers / Settings pages translated end-to-end' },
          { status: 'shipped', text: 'workspace keybinding descriptions translated (116 keys, ~156 call sites)' },
          { status: 'in_progress', text: 'Remaining settings_view subdirectories and keybinding-desc backfill' },
          { status: 'planned', text: 'Terminal context menus, command palette, and Drive views' },
          { status: 'planned', text: 'Community: Japanese / Spanish / others — locale templates & contributor guide' },
        ],
      },
      {
        id: 'tokenizer',
        eyebrow: '02 · Client-side tokenizer',
        title: 'Input classification beyond English',
        summary:
          'The terminal input classifier was trained on English only. We are extending it to CJK and other scripts so that non-English input is not misrouted as a shell command.',
        progress: 35,
        items: [
          { status: 'shipped', text: 'CJK early-return: Han / Ext-A / Hiragana / Katakana / Hangul / fullwidth punctuation routed to AI' },
          { status: 'shipped', text: 'contains_cjk wired into input.rs / agent.rs / universal.rs hot paths' },
          { status: 'in_progress', text: 'Other non-Latin scripts: Arabic / Cyrillic / Thai / Devanagari early-return rules' },
          { status: 'planned', text: 'Probabilistic weighting for mixed-script input (e.g. CJK + English) instead of hard rules' },
          { status: 'planned', text: 'Replace or augment natural_language_detection dictionaries with multilingual data' },
        ],
      },
      {
        id: 'providers',
        eyebrow: '03 · More providers',
        title: 'BYOP coverage',
        summary:
          'BYOP supports several native protocols via the genai adapter layer — not just OpenAI Chat Completions. Each native protocol means one less gateway and less token loss.',
        progress: 60,
        items: [
          { status: 'shipped', text: 'OpenAI Chat Completions (GPT-4o / GPT-5 / any compatible endpoint)' },
          { status: 'shipped', text: 'OpenAI Responses (native reasoning / built-in tools)' },
          { status: 'shipped', text: 'Anthropic native (Claude 4.x / 1M context / cache_control)' },
          { status: 'shipped', text: 'Google Gemini native protocol' },
          { status: 'shipped', text: 'DeepSeek native (deepseek-r1 reasoning)' },
          { status: 'shipped', text: 'Ollama local (no key, localhost direct)' },
          { status: 'shipped', text: 'base_url normalization: host-only entries auto-append /v1/ etc.' },
          { status: 'in_progress', text: 'Providers subpage: models.dev data source + quick-add search' },
          { status: 'planned', text: 'xAI Grok / Mistral / Cohere native protocols' },
          { status: 'planned', text: 'One-click templates for Azure OpenAI / Bedrock / Vertex enterprise gateways' },
        ],
      },
    ],
    footnote_title: 'How to read this roadmap',
    footnote_body:
      'This roadmap is maintained against merged commits, not a wishlist. Every ✓ maps to actual code; in-flight items have an open issue or draft PR. To contribute, head to GitHub.',
    cta_repo: 'Open repository',
    cta_issues: 'File an issue',
  },
  footer: {
    project: 'Project',
    community: 'Community',
    legal: 'Legal',
    docs: 'Docs',
    changelog: 'Changelog',
    roadmap: 'Roadmap',
    discussions: 'Discussions',
    issues: 'Issues',
    license: 'License',
    privacy: 'Privacy',
    rights: 'A community fork on top of Warp. Not affiliated with Warp.',
  },
};
