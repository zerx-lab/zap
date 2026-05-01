export const zhCN = {
  meta: {
    title: "OpenWarp — 为 Warp 解锁自定义 AI 提供商",
    description:
      "OpenWarp 是 Warp 的开放式增强项目。通过 genai 适配层原生接入 OpenAI / Anthropic / Gemini / DeepSeek / Ollama 等多协议提供商,自定义系统提示词,享受真正属于你的智能终端。",
  },
  nav: {
    how: "工作方式",
    features: "特性",
    providers: "自定义提供商",
    faq: "FAQ",
    docs: "文档",
    github: "GitHub",
  },
  hero: {
    badge: "社区版 · 进行中",
    title_1: "把",
    title_em: "任意 AI 模型",
    title_2: "装进你的终端",
    subtitle:
      "OpenWarp 在 Warp 之上加入 BYOP(自带提供商)能力 —— 通过 genai 适配层原生支持 6 种 API 协议、自定义模型与系统提示词、原生多语言。",
    cta_primary: "查看 GitHub",
    cta_secondary: "阅读文档",
    note: "当前项目处于早期开发,尚未发布正式版本",
    trust_lead: "兼容主流提供商",
  },
  terminal: {
    tabs: ["zsh", "openwarp", "agent"],
    breadcrumb: "~/projects/openwarp",
    scenarios: [
      {
        model: "deepseek-r1",
        tag: "推理",
        user: "帮我重构这个 Rust trait",
        reply:
          "识别到这个 trait 承担了三个职责。建议拆分为 Reader 与 Writer 两个 trait,以解耦读写并提升可测试性。",
        suggest: [
          "pub trait Reader { fn read(&self) -> Bytes; }",
          "pub trait Writer { fn write(&mut self, b: Bytes); }",
        ],
      },
      {
        model: "gpt-4o",
        tag: "OpenAI",
        user: "为 users 表生成迁移脚本",
        reply:
          "新增 last_login_at 字段并为 (email, last_login_at) 建立复合索引,以加速登录态相关查询。",
        suggest: [
          "ALTER TABLE users ADD COLUMN last_login_at TIMESTAMPTZ;",
          "CREATE INDEX idx_users_email_login ON users(email, last_login_at);",
        ],
      },
      {
        model: "qwen-2.5-coder",
        tag: "本地",
        user: "解释这段 unsafe 代码",
        reply:
          "这段代码通过裸指针直接读写内存,绕过了借用检查。仅当生命周期与对齐均可静态保证时,这种用法才是安全的。",
        suggest: [
          "// 仅在 layout 与 lifetime 可证明时安全",
          "unsafe { *ptr = value; }",
        ],
      },
    ],
    status: {
      tokens: "tokens",
      latency: "延迟",
      local: "本地",
      streaming: "流式中",
      ready: "就绪",
    },
  },
  stats: {
    items: [
      { value: "∞", label: "可接入提供商" },
      { value: "2", label: "内置语言" },
      { value: "AGPL", label: "开源许可" },
      { value: "100%", label: "本地凭证存储" },
    ],
  },
  how: {
    eyebrow: "工作方式",
    title: "三步,把 AI 完全握在自己手里",
    subtitle:
      "保留 Warp 全部交互,只把 AI 层完全开放 —— 密钥、模型、提示词全部由你掌控。",
    steps: [
      {
        num: "01",
        title: "接入任意提供商",
        desc: "在设置中显式选择 API 协议、粘贴 Base URL 与 API Key —— OpenAI / Anthropic / Gemini / Ollama / DeepSeek 6 种原生协议任意切换,凭证仅保存在本地。",
      },
      {
        num: "02",
        title: "编写动态提示词",
        desc: "基于 minijinja 的模板引擎,根据当前工作目录、语言、角色实时渲染系统提示词,精准引导模型。",
      },
      {
        num: "03",
        title: "在终端立即启用",
        desc: "一键切换模型、对话、命令补全 —— 体验与 Warp 一致,但完全由你掌控。",
      },
    ],
  },
  providers: {
    eyebrow: "自定义提供商",
    title: "一次配置,全模型可用",
    subtitle:
      "OpenWarp 通过 genai 适配层原生支持 6 种 API 协议:OpenAI / OpenAI Responses / Anthropic / Gemini / Ollama / DeepSeek —— 显式选择协议,绕过「按模型名识别」的猜测,密钥与请求直连 provider。",
    fields: {
      name: "提供商名称",
      protocol: "API 协议",
      base_url: "Base URL",
      endpoint: "请求端点",
      api_key: "API Key",
      model: "默认模型",
      prompt: "系统提示词模板",
    },
    bullets: [
      "✓ 6 种 API 协议原生路由,不再「OpenAI 兼容硬塞」",
      "✓ 推理思考多轮回传:DeepSeek reasoning_content / Claude thinking / Gemini",
      "✓ minijinja 模板渲染系统提示词",
      "✓ 凭证仅本地保存,直连 provider 端点不经任何中转",
    ],
    tabs: [
      {
        id: "deepseek",
        name: "DeepSeek",
        tag: "OpenAI 兼容",
        protocol: "OpenAI",
        baseUrl: "https://api.deepseek.com",
        endpoint: "POST /v1/chat/completions",
        apiKey: "sk-•••••••••••••••••••••",
        model: "deepseek-reasoner",
      },
      {
        id: "anthropic",
        name: "Anthropic",
        tag: "原生协议",
        protocol: "Anthropic",
        baseUrl: "https://api.anthropic.com",
        endpoint: "POST /v1/messages",
        apiKey: "sk-ant-•••••••••••••••••",
        model: "claude-sonnet-4-6",
      },
      {
        id: "ollama",
        name: "Ollama",
        tag: "本地",
        protocol: "Ollama",
        baseUrl: "http://localhost:11434",
        endpoint: "POST /api/chat",
        apiKey: "— 不需要 —",
        model: "qwen2.5-coder:7b",
      },
    ],
  },
  features: {
    eyebrow: "核心特性",
    title: "所有你期待的,我们都开放",
    items: [
      {
        title: "BYOP 自定义提供商",
        desc: "通过 genai 适配层原生支持 6 种 API 协议,Base URL / API Key / Model 自由组合。",
      },
      {
        title: "系统提示词模板",
        desc: "基于 minijinja 的强大模板,根据上下文动态渲染指令。",
      },
      {
        title: "多语言界面",
        desc: "原生中文与英文 UI,后续社区可继续扩展更多语种。",
      },
      {
        title: "隐私优先",
        desc: "关闭 Cloud Agent / Computer Use,默认不上传云端,凭证仅本地保存。",
      },
      {
        title: "保留 Warp 体验",
        desc: "基于 Warp 上游持续合并,完整保留块、AI 命令、工作流、键位。",
      },
      {
        title: "开源协议",
        desc: "与 Warp 上游一致,采用 AGPL / MIT 双许可,代码全部公开。",
      },
    ],
  },
  faq: {
    eyebrow: "常见问题",
    title: "关于 OpenWarp",
    items: [
      {
        q: "OpenWarp 与 Warp 官方是什么关系?",
        a: "OpenWarp 是基于 Warp 开源代码的社区分支,与 Warp 官方公司无附属关系,遵循上游的 AGPL / MIT 双许可。",
      },
      {
        q: "我的 API Key 会被上传吗?",
        a: "不会。所有自定义提供商凭证仅保存在本地配置文件中,直接由 OpenWarp 与你指定的 Base URL 通信,不经任何中转。",
      },
      {
        q: "支持哪些模型提供商?",
        a: "OpenWarp 内置 genai 多协议适配,原生支持 OpenAI / OpenAI Responses / Anthropic / Gemini / Ollama / DeepSeek 共 6 种协议;OpenAI 兼容端点(Qwen / Groq / Together / OpenRouter / SiliconFlow / LM Studio 等)选 OpenAI 协议并填 Base URL 即可接入。",
      },
      {
        q: "能继续收到 Warp 上游更新吗?",
        a: "会持续合并 Warp 上游主线,在保留体验的同时叠加 BYOP 与多语言增强。",
      },
    ],
  },
  cta: {
    title: "想第一时间体验?",
    desc: "克隆仓库本地构建,或关注 GitHub 接收每次发布更新。",
    button: "前往 GitHub",
    command_label: "克隆仓库",
    command: "git clone -b openWarp https://github.com/zerx-lab/warp",
    copy: "复制",
    copied: "已复制",
    steps: [
      "cargo build --release",
      "./target/release/openwarp",
      "在设置中添加自定义提供商",
    ],
  },
  bento: {
    byop: {
      tag: "模型路由",
      hint: "点击切换提供商",
    },
    privacy: {
      tag: "本地存储",
      bullets: ["不上传云端", "不收集遥测", "凭证零外发"],
    },
    i18n: {
      tag: "可扩展",
      pills: ["简体中文", "English", "日本語", "Español"],
    },
    templates: {
      tag: "minijinja",
      preview: "渲染输出 →",
    },
    warp: {
      tag: "体验保留",
      chips: ["Blocks", "Workflows", "AI 命令", "Keymaps", "主题"],
    },
    opensource: {
      tag: "完全开源",
      license: ["AGPL-3.0", "MIT"],
      links: ["查看源代码", "阅读 LICENSE", "提交 Issue"],
    },
  },
  roadmap: {
    meta_title: "OpenWarp 路线图 — 进行中与计划中的增强",
    meta_description:
      "OpenWarp 在 Warp 上游之上的增强路线图:国际化、客户端分词器多语言、提供商扩展。",
    eyebrow: "路线图",
    title: "我们正在把 Warp 进一步打开",
    subtitle:
      "所有进度都基于已合并的代码与提交,没有营销话术。绿色=已交付,蓝色=进行中,灰色=计划中。",
    legend: {
      shipped: "已交付",
      in_progress: "进行中",
      planned: "计划中",
    },
    progress_label: "完成度",
    tracks: [
      {
        id: "i18n",
        eyebrow: "01 · 国际化",
        title: "原生多语言界面",
        summary:
          "基于 Fluent (.ftl) 的 i18n 基础设施已落地,英文与简体中文双轨并行,后续依靠社区扩展更多语种。",
        progress: 80,
        items: [
          {
            status: "shipped",
            text: "Fluent 基础设施 + ANCHOR 锚点协议(英文与中文同步追加)",
          },
          {
            status: "shipped",
            text: "AI / Features / Teams / Code / MCP Servers / Settings 等设置页端到端翻译",
          },
          {
            status: "shipped",
            text: "workspace 快捷键描述全译(116 key,约 156 处 call site)",
          },
          {
            status: "in_progress",
            text: "剩余 settings_view 子目录与 keybinding-desc 续补",
          },
          {
            status: "planned",
            text: "终端右键菜单、命令面板、Drive 视图等运行时文案补全",
          },
          {
            status: "planned",
            text: "社区扩展:日本語 / Español / 其它语言模板与贡献指南",
          },
        ],
      },
      {
        id: "tokenizer",
        eyebrow: "02 · 客户端分词器",
        title: "不只是英文的输入分类",
        summary:
          "终端输入分类器(input_classifier)历史上只为英文训练。我们正在把它扩展到 CJK 与更多脚本,避免中文输入被误判为 shell 命令。",
        progress: 35,
        items: [
          {
            status: "shipped",
            text: "CJK 早返回:基本汉字 / 扩展 A / 平假名 / 片假名 / 韩文音节 / 全角标点统一判 AI",
          },
          {
            status: "shipped",
            text: "在 input.rs / agent.rs / universal.rs 等热路径接入 contains_cjk",
          },
          {
            status: "in_progress",
            text: "其他非拉丁脚本:阿拉伯语 / 西里尔字母 / 泰语 / 天城文等的早返回规则",
          },
          {
            status: "planned",
            text: "多脚本混排输入(中英混合)的概率加权,而非硬规则",
          },
          {
            status: "planned",
            text: "替换或补强 natural_language_detection 词典为多语种数据源",
          },
        ],
      },
      {
        id: "providers",
        eyebrow: "03 · 更多提供商",
        title: "BYOP 适配范围",
        summary:
          "BYOP 通过 genai 适配层支持多种原生协议,而不只是 OpenAI Chat Completions。每多支持一种,就少一层网关与 token 损耗。",
        progress: 60,
        items: [
          {
            status: "shipped",
            text: "OpenAI Chat Completions(GPT-4o / GPT-5 / 任意兼容端点)",
          },
          {
            status: "shipped",
            text: "OpenAI Responses(原生 reasoning / built-in tools)",
          },
          {
            status: "shipped",
            text: "Anthropic 原生(Claude 4.x / 1M context / cache_control)",
          },
          { status: "shipped", text: "Google Gemini 原生协议" },
          { status: "shipped", text: "DeepSeek 原生(推理模型 deepseek-r1)" },
          { status: "shipped", text: "Ollama 本地(零密钥,localhost 直连)" },
          {
            status: "shipped",
            text: "base_url 规范化:host-only 填法自动补 /v1/ 等版本路径",
          },
          {
            status: "in_progress",
            text: "Provider 子页 models.dev 数据源 + 搜索框快速添加",
          },
          { status: "planned", text: "xAI Grok / Mistral / Cohere 原生协议" },
          {
            status: "planned",
            text: "Azure OpenAI / Bedrock / Vertex 等企业网关一键配置模板",
          },
        ],
      },
      {
        id: "active-ai",
        eyebrow: "04 · 主动式 AI",
        title: "Next Command / 建议 / 检索全部走 BYOP",
        summary:
          "Warp 上游的主动式 AI(灰色补全 / Prompt Suggestions / NLD / Relevant Files)原本直连 ${server_root_url}/ai/*,现已全量切到 BYOP one-shot,凭证不再绕路云端。",
        progress: 70,
        items: [
          {
            status: "shipped",
            text: "Agent 主对话流走 BYOP(genai 6 协议显式路由)",
          },
          {
            status: "shipped",
            text: "Next Command 灰色补全 + zero-state 建议切到 BYOP one-shot",
          },
          {
            status: "shipped",
            text: "Prompt Suggestions / NLD predict / Relevant Files 全量切到 BYOP",
          },
          {
            status: "shipped",
            text: "新增 active_ai_model / next_command_model 独立模型字段",
          },
          {
            status: "shipped",
            text: "DeepSeek reasoning_content 多轮回传(genai DeepSeek adapter)",
          },
          {
            status: "shipped",
            text: "BYOP LRC tag-in 多轮上下文持续注入 + sanitize 双向补 placeholder",
          },
          {
            status: "in_progress",
            text: "Code Review(commit message / PR title / PR description)接入 BYOP",
          },
          {
            status: "planned",
            text: "passive suggestions(Workflow / Rule chips)BYOP 化",
          },
        ],
      },
      {
        id: "decouple",
        eyebrow: "05 · 解耦云端",
        title: "断开 Warp Inc 默认链路",
        summary:
          "OpenWarp 是纯本地分支:云账号刷新、远端用户持久化、Plan 同步、passive suggestions HTTP 等链路均已就地禁用,凭证与请求只走你配置的 provider。",
        progress: 75,
        items: [
          { status: "shipped", text: "移除 Cloud Agent / Computer Use 入口" },
          {
            status: "shipped",
            text: "auth_manager refresh_user / persist 整体 no-op,不再向 app.warp.dev 写入用户态",
          },
          {
            status: "shipped",
            text: "移除 Plan 自动同步 Warp Drive 开关与调用",
          },
          {
            status: "shipped",
            text: "passive suggestions 云端 HTTP 链路短路 + 静默 modal warn",
          },
          {
            status: "shipped",
            text: "Profile Editor 残留云端开关清理(autosync / web search)",
          },
          {
            status: "in_progress",
            text: "i18n 文案去除「云端 Agent / Oz」字样",
          },
          {
            status: "planned",
            text: "彻底审计仍然命中 ${server_root_url} 的所有路径",
          },
        ],
      },
      {
        id: "polish",
        eyebrow: "06 · 体验 & 稳定性",
        title: "本地化体验与崩溃修复",
        summary:
          "为 BYOP 多协议补齐边角:多轮 tool_use 配对、Take over Agent handoff、长命令 alt-screen 防崩、命令面板中英双向搜索、OpenWarp 打包与 Release 工作流。",
        progress: 65,
        items: [
          {
            status: "shipped",
            text: "BYOP 多轮 tool_use 双向 sanitize:孤儿 tool_response 不再触发 Anthropic 400 + 重试 flex panic",
          },
          {
            status: "shipped",
            text: "TUI / 长命令 Take over Agent → resume 链路修复(SetInputModeAgent alt-screen 死锁)",
          },
          {
            status: "shipped",
            text: "footer 工具提示 context_window_usage 实时同步(BYOP usage_metadata 透传)",
          },
          {
            status: "shipped",
            text: "命令面板 Fuzzy 搜索 + binding.name 中英双向匹配",
          },
          {
            status: "shipped",
            text: "63 个 Toggle 设置命令前后缀全译(Fluent {$suffix} 占位)",
          },
          {
            status: "shipped",
            text: "Windows 打包名 WarpOss → OpenWarp 对齐",
          },
          {
            status: "shipped",
            text: "macOS Release timeout 90 → 150 分钟",
          },
          {
            status: "planned",
            text: "Linux Release 工作流自动化",
          },
        ],
      },
    ],
    footnote_title: "路线图怎么读",
    footnote_body:
      '路线图按"已合并的提交"维护,不是 PM 的愿望清单。每个 ✓ 都对应代码库里的具体文件与函数;进行中条目代表已开 issue / 已起草 PR。希望参与的话,直接来 GitHub 提 issue 或 PR。',
    cta_repo: "查看仓库",
    cta_issues: "提交 issue",
  },
  footer: {
    project: "项目",
    community: "社区",
    legal: "法律",
    docs: "文档",
    changelog: "更新日志",
    roadmap: "路线图",
    discussions: "讨论",
    issues: "问题反馈",
    license: "许可协议",
    privacy: "隐私",
    rights: "基于 Warp 二次开发,与 Warp 官方无关",
  },
};

export type Dict = typeof zhCN;
