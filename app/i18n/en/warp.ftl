# Warp Desktop — English (source-of-truth locale)
# 本文件由多 agent 并行编辑,各自维护自己的 SECTION,key 以 surface 前缀隔离避免冲突。
# 加 key 时 ctrl-F 找到对应 SECTION 头追加;新 surface 在文件末尾加新 SECTION。
#
# 命名规范:kebab-case,前缀按 surface,例 settings-ai-title / drive-folder-rename-title
# 变量插值用 Fluent { $name } 语法,不要拼接

# =============================================================================
# SECTION: common (Owner: foundation)
# =============================================================================

app-name = Warp
app-tagline = The cloud-backed terminal for individuals and teams

common-ok = OK
common-cancel = Cancel
common-apply = Apply
common-save = Save
common-delete = Delete
common-confirm = Confirm
common-close = Close
common-reset = Reset
common-back = Back
common-next = Next
common-yes = Yes
common-no = No
common-edit = Edit
common-add = Add
common-remove = Remove
common-rename = Rename
common-copy = Copy
common-paste = Paste
common-search = Search
common-loading = Loading…
common-error = Error
common-warning = Warning
common-info = Info
common-success = Success

# =============================================================================
# SECTION: language (Owner: foundation)
# Files: app/src/settings_view/appearance_page.rs (Language widget + restart modal)
# =============================================================================

language-widget-label = Language
language-widget-secondary = Restart Warp for the change to fully take effect.
language-restart-required-title = Language changed
language-restart-required-body = Warp's UI language has been updated. Some text will switch immediately, but a full restart is required for the change to take effect everywhere.

# =============================================================================
# SECTION: settings (Owner: agent-settings)
# Files: app/src/settings_view/**
# =============================================================================

# --- ANCHOR-SUB-MOD-NAV (agent-settings-mod) ---
# settings_view/mod.rs SettingsSection Display labels + context menu pane actions

# Sidebar / SettingsSection labels (Display impl)
settings-section-about = About
settings-section-account = Account
settings-section-mcp-servers = MCP Servers
settings-section-billing-and-usage = Billing and usage
settings-section-appearance = Appearance
settings-section-features = Features
settings-section-keybindings = Keyboard shortcuts
settings-section-privacy = Privacy
settings-section-referrals = Referrals
settings-section-shared-blocks = Shared blocks
settings-section-teams = Teams
settings-section-warp-drive = Warp Drive
settings-section-warpify = Warpify
settings-section-ai = AI
settings-section-warp-agent = Warp Agent
settings-section-agent-profiles = Profiles
settings-section-agent-mcp-servers = MCP servers
settings-section-agent-providers = Providers
settings-section-knowledge = Knowledge
settings-section-third-party-cli-agents = Third party CLI agents
settings-section-code = Code
settings-section-code-indexing = Indexing and projects
settings-section-editor-and-code-review = Editor and Code Review
settings-section-cloud-environments = Environments
settings-section-oz-cloud-api-keys = Oz Cloud API Keys

# Context menu items (split / close pane)
settings-pane-split-right = Split pane right
settings-pane-split-left = Split pane left
settings-pane-split-down = Split pane down
settings-pane-split-up = Split pane up
settings-pane-close = Close pane

# Debug toggle setting descriptions (command palette)
settings-debug-show-init-block = Show initialization block
settings-debug-hide-init-block = Hide initialization block
settings-debug-show-inband-blocks = Show in-band command blocks
settings-debug-hide-inband-blocks = Hide in-band command blocks

# --- ANCHOR-SUB-ABOUT (agent-settings-about) ---
# 此锚点下放 settings_view/about_page.rs + main_page.rs 字符串
# 命名前缀:settings-about-* / settings-main-*

# about_page.rs
settings-about-copyright = Copyright 2026 Warp

# main_page.rs — referral / account
settings-main-referral-cta = Earn rewards by sharing Warp with friends & colleagues
settings-main-refer-a-friend = Refer a friend
settings-main-sign-up = Sign up
settings-main-plan-free = Free
settings-main-compare-plans = Compare plans
settings-main-contact-support = Contact support
settings-main-manage-billing = Manage billing
settings-main-upgrade-to-turbo = Upgrade to Turbo plan
settings-main-upgrade-to-lightspeed = Upgrade to Lightspeed plan

# main_page.rs — settings sync
settings-main-settings-sync-label = Settings sync

# main_page.rs — version / autoupdate
settings-main-version-label = Version
settings-main-status-up-to-date = Up to date
settings-main-cta-check-for-updates = Check for updates
settings-main-status-checking = checking for update...
settings-main-status-downloading = downloading update...
settings-main-status-update-available = Update available
settings-main-cta-relaunch-warp = Relaunch Warp
settings-main-status-updating = Updating...
settings-main-status-installed-update = Installed update
settings-main-status-cant-install = A new version of Warp is available but can't be installed
settings-main-status-cant-launch = A new version of Warp is installed but can't be launched.
settings-main-cta-update-manually = Update Warp manually

# --- ANCHOR-SUB-MCP (agent-settings-mcp) ---
# 此锚点下放 settings_view/mcp_servers_page.rs 字符串
# 命名前缀:settings-mcp-*
settings-mcp-page-title = MCP Servers
settings-mcp-logout-success-named = Successfully logged out of {$name} MCP server
settings-mcp-logout-success = Successfully logged out of MCP server
settings-mcp-install-modal-busy = Finish the current MCP install before opening another install link.
settings-mcp-unknown-server = Unknown MCP server '{$name}'
settings-mcp-install-from-link-failed = MCP server '{$name}' cannot be installed from this link.

# --- ANCHOR-SUB-PLATFORM (agent-settings-platform) ---
# 此锚点下放 settings_view/platform_page.rs 字符串
# 命名前缀:settings-platform-*
settings-platform-section-title = Oz Cloud API Keys
settings-platform-description = Create and manage API keys to allow other Oz cloud agents to access your Warp account.
    For more information, visit the
settings-platform-documentation-link = Documentation.
settings-platform-create-button = + Create API Key
settings-platform-modal-title-new = New API key
settings-platform-modal-title-save = Save your key
settings-platform-toast-deleted = API key deleted
settings-platform-column-name = Name
settings-platform-column-key = Key
settings-platform-column-scope = Scope
settings-platform-column-created = Created
settings-platform-column-last-used = Last used
settings-platform-column-expires-at = Expires at
settings-platform-value-never = Never
settings-platform-scope-personal = Personal
settings-platform-scope-team = Team
settings-platform-zero-state-title = No API Keys
settings-platform-zero-state-description = Create a key to manage external access to Warp

# --- ANCHOR-SUB-KEYBINDINGS (agent-settings-keybindings) ---
settings-keybindings-search-placeholder = Search by name or by keys (ex. "cmd d")
settings-keybindings-conflict-warning = This shortcut conflicts with other keybinds
settings-keybindings-button-default = Default
settings-keybindings-button-cancel = Cancel
settings-keybindings-button-clear = Clear
settings-keybindings-button-save = Save
settings-keybindings-press-new-shortcut = Press new keyboard shortcut
settings-keybindings-description = Add your own custom keybindings to existing actions below.
settings-keybindings-use-prefix = Use
settings-keybindings-use-suffix = to reference these keybindings in a side pane at anytime.
settings-keybindings-not-synced-tooltip = Keyboard shortcuts are not synced to the cloud
settings-keybindings-subheader = Configure keyboard shortcuts
settings-keybindings-command-column = Command

# --- ANCHOR-SUB-REFERRALS (agent-settings-referrals) ---
settings-referrals-page-title = Invite a friend to Warp
settings-referrals-anonymous-header = Sign up to participate in Warp's referral program
settings-referrals-sign-up = Sign up
settings-referrals-link-label = Link
settings-referrals-email-label = Email
settings-referrals-link-error = Failed to load referral code.
settings-referrals-loading = Loading...
settings-referrals-copy-link-button = Copy link
settings-referrals-email-send-button = Send
settings-referrals-email-sending-button = Sending...
settings-referrals-link-copied-toast = Link copied.
settings-referrals-email-success-toast = Successfully sent emails.
settings-referrals-email-failure-toast = Failed to send emails. Please try again.
settings-referrals-email-empty-error = Please enter an email.
settings-referrals-email-invalid-error = Please ensure the following email is valid: { $email }
settings-referrals-reward-intro = Get exclusive Warp goodies when you refer someone*
settings-referrals-claimed-count-singular = Current referral
settings-referrals-claimed-count-plural = Current referrals
settings-referrals-terms-link = Certain restrictions apply.
settings-referrals-terms-contact = { " " }If you have any questions about the referral program, please contact referrals@warp.dev.
settings-referrals-reward-theme = Exclusive theme
settings-referrals-reward-keycaps = Keycaps + stickers
settings-referrals-reward-tshirt = T-shirt
settings-referrals-reward-notebook = Notebook
settings-referrals-reward-cap = Baseball cap
settings-referrals-reward-hoodie = Hoodie
settings-referrals-reward-hydroflask = Premium Hydro Flask
settings-referrals-reward-backpack = Backpack

# --- ANCHOR-SUB-WARPIFY (agent-settings-warpify) ---
settings-warpify-page-title = Warpify
settings-warpify-description-prefix = Configure whether Warp attempts to "Warpify" (add support for blocks, input modes, etc) certain shells.
settings-warpify-learn-more = Learn more
settings-warpify-section-subshells = Subshells
settings-warpify-section-subshells-subtitle = Subshells supported: bash, zsh, and fish.
settings-warpify-section-ssh = SSH
settings-warpify-section-ssh-subtitle = Warpify your interactive SSH sessions.
settings-warpify-added-commands = Added commands
settings-warpify-denylisted-commands = Denylisted commands
settings-warpify-denylisted-hosts = Denylisted hosts
settings-warpify-command-placeholder = command (supports regex)
settings-warpify-host-placeholder = host (supports regex)
settings-warpify-enable-ssh = Warpify SSH Sessions
settings-warpify-install-ssh-extension = Install SSH extension
settings-warpify-install-ssh-extension-description = Controls the installation behavior for Warp's SSH extension when a remote host doesn't have it installed.
settings-warpify-use-tmux = Use Tmux Warpification
settings-warpify-tmux-description = The tmux ssh wrapper works in many situations where the default one does not, but may require you to hit a button to warpify. Takes effect in new tabs.
settings-warpify-ssh-tmux-toggle-binding-label = SSH session detection for Warpification

# --- ANCHOR-SUB-CODE (agent-settings-code) ---
settings-code-feature-name = Code
settings-code-initialization-settings-header = Initialization Settings
settings-code-codebase-indexing-label = Codebase indexing
settings-code-codebase-index-description = Warp can automatically index code repositories as you navigate them, helping agents quickly understand context and provide solutions. Code is never stored on the server. If a codebase is unable to be indexed, Warp can still navigate your codebase and gain insights via grep and find tool calling.
settings-code-warp-indexing-ignore-description = To exclude specific files or directories from indexing, add them to the .warpindexingignore file in your repository directory. These files will still be accessible to AI features, but they won't be included in codebase embeddings.
settings-code-auto-index-feature-name = Index new folders by default
settings-code-auto-index-description = When set to true, Warp will automatically index code repositories as you navigate them - helping agents quickly understand context and provide targeted solutions.
settings-code-indexing-disabled-admin = Team admins have disabled codebase indexing.
settings-code-indexing-workspace-enabled-admin = Team admins have enabled codebase indexing.
settings-code-indexing-disabled-global-ai = AI Features must be enabled to use codebase indexing.
settings-code-codebase-index-limit-reached = You have reached the maximum number of codebase indices for your plan. Delete existing indices to auto-index new codebases.
settings-code-subpage-indexing-title = Codebase Indexing
settings-code-subpage-editor-review-title = Editor and Code Review
settings-code-category-codebase-indexing = Codebase Indexing
settings-code-category-editor-review = Code Editor and Review
settings-code-index-new-folder = Index new folder
settings-code-initialized-folders-header = Initialized / indexed folders
settings-code-no-folders-initialized = No folders have been initialized yet.
settings-code-open-project-rules = Open project rules
settings-code-indexing-section-label = INDEXING
settings-code-no-index-created = No index created
settings-code-discovered-chunks = Discovered { $total } chunks
settings-code-syncing-progress = Syncing - { $completed } / { $total }
settings-code-syncing = Syncing...
settings-code-status-synced = Synced
settings-code-status-too-large = Codebase too large
settings-code-status-stale = Stale
settings-code-status-failed = Failed
settings-code-no-index-built = No index built
settings-code-lsp-section-label = LSP SERVERS
settings-code-lsp-installed = Installed
settings-code-lsp-installing = Installing...
settings-code-lsp-checking = Checking...
settings-code-lsp-available-for-download = Available for download
settings-code-lsp-restart-server = Restart server
settings-code-lsp-view-logs = View logs
settings-code-lsp-status-available = Available
settings-code-lsp-status-busy = Busy
settings-code-lsp-status-failed = Failed
settings-code-lsp-status-stopped = Stopped
settings-code-lsp-status-not-running = Not running
settings-code-auto-open-review-panel = Auto open code review panel
settings-code-auto-open-review-panel-desc = When this setting is on, the code review panel will open on the first accepted diff of a conversation
settings-code-show-code-review-button = Show code review button
settings-code-show-code-review-button-desc = Show a button in the top right of the window to toggle the code review panel.
settings-code-show-diff-stats = Show diff stats on code review button
settings-code-show-diff-stats-desc = Show lines added and removed counts on the code review button.
settings-code-project-explorer = Project explorer
settings-code-project-explorer-desc = Adds an IDE-style project explorer / file tree to the left side tools panel.
settings-code-global-search = Global file search
settings-code-global-search-desc = Adds global file search to the left side tools panel.

# --- ANCHOR-SUB-PRIVACY (agent-settings-privacy) ---
settings-privacy-page-title = Privacy
settings-privacy-modal-add-regex-title = Add regex pattern
settings-privacy-safe-mode-title = Secret redaction
settings-privacy-safe-mode-description = When this setting is enabled, Warp will scan blocks, the contents of Warp Drive objects, and Oz prompts for potential sensitive information and prevent saving or sending this data to any servers. You can customize this list via regexes.
settings-privacy-user-secret-regex-title = Custom secret redaction
settings-privacy-user-secret-regex-description = Use regex to define additional secrets or data you'd like to redact. This will take effect when the next command runs. You can use the inline (?i) flag as a prefix to your regex to make it case-insensitive.
settings-privacy-telemetry-title = Help improve Warp
settings-privacy-telemetry-description = App analytics help us make the product better for you. We may collect certain console interactions to improve Warp's AI capabilities.
settings-privacy-telemetry-description-old = App analytics help us make the product better for you. We only collect app usage metadata, never console input or output.
settings-privacy-telemetry-free-tier-note = On the free tier, analytics must be enabled to use AI features.
settings-privacy-telemetry-docs-link = Read more about Warp's use of data
settings-privacy-data-management-title = Manage your data
settings-privacy-data-management-description = At any time, you may choose to delete your Warp account permanently. You will no longer be able to use Warp.
settings-privacy-data-management-link = Visit the data management page
settings-privacy-policy-title = Privacy policy
settings-privacy-policy-link = Read Warp's privacy policy
settings-privacy-tab-personal = Personal
settings-privacy-tab-enterprise = Enterprise
settings-privacy-enterprise-readonly = Enterprise secret redaction cannot be modified.
settings-privacy-enterprise-empty = No enterprise regexes have been configured by your organization.
settings-privacy-recommended = Recommended
settings-privacy-add-all = Add all
settings-privacy-add-regex-button = Add regex
settings-privacy-enterprise-enabled-by-org = Enabled by your organization.
settings-privacy-zdr-badge = ZDR
settings-privacy-zdr-tooltip = Your administrator has enabled zero data retention for your team. User generated content will never be collected.
settings-privacy-secret-display-mode-title = Secret visual redaction mode
settings-privacy-secret-display-mode-description = Choose how secrets are visually presented in the block list while keeping them searchable. This setting only affects what you see in the block list.
settings-privacy-crash-reports-title = Send crash reports
settings-privacy-crash-reports-description = Crash reports assist with debugging and stability improvements.
settings-privacy-cloud-conv-title = Store AI conversations in the cloud
settings-privacy-cloud-conv-description-on = Agent conversations can be shared with others and are retained when you log in on different devices. This data is only stored for product functionality, and Warp will not use it for analytics.
settings-privacy-cloud-conv-description-off = Agent conversations are only stored locally on your machine, are lost upon logout, and cannot be shared. Note: conversation data for ambient agents are still stored in the cloud.
settings-privacy-org-managed-tooltip = This setting is managed by your organization.
settings-privacy-network-log-title = Network log console
settings-privacy-network-log-description = We've built a native console that allows you to view all communications from Warp to external servers to ensure you feel comfortable that your work is always kept safe.
settings-privacy-network-log-link = View network logging

# --- ANCHOR-SUB-EXEC-MODAL-BLOCKS (agent-settings-misc) ---
# ---- execution_profile_view ----
settings-exec-profile-edit-button = Edit
settings-exec-profile-auto = Auto
settings-exec-profile-section-models = MODELS
settings-exec-profile-section-permissions = PERMISSIONS
settings-exec-profile-base-model = Base model:
settings-exec-profile-full-terminal-use = Full terminal use:
settings-exec-profile-computer-use = Computer use:
settings-exec-profile-apply-code-diffs = Apply code diffs:
settings-exec-profile-read-files = Read files:
settings-exec-profile-execute-commands = Execute commands:
settings-exec-profile-interact-running-commands = Interact with running commands:
settings-exec-profile-ask-questions = Ask questions:
settings-exec-profile-call-mcp-servers = Call MCP servers:
settings-exec-profile-call-web-tools = Call web tools:
settings-exec-profile-autosync-plans = Auto-sync plans to Warp Drive:
settings-exec-profile-chips-none = None
settings-exec-profile-perm-agent-decides = Agent decides
settings-exec-profile-perm-always-allow = Always allow
settings-exec-profile-perm-always-ask = Always ask
settings-exec-profile-perm-unknown = Unknown
settings-exec-profile-perm-ask-on-first-write = Ask on first write
settings-exec-profile-perm-never = Never
settings-exec-profile-perm-never-ask = Never ask
settings-exec-profile-perm-ask-unless-auto-approve = Ask unless auto-approve
settings-exec-profile-perm-on = On
settings-exec-profile-perm-off = Off
settings-exec-profile-directory-allowlist = Directory allowlist:
settings-exec-profile-command-allowlist = Command allowlist:
settings-exec-profile-command-denylist = Command denylist:
settings-exec-profile-mcp-allowlist = MCP allowlist:
settings-exec-profile-mcp-denylist = MCP denylist:

# ---- agent_assisted_environment_modal ----
settings-env-modal-add-repo = Add repo
settings-env-modal-cancel = Cancel
settings-env-modal-create-environment = Create environment
settings-env-modal-selected-repos = Selected repos
settings-env-modal-no-repos-selected = No repos selected yet
settings-env-modal-available-repos = Available indexed repos
settings-env-modal-loading = Loading locally indexed repos…
settings-env-modal-empty-no-indexed = No locally indexed repos found yet. Index a repo, then try again.
settings-env-modal-unavailable-build = Local repo selection is unavailable in this build.
settings-env-modal-all-selected = All locally indexed repos are already selected.
settings-env-modal-unknown-repo-name = (unknown)
settings-env-modal-not-git-repo = Selected folder is not a Git repository: { $path }
settings-env-modal-no-directory-selected = No directory selected
settings-env-modal-dialog-title = Select repos for your environment
settings-env-modal-dialog-description-indexed = Select locally indexed repos to provide context for the environment creation agent.
settings-env-modal-dialog-description-default = Select repos to provide context for the environment creation agent.

# ---- show_blocks_view ----
settings-show-blocks-page-title = Shared blocks
settings-show-blocks-unshare-menu-item = Unshare
settings-show-blocks-copy-link = Copy link
settings-show-blocks-deleting = Deleting...
settings-show-blocks-executed-on = Executed on: { $time }
settings-show-blocks-empty = You don't have any shared blocks yet.
settings-show-blocks-loading = Getting blocks...
settings-show-blocks-load-failed = Failed to load blocks. Please try again.
settings-show-blocks-link-copied = Link copied.
settings-show-blocks-unshare-success = Block was successfully unshared.
settings-show-blocks-unshare-failed = Failed to unshare block. Please try again.
settings-show-blocks-confirm-dialog-title = Unshare block
settings-show-blocks-confirm-dialog-text = Are you sure you want to unshare this block?

    It will no longer be accessible by link and will be permanently deleted from Warp servers.
settings-show-blocks-confirm-cancel = Cancel
settings-show-blocks-confirm-unshare = Unshare

# --- ANCHOR-SUB-APPEARANCE (agent-settings-appearance) ---
# 此锚点下放 settings_view/appearance_page.rs 剩余字符串(不含已完成的 Language widget)
# 命名前缀:settings-appearance-*

# Categories
settings-appearance-category-themes = Themes
settings-appearance-category-language = Language
settings-appearance-category-icon = Icon
settings-appearance-category-window = Window
settings-appearance-category-input = Input
settings-appearance-category-panes = Panes
settings-appearance-category-blocks = Blocks
settings-appearance-category-text = Text
settings-appearance-category-cursor = Cursor
settings-appearance-category-tabs = Tabs
settings-appearance-category-fullscreen-apps = Full-screen Apps

# Theme widget
settings-appearance-theme-create-custom = Create your own custom theme
settings-appearance-theme-mode-light = Light
settings-appearance-theme-mode-dark = Dark
settings-appearance-theme-mode-current = Current theme
settings-appearance-theme-sync-os-label = Sync with OS
settings-appearance-theme-sync-os-description = Automatically switch between light and dark themes when your system does.

# Custom App Icon widget
settings-appearance-custom-icon-label = Customize your app icon
settings-appearance-custom-icon-bundle-warning = Changing the app icon requires the app to be bundled.
settings-appearance-custom-icon-restart-warning = You may need to restart Warp for MacOS to apply the preferred icon style.

# Window widgets
settings-appearance-window-custom-size-label = Open new windows with custom size
settings-appearance-window-columns-label = Columns
settings-appearance-window-rows-label = Rows
settings-appearance-window-opacity-label = Window Opacity:
settings-appearance-window-opacity-value = Window Opacity: { $value }
settings-appearance-window-opacity-not-supported = Transparency is not supported with your graphics drivers.
settings-appearance-window-opacity-graphics-warning = The selected graphics settings may not support rendering transparent windows.
settings-appearance-window-opacity-graphics-warning-hint = Try changing the settings for the graphics backend or integrated GPU in Features > System.
settings-appearance-window-blur-radius = Window Blur Radius: { $value }
settings-appearance-window-blur-texture-label = Use Window Blur (Acrylic texture)
settings-appearance-window-blur-texture-not-supported = The selected hardware may not support rendering transparent windows.
settings-appearance-tools-panel-consistent-label = Tools panel visibility is consistent across tabs

# Input
settings-appearance-input-type-label = Input type
settings-appearance-input-type-warp = Warp
settings-appearance-input-type-shell = Shell (PS1)
settings-appearance-input-position-label = Input position
settings-appearance-input-mode-pinned-bottom = Pin to the bottom (Warp mode)
settings-appearance-input-mode-pinned-top = Pin to the top (Reverse mode)
settings-appearance-input-mode-waterfall = Start at the top (Classic mode)

# Panes
settings-appearance-pane-dim-inactive-label = Dim inactive panes
settings-appearance-pane-focus-follows-mouse-label = Focus follows mouse

# Blocks
settings-appearance-block-compact-label = Compact mode
settings-appearance-block-jump-bottom-label = Show Jump to Bottom of Block button
settings-appearance-block-show-dividers-label = Show block dividers

# Text / Fonts
settings-appearance-font-agent-label = Agent font
settings-appearance-font-match-terminal = Match terminal
settings-appearance-font-terminal-label = Terminal font
settings-appearance-font-view-all-system = View all available system fonts
settings-appearance-font-weight-label = Font weight
settings-appearance-font-size-label = Font size (px)
settings-appearance-font-line-height-label = Line height
settings-appearance-font-reset-default = Reset to default
settings-appearance-font-notebook-size-label = Notebook font size
settings-appearance-font-thin-strokes-label = Use thin strokes
settings-appearance-font-thin-strokes-never = Never
settings-appearance-font-thin-strokes-low-dpi = On low-DPI displays
settings-appearance-font-thin-strokes-high-dpi = On high-DPI displays
settings-appearance-font-thin-strokes-always = Always
settings-appearance-font-min-contrast-label = Enforce minimum contrast
settings-appearance-font-min-contrast-always = Always
settings-appearance-font-min-contrast-named-only = Only for named colors
settings-appearance-font-min-contrast-never = Never
settings-appearance-font-ligatures-label = Show ligatures in terminal
settings-appearance-font-ligatures-perf-tooltip = Ligatures may reduce performance

# Cursor
settings-appearance-cursor-type-label = Cursor type
settings-appearance-cursor-disabled-vim = Cursor type is disabled in Vim mode
settings-appearance-cursor-blink-label = Blinking cursor

# Tabs
settings-appearance-tab-close-position-label = Tab close button position
settings-appearance-tab-close-position-right = Right
settings-appearance-tab-close-position-left = Left
settings-appearance-tab-show-indicators-label = Show tab indicators
settings-appearance-tab-show-code-review-label = Show code review button
settings-appearance-tab-preserve-active-color-label = Preserve active tab color for new tabs
settings-appearance-tab-vertical-layout-label = Use vertical tab layout
settings-appearance-tab-use-prompt-as-title-label = Use latest user prompt as conversation title in tab names
settings-appearance-tab-use-prompt-as-title-description = Show the latest user prompt instead of the generated conversation title for Oz and third-party agent sessions in vertical tabs.
settings-appearance-tab-toolbar-layout-label = Header toolbar layout
settings-appearance-tab-directory-colors-label = Directory tab colors
settings-appearance-tab-directory-colors-description = Automatically color tabs based on the directory or repo you're working in.
settings-appearance-tab-directory-color-default-tooltip = Default (no color)
settings-appearance-zen-mode-label = Show the tab bar
settings-appearance-zen-decoration-always = Always
settings-appearance-zen-decoration-windowed = When windowed
settings-appearance-zen-decoration-on-hover = Only on hover

# Full-screen apps
settings-appearance-alt-screen-padding-label = Use custom padding in alt-screen
settings-appearance-alt-screen-uniform-padding-label = Uniform padding (px)

# Zoom
settings-appearance-zoom-label = Zoom
settings-appearance-zoom-secondary = Adjusts the default zoom level across all windows

# --- ANCHOR-SUB-ENVIRONMENTS (agent-settings-environments) ---
settings-environments-page-title = Environments
settings-environments-page-description = Environments define where your ambient agents run. Set one up in minutes via GitHub (recommended), Warp-assisted setup, or manual configuration.
settings-environments-search-placeholder = Search environments...
settings-environments-no-matches = No environments match your search.
settings-environments-section-personal = Personal
settings-environments-section-team-default = Shared by Warp and your team
settings-environments-section-team-named = Shared by Warp and { $team }
settings-environments-env-id-prefix = Env ID: { $id }
settings-environments-detail-image = Image: { $image }
settings-environments-detail-repos = Repos: { $repos }
settings-environments-detail-setup-commands = Setup commands: { $commands }
settings-environments-last-edited = Last edited: { $time }
settings-environments-last-used = Last used: { $time }
settings-environments-last-used-never = Last used: never
settings-environments-view-my-runs = View my runs
settings-environments-tooltip-share = Share
settings-environments-tooltip-edit = Edit
settings-environments-empty-header = You haven’t set up any environments yet.
settings-environments-empty-subheader = Choose how you’d like to set up your environment:
settings-environments-empty-quick-setup-title = Quick setup
settings-environments-empty-suggested-badge = Suggested
settings-environments-empty-quick-setup-subtitle = Select the GitHub repositories you’d like to work with and we’ll suggest a base image and config
settings-environments-empty-use-agent-title = Use the agent
settings-environments-empty-use-agent-subtitle = Choose a locally set up project and we’ll help you set up an environment based on it
settings-environments-button-loading = Loading...
settings-environments-button-retry = Retry
settings-environments-button-authorize = Authorize
settings-environments-button-get-started = Get started
settings-environments-button-launch-agent = Launch agent
settings-environments-toast-update-success = Successfully updated environment
settings-environments-toast-create-success = Successfully created environment
settings-environments-toast-delete-success = Environment deleted successfully
settings-environments-toast-share-success = Successfully shared environment
settings-environments-toast-share-failure = Failed to share environment with team
settings-environments-toast-create-not-logged-in = Unable to create environment: not logged in.
settings-environments-toast-save-not-found = Unable to save: environment no longer exists.
settings-environments-toast-share-no-team = Unable to share environment: you are not currently on a team.
settings-environments-toast-share-not-synced = Unable to share environment: environment is not yet synced.

# --- ANCHOR-SUB-AGENT-PROVIDERS (agent-settings-agent-providers) ---
# 此锚点下放 settings_view/agent_providers_widget.rs 字符串
# 命名前缀:settings-agent-providers-*
settings-agent-providers-title = Agent providers
settings-agent-providers-description = Configure custom OpenAI-compatible Agent providers (DeepSeek, Zhipu GLM, Moonshot, Alibaba DashScope, SiliconFlow, OpenRouter, local Ollama, etc.). You can add models manually (display name + model ID mapping) or fetch them automatically from the API. Provider metadata is stored in the local settings.toml; API keys are stored securely in the system keychain.
settings-agent-providers-empty = No providers configured yet. Click the button below to add one.
settings-agent-providers-add-button = + Add OpenAI-compatible provider
settings-agent-providers-search-placeholder = Search providers…
settings-agent-providers-quick-add-title = Quick add
settings-agent-providers-refresh-catalog = Refresh catalog
settings-agent-providers-loading-catalog = Loading models.dev catalog… (the first load may take a few seconds)
settings-agent-providers-catalog-empty = models.dev catalog is empty. Click [Refresh catalog] to retry.
settings-agent-providers-no-match = No match for "{ $query }"
settings-agent-providers-collapse = Collapse ▲
settings-agent-providers-expand-remaining = Expand remaining { $count } ▼
settings-agent-providers-row-missing = (no editors bound for this provider yet: { $id })
settings-agent-providers-field-name = Name
settings-agent-providers-field-base-url = Base URL
settings-agent-providers-field-api-key = API Key
settings-agent-providers-field-api-type = API Type
settings-agent-providers-api-type-hint = (genai uses this to bind the adapter explicitly, avoiding misdetection by model name. If Base URL is empty, the default will be used: { $url })
settings-agent-providers-name-placeholder = Custom provider name (e.g. DeepSeek, local Ollama)
settings-agent-providers-api-key-placeholder = sk-... (saved to system keychain on blur or Enter)
settings-agent-providers-models-label = Models ({ $count })
settings-agent-providers-models-empty-hint = No models configured yet. Click [+ Add model] to add manually, or [Fetch from API] to fetch automatically.
settings-agent-providers-models-header-name = Display name
settings-agent-providers-models-header-id = Model ID
settings-agent-providers-models-header-context = Context (tok)
settings-agent-providers-models-header-output = Output (tok)
settings-agent-providers-model-name-placeholder = Display name (e.g. DS-V3 General)
settings-agent-providers-model-id-placeholder = Model ID (the `model` field sent to the API, e.g. deepseek-chat)
settings-agent-providers-model-context-placeholder = Context (tokens)
settings-agent-providers-model-output-placeholder = Output (tokens)
settings-agent-providers-add-model = + Add model
settings-agent-providers-fetch-from-api = Fetch from API
settings-agent-providers-sync-models-dev = Sync from models.dev
settings-agent-providers-remove = Remove

# ---- AI page (settings_view/ai_page.rs) ----
settings-ai-title = AI
settings-ai-active-ai = Active AI
settings-ai-input-autodetection = terminal command autodetection in agent input
settings-ai-input-autodetection-legacy = natural language detection
settings-ai-next-command-description = Let AI suggest the next command to run based on your command history, outputs, and common workflows.
settings-ai-prompt-suggestions-description = Let AI suggest natural language prompts, as inline banners in the input, based on recent commands and their outputs.
settings-ai-suggested-code-banners-description = Let AI suggest code diffs and queries as inline banners in the blocklist, based on recent commands and their outputs.
settings-ai-natural-language-autosuggestions = Let AI suggest natural language autosuggestions, based on recent commands and their outputs.
settings-ai-shared-block-title-generation-description = Let AI generate a title for your shared block based on the command and output.
settings-ai-git-operations-autogen-description = Let AI generate commit messages and pull request titles and descriptions.

# =============================================================================
# SECTION: ai (Owner: agent-ai)
# Files: app/src/ai/**, app/src/ai_assistant/**
# =============================================================================

# (placeholder — to be filled by agent-ai)

# =============================================================================
# SECTION: command-palette (Owner: agent-cmdpal)
# Files: app/src/command_palette.rs, app/src/palette/**
# =============================================================================

# (placeholder)

# =============================================================================
# SECTION: drive (Owner: agent-drive)
# Files: app/src/drive/**
# =============================================================================

# (placeholder)

# =============================================================================
# SECTION: workspace (Owner: agent-workspace)
# Files: app/src/workspace/**, app/src/workspaces/**
# =============================================================================

# (placeholder)

# =============================================================================
# SECTION: modal (Owner: agent-modal)
# Files: app/src/modal/**, app/src/prompt/**, app/src/quit_warning/**
# =============================================================================

# (placeholder)

# =============================================================================
# SECTION: auth (Owner: agent-auth)
# Files: app/src/auth/**
# =============================================================================

# (placeholder)

# =============================================================================
# SECTION: banner (Owner: agent-banner)
# Files: app/src/banner/**
# =============================================================================

banner-dont-show-again = Don't show me again

# =============================================================================
# SECTION: quit-warning (Owner: agent-quit-warning)
# Files: app/src/quit_warning/mod.rs
# =============================================================================

# ---- Dialog titles ----
quit-warning-title-pane = Close pane?
quit-warning-title-tab-singular = Close tab?
quit-warning-title-tab-plural = Close tabs?
quit-warning-title-window = Close window?
quit-warning-title-app = Quit Warp?
quit-warning-title-editor-tab = Save changes?

# ---- Buttons ----
quit-warning-button-confirm-close = Yes, close
quit-warning-button-confirm-quit = Yes, quit
quit-warning-button-save = Save
quit-warning-button-discard = Don't Save
quit-warning-button-show-processes = Show running processes
quit-warning-button-cancel = Cancel

# ---- Warning body lines ----
# Suffix appended to each warning line, indicating the scope.
quit-warning-suffix-tab = { " " }in this tab.
quit-warning-suffix-window = { " " }in this window.
quit-warning-suffix-pane = { " " }in this pane.
quit-warning-suffix-default = .

# Process info: "{count} process(es) running" with optional window/tab qualifier.
quit-warning-processes-running = You have { $count } { $count ->
        [one] process
       *[other] processes
    } running
quit-warning-processes-in-windows = { " " }in { $count } windows
quit-warning-processes-in-tabs = { " " }in { $count } tabs

# Shared sessions line.
quit-warning-shared-sessions = You are sharing { $count } { $count ->
        [one] session
       *[other] sessions
    }

# Unsaved code changes (generic scope).
quit-warning-unsaved-changes = You have unsaved file changes

# Unsaved code changes for a specific editor tab.
quit-warning-unsaved-editor-tab = Do you want to save the changes you made to { $file }? Your changes will be discarded if you don't save them.
quit-warning-unsaved-editor-tab-fallback-name = this file
