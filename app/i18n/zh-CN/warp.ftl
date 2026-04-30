# Warp 桌面端 — 简体中文
# 缺失的 key 会自动 fallback 到 en/warp.ftl,所以可以分批补译。
# 术语统一:Agent → 智能体 / Block → 命令块 / Drive → 云盘 / Workflow → 工作流 / Profile → 配置档

# =============================================================================
# SECTION: common (Owner: foundation)
# =============================================================================

app-name = Warp
app-tagline = 面向个人与团队的云端终端

common-ok = 确定
common-cancel = 取消
common-apply = 应用
common-save = 保存
common-delete = 删除
common-confirm = 确认
common-close = 关闭
common-reset = 重置
common-back = 返回
common-next = 下一步
common-yes = 是
common-no = 否
common-edit = 编辑
common-add = 添加
common-remove = 移除
common-rename = 重命名
common-copy = 复制
common-paste = 粘贴
common-search = 搜索
common-loading = 加载中…
common-error = 错误
common-warning = 警告
common-info = 提示
common-success = 成功

# =============================================================================
# SECTION: language (Owner: foundation)
# =============================================================================

language-widget-label = 语言
language-widget-secondary = 重启 Warp 以让此更改完全生效。
language-restart-required-title = 语言已切换
language-restart-required-body = Warp 的界面语言已更新。部分文字会立即切换,完整生效需要重启 Warp。

# =============================================================================
# SECTION: settings (Owner: agent-settings)
# =============================================================================

# --- ANCHOR-SUB-MOD-NAV (agent-settings-mod) ---
# settings_view/mod.rs SettingsSection 标签 + 上下文菜单分屏/关闭操作

# 侧边栏 / SettingsSection 标签 (Display impl)
settings-section-about = 关于
settings-section-account = 账户
settings-section-mcp-servers = MCP 服务器
settings-section-billing-and-usage = 账单与用量
settings-section-appearance = 外观
settings-section-features = 功能
settings-section-keybindings = 快捷键
settings-section-privacy = 隐私
settings-section-referrals = 推荐
settings-section-shared-blocks = 共享命令块
settings-section-teams = 团队
settings-section-warp-drive = Warp Drive
settings-section-warpify = Warpify
settings-section-ai = AI
settings-section-warp-agent = Warp 智能体
settings-section-agent-profiles = 配置档
settings-section-agent-mcp-servers = MCP 服务器
settings-section-agent-providers = 提供商
settings-section-knowledge = 知识库
settings-section-third-party-cli-agents = 第三方 CLI 智能体
settings-section-code = 代码
settings-section-code-indexing = 索引与项目
settings-section-editor-and-code-review = 编辑器与代码评审
settings-section-cloud-environments = 环境
settings-section-oz-cloud-api-keys = Oz Cloud API 密钥

# 上下文菜单项(分屏 / 关闭窗格)
settings-pane-split-right = 向右拆分窗格
settings-pane-split-left = 向左拆分窗格
settings-pane-split-down = 向下拆分窗格
settings-pane-split-up = 向上拆分窗格
settings-pane-close = 关闭窗格

# 调试开关设置描述(命令面板)
settings-debug-show-init-block = 显示初始化命令块
settings-debug-hide-init-block = 隐藏初始化命令块
settings-debug-show-inband-blocks = 显示行内命令块
settings-debug-hide-inband-blocks = 隐藏行内命令块

# --- ANCHOR-SUB-ABOUT (agent-settings-about) ---

# about_page.rs
settings-about-copyright = 版权所有 2026 Warp

# main_page.rs — referral / account
settings-main-referral-cta = 与朋友和同事分享 Warp,获得奖励
settings-main-refer-a-friend = 推荐朋友
settings-main-sign-up = 注册
settings-main-plan-free = 免费版
settings-main-compare-plans = 对比方案
settings-main-contact-support = 联系支持
settings-main-manage-billing = 管理账单
settings-main-upgrade-to-turbo = 升级到 Turbo 方案
settings-main-upgrade-to-lightspeed = 升级到 Lightspeed 方案

# main_page.rs — settings sync
settings-main-settings-sync-label = 设置同步

# main_page.rs — version / autoupdate
settings-main-version-label = 版本
settings-main-status-up-to-date = 已是最新
settings-main-cta-check-for-updates = 检查更新
settings-main-status-checking = 正在检查更新...
settings-main-status-downloading = 正在下载更新...
settings-main-status-update-available = 有可用更新
settings-main-cta-relaunch-warp = 重启 Warp
settings-main-status-updating = 正在更新...
settings-main-status-installed-update = 更新已安装
settings-main-status-cant-install = 有新版本的 Warp 可用,但无法安装
settings-main-status-cant-launch = 新版本的 Warp 已安装,但无法启动。
settings-main-cta-update-manually = 手动更新 Warp

# --- ANCHOR-SUB-MCP (agent-settings-mcp) ---
settings-mcp-page-title = MCP 服务器
settings-mcp-logout-success-named = 已成功登出 {$name} MCP 服务器
settings-mcp-logout-success = 已成功登出 MCP 服务器
settings-mcp-install-modal-busy = 请先完成当前 MCP 安装,然后再打开另一个安装链接。
settings-mcp-unknown-server = 未知的 MCP 服务器 '{$name}'
settings-mcp-install-from-link-failed = MCP 服务器 '{$name}' 无法通过此链接安装。

# --- ANCHOR-SUB-PLATFORM (agent-settings-platform) ---
settings-platform-section-title = Oz Cloud API 密钥
settings-platform-description = 创建并管理 API 密钥,允许其他 Oz 云端 agent 访问你的 Warp 账户。
    了解更多请访问
settings-platform-documentation-link = 文档。
settings-platform-create-button = + 创建 API 密钥
settings-platform-modal-title-new = 新建 API 密钥
settings-platform-modal-title-save = 保存你的密钥
settings-platform-toast-deleted = API 密钥已删除
settings-platform-column-name = 名称
settings-platform-column-key = 密钥
settings-platform-column-scope = 范围
settings-platform-column-created = 创建时间
settings-platform-column-last-used = 最近使用
settings-platform-column-expires-at = 过期时间
settings-platform-value-never = 从未
settings-platform-scope-personal = 个人
settings-platform-scope-team = 团队
settings-platform-zero-state-title = 暂无 API 密钥
settings-platform-zero-state-description = 创建密钥以管理对 Warp 的外部访问

# --- ANCHOR-SUB-KEYBINDINGS (agent-settings-keybindings) ---
settings-keybindings-search-placeholder = 按名称或按键搜索(例如 "cmd d")
settings-keybindings-conflict-warning = 此快捷键与其他快捷键冲突
settings-keybindings-button-default = 默认
settings-keybindings-button-cancel = 取消
settings-keybindings-button-clear = 清除
settings-keybindings-button-save = 保存
settings-keybindings-press-new-shortcut = 按下新的快捷键
settings-keybindings-description = 在下方为已有操作添加你自己的自定义快捷键。
settings-keybindings-use-prefix = 使用
settings-keybindings-use-suffix = 可随时在侧边栏中参考这些快捷键。
settings-keybindings-not-synced-tooltip = 快捷键不会同步到云端
settings-keybindings-subheader = 配置键盘快捷键
settings-keybindings-command-column = 命令

# --- ANCHOR-SUB-REFERRALS (agent-settings-referrals) ---
settings-referrals-page-title = 邀请朋友加入 Warp
settings-referrals-anonymous-header = 注册以参与 Warp 推荐计划
settings-referrals-sign-up = 注册
settings-referrals-link-label = 链接
settings-referrals-email-label = 邮箱
settings-referrals-link-error = 加载邀请码失败。
settings-referrals-loading = 加载中...
settings-referrals-copy-link-button = 复制链接
settings-referrals-email-send-button = 发送
settings-referrals-email-sending-button = 发送中...
settings-referrals-link-copied-toast = 链接已复制。
settings-referrals-email-success-toast = 邮件发送成功。
settings-referrals-email-failure-toast = 邮件发送失败,请重试。
settings-referrals-email-empty-error = 请输入邮箱。
settings-referrals-email-invalid-error = 请确认下列邮箱有效:{ $email }
settings-referrals-reward-intro = 推荐朋友即可获得 Warp 专属周边*
settings-referrals-claimed-count-singular = 当前推荐
settings-referrals-claimed-count-plural = 当前推荐
settings-referrals-terms-link = 部分条款适用。
settings-referrals-terms-contact = { " " }如对推荐计划有任何疑问,请联系 referrals@warp.dev。
settings-referrals-reward-theme = 专属主题
settings-referrals-reward-keycaps = 键帽 + 贴纸
settings-referrals-reward-tshirt = T 恤
settings-referrals-reward-notebook = 笔记本
settings-referrals-reward-cap = 棒球帽
settings-referrals-reward-hoodie = 连帽衫
settings-referrals-reward-hydroflask = 高级 Hydro Flask 水壶
settings-referrals-reward-backpack = 双肩包

# --- ANCHOR-SUB-WARPIFY (agent-settings-warpify) ---
settings-warpify-page-title = Warpify
settings-warpify-description-prefix = 配置 Warp 是否尝试对特定 Shell 执行 "Warpify"(为其添加命令块、输入模式等支持)。
settings-warpify-learn-more = 了解更多
settings-warpify-section-subshells = 子 Shell
settings-warpify-section-subshells-subtitle = 支持的子 Shell:bash、zsh、fish。
settings-warpify-section-ssh = SSH
settings-warpify-section-ssh-subtitle = 对交互式 SSH 会话启用 Warpify。
settings-warpify-added-commands = 已添加的命令
settings-warpify-denylisted-commands = 拒绝列表中的命令
settings-warpify-denylisted-hosts = 拒绝列表中的主机
settings-warpify-command-placeholder = 命令(支持正则)
settings-warpify-host-placeholder = 主机(支持正则)
settings-warpify-enable-ssh = 对 SSH 会话启用 Warpify
settings-warpify-install-ssh-extension = 安装 SSH 扩展
settings-warpify-install-ssh-extension-description = 控制远程主机未安装 Warp 的 SSH 扩展时的安装行为。
settings-warpify-use-tmux = 使用 Tmux Warpify
settings-warpify-tmux-description = tmux ssh 包装器在许多默认方式无效的场景下能正常工作,但可能需要你手动点击按钮才能 Warpify。在新标签页中生效。
settings-warpify-ssh-tmux-toggle-binding-label = 用于 Warpify 的 SSH 会话检测

# --- ANCHOR-SUB-AI-PAGE (agent-settings-ai-page) ---
# 章节 / 副标题
settings-ai-warp-agent-header = Warp 智能体
settings-ai-active-ai-section = 主动 AI
settings-ai-input-section = 输入
settings-ai-mcp-servers-section = MCP 服务器
settings-ai-knowledge-section = 知识库
settings-ai-voice-section = 语音
settings-ai-other-section = 其他
settings-ai-third-party-cli-section = 第三方 CLI 智能体
settings-ai-agent-attribution-section = 智能体署名
settings-ai-experimental-section = 实验性
settings-ai-aws-bedrock-section = AWS Bedrock
settings-ai-agents-header = 智能体
settings-ai-profiles-header = 配置档
settings-ai-models-subheader = 模型
settings-ai-permissions-subheader = 权限
settings-ai-usage-header = 使用量
settings-ai-credits-label = 额度

# 主动 AI 开关标签
settings-ai-next-command-label = 下一条命令
settings-ai-prompt-suggestions-label = 提示建议
settings-ai-suggested-code-banners-label = 代码建议横幅
settings-ai-natural-language-autosuggestions-label = 自然语言自动建议
settings-ai-shared-block-title-generation-label = 共享 Block 标题生成
settings-ai-git-operations-autogen-label = 提交与 Pull Request 生成

# 权限下拉项
settings-ai-permission-agent-decides = 智能体决定
settings-ai-permission-always-allow = 始终允许
settings-ai-permission-always-ask = 始终询问
settings-ai-permission-ask-on-first-write = 首次写入时询问
settings-ai-permission-read-only = 只读
settings-ai-permission-supervised = 受监督
settings-ai-permission-allow-specific-dirs = 在指定目录中允许

# 权限行标签
settings-ai-apply-code-diffs = 应用代码 diff
settings-ai-read-files = 读取文件
settings-ai-execute-commands = 执行命令
settings-ai-interact-running-commands = 与运行中的命令交互
settings-ai-call-mcp-servers = 调用 MCP 服务器
settings-ai-command-denylist = 命令拒绝列表
settings-ai-command-denylist-description = 匹配命令的正则表达式,Warp 智能体执行这些命令前必须征得许可。
settings-ai-command-allowlist = 命令允许列表
settings-ai-command-allowlist-description = 匹配命令的正则表达式,Warp 智能体可自动执行这些命令。
settings-ai-directory-allowlist = 目录允许列表
settings-ai-directory-allowlist-description = 授予智能体对指定目录的文件访问权限。
settings-ai-mcp-allowlist = MCP 允许列表
settings-ai-mcp-allowlist-description = 允许 Warp 智能体调用这些 MCP 服务器。
settings-ai-mcp-denylist = MCP 拒绝列表
settings-ai-mcp-denylist-description = Warp 智能体调用此列表中的任何 MCP 服务器前都必须征得许可。
settings-ai-info-banner-managed-by-workspace = 你的部分权限由工作区管理。

# 模型 / 配置档
settings-ai-base-model = 基础模型
settings-ai-base-model-description = 此模型作为 Warp 智能体背后的主要引擎,驱动大部分交互,并在需要时调用其他模型完成规划或代码生成等任务。Warp 可能根据模型可用性自动切换备用模型,或将其用于会话摘要等辅助任务。
settings-ai-show-model-picker-in-prompt = 在提示中显示模型选择器
settings-ai-codebase-context = 代码库上下文
settings-ai-codebase-context-description = 允许 Warp 智能体生成代码库的概要作为上下文。代码从不存储到我们的服务器。
settings-ai-add-profile = 新建配置档
settings-ai-agents-description = 设定智能体的运行边界:它能访问什么、拥有多少自主权、以及何时必须征得你的同意。你也可以微调自然语言输入、代码库感知等行为。
settings-ai-profiles-description = 配置档让你定义智能体的运行方式 —— 包括它可执行的动作、何时需要审批,以及编码、规划等任务使用的模型。你也可以将其作用于具体项目。

# 匿名 / 组织限制
settings-ai-sign-up = 注册
settings-ai-anonymous-create-account = 要使用 AI 功能,请先创建账户。
settings-ai-org-disallows-remote-session = 当活动窗格包含来自远程会话的内容时,你的组织禁止使用 AI
settings-ai-org-enforced-tooltip = 此选项由你所在组织的设置强制启用,无法自定义。
settings-ai-restricted-billing = 因账单问题受限
settings-ai-unlimited = 不限量

# AI 输入区段
settings-ai-show-input-hint-text = 显示输入提示文本
settings-ai-show-agent-tips = 显示智能体提示
settings-ai-include-agent-commands-in-history = 将智能体执行的命令纳入历史
settings-ai-autodetect-agent-prompts = 在终端输入中自动检测智能体提示
settings-ai-autodetect-terminal-commands = 在智能体输入中自动检测终端命令
settings-ai-natural-language-detection = 自然语言检测
settings-ai-natural-language-denylist = 自然语言拒绝列表
settings-ai-natural-language-denylist-description = 列出的命令永远不会触发自然语言检测。
settings-ai-let-us-know = 告诉我们

# MCP 服务器
settings-ai-learn-more = 了解更多
settings-ai-add-server = 添加服务器
settings-ai-manage-mcp-servers = 管理 MCP 服务器
settings-ai-file-based-mcp-toggle = 从第三方智能体自动启动服务器
settings-ai-file-based-mcp-supported-providers = 查看支持的提供商。
settings-ai-mcp-dropdown-header = 选择 MCP 服务器

# 知识库 / 规则
settings-ai-rules-label = 规则
settings-ai-suggested-rules-label = 规则建议
settings-ai-suggested-rules-description = 让 AI 根据你的交互建议要保存的规则。
settings-ai-manage-rules = 管理规则

# 语音
settings-ai-voice-input-label = 语音输入
settings-ai-voice-key = 激活语音输入的按键
settings-ai-voice-key-hint = 按住以激活。

# 其他区段
settings-ai-show-oz-changelog = 在新会话视图中显示 Oz 更新日志
settings-ai-show-use-agent-footer = 显示"使用智能体"页脚
settings-ai-use-agent-footer-description = 在长时间运行的命令中提示使用启用了"完整终端使用"的智能体。
settings-ai-show-conversation-history = 在工具面板中显示会话历史
settings-ai-thinking-display = 智能体思考显示
settings-ai-thinking-display-description = 控制推理/思考过程的显示方式。
settings-ai-conversation-layout-label = 打开已有智能体会话时的首选布局
settings-ai-conversation-layout-newtab = 新标签页
settings-ai-conversation-layout-splitpane = 拆分窗格
settings-ai-toolbar-layout = 工具栏布局

# 第三方 CLI 智能体
settings-ai-show-coding-agent-toolbar = 显示编码智能体工具栏
settings-ai-auto-show-rich-input = 根据智能体状态自动显示/隐藏富输入
settings-ai-auto-show-rich-input-tooltip = 需要为你的编码智能体安装 Warp 插件
settings-ai-auto-open-rich-input = 编码智能体会话启动时自动打开富输入
settings-ai-auto-dismiss-rich-input = 提交提示后自动关闭富输入
settings-ai-toolbar-commands-label = 启用工具栏的命令
settings-ai-toolbar-commands-description = 添加正则表达式,匹配的命令将显示编码智能体工具栏。
settings-ai-coding-agent-other = 其他
settings-ai-coding-agent-select-header = 选择编码智能体

# 智能体署名
settings-ai-enable-agent-attribution = 启用智能体署名
settings-ai-agent-attribution-description = Oz 可在其创建的提交信息和 Pull Request 中添加署名

# 实验性 / Cloud Agent
settings-ai-cloud-agent-computer-use = 在 Cloud Agent 中启用计算机使用
settings-ai-cloud-agent-computer-use-description = 在 Warp 应用中启动的 Cloud Agent 会话中启用计算机使用。
settings-ai-orchestration-label = 编排
settings-ai-orchestration-description = 启用多智能体编排,允许智能体派生并协调并行的子智能体。

# AWS Bedrock
settings-ai-aws-bedrock-toggle = 使用 AWS Bedrock 凭证
settings-ai-aws-bedrock-description = Warp 加载并发送本地 AWS CLI 凭证以使用 Bedrock 支持的模型。
settings-ai-aws-bedrock-description-managed = Warp 加载并发送本地 AWS CLI 凭证以使用 Bedrock 支持的模型。此设置由你的组织管理。
settings-ai-aws-login-command = 登录命令
settings-ai-aws-profile = AWS Profile
settings-ai-aws-auto-login = 自动运行登录命令
settings-ai-aws-auto-login-description = 启用后,AWS Bedrock 凭证过期时将自动运行登录命令。
settings-ai-refresh = 刷新

# --- ANCHOR-SUB-FEATURES (agent-settings-features) ---
settings-features-category-general = 通用
settings-features-category-session = 会话
settings-features-category-keys = 按键
settings-features-category-text-editing = 文本编辑
settings-features-category-terminal-input = 终端输入
settings-features-category-terminal = 终端
settings-features-category-notifications = 通知
settings-features-category-workflows = 工作流
settings-features-category-system = 系统
settings-features-open-links-in-desktop = 在桌面应用中打开链接
settings-features-open-links-in-desktop-tooltip = 尽可能自动在桌面应用中打开链接。
settings-features-restore-session = 启动时恢复窗口、标签页和面板
settings-features-show-sticky-command-header = 显示固定的命令标题栏
settings-features-show-link-tooltip = 点击链接时显示提示
settings-features-show-quit-warning = 退出/登出前显示警告
settings-features-quit-on-last-window-closed = 关闭所有窗口时退出应用
settings-features-show-changelog-after-update = 更新后显示更新日志提示
settings-features-mouse-scroll-multiplier = 鼠标滚轮每次滚动的行数
settings-features-auto-open-code-review = 自动打开代码评审面板
settings-features-max-rows-per-block = 命令块最大行数
settings-features-ssh-wrapper = Warp SSH 包装器
settings-features-receive-desktop-notifications = 接收来自 Warp 的桌面通知
settings-features-show-in-app-agent-notifications = 显示应用内 Agent 通知
settings-features-confirm-close-shared-session = 关闭共享会话前确认
settings-features-global-hotkey-label = 全局快捷键:
settings-features-global-hotkey-not-supported-on-wayland = Wayland 下不支持。
settings-features-autocomplete-symbols = 自动补全引号、圆括号和方括号
settings-features-error-underlining = 命令错误下划线提示
settings-features-syntax-highlighting = 命令语法高亮
settings-features-completions-while-typing = 输入时自动打开补全菜单
settings-features-command-corrections = 建议修正后的命令
settings-features-expand-aliases = 输入时展开别名
settings-features-middle-click-paste = 中键点击粘贴
settings-features-vim-mode = 使用 Vim 快捷键编辑代码和命令
settings-features-at-context-menu = 在终端模式下启用 “@” 上下文菜单
settings-features-slash-commands-in-terminal = 在终端模式下启用斜杠命令
settings-features-outline-codebase-symbols = 为 “@” 上下文菜单提取代码库符号大纲
settings-features-show-input-message-bar = 显示终端输入消息行
settings-features-show-autosuggestion-hint = 显示自动建议快捷键提示
settings-features-show-autosuggestion-ignore = 显示自动建议忽略按钮
settings-features-enable-mouse-reporting = 启用鼠标事件上报
settings-features-enable-scroll-reporting = 启用滚动事件上报
settings-features-enable-focus-reporting = 启用焦点事件上报
settings-features-use-audible-bell = 启用响铃
settings-features-double-click-smart-selection = 双击智能选择
settings-features-show-help-block-in-new-sessions = 新会话中显示帮助命令块
settings-features-copy-on-select = 选中即复制
settings-features-show-global-workflows-in-command-search = 在命令搜索 (ctrl-r) 中显示全局工作流
settings-features-linux-selection-clipboard = 兼容 Linux 选区剪贴板
settings-features-prefer-low-power-gpu = 新窗口优先使用集成 GPU 渲染(低功耗)
settings-features-use-wayland = 使用 Wayland 进行窗口管理
settings-features-use-wayland-tooltip = 启用 Wayland 支持
settings-features-ctrl-tab-behavior-label = Ctrl+Tab 行为:
settings-features-extra-meta-key-left-mac = 左 Option 键作为 Meta
settings-features-extra-meta-key-right-mac = 右 Option 键作为 Meta
settings-features-extra-meta-key-left-other = 左 Alt 键作为 Meta
settings-features-extra-meta-key-right-other = 右 Alt 键作为 Meta
settings-features-default-shell-header = 新会话默认 shell
settings-features-working-directory-header = 新会话工作目录
settings-features-notify-agent-task-completed = Agent 完成任务时通知
settings-features-notify-needs-attention = 命令或 Agent 需要继续操作时通知
settings-features-play-notification-sounds = 播放通知声音

# --- ANCHOR-SUB-TEAMS (agent-settings-teams) ---
settings-teams-page-title = 团队
settings-teams-create-page-subtitle = 创建团队
settings-teams-create-description = 创建团队后,您可以通过共享云端 agent 运行、环境、自动化与产物来协作进行 agent 驱动开发,也可以为队友与 agent 共享一个统一的知识库。
settings-teams-create-button = 创建
settings-teams-team-name-placeholder = 团队名称
settings-teams-rename-placeholder = 新的团队名称
settings-teams-leave-team-button = 退出团队
settings-teams-delete-team-button = 删除团队
settings-teams-emails-placeholder = 邮箱(逗号分隔)
settings-teams-domains-placeholder = 域名(逗号分隔)
settings-teams-set-button = 设置
settings-teams-invite-button = 邀请
settings-teams-join-button = 加入
settings-teams-contact-admin-button = 联系管理员申请访问
settings-teams-tab-link = 邀请链接
settings-teams-tab-email = 邮箱邀请
settings-teams-section-team-members = 团队成员
settings-teams-section-team-members-pricing = 团队成员
settings-teams-section-invite-by-link = 通过链接邀请
settings-teams-section-invite-by-email = 通过邮箱邀请
settings-teams-section-restrict-by-domain = 按域名限制
settings-teams-section-make-discoverable = 设为可被发现
settings-teams-section-plan-usage-free = 免费方案用量限制
settings-teams-section-plan-usage = 方案用量限制
settings-teams-shared-notebooks = 共享笔记本
settings-teams-shared-workflows = 共享工作流
settings-teams-reset-links = 重置链接
settings-teams-compare-plans = 对比方案
settings-teams-upgrade-build = 升级到 Build
settings-teams-upgrade-turbo = 升级到 Turbo 方案
settings-teams-upgrade-lightspeed = 升级到 Lightspeed 方案
settings-teams-contact-support = 联系支持
settings-teams-manage-billing = 管理账单
settings-teams-manage-plan = 管理方案
settings-teams-open-admin-panel = 打开管理员面板
settings-teams-or-join-existing = 或加入您公司内已有的团队
settings-teams-discovery-cta = 加入此团队,开始一起协作工作流、笔记本等内容。
settings-teams-discovery-1-teammate = 1 名队友
settings-teams-discovery-n-teammates = { $count } 名队友
settings-teams-transfer-modal-title = 转移团队所有权?
settings-teams-action-cancel-invite = 取消邀请
settings-teams-action-transfer-ownership = 转移所有权
settings-teams-action-demote-from-admin = 取消管理员
settings-teams-action-promote-to-admin = 设为管理员
settings-teams-action-remove-from-team = 从团队移除
settings-teams-action-remove-domain = 移除域名
settings-teams-state-expired = 已过期
settings-teams-state-pending = 待处理
settings-teams-state-owner = 拥有者
settings-teams-state-admin = 管理员
settings-teams-badge-past-due = 已逾期
settings-teams-badge-unpaid = 未付款
settings-teams-offline = 您当前处于离线状态。
settings-teams-failed-load-invite-link = 加载邀请链接失败。
settings-teams-toast-link-copied = 链接已复制到剪贴板!
settings-teams-toast-invite-sent-one = 邀请已发出!
settings-teams-toast-invites-sent = 已发出 { $count } 封邀请!
settings-teams-toast-domain-added = 已添加域名限制:{ $count }
settings-teams-toast-invalid-domains = 无效域名:{ $count }
settings-teams-toast-invalid-emails = 无效邮箱:{ $count }
settings-teams-toast-toggled-invite-links = 已切换邀请链接状态
settings-teams-toast-reset-invite-links = 已重置邀请链接
settings-teams-toast-deleted-invite = 已删除邀请
settings-teams-toast-toggled-discoverability = 已切换团队可发现状态
settings-teams-toast-joined-team = 已成功加入团队
settings-teams-toast-joined-team-named = 已成功加入 { $name }
settings-teams-toast-transferred-ownership = 已成功转移团队所有权
settings-teams-toast-updated-role = 已成功更新团队成员角色
settings-teams-toast-left-team = 已成功退出团队
settings-teams-toast-renamed-team = 已成功重命名团队
settings-teams-error-leave-team = 退出团队失败
settings-teams-error-rename-team = 重命名团队失败
settings-teams-error-send-invite = 发送邀请失败
settings-teams-error-toggle-invite-links = 切换邀请链接失败
settings-teams-error-reset-invite-links = 重置邀请链接失败
settings-teams-error-delete-invite = 删除邀请失败
settings-teams-error-add-domain = 添加域名限制失败
settings-teams-error-delete-domain = 删除域名限制失败
settings-teams-error-upgrade-link = 生成升级链接失败,请联系 feedback@warp.dev
settings-teams-error-billing-link = 生成账单链接失败,请联系 feedback@warp.dev
settings-teams-error-toggle-discoverability = 切换团队可发现状态失败
settings-teams-error-join-team = 加入团队失败
settings-teams-error-transfer-ownership = 转移团队所有权失败
settings-teams-error-update-role = 更新团队成员角色失败

# --- ANCHOR-SUB-SETTINGS-PAGE-NAV (agent-settings-page-nav) ---

# ---- settings_page.rs ----
settings-page-info-icon-tooltip = 点击查看文档详情
settings-page-local-only-icon-tooltip = 此设置不会同步到你的其他设备
settings-page-reset-to-default = 重置为默认值

# ---- delete_environment_confirmation_dialog.rs ----
settings-confirm-cancel = 取消
settings-confirm-delete-environment-button = 删除环境
settings-confirm-delete-environment-title = 删除环境?
settings-confirm-delete-environment-description = 确定要删除 { $name } 环境吗?

# ---- directory_color_add_picker.rs ----
settings-color-picker-add-directory-footer = + 添加目录…
settings-color-picker-add-directory-color = 添加目录颜色

# --- ANCHOR-SUB-CODE (agent-settings-code) ---
settings-code-feature-name = 代码
settings-code-initialization-settings-header = 初始化设置
settings-code-codebase-indexing-label = 代码库索引
settings-code-codebase-index-description = Warp 可在你浏览代码仓库时自动索引,帮助 agent 快速理解上下文并给出解决方案。代码从不存储到服务器。即使代码库无法被索引,Warp 仍可通过 grep 与 find 工具调用浏览代码库并获取信息。
settings-code-warp-indexing-ignore-description = 若要将特定文件或目录排除在索引外,请将其添加到仓库目录下的 .warpindexingignore 文件中。这些文件仍可被 AI 功能访问,但不会进入代码库嵌入。
settings-code-auto-index-feature-name = 默认索引新文件夹
settings-code-auto-index-description = 启用后,Warp 将在你浏览代码仓库时自动索引,帮助 agent 快速理解上下文并提供有针对性的解决方案。
settings-code-indexing-disabled-admin = 团队管理员已禁用代码库索引。
settings-code-indexing-workspace-enabled-admin = 团队管理员已启用代码库索引。
settings-code-indexing-disabled-global-ai = 必须启用 AI 功能才能使用代码库索引。
settings-code-codebase-index-limit-reached = 你已达到当前套餐允许的代码库索引上限。请移除现有索引以便自动索引新代码库。
settings-code-subpage-indexing-title = 代码库索引
settings-code-subpage-editor-review-title = 编辑器与代码评审
settings-code-category-codebase-indexing = 代码库索引
settings-code-category-editor-review = 代码编辑器与评审
settings-code-index-new-folder = 索引新文件夹
settings-code-initialized-folders-header = 已初始化 / 已索引文件夹
settings-code-no-folders-initialized = 尚未初始化任何文件夹。
settings-code-open-project-rules = 打开项目规则
settings-code-indexing-section-label = 索引
settings-code-no-index-created = 尚未创建索引
settings-code-discovered-chunks = 已发现 { $total } 个分块
settings-code-syncing-progress = 同步中 - { $completed } / { $total }
settings-code-syncing = 同步中...
settings-code-status-synced = 已同步
settings-code-status-too-large = 代码库过大
settings-code-status-stale = 已过期
settings-code-status-failed = 失败
settings-code-no-index-built = 尚未构建索引
settings-code-lsp-section-label = LSP 服务器
settings-code-lsp-installed = 已安装
settings-code-lsp-installing = 安装中...
settings-code-lsp-checking = 检查中...
settings-code-lsp-available-for-download = 可供下载
settings-code-lsp-restart-server = 重启服务器
settings-code-lsp-view-logs = 查看日志
settings-code-lsp-status-available = 可用
settings-code-lsp-status-busy = 繁忙
settings-code-lsp-status-failed = 失败
settings-code-lsp-status-stopped = 已停止
settings-code-lsp-status-not-running = 未运行
settings-code-auto-open-review-panel = 自动打开代码评审面板
settings-code-auto-open-review-panel-desc = 启用后,代码评审面板将在会话首次接受 diff 时自动打开。
settings-code-show-code-review-button = 显示代码评审按钮
settings-code-show-code-review-button-desc = 在窗口右上角显示用于切换代码评审面板的按钮。
settings-code-show-diff-stats = 在代码评审按钮上显示差异统计
settings-code-show-diff-stats-desc = 在代码评审按钮上显示新增与删除行数。
settings-code-project-explorer = 项目浏览器
settings-code-project-explorer-desc = 在左侧工具面板添加 IDE 风格的项目浏览器 / 文件树。
settings-code-global-search = 全局文件搜索
settings-code-global-search-desc = 在左侧工具面板添加全局文件搜索。

# --- ANCHOR-SUB-PRIVACY (agent-settings-privacy) ---
settings-privacy-page-title = 隐私
settings-privacy-modal-add-regex-title = 添加正则表达式
settings-privacy-safe-mode-title = 敏感信息混淆
settings-privacy-safe-mode-description = 启用此设置后,Warp 会扫描 Block、Warp Drive 对象内容以及 Oz 提示词中可能包含的敏感信息,并阻止将这些数据保存或发送到任何服务器。你可以通过正则表达式自定义匹配规则。
settings-privacy-user-secret-regex-title = 自定义敏感信息混淆
settings-privacy-user-secret-regex-description = 使用正则表达式定义你希望额外混淆的敏感信息或数据。规则将在下一条命令执行时生效。可在正则表达式前加 (?i) 标志使其忽略大小写。
settings-privacy-telemetry-title = 帮助改进 Warp
settings-privacy-telemetry-description = 应用分析数据帮助我们为你改进产品。我们可能会采集部分控制台交互数据,用于改进 Warp 的 AI 能力。
settings-privacy-telemetry-description-old = 应用分析数据帮助我们为你改进产品。我们仅采集应用使用元数据,绝不采集控制台输入或输出内容。
settings-privacy-telemetry-free-tier-note = 免费版必须启用分析才能使用 AI 功能。
settings-privacy-telemetry-docs-link = 详细了解 Warp 如何使用数据
settings-privacy-data-management-title = 管理你的数据
settings-privacy-data-management-description = 你可以随时选择永久删除 Warp 账户。删除后将无法继续使用 Warp。
settings-privacy-data-management-link = 访问数据管理页面
settings-privacy-policy-title = 隐私政策
settings-privacy-policy-link = 阅读 Warp 的隐私政策
settings-privacy-tab-personal = 个人
settings-privacy-tab-enterprise = 企业
settings-privacy-enterprise-readonly = 企业版敏感信息混淆规则不可修改。
settings-privacy-enterprise-empty = 你所在组织尚未配置任何企业版正则规则。
settings-privacy-recommended = 推荐
settings-privacy-add-all = 全部添加
settings-privacy-add-regex-button = 添加正则
settings-privacy-enterprise-enabled-by-org = 已由你所在组织启用。
settings-privacy-zdr-badge = ZDR
settings-privacy-zdr-tooltip = 你的管理员已为团队启用零数据保留。用户生成的内容将永远不会被采集。
settings-privacy-secret-display-mode-title = 敏感信息可视化混淆模式
settings-privacy-secret-display-mode-description = 选择敏感信息在 Block 列表中的视觉呈现方式,同时保持可搜索。此设置仅影响 Block 列表中的显示。
settings-privacy-crash-reports-title = 发送崩溃报告
settings-privacy-crash-reports-description = 崩溃报告有助于排查问题并提升稳定性。
settings-privacy-cloud-conv-title = 将 AI 对话存储到云端
settings-privacy-cloud-conv-description-on = 智能体对话可与他人共享,并在你于其它设备登录时保留。此数据仅用于产品功能,Warp 不会将其用于分析。
settings-privacy-cloud-conv-description-off = 智能体对话仅本地存储,登出后丢失,且无法共享。注意:Ambient Agent 的对话数据仍存储在云端。
settings-privacy-org-managed-tooltip = 此设置由你所在组织管理。
settings-privacy-network-log-title = 网络日志控制台
settings-privacy-network-log-description = 我们构建了原生控制台,允许你查看 Warp 与外部服务器的全部通信,让你确信工作始终安全。
settings-privacy-network-log-link = 查看网络日志

# --- ANCHOR-SUB-EXEC-MODAL-BLOCKS (agent-settings-misc) ---
# ---- execution_profile_view ----
settings-exec-profile-edit-button = 编辑
settings-exec-profile-auto = 自动
settings-exec-profile-section-models = 模型
settings-exec-profile-section-permissions = 权限
settings-exec-profile-base-model = 基础模型:
settings-exec-profile-full-terminal-use = 完整终端使用:
settings-exec-profile-computer-use = 电脑使用:
settings-exec-profile-apply-code-diffs = 应用代码 diff:
settings-exec-profile-read-files = 读取文件:
settings-exec-profile-execute-commands = 执行命令:
settings-exec-profile-interact-running-commands = 与运行中命令交互:
settings-exec-profile-ask-questions = 提问:
settings-exec-profile-call-mcp-servers = 调用 MCP 服务器:
settings-exec-profile-call-web-tools = 调用 Web 工具:
settings-exec-profile-autosync-plans = 自动同步计划到 Warp Drive:
settings-exec-profile-chips-none = 无
settings-exec-profile-perm-agent-decides = Agent 自行决定
settings-exec-profile-perm-always-allow = 始终允许
settings-exec-profile-perm-always-ask = 始终询问
settings-exec-profile-perm-unknown = 未知
settings-exec-profile-perm-ask-on-first-write = 首次写入时询问
settings-exec-profile-perm-never = 从不
settings-exec-profile-perm-never-ask = 从不询问
settings-exec-profile-perm-ask-unless-auto-approve = 除非自动批准否则询问
settings-exec-profile-perm-on = 开
settings-exec-profile-perm-off = 关
settings-exec-profile-directory-allowlist = 目录允许列表:
settings-exec-profile-command-allowlist = 命令允许列表:
settings-exec-profile-command-denylist = 命令拒绝列表:
settings-exec-profile-mcp-allowlist = MCP 允许列表:
settings-exec-profile-mcp-denylist = MCP 拒绝列表:

# ---- agent_assisted_environment_modal ----
settings-env-modal-add-repo = 添加仓库
settings-env-modal-cancel = 取消
settings-env-modal-create-environment = 创建环境
settings-env-modal-selected-repos = 已选仓库
settings-env-modal-no-repos-selected = 尚未选择仓库
settings-env-modal-available-repos = 可用的已索引仓库
settings-env-modal-loading = 正在加载本地已索引仓库…
settings-env-modal-empty-no-indexed = 尚未发现本地已索引仓库。请先索引一个仓库后再试。
settings-env-modal-unavailable-build = 当前构建不支持选择本地仓库。
settings-env-modal-all-selected = 所有本地已索引仓库均已选中。
settings-env-modal-unknown-repo-name = (未知)
settings-env-modal-not-git-repo = 所选文件夹不是 Git 仓库:{ $path }
settings-env-modal-no-directory-selected = 未选择目录
settings-env-modal-dialog-title = 为你的环境选择仓库
settings-env-modal-dialog-description-indexed = 选择本地已索引仓库,为环境创建 agent 提供上下文。
settings-env-modal-dialog-description-default = 选择仓库,为环境创建 agent 提供上下文。

# ---- show_blocks_view ----
settings-show-blocks-page-title = 共享命令块
settings-show-blocks-unshare-menu-item = 取消共享
settings-show-blocks-copy-link = 复制链接
settings-show-blocks-deleting = 正在删除...
settings-show-blocks-executed-on = 执行于:{ $time }
settings-show-blocks-empty = 你还没有任何共享命令块。
settings-show-blocks-loading = 正在获取命令块...
settings-show-blocks-load-failed = 加载命令块失败,请重试。
settings-show-blocks-link-copied = 链接已复制。
settings-show-blocks-unshare-success = 命令块已成功取消共享。
settings-show-blocks-unshare-failed = 取消共享命令块失败,请重试。
settings-show-blocks-confirm-dialog-title = 取消共享命令块
settings-show-blocks-confirm-dialog-text = 确定要取消共享此命令块吗?

    取消后将无法通过链接访问,并将从 Warp 服务器永久删除。
settings-show-blocks-confirm-cancel = 取消
settings-show-blocks-confirm-unshare = 取消共享

# --- ANCHOR-SUB-APPEARANCE (agent-settings-appearance) ---

# Categories
settings-appearance-category-themes = 主题
settings-appearance-category-language = 语言
settings-appearance-category-icon = 图标
settings-appearance-category-window = 窗口
settings-appearance-category-input = 输入
settings-appearance-category-panes = 窗格
settings-appearance-category-blocks = 命令块
settings-appearance-category-text = 文本
settings-appearance-category-cursor = 光标
settings-appearance-category-tabs = 标签页
settings-appearance-category-fullscreen-apps = 全屏应用

# Theme widget
settings-appearance-theme-create-custom = 创建你自己的自定义主题
settings-appearance-theme-mode-light = 浅色
settings-appearance-theme-mode-dark = 深色
settings-appearance-theme-mode-current = 当前主题
settings-appearance-theme-sync-os-label = 跟随系统
settings-appearance-theme-sync-os-description = 当系统切换浅色/深色时自动跟随。

# Custom App Icon widget
settings-appearance-custom-icon-label = 自定义应用图标
settings-appearance-custom-icon-bundle-warning = 修改应用图标需要应用以 bundle 形式运行。
settings-appearance-custom-icon-restart-warning = 你可能需要重启 Warp 才能让 macOS 应用所选图标样式。

# Window widgets
settings-appearance-window-custom-size-label = 以自定义尺寸打开新窗口
settings-appearance-window-columns-label = 列数
settings-appearance-window-rows-label = 行数
settings-appearance-window-opacity-label = 窗口不透明度:
settings-appearance-window-opacity-value = 窗口不透明度:{ $value }
settings-appearance-window-opacity-not-supported = 当前显卡驱动不支持透明效果。
settings-appearance-window-opacity-graphics-warning = 当前图形设置可能不支持透明窗口渲染。
settings-appearance-window-opacity-graphics-warning-hint = 请尝试在 Features > System 中调整图形后端或集成 GPU 设置。
settings-appearance-window-blur-radius = 窗口模糊半径:{ $value }
settings-appearance-window-blur-texture-label = 启用窗口模糊(Acrylic 纹理)
settings-appearance-window-blur-texture-not-supported = 当前硬件可能不支持透明窗口渲染。
settings-appearance-tools-panel-consistent-label = 工具面板在所有标签页保持一致显示

# Input
settings-appearance-input-type-label = 输入类型
settings-appearance-input-type-warp = Warp
settings-appearance-input-type-shell = Shell (PS1)
settings-appearance-input-position-label = 输入位置
settings-appearance-input-mode-pinned-bottom = 固定在底部(Warp 模式)
settings-appearance-input-mode-pinned-top = 固定在顶部(反转模式)
settings-appearance-input-mode-waterfall = 从顶部开始(经典模式)

# Panes
settings-appearance-pane-dim-inactive-label = 暗化非活动窗格
settings-appearance-pane-focus-follows-mouse-label = 焦点跟随鼠标

# Blocks
settings-appearance-block-compact-label = 紧凑模式
settings-appearance-block-jump-bottom-label = 显示「跳到命令块底部」按钮
settings-appearance-block-show-dividers-label = 显示命令块分隔线

# Text / Fonts
settings-appearance-font-agent-label = Agent 字体
settings-appearance-font-match-terminal = 匹配终端
settings-appearance-font-terminal-label = 终端字体
settings-appearance-font-view-all-system = 查看所有可用系统字体
settings-appearance-font-weight-label = 字重
settings-appearance-font-size-label = 字号(像素)
settings-appearance-font-line-height-label = 行高
settings-appearance-font-reset-default = 恢复默认
settings-appearance-font-notebook-size-label = 笔记本字号
settings-appearance-font-thin-strokes-label = 使用细笔画
settings-appearance-font-thin-strokes-never = 从不
settings-appearance-font-thin-strokes-low-dpi = 仅低 DPI 显示器
settings-appearance-font-thin-strokes-high-dpi = 仅高 DPI 显示器
settings-appearance-font-thin-strokes-always = 始终
settings-appearance-font-min-contrast-label = 强制最低对比度
settings-appearance-font-min-contrast-always = 始终
settings-appearance-font-min-contrast-named-only = 仅命名颜色
settings-appearance-font-min-contrast-never = 从不
settings-appearance-font-ligatures-label = 在终端显示连字
settings-appearance-font-ligatures-perf-tooltip = 连字可能影响性能

# Cursor
settings-appearance-cursor-type-label = 光标类型
settings-appearance-cursor-disabled-vim = Vim 模式下光标类型已禁用
settings-appearance-cursor-blink-label = 闪烁光标

# Tabs
settings-appearance-tab-close-position-label = 标签页关闭按钮位置
settings-appearance-tab-close-position-right = 右侧
settings-appearance-tab-close-position-left = 左侧
settings-appearance-tab-show-indicators-label = 显示标签页指示器
settings-appearance-tab-show-code-review-label = 显示代码审查按钮
settings-appearance-tab-preserve-active-color-label = 新标签页保留当前标签页颜色
settings-appearance-tab-vertical-layout-label = 使用垂直标签页布局
settings-appearance-tab-use-prompt-as-title-label = 在标签页名称中使用最近的用户提示作为对话标题
settings-appearance-tab-use-prompt-as-title-description = 在垂直标签页中,对 Oz 与第三方 agent 会话显示最近的用户提示,而不是生成的对话标题。
settings-appearance-tab-toolbar-layout-label = 标题栏工具条布局
settings-appearance-tab-directory-colors-label = 目录标签页颜色
settings-appearance-tab-directory-colors-description = 根据当前目录或仓库自动为标签页着色。
settings-appearance-tab-directory-color-default-tooltip = 默认(无颜色)
settings-appearance-zen-mode-label = 显示标签栏
settings-appearance-zen-decoration-always = 始终
settings-appearance-zen-decoration-windowed = 仅在窗口模式
settings-appearance-zen-decoration-on-hover = 仅悬停时

# Full-screen apps
settings-appearance-alt-screen-padding-label = 在 alt 屏幕中使用自定义内边距
settings-appearance-alt-screen-uniform-padding-label = 统一内边距(像素)

# Zoom
settings-appearance-zoom-label = 缩放
settings-appearance-zoom-secondary = 调整所有窗口的默认缩放级别

# --- ANCHOR-SUB-ENVIRONMENTS (agent-settings-environments) ---
settings-environments-page-title = 环境
settings-environments-page-description = 环境定义了你的 ambient agent 在哪里运行。可通过 GitHub(推荐)、Warp 辅助配置或手动配置在几分钟内创建一个。
settings-environments-search-placeholder = 搜索环境...
settings-environments-no-matches = 没有符合搜索条件的环境。
settings-environments-section-personal = 个人
settings-environments-section-team-default = 由 Warp 和你的团队共享
settings-environments-section-team-named = 由 Warp 和 { $team } 共享
settings-environments-env-id-prefix = 环境 ID:{ $id }
settings-environments-detail-image = 镜像:{ $image }
settings-environments-detail-repos = 仓库:{ $repos }
settings-environments-detail-setup-commands = 初始命令:{ $commands }
settings-environments-last-edited = 最近编辑:{ $time }
settings-environments-last-used = 最近使用:{ $time }
settings-environments-last-used-never = 最近使用:从未
settings-environments-view-my-runs = 查看我的运行记录
settings-environments-tooltip-share = 分享
settings-environments-tooltip-edit = 编辑
settings-environments-empty-header = 你还没有创建任何环境。
settings-environments-empty-subheader = 选择你想要的环境创建方式:
settings-environments-empty-quick-setup-title = 快速创建
settings-environments-empty-suggested-badge = 推荐
settings-environments-empty-quick-setup-subtitle = 选择你想要使用的 GitHub 仓库,我们会为你建议基础镜像与配置
settings-environments-empty-use-agent-title = 使用 agent
settings-environments-empty-use-agent-subtitle = 选择一个本地已配置好的项目,我们会基于它帮你创建环境
settings-environments-button-loading = 加载中...
settings-environments-button-retry = 重试
settings-environments-button-authorize = 授权
settings-environments-button-get-started = 开始使用
settings-environments-button-launch-agent = 启动 agent
settings-environments-toast-update-success = 环境更新成功
settings-environments-toast-create-success = 环境创建成功
settings-environments-toast-delete-success = 环境删除成功
settings-environments-toast-share-success = 环境分享成功
settings-environments-toast-share-failure = 环境分享给团队失败
settings-environments-toast-create-not-logged-in = 无法创建环境:未登录。
settings-environments-toast-save-not-found = 无法保存:环境已不存在。
settings-environments-toast-share-no-team = 无法分享环境:你当前未加入任何团队。
settings-environments-toast-share-not-synced = 无法分享环境:环境尚未同步。

# --- ANCHOR-SUB-AGENT-PROVIDERS (agent-settings-agent-providers) ---
settings-agent-providers-title = Agent 提供商
settings-agent-providers-description = 配置自定义 OpenAI 兼容的 Agent 提供商(如 DeepSeek、智谱 GLM、Moonshot、通义千问 DashScope、SiliconFlow、OpenRouter、本地 Ollama 等)。可以手动添加模型(显示名 + 模型 ID 映射),也可以从 API 自动抓取。提供商元数据存储在本地 settings.toml,API 密钥安全存储在系统密钥库。
settings-agent-providers-empty = 尚未配置任何提供商。点击下面按钮添加。
settings-agent-providers-add-button = + 添加 OpenAI 兼容提供商
settings-agent-providers-search-placeholder = 搜索提供商…
settings-agent-providers-quick-add-title = 快速添加
settings-agent-providers-refresh-catalog = 刷新目录
settings-agent-providers-loading-catalog = 正在拉取 models.dev 目录…(第一次可能需要几秒)
settings-agent-providers-catalog-empty = models.dev 目录为空,点 [刷新目录] 重试。
settings-agent-providers-no-match = 无匹配 "{ $query }"
settings-agent-providers-collapse = 收起 ▲
settings-agent-providers-expand-remaining = 展开剩余 { $count } 个 ▼
settings-agent-providers-row-missing = (此提供商还未关联编辑器: { $id })
settings-agent-providers-field-name = 名称
settings-agent-providers-field-base-url = 接口地址
settings-agent-providers-field-api-key = API 密钥
settings-agent-providers-field-api-type = API 协议
settings-agent-providers-api-type-hint = (genai 据此显式绑定 adapter,避免按模型名误识别。接口地址留空将使用默认: { $url })
settings-agent-providers-name-placeholder = 自定义提供商名称(例如: DeepSeek、本地 Ollama)
settings-agent-providers-api-key-placeholder = sk-... (失焦或按 Enter 保存到系统密钥库)
settings-agent-providers-models-label = 模型列表 ({ $count } 个)
settings-agent-providers-models-empty-hint = 还未配置模型。点 [+ 添加模型] 手动添加,或点 [Fetch from API] 自动抓取。
settings-agent-providers-models-header-name = 显示名
settings-agent-providers-models-header-id = 模型 ID
settings-agent-providers-models-header-context = 上下文 (tok)
settings-agent-providers-models-header-output = 输出 (tok)
settings-agent-providers-model-name-placeholder = 显示名(例如: DS-V3 通用)
settings-agent-providers-model-id-placeholder = 模型 ID(发给 API 的 model 字段, 如: deepseek-chat)
settings-agent-providers-model-context-placeholder = 上下文 (tokens)
settings-agent-providers-model-output-placeholder = 输出 (tokens)
settings-agent-providers-add-model = + 添加模型
settings-agent-providers-fetch-from-api = 从 API 抓取
settings-agent-providers-sync-models-dev = 从 models.dev 同步
settings-agent-providers-remove = 移除

# ---- AI 子页 ----
settings-ai-title = AI
settings-ai-active-ai = 主动 AI
settings-ai-input-autodetection = 智能体输入框中的终端命令自动识别
settings-ai-input-autodetection-legacy = 自然语言识别
settings-ai-next-command-description = 根据你的历史命令、输出与常见工作流,让 AI 推荐下一条要执行的命令。
settings-ai-prompt-suggestions-description = 让 AI 根据最近命令与输出,在输入框中以行内横幅形式建议自然语言提示。
settings-ai-suggested-code-banners-description = 让 AI 根据最近命令与输出,在命令块列表中以行内横幅形式建议代码差异与查询。
settings-ai-natural-language-autosuggestions = 让 AI 根据最近命令与输出,提供自然语言自动建议。
settings-ai-shared-block-title-generation-description = 让 AI 根据命令与输出为共享命令块生成标题。
settings-ai-git-operations-autogen-description = 让 AI 自动生成提交信息以及 Pull Request 的标题与描述。

# =============================================================================
# 其余 surface 章节缺失 key 会自动 fallback 到英文,见 en/warp.ftl
# =============================================================================

# =============================================================================
# SECTION: banner (Owner: agent-banner)
# Files: app/src/banner/**
# =============================================================================

banner-dont-show-again = 不再显示


# =============================================================================
# SECTION: quit-warning (Owner: agent-quit-warning)
# Files: app/src/quit_warning/mod.rs
# =============================================================================

# ---- 对话框标题 ----
quit-warning-title-pane = 关闭窗格?
quit-warning-title-tab-singular = 关闭标签页?
quit-warning-title-tab-plural = 关闭标签页?
quit-warning-title-window = 关闭窗口?
quit-warning-title-app = 退出 Warp?
quit-warning-title-editor-tab = 保存更改?

# ---- 按钮 ----
quit-warning-button-confirm-close = 确定关闭
quit-warning-button-confirm-quit = 确定退出
quit-warning-button-save = 保存
quit-warning-button-discard = 不保存
quit-warning-button-show-processes = 查看运行中的进程
quit-warning-button-cancel = 取消

# ---- 提示正文 ----
quit-warning-suffix-tab = { " " }(此标签页)。
quit-warning-suffix-window = { " " }(此窗口)。
quit-warning-suffix-pane = { " " }(此窗格)。
quit-warning-suffix-default = 。

quit-warning-processes-running = 你有 { $count } 个进程正在运行
quit-warning-processes-in-windows = ,分布在 { $count } 个窗口
quit-warning-processes-in-tabs = ,分布在 { $count } 个标签页

quit-warning-shared-sessions = 你正在共享 { $count } 个会话

quit-warning-unsaved-changes = 你有未保存的文件更改

quit-warning-unsaved-editor-tab = 是否保存对 { $file } 所做的更改?如果不保存,这些更改将被丢弃。
quit-warning-unsaved-editor-tab-fallback-name = 此文件
