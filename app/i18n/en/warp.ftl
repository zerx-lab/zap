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
# 此锚点下放 settings_view/code_page.rs 字符串
# 命名前缀:settings-code-*
# (待 agent-settings-code 填充)

# --- ANCHOR-SUB-PRIVACY (agent-settings-privacy) ---
# 此锚点下放 settings_view/privacy_page.rs 字符串
# 命名前缀:settings-privacy-*
# (待 agent-settings-privacy 填充)

# --- ANCHOR-SUB-EXEC-MODAL-BLOCKS (agent-settings-misc) ---
# 此锚点下放 execution_profile_view.rs / agent_assisted_environment_modal.rs / show_blocks_view.rs 字符串
# 命名前缀:settings-exec-profile-* / settings-env-modal-* / settings-show-blocks-*
# (待 agent-settings-misc 填充)

# --- ANCHOR-SUB-APPEARANCE (agent-settings-appearance) ---
# 此锚点下放 settings_view/appearance_page.rs 剩余字符串(不含已完成的 Language widget)
# 命名前缀:settings-appearance-*
# (待 agent-settings-appearance 填充)

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
