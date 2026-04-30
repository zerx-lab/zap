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
