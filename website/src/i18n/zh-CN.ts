export const zhCN = {
  meta: {
    title: 'OpenWarp — 为 Warp 解锁自定义 AI 提供商',
    description:
      'OpenWarp 是 Warp 的开放式增强项目。自由接入任何 OpenAI 兼容模型,自定义系统提示词,享受真正属于你的智能终端。',
  },
  nav: {
    how: '工作方式',
    features: '特性',
    providers: '自定义提供商',
    faq: 'FAQ',
    docs: '文档',
    github: 'GitHub',
  },
  hero: {
    badge: '社区版 · 进行中',
    title_1: '把',
    title_em: '任意 AI 模型',
    title_2: '装进你的终端',
    subtitle:
      'OpenWarp 在 Warp 之上加入 BYOP(自带提供商)能力 —— 自由配置 OpenAI 兼容接口、自定义模型与系统提示词、原生多语言。',
    cta_primary: '查看 GitHub',
    cta_secondary: '阅读文档',
    note: '当前项目处于早期开发,尚未发布正式版本',
    trust_lead: '兼容主流提供商',
  },
  terminal: {
    tabs: ['zsh', 'openwarp', 'agent'],
    breadcrumb: '~/projects/openwarp',
    scenarios: [
      {
        model: 'deepseek-r1',
        tag: '推理',
        user: '帮我重构这个 Rust trait',
        reply:
          '识别到这个 trait 承担了三个职责。建议拆分为 Reader 与 Writer 两个 trait,以解耦读写并提升可测试性。',
        suggest: [
          'pub trait Reader { fn read(&self) -> Bytes; }',
          'pub trait Writer { fn write(&mut self, b: Bytes); }',
        ],
      },
      {
        model: 'gpt-4o',
        tag: 'OpenAI',
        user: '为 users 表生成迁移脚本',
        reply:
          '新增 last_login_at 字段并为 (email, last_login_at) 建立复合索引,以加速登录态相关查询。',
        suggest: [
          'ALTER TABLE users ADD COLUMN last_login_at TIMESTAMPTZ;',
          'CREATE INDEX idx_users_email_login ON users(email, last_login_at);',
        ],
      },
      {
        model: 'qwen-2.5-coder',
        tag: '本地',
        user: '解释这段 unsafe 代码',
        reply:
          '这段代码通过裸指针直接读写内存,绕过了借用检查。仅当生命周期与对齐均可静态保证时,这种用法才是安全的。',
        suggest: [
          '// 仅在 layout 与 lifetime 可证明时安全',
          'unsafe { *ptr = value; }',
        ],
      },
    ],
    status: {
      tokens: 'tokens',
      latency: '延迟',
      local: '本地',
      streaming: '流式中',
      ready: '就绪',
    },
  },
  stats: {
    items: [
      { value: '∞',     label: '可接入提供商'   },
      { value: '2',     label: '内置语言' },
      { value: 'AGPL',  label: '开源许可'      },
      { value: '100%',  label: '本地凭证存储'  },
    ],
  },
  how: {
    eyebrow: '工作方式',
    title: '三步,把 AI 完全握在自己手里',
    subtitle: '保留 Warp 全部交互,只把 AI 层完全开放 —— 密钥、模型、提示词全部由你掌控。',
    steps: [
      {
        num: '01',
        title: '接入任意提供商',
        desc: '在设置中粘贴 Base URL 与 API Key —— 任何 OpenAI 兼容端点都即插即用,凭证仅保存在本地。',
      },
      {
        num: '02',
        title: '编写动态提示词',
        desc: '基于 minijinja 的模板引擎,根据当前工作目录、语言、角色实时渲染系统提示词,精准引导模型。',
      },
      {
        num: '03',
        title: '在终端立即启用',
        desc: '一键切换模型、对话、命令补全 —— 体验与 Warp 一致,但完全由你掌控。',
      },
    ],
  },
  providers: {
    eyebrow: '自定义提供商',
    title: '一次配置,全模型可用',
    subtitle:
      '兼容 OpenAI Chat Completions 协议的任何端点都可以接入 —— OpenAI、Anthropic 网关、DeepSeek、Qwen、本地 Ollama 都没有边界。',
    fields: {
      name: '提供商名称',
      base_url: 'Base URL',
      api_key: 'API Key',
      model: '默认模型',
      prompt: '系统提示词模板',
    },
    bullets: [
      '✓ OpenAI 兼容流式协议',
      '✓ minijinja 模板渲染系统提示词',
      '✓ 多账号、多模型一键切换',
      '✓ 本地保存,凭证不离开设备',
    ],
    tabs: [
      {
        id: 'deepseek',
        name: 'DeepSeek',
        tag: '推理',
        baseUrl: 'https://api.deepseek.com/v1',
        apiKey: 'sk-•••••••••••••••••••••',
        model: 'deepseek-r1',
      },
      {
        id: 'ollama',
        name: 'Ollama',
        tag: '本地',
        baseUrl: 'http://localhost:11434/v1',
        apiKey: '— 不需要 —',
        model: 'qwen2.5-coder:7b',
      },
      {
        id: 'openrouter',
        name: 'OpenRouter',
        tag: '聚合',
        baseUrl: 'https://openrouter.ai/api/v1',
        apiKey: 'sk-or-•••••••••••••••••••',
        model: 'anthropic/claude-3.5-sonnet',
      },
    ],
  },
  features: {
    eyebrow: '核心特性',
    title: '所有你期待的,我们都开放',
    items: [
      { title: 'BYOP 自定义提供商', desc: '内置 OpenAI 兼容客户端,Base URL / API Key / Model 自由组合。' },
      { title: '系统提示词模板',     desc: '基于 minijinja 的强大模板,根据上下文动态渲染指令。' },
      { title: '多语言界面',         desc: '原生中文与英文 UI,后续社区可继续扩展更多语种。' },
      { title: '隐私优先',           desc: '关闭 Cloud Agent / Computer Use,默认不上传云端,凭证仅本地保存。' },
      { title: '保留 Warp 体验',     desc: '基于 Warp 上游持续合并,完整保留块、AI 命令、工作流、键位。' },
      { title: '开源协议',           desc: '与 Warp 上游一致,采用 AGPL / MIT 双许可,代码全部公开。' },
    ],
  },
  faq: {
    eyebrow: '常见问题',
    title: '关于 OpenWarp',
    items: [
      {
        q: 'OpenWarp 与 Warp 官方是什么关系?',
        a: 'OpenWarp 是基于 Warp 开源代码的社区分支,与 Warp 官方公司无附属关系,遵循上游的 AGPL / MIT 双许可。',
      },
      {
        q: '我的 API Key 会被上传吗?',
        a: '不会。所有自定义提供商凭证仅保存在本地配置文件中,直接由 OpenWarp 与你指定的 Base URL 通信,不经任何中转。',
      },
      {
        q: '支持哪些模型提供商?',
        a: '只要兼容 OpenAI Chat Completions 流式协议都可以接入:OpenAI、DeepSeek、Qwen、Groq、Together、本地 Ollama / LM Studio,以及众多代理网关。',
      },
      {
        q: '能继续收到 Warp 上游更新吗?',
        a: '会持续合并 Warp 上游主线,在保留体验的同时叠加 BYOP 与多语言增强。',
      },
    ],
  },
  cta: {
    title: '想第一时间体验?',
    desc: '克隆仓库本地构建,或关注 GitHub 接收每次发布更新。',
    button: '前往 GitHub',
    secondary: '订阅更新',
    command_label: '克隆仓库',
    command: 'git clone https://github.com/zerx-lab/openwarp',
    copy: '复制',
    copied: '已复制',
    steps: [
      'cargo build --release',
      './target/release/openwarp',
      '在设置中添加自定义提供商',
    ],
  },
  bento: {
    byop: {
      tag: '01 · 大卡',
      hint: '点击切换提供商',
    },
    privacy: {
      tag: '本地存储',
      bullets: ['不上传云端', '不收集遥测', '凭证零外发'],
    },
    i18n: {
      tag: '可扩展',
      pills: ['简体中文', 'English', '日本語', 'Español'],
    },
    templates: {
      tag: 'minijinja',
      preview: '渲染输出 →',
    },
    warp: {
      tag: '体验保留',
      chips: ['Blocks', 'Workflows', 'AI 命令', 'Keymaps', '主题'],
    },
    opensource: {
      tag: '完全开源',
      license: ['AGPL-3.0', 'MIT'],
      links: ['查看源代码', '阅读 LICENSE', '提交 Issue'],
    },
  },
  footer: {
    project: '项目',
    community: '社区',
    legal: '法律',
    docs: '文档',
    changelog: '更新日志',
    roadmap: '路线图',
    discussions: '讨论',
    issues: '问题反馈',
    license: '许可协议',
    privacy: '隐私',
    rights: '基于 Warp 二次开发,与 Warp 官方无关',
  },
};

export type Dict = typeof zhCN;
