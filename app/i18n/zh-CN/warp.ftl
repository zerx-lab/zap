# Warp 桌面端 — 简体中文
# 缺失的 key 会自动 fallback 到 en/warp.ftl,所以可以分批补译。
# 术语统一:Agent → 智能体 / Block → 命令块 / Drive → 云盘 / Workflow → 工作流 / Profile → 配置

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
settings-section-agent-profiles = 配置
settings-section-agent-mcp-servers = MCP 服务器
settings-section-agent-providers = 提供商
settings-section-knowledge = Rules
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

# ---- destructive_mcp_confirmation_dialog.rs ----
settings-mcp-confirm-delete-local-title = 删除 MCP 服务器?
settings-mcp-confirm-delete-local-description = 这将从你所有的设备上卸载并移除此 MCP 服务器。
settings-mcp-confirm-delete-shared-title = 删除共享的 MCP 服务器?
settings-mcp-confirm-delete-shared-description = 此操作不仅会为你自己删除此 MCP 服务器,还会从 Warp 以及所有团队成员的设备上卸载并移除此 MCP 服务器。
settings-mcp-confirm-unshare-title = 从团队中移除共享的 MCP 服务器?
settings-mcp-confirm-unshare-description = 这将从 Warp 以及所有团队成员的设备上卸载并移除此 MCP 服务器。
settings-mcp-confirm-delete-button = 删除 MCP
settings-mcp-confirm-remove-from-team-button = 从团队移除
settings-mcp-confirm-cancel-button = 取消

# ---- edit_page.rs ----
settings-mcp-edit-save = 保存
settings-mcp-edit-edit-variables = 编辑变量
settings-mcp-edit-delete = 删除 MCP
settings-mcp-edit-remove-from-team = 从团队移除
settings-mcp-edit-editing-disabled-banner = 仅团队管理员和 MCP 服务器的创建者可以编辑此 MCP 服务器。
settings-mcp-edit-add-new-title = 添加新 MCP 服务器
settings-mcp-edit-edit-named-title = 编辑 { $name } MCP 服务器
settings-mcp-edit-edit-title = 编辑 MCP 服务器
settings-mcp-edit-logout-tooltip = 登出
settings-mcp-edit-secrets-error = 此 MCP 服务器包含敏感信息。请前往 设置 > 隐私 修改你的敏感信息脱敏设置。
settings-mcp-edit-no-server-error = 未指定 MCP 服务器。
settings-mcp-edit-multiple-servers-error = 编辑单个服务器时无法添加多个 MCP 服务器。

# ---- installation_modal.rs ----
settings-mcp-install-modal-title = 安装 { $name }
settings-mcp-install-modal-source-shared = 团队共享
settings-mcp-install-modal-source-other-device = 来自其他设备
settings-mcp-install-modal-cancel = 取消
settings-mcp-install-modal-install = 安装
settings-mcp-install-modal-no-server = 未选择 MCP 服务器

# ---- list_page.rs ----
settings-mcp-list-description = 添加 MCP 服务器以扩展 Warp Agent 的能力。MCP 服务器通过标准化接口向 agent 暴露数据源或工具,本质上类似插件。你可以添加自定义服务器,或使用预设快速开始使用流行的服务器。你也可以在此找到团队共享给你的服务器。
settings-mcp-list-learn-more = 了解更多。
settings-mcp-list-empty-state = 添加 MCP 服务器后,它将显示在此处。
settings-mcp-list-no-search-results = 未找到搜索结果
settings-mcp-list-search-placeholder = 搜索 MCP 服务器
settings-mcp-list-add-button = 添加
settings-mcp-list-file-based-toggle-label = 自动启动来自第三方 agent 的服务器
settings-mcp-list-file-based-description = 自动检测并启动来自全局范围的第三方 AI agent 配置文件(例如位于你的主目录)中的 MCP 服务器。在仓库内部检测到的服务器永远不会自动启动,必须在下方"检测自"分组中单独启用。
settings-mcp-list-file-based-supported-providers = 查看支持的 provider。
settings-mcp-list-template-available-to-install = 可安装
settings-mcp-list-file-based-detected = 来自配置文件的检测
settings-mcp-list-toast-server-updated = MCP 服务器已更新
settings-mcp-list-section-my-mcps = 我的 MCP
settings-mcp-list-section-shared-by-warp-and-team = 由 Warp 和 { $name } 共享
settings-mcp-list-section-shared-by-warp-and-other-devices = 由 Warp 和其他设备共享
settings-mcp-list-section-shared-from-warp = 来自 Warp 的共享
settings-mcp-list-section-detected-from = 检测自 { $provider }
settings-mcp-list-chip-global = 全局
settings-mcp-list-chip-shared-by-creator = 由 { $creator } 共享
settings-mcp-list-chip-shared-by-team-member = 由团队成员共享
settings-mcp-list-chip-from-another-device = 来自其他设备

# ---- server_card.rs ----
settings-mcp-card-tooltip-show-logs = 查看日志
settings-mcp-card-tooltip-log-out = 登出
settings-mcp-card-tooltip-share-server = 共享服务器
settings-mcp-card-tooltip-edit = 编辑
settings-mcp-card-tooltip-update-available = 有可用的服务器更新
settings-mcp-card-button-view-logs = 查看日志
settings-mcp-card-button-edit-config = 编辑配置
settings-mcp-card-button-set-up = 设置
settings-mcp-card-tools-none = 暂无可用工具
settings-mcp-card-tools-available = { $count } 个可用工具
settings-mcp-card-status-offline = 离线
settings-mcp-card-status-starting = 正在启动服务器…
settings-mcp-card-status-authenticating = 正在认证…
settings-mcp-card-status-shutting-down = 正在关闭…

# ---- update_modal.rs ----
settings-mcp-update-modal-default-name = 服务器
settings-mcp-update-modal-title = 更新 { $name }
settings-mcp-update-modal-description = 此服务器有 { $count } 个可用更新,你想使用哪一个?
settings-mcp-update-modal-publisher-another-device = 其他设备
settings-mcp-update-modal-publisher-team-member = 团队成员
settings-mcp-update-modal-update-from = 来自 { $publisher } 的更新
settings-mcp-update-modal-version = 版本 { $version }
settings-mcp-update-modal-cancel = 取消
settings-mcp-update-modal-update = 更新
settings-mcp-update-modal-no-updates = 暂无可用更新

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
settings-ai-knowledge-section = Rules
settings-ai-voice-section = 语音
settings-ai-other-section = 其他
settings-ai-third-party-cli-section = 第三方 CLI 智能体
settings-ai-experimental-section = 实验性
settings-ai-aws-bedrock-section = AWS Bedrock
settings-ai-agents-header = 智能体
settings-ai-profiles-header = 配置
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

# 模型 / 配置
settings-ai-base-model = 基础模型
settings-ai-base-model-description = 此模型作为 Warp 智能体背后的主要引擎,驱动大部分交互,并在需要时调用其他模型完成规划或代码生成等任务。Warp 可能根据模型可用性自动切换备用模型,或将其用于会话摘要等辅助任务。
settings-ai-show-model-picker-in-prompt = 在提示中显示模型选择器
settings-ai-codebase-context = 代码库上下文
settings-ai-codebase-context-description = 允许 Warp 智能体生成代码库的概要作为上下文。代码从不存储到我们的服务器。
settings-ai-add-profile = 新建配置
settings-ai-agents-description = 设定智能体的运行边界:它能访问什么、拥有多少自主权、以及何时必须征得你的同意。你也可以微调自然语言输入、代码库感知等行为。
settings-ai-profiles-description = 配置让你定义智能体的运行方式 —— 包括它可执行的动作、何时需要审批,以及编码、规划等任务使用的模型。你也可以将其作用于具体项目。

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
settings-ai-rules-description = 规则帮助 Warp 智能体遵循你的约定,无论是针对代码库还是特定工作流。

# 语音
settings-ai-voice-input-label = 语音输入
settings-ai-voice-key = 激活语音输入的按键
settings-ai-voice-key-hint = 按住以激活。

# 其他区段
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
settings-features-default-session-mode = 新会话默认模式
settings-features-block-rows-description = 将上限设置为超过 10 万行可能影响性能。最大支持行数为 { $max_rows }。
settings-features-toast-duration-label = Toast 通知保持显示时长
settings-features-tab-key-behavior = Tab 键行为
settings-features-graphics-backend-label = 首选图形后端
settings-features-graphics-backend-current = 当前后端:{ $backend }
settings-features-working-dir-home = 用户主目录
settings-features-working-dir-previous = 上一个会话的目录
settings-features-working-dir-custom = 自定义目录
settings-features-undo-close-enable = 启用重新打开已关闭的会话
settings-features-undo-close-grace-period = 宽限期(秒)

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

# ---- settings_file_footer.rs ----
settings-footer-open-file = 打开设置文件
settings-footer-alert-open-file = 打开文件
settings-footer-alert-fix-with-oz = 用 Oz 修复

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
settings-exec-profile-title-model = 标题生成:
settings-exec-profile-active-ai-model = 主动式 AI:
settings-exec-profile-next-command-model = Next Command:
settings-exec-profile-computer-use = 电脑使用:
settings-exec-profile-apply-code-diffs = 应用代码 diff:
settings-exec-profile-read-files = 读取文件:
settings-exec-profile-execute-commands = 执行命令:
settings-exec-profile-interact-running-commands = 与运行中命令交互:
settings-exec-profile-ask-questions = 提问:
settings-exec-profile-call-mcp-servers = 调用 MCP 服务器:
settings-exec-profile-call-web-tools = 调用 Web 工具:
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

# ---- execution_profile_editor (Profile Editor pane) ----
settings-exec-profile-editor-header = 配置文件编辑器
settings-exec-profile-editor-title = 编辑配置文件
settings-exec-profile-editor-name-label = 名称
settings-exec-profile-editor-default-name-info = 默认配置文件的名称无法修改。
settings-exec-profile-editor-workspace-override-tooltip = 该选项由你所在组织的设置强制指定,无法自定义。
settings-exec-profile-editor-section-models = 模型
settings-exec-profile-editor-section-permissions = 权限
settings-exec-profile-editor-base-model = 基础模型
settings-exec-profile-editor-base-model-desc = 该模型作为智能体的主要引擎,驱动绝大多数交互,并在需要时调用其他模型完成规划或代码生成等任务。Warp 可能基于模型可用性或辅助任务(如对话摘要)自动切换到备选模型。
settings-exec-profile-editor-full-terminal-use-model = 完整终端使用模型
settings-exec-profile-editor-full-terminal-use-model-desc = 智能体在交互式终端应用(如数据库 shell、调试器、REPL、开发服务器)内运行时使用的模型——读取实时输出并向 PTY 写入命令。
settings-exec-profile-editor-title-model = 标题生成模型
settings-exec-profile-editor-title-model-desc = 用于生成简洁对话标题的模型。默认沿用基础模型——在此选择更便宜 / 更快的模型可在不影响智能体主推理的前提下节省标题摘要的 token。
settings-exec-profile-editor-active-ai-model = 主动式 AI 模型
settings-exec-profile-editor-active-ai-model-desc = 主动式 AI 功能使用的模型:命令完成后的提示建议、智能体输入框中的自然语言自动补全,以及代码库相关性排序。默认沿用基础模型——在此选择小型 / 快速模型可让这些功能保持流畅,而不影响智能体的主推理。
settings-exec-profile-editor-next-command-model = Next Command 模型
settings-exec-profile-editor-next-command-model-desc = 用于预测你下一条 shell 命令的模型(灰色行内自动建议 + 块结束后的零状态建议)。对延迟敏感——请选择你拥有的最小 / 最快 BYOP 模型。默认沿用基础模型。
settings-exec-profile-editor-computer-use-model = 电脑使用模型
settings-exec-profile-editor-computer-use-model-desc = 智能体接管你的电脑、通过鼠标移动、点击和键盘输入与图形化应用交互时使用的模型。
settings-exec-profile-editor-apply-code-diffs = 应用代码 diff
settings-exec-profile-editor-read-files = 读取文件
settings-exec-profile-editor-execute-commands = 执行命令
settings-exec-profile-editor-interact-running-commands = 与运行中命令交互
settings-exec-profile-editor-computer-use = 电脑使用
settings-exec-profile-editor-ask-questions = 提问
settings-exec-profile-editor-call-mcp-servers = 调用 MCP 服务器
settings-exec-profile-editor-call-web-tools = 调用 Web 工具
settings-exec-profile-editor-call-web-tools-desc = 智能体可在有助于完成任务时使用 Web 搜索。
settings-exec-profile-editor-directory-allowlist = 目录允许列表
settings-exec-profile-editor-directory-allowlist-desc = 授予智能体对特定目录的文件访问权限。
settings-exec-profile-editor-command-allowlist = 命令允许列表
settings-exec-profile-editor-command-allowlist-desc = 用于匹配可被 Oz 自动执行的命令的正则表达式。
settings-exec-profile-editor-command-denylist = 命令拒绝列表
settings-exec-profile-editor-command-denylist-desc = 用于匹配 Oz 必须每次询问权限才能执行的命令的正则表达式。
settings-exec-profile-editor-mcp-allowlist = MCP 允许列表
settings-exec-profile-editor-mcp-allowlist-desc = 允许被 Oz 调用的 MCP 服务器。
settings-exec-profile-editor-mcp-denylist = MCP 拒绝列表
settings-exec-profile-editor-mcp-denylist-desc = 不允许被 Oz 调用的 MCP 服务器。

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
settings-agent-providers-description = 配置自定义 Agent 提供商,支持多协议——OpenAI 兼容(DeepSeek、智谱 GLM、Moonshot、通义千问 DashScope、SiliconFlow、OpenRouter 等)、Anthropic、Gemini、本地 Ollama。可以手动添加模型(显示名 + 模型 ID 映射),也可以从 API 自动抓取。提供商元数据存储在本地 settings.toml,API 密钥安全存储在系统密钥库。
settings-agent-providers-empty = 尚未配置任何提供商。点击右上角 [+ 添加提供商] 按钮添加。
settings-agent-providers-add-button = + 添加提供商
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

# --- ANCHOR-SUB-RULES-PAGE (agent-rules-page) ---
# Manage Rules 页面(Warp Drive 中的 AI Fact Collection)。
rules-collection-name = 规则

# --- ANCHOR-SUB-KEYBINDING-DESC (agent-keybinding-descriptions) ---
# 键盘快捷键设置页 / 命令面板里每个 binding 的 description 文案。
# binding name(如 `workspace:open_settings_file`)是协议字段(用户自定义快捷键持久化用),
# **不翻译**。

# 标签页 / 会话
keybinding-desc-workspace-cycle-next-session = 切换到下一个标签页
keybinding-desc-workspace-cycle-prev-session = 切换到上一个标签页
keybinding-desc-workspace-add-window = 创建新窗口
keybinding-desc-workspace-new-file = 新建文件
keybinding-desc-workspace-zoom-in = 放大
keybinding-desc-workspace-zoom-out = 缩小
keybinding-desc-workspace-reset-zoom = 重置缩放
keybinding-desc-workspace-increase-font-size = 增大字号
keybinding-desc-workspace-decrease-font-size = 减小字号
keybinding-desc-workspace-reset-font-size = 重置字号为默认值
keybinding-desc-workspace-increase-zoom = 放大缩放级别
keybinding-desc-workspace-decrease-zoom = 缩小缩放级别
keybinding-desc-workspace-reset-zoom-level = 重置缩放级别为默认值
keybinding-desc-workspace-save-launch-config = 保存新启动配置

# 项目浏览器 / 面板
keybinding-desc-workspace-toggle-project-explorer = 切换项目浏览器
keybinding-desc-workspace-toggle-project-explorer-menu = 项目浏览器
keybinding-desc-workspace-show-theme-chooser = 打开主题选择器
keybinding-desc-workspace-toggle-tab-configs-menu = 打开标签页配置菜单

# 切换到第 N 个标签页
keybinding-desc-workspace-activate-1st-tab = 切换到第 1 个标签页
keybinding-desc-workspace-activate-2nd-tab = 切换到第 2 个标签页
keybinding-desc-workspace-activate-3rd-tab = 切换到第 3 个标签页
keybinding-desc-workspace-activate-4th-tab = 切换到第 4 个标签页
keybinding-desc-workspace-activate-5th-tab = 切换到第 5 个标签页
keybinding-desc-workspace-activate-6th-tab = 切换到第 6 个标签页
keybinding-desc-workspace-activate-7th-tab = 切换到第 7 个标签页
keybinding-desc-workspace-activate-8th-tab = 切换到第 8 个标签页
keybinding-desc-workspace-activate-last-tab = 切换到最后一个标签页
keybinding-desc-workspace-activate-prev-tab = 激活上一个标签页
keybinding-desc-workspace-activate-next-tab = 激活下一个标签页

# 窗格导航
keybinding-desc-pane-group-navigate-prev = 激活上一个窗格
keybinding-desc-pane-group-navigate-next = 激活下一个窗格

# 鼠标 / 笔记本 / 工作流 / 文件夹
keybinding-desc-workspace-toggle-mouse-reporting = 切换鼠标报告
keybinding-desc-workspace-create-team-notebook = 新建团队笔记本
keybinding-desc-workspace-create-team-notebook-menu = 新建团队笔记本
keybinding-desc-workspace-create-personal-notebook = 新建个人笔记本
keybinding-desc-workspace-create-personal-notebook-menu = 新建个人笔记本
keybinding-desc-workspace-create-team-workflow = 新建团队工作流
keybinding-desc-workspace-create-team-workflow-menu = 新建团队工作流
keybinding-desc-workspace-create-personal-workflow = 新建个人工作流
keybinding-desc-workspace-create-personal-workflow-menu = 新建个人工作流
keybinding-desc-workspace-create-team-folder = 新建团队文件夹
keybinding-desc-workspace-create-team-folder-menu = 新建团队文件夹
keybinding-desc-workspace-create-personal-folder = 新建个人文件夹
keybinding-desc-workspace-create-personal-folder-menu = 新建个人文件夹

# 新建标签页变体
keybinding-desc-workspace-new-tab = 创建新标签页
keybinding-desc-workspace-new-terminal-tab = 新建终端标签页
keybinding-desc-workspace-new-agent-tab = 新建 Agent 标签页
keybinding-desc-workspace-new-cloud-agent-tab = 新建云 Agent 标签页

# 左 / 右面板切换
keybinding-desc-workspace-toggle-left-panel = 打开左侧面板
keybinding-desc-workspace-toggle-right-panel = 切换代码评审
keybinding-desc-workspace-toggle-right-panel-menu = 切换代码评审
keybinding-desc-workspace-toggle-vertical-tabs = 切换垂直标签页面板
keybinding-desc-workspace-toggle-vertical-tabs-menu = 切换垂直标签页面板
keybinding-desc-workspace-left-panel-agent-conversations = 左侧面板:Agent 对话
keybinding-desc-workspace-left-panel-project-explorer = 左侧面板:项目浏览器
keybinding-desc-workspace-left-panel-global-search = 左侧面板:全局搜索
keybinding-desc-workspace-left-panel-warp-drive = 左侧面板:Warp Drive
keybinding-desc-workspace-open-global-search = 打开全局搜索
keybinding-desc-workspace-open-global-search-menu = 全局搜索
keybinding-desc-workspace-toggle-warp-drive = 切换 Warp Drive
keybinding-desc-workspace-toggle-warp-drive-menu = Warp Drive
keybinding-desc-workspace-toggle-conversation-list-view = 切换 Agent 对话列表视图
keybinding-desc-workspace-toggle-conversation-list-view-menu = Agent 对话列表视图
keybinding-desc-workspace-close-panel = 关闭聚焦面板

# 命令面板 / 导航
keybinding-desc-workspace-toggle-command-palette = 切换命令面板
keybinding-desc-workspace-toggle-command-palette-menu = 命令面板
keybinding-desc-workspace-toggle-navigation-palette = 切换导航面板
keybinding-desc-workspace-toggle-navigation-palette-menu = 导航面板
keybinding-desc-workspace-toggle-launch-config-palette = 启动配置面板
keybinding-desc-workspace-toggle-files-palette = 切换文件面板
keybinding-desc-workspace-search-drive = 搜索 Warp Drive
keybinding-desc-workspace-move-tab-left = 标签页左移
keybinding-desc-workspace-move-tab-up = 标签页上移
keybinding-desc-workspace-move-tab-right = 标签页右移
keybinding-desc-workspace-move-tab-down = 标签页下移

# 快捷键设置
keybinding-desc-workspace-toggle-keybindings-page = 切换键盘快捷键
keybinding-desc-workspace-show-keybinding-settings = 打开快捷键编辑器
keybinding-desc-workspace-toggle-block-snackbar = 切换粘性命令头

# 窗口 / 标签页关闭
keybinding-desc-workspace-rename-active-tab = 重命名当前标签页
keybinding-desc-workspace-terminate-app = 退出 Warp
keybinding-desc-workspace-close-window = 关闭窗口
keybinding-desc-workspace-close-active-tab = 关闭当前标签页
keybinding-desc-workspace-close-other-tabs = 关闭其他标签页
keybinding-desc-workspace-close-tabs-right = 关闭右侧的标签页
keybinding-desc-workspace-close-tabs-below = 关闭下方的标签页

# 通知
keybinding-desc-workspace-toggle-notifications-on = 开启通知
keybinding-desc-workspace-toggle-notifications-off = 关闭通知

# 更新 / 更新日志
keybinding-desc-workspace-update-and-relaunch = 安装更新并重启
keybinding-desc-workspace-check-for-updates = 检查更新
keybinding-desc-workspace-view-changelog = 查看最新更新日志

# 资源中心 / Drive 导出 / CLI
keybinding-desc-workspace-toggle-resource-center = 切换资源中心
keybinding-desc-workspace-export-all-warp-drive-objects = 导出所有 Warp Drive 对象
keybinding-desc-workspace-install-cli = 安装 Oz CLI 命令
keybinding-desc-workspace-uninstall-cli = 卸载 Oz CLI 命令

# AI 助手 / agent
keybinding-desc-workspace-toggle-ai-assistant = 切换 Warp AI

# 环境变量 / Prompt
keybinding-desc-workspace-create-team-env-vars = 新建团队环境变量
keybinding-desc-workspace-create-team-env-vars-menu = 新建团队环境变量
keybinding-desc-workspace-create-personal-env-vars = 新建个人环境变量
keybinding-desc-workspace-create-personal-env-vars-menu = 新建个人环境变量
keybinding-desc-workspace-create-personal-ai-prompt = 新建个人 Prompt
keybinding-desc-workspace-create-personal-ai-prompt-menu = 新建个人 Prompt
keybinding-desc-workspace-create-team-ai-prompt = 新建团队 Prompt
keybinding-desc-workspace-create-team-ai-prompt-menu = 新建团队 Prompt

# 焦点 / 导入
keybinding-desc-workspace-shift-focus-left = 切换焦点到左侧面板
keybinding-desc-workspace-shift-focus-right = 切换焦点到右侧面板
keybinding-desc-workspace-import-to-personal-drive = 导入到个人 Drive
keybinding-desc-workspace-import-to-team-drive = 导入到团队 Drive

# Drive / 仓库 / AI Rules / MCP
keybinding-desc-workspace-open-repository = 打开仓库
keybinding-desc-workspace-open-repository-menu = 打开仓库
keybinding-desc-workspace-open-ai-fact-collection = 打开 AI Rules
keybinding-desc-workspace-open-mcp-servers = 打开 MCP 服务器
keybinding-desc-workspace-jump-to-latest-toast = 跳转到最新 agent 任务
keybinding-desc-workspace-toggle-notification-mailbox = 切换通知邮箱
keybinding-desc-workspace-toggle-agent-management-view = 切换 agent 管理视图

# 设置页面
keybinding-desc-workspace-show-settings = 打开设置
keybinding-desc-workspace-show-settings-menu = 设置
keybinding-desc-workspace-show-settings-account = 打开设置:账户
keybinding-desc-workspace-show-settings-appearance = 打开设置:外观
keybinding-desc-workspace-show-settings-appearance-menu = 外观...
keybinding-desc-workspace-show-settings-features = 打开设置:功能
keybinding-desc-workspace-show-settings-shared-blocks = 打开设置:共享命令块
keybinding-desc-workspace-show-settings-shared-blocks-menu = 查看共享命令块...
keybinding-desc-workspace-show-settings-keyboard-shortcuts = 打开设置:键盘快捷键
keybinding-desc-workspace-show-settings-keyboard-shortcuts-menu = 配置键盘快捷键...
keybinding-desc-workspace-show-settings-about = 打开设置:关于
keybinding-desc-workspace-show-settings-about-menu = 关于 Warp
keybinding-desc-workspace-show-settings-teams = 打开设置:团队
keybinding-desc-workspace-show-settings-teams-menu = 打开团队设置
keybinding-desc-workspace-show-settings-privacy = 打开设置:隐私
keybinding-desc-workspace-show-settings-warpify = 打开设置:Warpify
keybinding-desc-workspace-show-settings-warpify-menu = 配置 Warpify...
keybinding-desc-workspace-show-settings-ai = 打开设置:AI
keybinding-desc-workspace-show-settings-code = 打开设置:代码
keybinding-desc-workspace-show-settings-referrals = 打开设置:推荐
keybinding-desc-workspace-show-settings-environments = 打开设置:环境
keybinding-desc-workspace-show-settings-mcp-servers = 打开设置:MCP 服务器
keybinding-desc-workspace-open-settings-file = 打开设置文件

# 溢出菜单 / 外部链接
keybinding-desc-workspace-link-to-slack = 加入我们的 Slack 社区(打开外部链接)
keybinding-desc-workspace-link-to-user-docs = 查看用户文档(打开外部链接)
keybinding-desc-workspace-send-feedback = 发送反馈(打开外部链接)
keybinding-desc-workspace-send-feedback-oz = 用 Oz 发送反馈
keybinding-desc-workspace-view-logs = 查看 Warp 日志
keybinding-desc-workspace-link-to-privacy-policy = 查看隐私政策(打开外部链接)

# 输入 / 终端 / 项目相关 binding(注册在 workspace/mod.rs 之外)
keybinding-desc-input-edit-prompt = 编辑 Prompt
keybinding-desc-terminal-attach-block-as-context = 将所选块作为 Agent 上下文附加
keybinding-desc-terminal-attach-text-as-context = 将所选文本作为 Agent 上下文附加
keybinding-desc-terminal-attach-as-context-menu = 将所选内容作为 Agent 上下文附加
keybinding-desc-workspace-write-codebase-index = 写入当前代码库索引快照
keybinding-desc-workspace-init-project = 为 Warp 初始化项目
keybinding-desc-workspace-add-current-folder = 将当前文件夹添加为项目

# Workspace 调试 / crash / sentry / 堆分析相关 binding
keybinding-desc-workspace-crash-macos = 触发崩溃(用于测试 sentry-cocoa)
keybinding-desc-workspace-crash-other = 触发崩溃(用于测试 sentry-native)
keybinding-desc-workspace-log-review-comment-send-status = [调试] 记录当前标签页的评审评论发送状态
keybinding-desc-workspace-panic = 触发 panic(用于测试 sentry-rust)
keybinding-desc-workspace-open-view-tree-debugger = 打开视图树调试器
keybinding-desc-workspace-view-first-time-user-experience = [调试] 查看首次启动引导体验
keybinding-desc-workspace-open-build-plan-migration-modal = [调试] 打开构建计划迁移弹窗
keybinding-desc-workspace-reset-build-plan-migration-modal-state = [调试] 重置构建计划迁移弹窗状态
keybinding-desc-workspace-undismiss-aws-login-banner = [调试] 取消关闭 AWS 登录提示条
keybinding-desc-workspace-open-oz-launch-modal = [调试] 打开 Oz 启动弹窗
keybinding-desc-workspace-reset-oz-launch-modal-state = [调试] 重置 Oz 启动弹窗状态
keybinding-desc-workspace-open-openwarp-launch-modal = [调试] 打开 OpenWarp 启动弹窗
keybinding-desc-workspace-reset-openwarp-launch-modal-state = [调试] 重置 OpenWarp 启动弹窗状态
keybinding-desc-workspace-install-opencode-warp-plugin = [调试] 安装 OpenCode Warp 插件
keybinding-desc-workspace-use-local-opencode-warp-plugin = [调试] 使用本地 OpenCode Warp 插件(仅测试用)
keybinding-desc-workspace-open-session-config-modal = [调试] 打开会话配置弹窗
keybinding-desc-workspace-start-hoa-onboarding-flow = [调试] 启动 HOA 引导流程
keybinding-desc-workspace-sample-process = 采样进程
keybinding-desc-workspace-dump-heap-profile = 导出堆分析(只能执行一次)

# 终端输入相关 binding
keybinding-desc-input-show-network-log = 显示 Warp 网络日志
keybinding-desc-input-clear-screen = 清屏
keybinding-desc-input-toggle-classic-completions = (实验性)切换经典补全模式
keybinding-desc-input-command-search = 命令搜索
keybinding-desc-input-history-search = 历史记录搜索
keybinding-desc-input-open-completions-menu = 打开补全菜单
keybinding-desc-input-workflows = 工作流
keybinding-desc-input-open-ai-command-suggestions = 打开 AI 命令建议
keybinding-desc-input-new-agent-conversation = 新建智能体对话
keybinding-desc-input-trigger-auto-detection = 触发自动识别
keybinding-desc-input-clear-and-reset-ai-context-menu-query = 清空并重置 AI 上下文菜单查询

# 终端视图相关 binding
keybinding-desc-terminal-alternate-paste = 终端备用粘贴
keybinding-desc-terminal-toggle-cli-agent-rich-input = 切换 CLI 智能体富文本输入
keybinding-desc-terminal-warpify-subshell = Warpify 子 shell
keybinding-desc-terminal-warpify-ssh-session = Warpify SSH 会话
keybinding-desc-terminal-accept-prompt-suggestion = 接受 Prompt 建议
keybinding-desc-terminal-cancel-process-windows = 复制文本或取消正在运行的进程
keybinding-desc-terminal-cancel-process = 取消正在运行的进程
keybinding-desc-terminal-focus-input = 聚焦终端输入
keybinding-desc-terminal-paste = 粘贴
keybinding-desc-terminal-copy = 复制
keybinding-desc-terminal-reinput-commands = 重新输入所选命令
keybinding-desc-terminal-reinput-commands-sudo = 以 root 身份重新输入所选命令
keybinding-desc-terminal-find = 在终端中查找
keybinding-desc-terminal-select-bookmark-up = 选择上方最近的书签
keybinding-desc-terminal-select-bookmark-down = 选择下方最近的书签
keybinding-desc-terminal-open-block-context-menu = 打开命令块上下文菜单
keybinding-desc-terminal-toggle-team-workflows-modal = 切换团队工作流弹窗
keybinding-desc-terminal-copy-git-branch = 复制 git 分支
keybinding-desc-terminal-clear-blocks = 清空命令块
keybinding-desc-terminal-cursor-word-left = 在执行中的命令内向左移动一个单词
keybinding-desc-terminal-cursor-word-right = 在执行中的命令内向右移动一个单词
keybinding-desc-terminal-cursor-home = 在执行中的命令内移动到行首
keybinding-desc-terminal-cursor-end = 在执行中的命令内移动到行尾
keybinding-desc-terminal-delete-word-left = 在执行中的命令内向左删除一个单词
keybinding-desc-terminal-delete-line-start = 在执行中的命令内删除到行首
keybinding-desc-terminal-delete-line-end = 在执行中的命令内删除到行尾
keybinding-desc-terminal-backward-tabulation = 在执行中的命令内反向跳格
keybinding-desc-terminal-select-previous-block = 选择上一个命令块
keybinding-desc-terminal-select-next-block = 选择下一个命令块
keybinding-desc-terminal-share-selected-block = 分享所选命令块
keybinding-desc-terminal-bookmark-selected-block = 收藏所选命令块
keybinding-desc-terminal-find-within-selected-block = 在所选命令块内查找
keybinding-desc-terminal-copy-command-and-output = 复制命令与输出
keybinding-desc-terminal-copy-command-output = 复制命令输出
keybinding-desc-terminal-copy-command = 复制命令
keybinding-desc-terminal-scroll-up-one-line = 终端输出向上滚动一行
keybinding-desc-terminal-scroll-down-one-line = 终端输出向下滚动一行
keybinding-desc-terminal-scroll-to-top-of-block = 滚动到所选命令块顶部
keybinding-desc-terminal-scroll-to-bottom-of-block = 滚动到所选命令块底部
keybinding-desc-terminal-select-all-blocks = 选择全部命令块
keybinding-desc-terminal-expand-blocks-above = 向上扩展所选命令块
keybinding-desc-terminal-expand-blocks-below = 向下扩展所选命令块
keybinding-desc-terminal-insert-command-correction = 插入命令纠错
keybinding-desc-terminal-setup-guide = 设置向导
keybinding-desc-terminal-onboarding-warp-input-terminal = [调试] 引导提示:WarpInput - 终端
keybinding-desc-terminal-onboarding-warp-input-project = [调试] 引导提示:WarpInput - 项目
keybinding-desc-terminal-onboarding-warp-input-no-project = [调试] 引导提示:WarpInput - 无项目
keybinding-desc-terminal-onboarding-modality-project = [调试] 引导提示:Modality - 项目
keybinding-desc-terminal-onboarding-modality-no-project = [调试] 引导提示:Modality - 无项目
keybinding-desc-terminal-onboarding-modality-terminal = [调试] 引导提示:Modality - 终端
keybinding-desc-terminal-import-external-settings = 导入外部设置
keybinding-desc-terminal-share-current-session = 分享当前会话
keybinding-desc-terminal-stop-sharing-current-session = 停止分享当前会话
keybinding-desc-terminal-toggle-block-filter = 在所选或最近的命令块上切换块过滤
keybinding-desc-terminal-toggle-sticky-command-header = 在当前面板切换粘性命令头
keybinding-desc-terminal-toggle-autoexecute-mode = 切换自动执行模式
keybinding-desc-terminal-toggle-queue-next-prompt = 切换排队下一条 Prompt
keybinding-desc-terminal-generate-codebase-index = [调试] 生成代码库索引

# 面板组相关 binding
keybinding-desc-pane-group-close-current-session = 关闭当前会话
keybinding-desc-pane-group-split-left = 向左分屏
keybinding-desc-pane-group-split-up = 向上分屏
keybinding-desc-pane-group-split-down = 向下分屏
keybinding-desc-pane-group-split-right = 向右分屏
keybinding-desc-pane-group-switch-left = 切换到左侧面板
keybinding-desc-pane-group-switch-right = 切换到右侧面板
keybinding-desc-pane-group-switch-up = 切换到上方面板
keybinding-desc-pane-group-switch-down = 切换到下方面板
keybinding-desc-pane-group-resize-left = 调整面板 > 分隔条左移
keybinding-desc-pane-group-resize-right = 调整面板 > 分隔条右移
keybinding-desc-pane-group-resize-up = 调整面板 > 分隔条上移
keybinding-desc-pane-group-resize-down = 调整面板 > 分隔条下移
keybinding-desc-pane-group-toggle-maximize = 切换最大化当前面板

# 根视图相关 binding
keybinding-desc-root-view-toggle-fullscreen = 切换全屏
keybinding-desc-root-view-enter-onboarding-state = [调试] 进入引导状态

# 工作流视图相关 binding
keybinding-desc-workflow-view-save = 保存工作流
keybinding-desc-workflow-view-close = 关闭

# 编辑器视图 binding desc(由 editor/view/mod.rs、code/editor/view/actions.rs、notebooks/editor/view.rs 共用)
keybinding-desc-editor-copy = 复制
keybinding-desc-editor-cut = 剪切
keybinding-desc-editor-paste = 粘贴
keybinding-desc-editor-undo = 撤销
keybinding-desc-editor-redo = 重做
keybinding-desc-editor-select-left-by-word = 向左按词选择
keybinding-desc-editor-select-right-by-word = 向右按词选择
keybinding-desc-editor-select-left = 向左选中一个字符
keybinding-desc-editor-select-right = 向右选中一个字符
keybinding-desc-editor-select-up = 向上选择
keybinding-desc-editor-select-down = 向下选择
keybinding-desc-editor-select-all = 全选
keybinding-desc-editor-select-to-line-start = 选中到行首
keybinding-desc-editor-select-to-line-end = 选中到行尾
keybinding-desc-editor-select-to-line-start-cap = 选中到行首
keybinding-desc-editor-select-to-line-end-cap = 选中到行尾
keybinding-desc-editor-clear-and-copy-lines = 复制并清除选中行
keybinding-desc-editor-add-next-occurrence = 添加下一处匹配到选区
keybinding-desc-editor-up = 光标上移
keybinding-desc-editor-down = 光标下移
keybinding-desc-editor-left = 光标左移
keybinding-desc-editor-right = 光标右移
keybinding-desc-editor-move-to-line-start = 移动到行首
keybinding-desc-editor-move-to-line-end = 移动到行尾
keybinding-desc-editor-move-to-line-start-short = 移动到行首
keybinding-desc-editor-move-to-line-end-short = 移动到行尾
keybinding-desc-editor-home = 行首
keybinding-desc-editor-end = 行尾
keybinding-desc-editor-cmd-down = 移动到末尾
keybinding-desc-editor-cmd-up = 移动到开头
keybinding-desc-editor-move-to-and-select-buffer-start = 选中并移动到开头
keybinding-desc-editor-move-to-and-select-buffer-end = 选中并移动到末尾
keybinding-desc-editor-move-forward-one-word = 向后移动一个词
keybinding-desc-editor-move-backward-one-word = 向前移动一个词
keybinding-desc-editor-move-forward-one-word-cap = 向后移动一个词
keybinding-desc-editor-move-backward-one-word-cap = 向前移动一个词
keybinding-desc-editor-move-to-paragraph-start = 移动到段落开头
keybinding-desc-editor-move-to-paragraph-end = 移动到段落末尾
keybinding-desc-editor-move-to-paragraph-start-short = 移动到段落开头
keybinding-desc-editor-move-to-paragraph-end-short = 移动到段落末尾
keybinding-desc-editor-move-to-buffer-start = 移动到缓冲区开头
keybinding-desc-editor-move-to-buffer-end = 移动到缓冲区末尾
keybinding-desc-editor-cursor-at-buffer-start = 光标移到缓冲区开头
keybinding-desc-editor-cursor-at-buffer-end = 光标移到缓冲区末尾
keybinding-desc-editor-backspace = 删除前一个字符
keybinding-desc-editor-cut-word-left = 剪切左侧词
keybinding-desc-editor-cut-word-right = 剪切右侧词
keybinding-desc-editor-delete-word-left = 删除左侧词
keybinding-desc-editor-delete-word-right = 删除右侧词
keybinding-desc-editor-cut-all-left = 剪切左侧全部
keybinding-desc-editor-cut-all-right = 剪切右侧全部
keybinding-desc-editor-delete-all-left = 删除左侧全部
keybinding-desc-editor-delete-all-right = 删除右侧全部
keybinding-desc-editor-delete = 删除
keybinding-desc-editor-clear-lines = 清除选中行
keybinding-desc-editor-insert-newline = 插入换行
keybinding-desc-editor-fold = 折叠
keybinding-desc-editor-unfold = 展开
keybinding-desc-editor-fold-selected-ranges = 折叠选中范围
keybinding-desc-editor-insert-last-word-prev-cmd = 插入上一条命令的最后一个词
keybinding-desc-editor-move-backward-one-subword = 向前移动一个子词
keybinding-desc-editor-move-forward-one-subword = 向后移动一个子词
keybinding-desc-editor-select-left-by-subword = 向左按子词选择
keybinding-desc-editor-select-right-by-subword = 向右按子词选择
keybinding-desc-editor-accept-autosuggestion = 接受自动建议
keybinding-desc-editor-inspect-command = 检查命令
keybinding-desc-editor-clear-buffer = 清空命令编辑器
keybinding-desc-editor-add-cursor-above = 在上方添加光标
keybinding-desc-editor-add-cursor-below = 在下方添加光标
keybinding-desc-editor-insert-nonexpanding-space = 插入不可扩展空格
keybinding-desc-editor-vim-exit-insert-mode = 退出 Vim 插入模式
keybinding-desc-editor-toggle-comment = 切换注释
keybinding-desc-editor-go-to-line = 跳转到行
keybinding-desc-editor-find-in-code-editor = 在代码编辑器中查找

# 代码编辑器(Code)binding desc
keybinding-desc-code-save-as = 文件另存为
keybinding-desc-code-close-all-tabs = 关闭所有标签页
keybinding-desc-code-close-saved-tabs = 关闭已保存的标签页

# 欢迎视图 binding desc
keybinding-desc-welcome-terminal-session = 终端会话
keybinding-desc-welcome-add-repository = 添加仓库

# AI 助手面板 binding desc
keybinding-desc-ai-assistant-close = 关闭 Warp AI
keybinding-desc-ai-assistant-focus-terminal-input = 从 Warp AI 切回终端输入
keybinding-desc-ai-assistant-restart = 重启 Warp AI

# 代码审阅 binding desc
keybinding-desc-code-review-save-all = 保存代码审阅中所有未保存的文件
keybinding-desc-code-review-show-find = 在代码审阅中显示查找栏

# 项目按钮 binding desc
keybinding-desc-project-buttons-open-repository = 打开仓库
keybinding-desc-project-buttons-create-new-project = 创建新项目

# 查找视图 binding desc
keybinding-desc-find-next-occurrence = 查找下一处匹配
keybinding-desc-find-prev-occurrence = 查找上一处匹配

# Notebook 文件 / 笔记本 binding desc
keybinding-desc-notebook-focus-terminal-input-from-file = 从文件切回终端输入
keybinding-desc-notebook-reload-file = 重新加载文件
keybinding-desc-notebook-increase-font-size = 增大笔记本字号
keybinding-desc-notebook-decrease-font-size = 减小笔记本字号
keybinding-desc-notebook-reset-font-size = 重置笔记本字号
keybinding-desc-notebook-focus-terminal-input = 从笔记本切回终端输入
keybinding-desc-notebook-fb-increase-font-size = 增大字号
keybinding-desc-notebook-fb-decrease-font-size = 减小字号

# Notebook 编辑器 binding desc(在共享编辑器 key 之外的)
keybinding-desc-nbeditor-deselect-command = 取消选中 shell 命令
keybinding-desc-nbeditor-select-command = 选中光标处的 shell 命令
keybinding-desc-nbeditor-select-previous-command = 选中上一条命令
keybinding-desc-nbeditor-select-next-command = 选中下一条命令
keybinding-desc-nbeditor-run-commands = 运行选中的命令
keybinding-desc-nbeditor-toggle-debug = 切换富文本调试模式
keybinding-desc-nbeditor-debug-copy-buffer = 复制富文本缓冲区
keybinding-desc-nbeditor-debug-copy-selection = 复制富文本选区
keybinding-desc-nbeditor-log-state = 输出编辑器状态日志
keybinding-desc-nbeditor-edit-link = 创建或编辑链接
keybinding-desc-nbeditor-inline-code = 切换行内代码样式
keybinding-desc-nbeditor-strikethrough = 切换删除线样式
keybinding-desc-nbeditor-underline = 切换下划线样式
keybinding-desc-nbeditor-find = 在笔记本中查找
keybinding-desc-nbeditor-next-find-match = 聚焦下一处匹配
keybinding-desc-nbeditor-previous-find-match = 聚焦上一处匹配
keybinding-desc-nbeditor-toggle-regex-find = 切换正则表达式搜索
keybinding-desc-nbeditor-toggle-case-sensitive-find = 切换大小写敏感搜索

# 面板组 / 撤销关闭 binding desc
keybinding-desc-get-started-terminal-session = 终端会话
keybinding-desc-undo-close-reopen-session = 重新打开已关闭的会话
keybinding-desc-pane-share-pane = 分享面板
keybinding-desc-right-panel-toggle-maximize-code-review = 切换最大化代码审阅面板

# 工作区输入同步 binding desc
keybinding-desc-workspace-disable-sync-inputs = 停止同步所有面板
keybinding-desc-workspace-toggle-sync-inputs-tab = 切换同步当前标签页所有面板
keybinding-desc-workspace-toggle-sync-inputs-all-tabs = 切换同步所有标签页中的所有面板

# 工作区辅助功能 / 调试 binding desc
keybinding-desc-workspace-a11y-concise = [a11y] 设为简洁辅助播报
keybinding-desc-workspace-a11y-verbose = [a11y] 设为详细辅助播报
keybinding-desc-workspace-copy-access-token = 复制访问令牌到剪贴板

# 环境变量集合 binding desc
keybinding-desc-env-var-collection-close = 关闭

# 鉴权 / 分享模态 binding desc
keybinding-desc-share-block-copy = 复制
keybinding-desc-auth-paste-token = 粘贴
keybinding-desc-conversation-details-copy = 复制

# 终端补充 binding desc
keybinding-desc-terminal-show-history = 显示历史
keybinding-desc-terminal-ask-ai-selection = 就所选内容询问 Warp AI
keybinding-desc-terminal-ask-ai-last-block = 就最近的命令块询问 Warp AI
keybinding-desc-terminal-ask-ai = 询问 Warp AI
keybinding-desc-terminal-load-agent-conversation = 加载智能体模式会话(从剪贴板调试链接)
keybinding-desc-terminal-toggle-session-recording = 切换会话 PTY 录制

# Notebook 编辑器补充
keybinding-desc-nbeditor-select-to-paragraph-start = 选中到段落开头
keybinding-desc-nbeditor-select-to-paragraph-end = 选中到段落末尾

# 杂项 binding desc(收尾批次:常量/LazyLock/动态描述去硬编码)
keybinding-desc-save-file = 保存文件
keybinding-desc-new-agent-pane = 新建 Agent 窗格
keybinding-desc-edit-code-diff = 编辑代码差异
keybinding-desc-edit-requested-command = 编辑请求的命令
keybinding-desc-set-input-mode-agent = 切换输入模式为 Agent 模式
keybinding-desc-set-input-mode-terminal = 切换输入模式为终端模式
keybinding-desc-toggle-hide-cli-responses = 切换隐藏 CLI 回应
keybinding-desc-slash-command = 斜杠命令:{ $name }
keybinding-desc-take-control-of-running-command = 接管正在运行的命令

# --- 终端零状态块(欢迎提示) ---
terminal-zero-state-title = 新建终端会话
terminal-zero-state-start-agent = 开始新的 Agent 对话
terminal-zero-state-cycle-history = 翻阅历史命令与对话
terminal-zero-state-open-code-review = 打开代码评审
terminal-zero-state-autodetect-prompts = 在终端会话中自动检测 Agent 提示
terminal-zero-state-dismiss = 不再显示

# --- Rules 页面(ai/facts/view/rule.rs) ---
rules-description = Rules 通过提供结构化指引来增强 Agent,帮助保持一致性、贯彻最佳实践,并适应特定工作流,包括代码库或更宏观的任务。
rules-search-placeholder = 搜索规则
rules-zero-state-global = 添加规则后,它将显示在这里。
rules-zero-state-project = 为项目生成 WARP.md 规则文件后,它将显示在这里。
rules-disabled-banner-prefix = 你的规则已禁用,不会在会话中作为上下文使用。你可以随时
rules-disabled-banner-link = 重新开启
rules-disabled-banner-suffix = 。
rules-tab-global = 全局
rules-tab-project = 项目级
rules-add-button = 添加
rules-init-project-button = 初始化项目

# --- Agent 视图零状态 + 消息栏 ---
agent-zero-state-title = 新建 Agent 对话
# OpenWarp 已移除云端 Agent 入口,此 key 实际不会被渲染;保留以匹配 en 兜底链。
agent-zero-state-title-cloud = 新建 Agent 对话
agent-zero-state-description = 在下方输入提示开始新的对话
agent-zero-state-description-with-location = 在下方输入提示,于 `{ $location }` 开始新的对话
agent-zero-state-switch-model = 切换模型
agent-zero-state-go-back-to-terminal = 返回终端
agent-message-bar-for-help = 查看帮助
agent-message-bar-for-commands = 查看命令
agent-message-bar-open-conversation = 打开对话
agent-message-bar-for-code-review = 进入代码评审

# --- ANCHOR-SUB-TOGGLE-PAIR (settings-toggle-pair) ---
toggle-setting-enable = 启用{ $suffix }
toggle-setting-disable = 禁用{ $suffix }

toggle-suffix-ai = AI
toggle-suffix-active-ai = 主动式 AI
toggle-suffix-ai-input-autodetect-agent = Agent 输入中的终端命令检测
toggle-suffix-ai-input-autodetect-nld = 自然语言检测
toggle-suffix-nld-in-terminal = 终端输入中的 Agent 提示词检测
toggle-suffix-next-command = Next Command 补全
toggle-suffix-prompt-suggestions = 提示词建议
toggle-suffix-code-suggestions = 代码建议
toggle-suffix-nl-autosuggestions = 自然语言自动建议
toggle-suffix-shared-block-title-gen = 共享块标题生成
toggle-suffix-voice-input = 语音输入
toggle-suffix-codebase-index = 代码库索引
toggle-suffix-auto-indexing = 自动索引
toggle-suffix-compact-mode = 紧凑模式
toggle-suffix-themes-sync-os = 主题:跟随系统
toggle-suffix-cursor-blink = 光标闪烁
toggle-suffix-jump-bottom-block = 跳到块底部按钮
toggle-suffix-block-dividers = 块分隔线
toggle-suffix-dim-inactive-panes = 非活动面板调暗
toggle-suffix-tab-indicators = 标签页指示器
toggle-suffix-focus-follows-mouse = 焦点跟随鼠标
toggle-suffix-zen-mode = 禅模式
toggle-suffix-vertical-tabs = 垂直标签栏布局
toggle-suffix-ligature-rendering = 连字渲染
toggle-suffix-copy-on-select = 终端内选中即复制
toggle-suffix-linux-selection-clipboard = Linux 主选择剪贴板
toggle-suffix-autocomplete-symbols = 自动补全引号、圆括号和方括号
toggle-suffix-restore-session = 启动时恢复窗口、标签页和面板
toggle-suffix-left-option-meta = 左 Option 键作为 Meta
toggle-suffix-left-alt-meta = 左 Alt 键作为 Meta
toggle-suffix-right-option-meta = 右 Option 键作为 Meta
toggle-suffix-right-alt-meta = 右 Alt 键作为 Meta
toggle-suffix-scroll-reporting = 滚动事件上报
toggle-suffix-completions-while-typing = 输入时补全
toggle-suffix-command-corrections = 命令纠错
toggle-suffix-error-underlining = 错误下划线
toggle-suffix-syntax-highlighting = 语法高亮
toggle-suffix-audible-bell = 终端响铃
toggle-suffix-autosuggestions = 自动建议
toggle-suffix-autosuggestion-keybinding-hint = 自动建议快捷键提示
toggle-suffix-ssh-wrapper = Warp SSH 包装器
toggle-suffix-link-tooltip = 点击链接显示提示
toggle-suffix-quit-warning = 退出警告弹窗
toggle-suffix-alias-expansion = 别名展开
toggle-suffix-middle-click-paste = 中键粘贴
toggle-suffix-code-as-default-editor = VS Code 作为默认编辑器
toggle-suffix-input-hint-text = 输入提示文字
toggle-suffix-vim-keybindings = 用 Vim 快捷键编辑命令
toggle-suffix-vim-clipboard = Vim 默认寄存器使用系统剪贴板
toggle-suffix-vim-status-bar = Vim 状态栏
toggle-suffix-focus-reporting = 焦点上报
toggle-suffix-smart-select = 智能选择
toggle-suffix-input-message-line = 终端输入提示行
toggle-suffix-slash-commands-terminal = 终端模式斜杠命令
toggle-suffix-integrated-gpu = 集成 GPU 渲染(低功耗)
toggle-suffix-wayland = Wayland 窗口管理
toggle-suffix-settings-sync = 设置同步
toggle-suffix-app-analytics = 应用分析
toggle-suffix-crash-reporting = 崩溃上报
toggle-suffix-secret-redaction = 敏感信息脱敏
toggle-suffix-recording-mode = 录制模式
toggle-suffix-inband-generators = 新会话使用 in-band 生成器
toggle-suffix-debug-network = 网络状态调试
toggle-suffix-memory-stats = 内存统计

# Set agent thinking display
agent-thinking-display-show-collapse = 设置 Agent 思考展示:展示并折叠
agent-thinking-display-always-show = 设置 Agent 思考展示:始终展示
agent-thinking-display-never-show = 设置 Agent 思考展示:从不展示

# --- ANCHOR-SUB-EXTERNAL-EDITOR (settings-external-editor) ---
settings-external-editor-choose-default = 选择打开文件链接的编辑器
settings-external-editor-choose-code-panels = 选择从代码评审面板、项目浏览器和全局搜索打开文件的编辑器
settings-external-editor-choose-layout = 选择在 Warp 中打开文件的布局
settings-external-editor-tabbed-header = 多个文件合并到同一编辑器面板
settings-external-editor-tabbed-desc = 开启后,同一标签页中打开的文件会自动归并到单一编辑器面板。
settings-external-editor-prefer-markdown = 默认用 Warp Markdown 查看器打开 Markdown 文件
settings-external-editor-layout-split-pane = 分屏面板
settings-external-editor-layout-new-tab = 新建标签页
settings-external-editor-default-app = 系统默认

# =============================================================================
# SECTION: context-menu (Owner: agent-context-menu)
# 鼠标右键弹出菜单
# =============================================================================

# --- block 右键菜单(terminal/view.rs) ---
menu-block-copy = 复制
menu-block-copy-url = 复制 URL
menu-block-copy-path = 复制路径
menu-block-show-in-finder = 在 Finder 中显示
menu-block-show-containing-folder = 显示所在文件夹
menu-block-open-in-warp = 在 Warp 中打开
menu-block-open-in-editor = 在编辑器中打开
menu-block-insert-into-input = 插入到输入框
menu-block-copy-command = 复制命令
menu-block-copy-commands = 复制命令
menu-block-find-within-block = 在块内查找
menu-block-find-within-blocks = 在块内查找
menu-block-scroll-to-top-of-block = 滚动到块顶部
menu-block-scroll-to-top-of-blocks = 滚动到块顶部
menu-block-scroll-to-bottom-of-block = 滚动到块底部
menu-block-scroll-to-bottom-of-blocks = 滚动到块底部
menu-block-save-as-workflow = 另存为工作流
menu-block-ask-warp-ai = 询问 Warp AI
menu-block-copy-output = 复制输出
menu-block-copy-filtered-output = 复制过滤后的输出
menu-block-toggle-block-filter = 切换块过滤器
menu-block-toggle-bookmark = 切换收藏
menu-block-copy-prompt = 复制提示符
menu-block-copy-right-prompt = 复制右侧提示符
menu-block-copy-working-directory = 复制工作目录
menu-block-copy-git-branch = 复制 git 分支
menu-block-edit-prompt = 编辑提示符
menu-block-edit-cli-agent-toolbelt = 编辑 CLI Agent 工具带
menu-block-edit-agent-toolbelt = 编辑 Agent 工具带
menu-block-split-pane-right = 向右分割面板
menu-block-split-pane-left = 向左分割面板
menu-block-split-pane-down = 向下分割面板
menu-block-split-pane-up = 向上分割面板
menu-block-close-pane = 关闭面板

# --- input 右键菜单(terminal/view.rs) ---
menu-input-cut = 剪切
menu-input-copy = 复制
menu-input-paste = 粘贴
menu-input-select-all = 全选
menu-input-command-search = 命令搜索
menu-input-ai-command-search = AI 命令搜索
menu-input-ask-warp-ai = 询问 Warp AI
menu-input-save-as-workflow = 另存为工作流
menu-input-hide-hint-text = 隐藏输入框提示文本
menu-input-show-hint-text = 显示输入框提示文本

# --- AI block overflow 菜单(terminal/view.rs) ---
menu-ai-block-copy = 复制
menu-ai-block-copy-prompt = 复制提示词
menu-ai-block-copy-output-as-markdown = 复制输出为 Markdown
menu-ai-block-copy-url = 复制 URL
menu-ai-block-copy-path = 复制路径
menu-ai-block-copy-command = 复制命令
menu-ai-block-copy-git-branch = 复制 git 分支
menu-ai-block-save-as-prompt = 另存为提示词
menu-ai-block-share-conversation = 分享对话
menu-ai-block-copy-conversation-text = 复制对话文本
menu-ai-block-fork-from-here = 从此处分叉
menu-ai-block-rewind-to-before-here = 回退到此处之前
menu-ai-block-fork-from-last-query = 从上一次提问分叉
menu-ai-block-fork-from-query = 从"{ $query }"分叉

# --- tab 右键菜单(tab.rs) ---
menu-tab-stop-sharing = 停止共享
menu-tab-share-session = 共享会话
menu-tab-stop-sharing-all = 停止共享全部
menu-tab-copy-link = 复制链接
menu-tab-rename = 重命名标签页
menu-tab-reset-name = 重置标签页名称
menu-tab-move-down = 向下移动标签页
menu-tab-move-right = 向右移动标签页
menu-tab-move-up = 向上移动标签页
menu-tab-move-left = 向左移动标签页
menu-tab-close = 关闭标签页
menu-tab-close-other = 关闭其他标签页
menu-tab-close-below = 关闭下方标签页
menu-tab-close-right = 关闭右侧标签页
menu-tab-save-as-new-config = 另存为新配置
menu-tab-default-no-color = 默认(无颜色)

# --- pane header 溢出菜单(terminal/view/pane_impl.rs) ---
menu-pane-copy-link = 复制链接
menu-pane-stop-sharing-session = 停止共享会话
menu-pane-open-on-desktop = 在桌面端打开

# --- 文件树右键菜单(code/file_tree/view.rs) ---
menu-filetree-open-in-new-pane = 在新面板中打开
menu-filetree-open-in-new-tab = 在新标签页中打开
menu-filetree-open-file = 打开文件
menu-filetree-new-file = 新建文件
menu-filetree-cd-to-directory = cd 到该目录
menu-filetree-reveal-finder = 在 Finder 中显示
menu-filetree-reveal-explorer = 在资源管理器中显示
menu-filetree-reveal-file-manager = 在文件管理器中显示
menu-filetree-rename = 重命名
menu-filetree-delete = 删除
menu-filetree-attach-as-context = 附加为上下文
menu-filetree-copy-path = 复制路径
menu-filetree-copy-relative-path = 复制相对路径

# --- 代码编辑器右键菜单(code/local_code_editor.rs) ---
menu-codeeditor-go-to-definition = 跳转到定义
menu-codeeditor-find-references = 查找引用

# --- 共享标签:附加为 agent 上下文(blocklist/view_util.rs) ---
menu-attach-as-agent-context = 附加为 agent 上下文

# --- ANCHOR-SUB-SLASH-COMMANDS (agent-slash-commands) ---
# 斜杠命令面板的描述与参数提示
# (app/src/search/slash_command_menu/static_commands/commands.rs)
slash-cmd-agent-desc = 开始新对话
slash-cmd-add-mcp-desc = 添加新的 MCP 服务器
slash-cmd-pr-comments-desc = 拉取 GitHub PR 评审评论
slash-cmd-create-environment-desc = 通过引导式流程创建 Oz 环境(Docker 镜像 + 仓库)
slash-cmd-create-environment-hint = <可选:仓库路径或 GitHub URL>
slash-cmd-docker-sandbox-desc = 创建新的 Docker 沙盒终端会话
slash-cmd-create-new-project-desc = 由 Oz 引导你创建新的代码项目
slash-cmd-create-new-project-hint = <描述你想构建什么>
slash-cmd-open-skill-desc = 在 Warp 内置编辑器中打开技能的 markdown 文件
slash-cmd-skills-desc = 调用技能
slash-cmd-add-prompt-desc = 添加新的智能体提示词
slash-cmd-add-rule-desc = 为智能体添加新的全局规则
slash-cmd-open-file-desc = 在 Warp 代码编辑器中打开文件
slash-cmd-open-file-hint = <path/to/file[:line[:col]]> 或输入 "@" 搜索
slash-cmd-rename-tab-desc = 重命名当前标签页
slash-cmd-rename-tab-hint = <标签页名称>
slash-cmd-fork-desc = 在新窗格或新标签页中分叉当前对话
slash-cmd-fork-hint = <可选:在分叉后的对话中发送的提示词>
slash-cmd-open-code-review-desc = 打开代码评审
slash-cmd-index-desc = 索引此代码库
slash-cmd-init-desc = 索引此代码库并生成 AGENTS.md 文件
slash-cmd-open-project-rules-desc = 打开项目规则文件(AGENTS.md)
slash-cmd-open-mcp-servers-desc = 打开 MCP 服务器
slash-cmd-open-settings-file-desc = 打开设置文件(TOML)
slash-cmd-changelog-desc = 打开最新更新日志
slash-cmd-open-repo-desc = 切换到另一个已索引的仓库
slash-cmd-open-rules-desc = 查看你的全部全局规则与项目规则
slash-cmd-new-desc = 开始新对话(/agent 的别名)
slash-cmd-model-desc = 切换基础智能体模型
slash-cmd-profile-desc = 切换当前激活的执行配置
slash-cmd-plan-desc = 让智能体调研并为任务创建计划
slash-cmd-plan-hint = <描述你的任务>
slash-cmd-orchestrate-desc = 将任务拆分为子任务并由多个智能体并行执行
slash-cmd-orchestrate-hint = <描述你的任务>
slash-cmd-compact-desc = 通过摘要对话历史来释放上下文
slash-cmd-compact-hint = <可选:自定义摘要指令>
slash-cmd-compact-and-desc = 压缩对话并随后发送一条后续提示词
slash-cmd-compact-and-hint = <压缩后要发送的提示词>
slash-cmd-queue-desc = 排队一条提示词,在智能体完成响应后再发送
slash-cmd-queue-hint = <智能体完成后要发送的提示词>
slash-cmd-fork-and-compact-desc = 分叉当前对话并在分叉副本中压缩
slash-cmd-fork-and-compact-hint = <可选:压缩后要发送的提示词>
slash-cmd-fork-from-desc = 从特定查询处分叉对话
slash-cmd-remote-control-desc = 为此会话启动远程控制
slash-cmd-conversations-desc = 打开对话历史
slash-cmd-prompts-desc = 搜索已保存的提示词
slash-cmd-rewind-desc = 倒回到对话中的上一个节点
slash-cmd-export-to-clipboard-desc = 以 markdown 格式将当前对话导出到剪贴板
slash-cmd-export-to-file-desc = 将当前对话导出为 markdown 文件
slash-cmd-export-to-file-hint = <可选:文件名>
