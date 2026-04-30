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

# ---- destructive_mcp_confirmation_dialog.rs ----
settings-mcp-confirm-delete-local-title = Delete MCP server?
settings-mcp-confirm-delete-local-description = This will uninstall and remove this MCP server from all your devices.
settings-mcp-confirm-delete-shared-title = Delete shared MCP server?
settings-mcp-confirm-delete-shared-description = This will not only delete this MCP server for yourself, but also uninstall and remove this MCP server from Warp and across all of your teammates' devices.
settings-mcp-confirm-unshare-title = Remove shared MCP server from team?
settings-mcp-confirm-unshare-description = This will uninstall and remove this MCP server from Warp and across all of your teammates' devices.
settings-mcp-confirm-delete-button = Delete MCP
settings-mcp-confirm-remove-from-team-button = Remove from team
settings-mcp-confirm-cancel-button = Cancel

# ---- edit_page.rs ----
settings-mcp-edit-save = Save
settings-mcp-edit-edit-variables = Edit Variables
settings-mcp-edit-delete = Delete MCP
settings-mcp-edit-remove-from-team = Remove from team
settings-mcp-edit-editing-disabled-banner = Only team admins and the creator of the MCP server can edit the MCP server.
settings-mcp-edit-add-new-title = Add New MCP Server
settings-mcp-edit-edit-named-title = Edit { $name } MCP Server
settings-mcp-edit-edit-title = Edit MCP Server
settings-mcp-edit-logout-tooltip = Log out
settings-mcp-edit-secrets-error = This MCP server contains secrets. Visit Settings > Privacy to modify your secret redaction settings.
settings-mcp-edit-no-server-error = No MCP Server specified.
settings-mcp-edit-multiple-servers-error = Cannot add multiple MCP servers while editing a single server.

# ---- installation_modal.rs ----
settings-mcp-install-modal-title = Install { $name }
settings-mcp-install-modal-source-shared = Shared from team
settings-mcp-install-modal-source-other-device = From another device
settings-mcp-install-modal-cancel = Cancel
settings-mcp-install-modal-install = Install
settings-mcp-install-modal-no-server = No MCP server selected

# ---- list_page.rs ----
settings-mcp-list-description = Add MCP servers to extend the Warp Agent's capabilities. MCP servers expose data sources or tools to agents through a standardized interface, essentially acting like plugins. Add a custom server, or use the presets to get started with popular servers. You can also find team servers that have been shared with you here.
settings-mcp-list-learn-more = Learn more.
settings-mcp-list-empty-state = Once you add a MCP server, it will be shown here.
settings-mcp-list-no-search-results = No search results found
settings-mcp-list-search-placeholder = Search MCP Servers
settings-mcp-list-add-button = Add
settings-mcp-list-file-based-toggle-label = Auto-spawn servers from third-party agents
settings-mcp-list-file-based-description = Automatically detect and spawn MCP servers from globally-scoped third-party AI agent configuration files (e.g. in your home directory). Servers detected inside a repository are never spawned automatically and must be enabled individually in the "Detected from" sections below.
settings-mcp-list-file-based-supported-providers = See supported providers.
settings-mcp-list-template-available-to-install = Available to install
settings-mcp-list-file-based-detected = Detected from config file
settings-mcp-list-toast-server-updated = MCP server updated
settings-mcp-list-section-my-mcps = My MCPs
settings-mcp-list-section-shared-by-warp-and-team = Shared by Warp and { $name }
settings-mcp-list-section-shared-by-warp-and-other-devices = Shared by Warp and from other devices
settings-mcp-list-section-shared-from-warp = Shared from Warp
settings-mcp-list-section-detected-from = Detected from { $provider }
settings-mcp-list-chip-global = global
settings-mcp-list-chip-shared-by-creator = Shared by: { $creator }
settings-mcp-list-chip-shared-by-team-member = Shared by a team member
settings-mcp-list-chip-from-another-device = From another device

# ---- server_card.rs ----
settings-mcp-card-tooltip-show-logs = Show logs
settings-mcp-card-tooltip-log-out = Log out
settings-mcp-card-tooltip-share-server = Share server
settings-mcp-card-tooltip-edit = Edit
settings-mcp-card-tooltip-update-available = Server update available
settings-mcp-card-button-view-logs = View logs
settings-mcp-card-button-edit-config = Edit config
settings-mcp-card-button-set-up = Set up
settings-mcp-card-tools-none = No tools available
settings-mcp-card-tools-available = { $count } tools available
settings-mcp-card-status-offline = Offline
settings-mcp-card-status-starting = Starting server...
settings-mcp-card-status-authenticating = Authenticating...
settings-mcp-card-status-shutting-down = Shutting down...

# ---- update_modal.rs ----
settings-mcp-update-modal-default-name = Server
settings-mcp-update-modal-title = Update { $name }
settings-mcp-update-modal-description = This server has { $count } updates available, which would you like to proceed with?
settings-mcp-update-modal-publisher-another-device = another device
settings-mcp-update-modal-publisher-team-member = a team member
settings-mcp-update-modal-update-from = Update from { $publisher }
settings-mcp-update-modal-version = Version { $version }
settings-mcp-update-modal-cancel = Cancel
settings-mcp-update-modal-update = Update
settings-mcp-update-modal-no-updates = No updates available

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

# --- ANCHOR-SUB-AI-PAGE (agent-settings-ai-page) ---
# Section / sub-headers
settings-ai-warp-agent-header = Warp Agent
settings-ai-active-ai-section = Active AI
settings-ai-input-section = Input
settings-ai-mcp-servers-section = MCP Servers
settings-ai-knowledge-section = Knowledge
settings-ai-voice-section = Voice
settings-ai-other-section = Other
settings-ai-third-party-cli-section = Third party CLI agents
settings-ai-agent-attribution-section = Agent Attribution
settings-ai-experimental-section = Experimental
settings-ai-aws-bedrock-section = AWS Bedrock
settings-ai-agents-header = Agents
settings-ai-profiles-header = Profiles
settings-ai-models-subheader = Models
settings-ai-permissions-subheader = Permissions
settings-ai-usage-header = Usage
settings-ai-credits-label = Credits

# Active AI toggle labels
settings-ai-next-command-label = Next Command
settings-ai-prompt-suggestions-label = Prompt Suggestions
settings-ai-suggested-code-banners-label = Suggested Code Banners
settings-ai-natural-language-autosuggestions-label = Natural Language Autosuggestions
settings-ai-shared-block-title-generation-label = Shared Block Title Generation
settings-ai-git-operations-autogen-label = Commit & Pull Request Generation

# Permissions dropdown options
settings-ai-permission-agent-decides = Agent decides
settings-ai-permission-always-allow = Always allow
settings-ai-permission-always-ask = Always ask
settings-ai-permission-ask-on-first-write = Ask on first write
settings-ai-permission-read-only = Read only
settings-ai-permission-supervised = Supervised
settings-ai-permission-allow-specific-dirs = Allow in specific directories

# Permission row labels
settings-ai-apply-code-diffs = Apply code diffs
settings-ai-read-files = Read files
settings-ai-execute-commands = Execute commands
settings-ai-interact-running-commands = Interact with running commands
settings-ai-call-mcp-servers = Call MCP servers
settings-ai-command-denylist = Command denylist
settings-ai-command-denylist-description = Regular expressions to match commands that the Warp Agent should always ask permission to execute.
settings-ai-command-allowlist = Command allowlist
settings-ai-command-allowlist-description = Regular expressions to match commands that can be automatically executed by the Warp Agent.
settings-ai-directory-allowlist = Directory allowlist
settings-ai-directory-allowlist-description = Give the agent file access to certain directories.
settings-ai-mcp-allowlist = MCP allowlist
settings-ai-mcp-allowlist-description = Allow the Warp Agent to call these MCP servers.
settings-ai-mcp-denylist = MCP denylist
settings-ai-mcp-denylist-description = The Warp Agent will always ask for permission before calling any MCP servers on this list.
settings-ai-info-banner-managed-by-workspace = Some of your permissions are managed by your workspace.

# Models / Profiles
settings-ai-base-model = Base model
settings-ai-base-model-description = This model serves as the primary engine behind the Warp Agent. It powers most interactions and invokes other models for tasks like planning or code generation when necessary. Warp may automatically switch to alternate models based on model availability or for auxiliary tasks such as conversation summarization.
settings-ai-show-model-picker-in-prompt = Show model picker in prompt
settings-ai-codebase-context = Codebase Context
settings-ai-codebase-context-description = Allow the Warp Agent to generate an outline of your codebase that can be used for context. No code is ever stored on our servers.
settings-ai-add-profile = Add Profile
settings-ai-agents-description = Set the boundaries for how your Agent operates. Choose what it can access, how much autonomy it has, and when it must ask for your approval. You can also fine-tune behavior around natural language input, codebase awareness, and more.
settings-ai-profiles-description = Profiles let you define how your Agent operates — from the actions it can take and when it needs approval, to the models it uses for tasks like coding and planning. You can also scope them to individual projects.

# Anonymous / org gates
settings-ai-sign-up = Sign up
settings-ai-anonymous-create-account = To use AI features, please create an account.
settings-ai-org-disallows-remote-session = Your organization disallows AI when the active pane contains content from a remote session
settings-ai-org-enforced-tooltip = This option is enforced by your organization's settings and cannot be customized.
settings-ai-restricted-billing = Restricted due to billing issue
settings-ai-unlimited = Unlimited

# AI Input section
settings-ai-show-input-hint-text = Show input hint text
settings-ai-show-agent-tips = Show agent tips
settings-ai-include-agent-commands-in-history = Include agent-executed commands in history
settings-ai-autodetect-agent-prompts = Autodetect agent prompts in terminal input
settings-ai-autodetect-terminal-commands = Autodetect terminal commands in agent input
settings-ai-natural-language-detection = Natural language detection
settings-ai-natural-language-denylist = Natural language denylist
settings-ai-natural-language-denylist-description = Commands listed here will never trigger natural language detection.
settings-ai-let-us-know = Let us know

# MCP Servers
settings-ai-learn-more = Learn more
settings-ai-add-server = Add a server
settings-ai-manage-mcp-servers = Manage MCP servers
settings-ai-file-based-mcp-toggle = Auto-spawn servers from third-party agents
settings-ai-file-based-mcp-supported-providers = See supported providers.
settings-ai-mcp-dropdown-header = Select MCP servers

# Knowledge / Rules
settings-ai-rules-label = Rules
settings-ai-suggested-rules-label = Suggested Rules
settings-ai-suggested-rules-description = Let AI suggest rules to save based on your interactions.
settings-ai-manage-rules = Manage rules
settings-ai-rules-description = Rules help the Warp Agent follow your conventions, whether for codebases or specific workflows.

# Voice
settings-ai-voice-input-label = Voice Input
settings-ai-voice-key = Key for Activating Voice Input
settings-ai-voice-key-hint = Press and hold to activate.

# Other section
settings-ai-show-oz-changelog = Show Oz changelog in new conversation view
settings-ai-show-use-agent-footer = Show "Use Agent" footer
settings-ai-use-agent-footer-description = Shows hint to use the "Full Terminal Use"-enabled agent in long running commands.
settings-ai-show-conversation-history = Show conversation history in tools panel
settings-ai-thinking-display = Agent thinking display
settings-ai-thinking-display-description = Controls how reasoning/thinking traces are displayed.
settings-ai-conversation-layout-label = Preferred layout when opening existing agent conversations
settings-ai-conversation-layout-newtab = New Tab
settings-ai-conversation-layout-splitpane = Split Pane
settings-ai-toolbar-layout = Toolbar layout

# Third-party CLI agents
settings-ai-show-coding-agent-toolbar = Show coding agent toolbar
settings-ai-auto-show-rich-input = Auto show/hide Rich Input based on agent status
settings-ai-auto-show-rich-input-tooltip = Requires the Warp plugin for your coding agent
settings-ai-auto-open-rich-input = Auto open Rich Input when a coding agent session starts
settings-ai-auto-dismiss-rich-input = Auto dismiss Rich Input after prompt submission
settings-ai-toolbar-commands-label = Commands that enable the toolbar
settings-ai-toolbar-commands-description = Add regex patterns to show the coding agent toolbar for matching commands.
settings-ai-coding-agent-other = Other
settings-ai-coding-agent-select-header = Select coding agent

# Agent Attribution
settings-ai-enable-agent-attribution = Enable agent attribution
settings-ai-agent-attribution-description = Oz can add attribution to commit messages and pull requests it creates

# Experimental / Cloud Agent
settings-ai-cloud-agent-computer-use = Computer use in Cloud Agents
settings-ai-cloud-agent-computer-use-description = Enable computer use in cloud agent conversations started from the Warp app.
settings-ai-orchestration-label = Orchestration
settings-ai-orchestration-description = Enable multi-agent orchestration, allowing the agent to spawn and coordinate parallel sub-agents.

# AWS Bedrock
settings-ai-aws-bedrock-toggle = Use AWS Bedrock credentials
settings-ai-aws-bedrock-description = Warp loads and sends local AWS CLI credentials for Bedrock-supported models.
settings-ai-aws-bedrock-description-managed = Warp loads and sends local AWS CLI credentials for Bedrock-supported models. This setting is managed by your organization.
settings-ai-aws-login-command = Login Command
settings-ai-aws-profile = AWS Profile
settings-ai-aws-auto-login = Automatically run login command
settings-ai-aws-auto-login-description = When enabled, the login command will run automatically when AWS Bedrock credentials expire.
settings-ai-refresh = Refresh

# --- ANCHOR-SUB-FEATURES (agent-settings-features) ---
# settings_view/features_page.rs P0 + P1(category + toggle labels)
# 命名前缀:settings-features-*
settings-features-category-general = General
settings-features-category-session = Session
settings-features-category-keys = Keys
settings-features-category-text-editing = Text Editing
settings-features-category-terminal-input = Terminal Input
settings-features-category-terminal = Terminal
settings-features-category-notifications = Notifications
settings-features-category-workflows = Workflows
settings-features-category-system = System
settings-features-open-links-in-desktop = Open links in desktop app
settings-features-open-links-in-desktop-tooltip = Automatically open links in desktop app whenever possible.
settings-features-restore-session = Restore windows, tabs, and panes on startup
settings-features-show-sticky-command-header = Show sticky command header
settings-features-show-link-tooltip = Show tooltip on click on links
settings-features-show-quit-warning = Show warning before quitting/logging out
settings-features-quit-on-last-window-closed = Quit when all windows are closed
settings-features-show-changelog-after-update = Show changelog toast after updates
settings-features-mouse-scroll-multiplier = Lines scrolled by mouse wheel interval
settings-features-auto-open-code-review = Auto open code review panel
settings-features-max-rows-per-block = Maximum rows in a block
settings-features-ssh-wrapper = Warp SSH Wrapper
settings-features-receive-desktop-notifications = Receive desktop notifications from Warp
settings-features-show-in-app-agent-notifications = Show in-app agent notifications
settings-features-confirm-close-shared-session = Confirm before closing shared session
settings-features-global-hotkey-label = Global hotkey:
settings-features-global-hotkey-not-supported-on-wayland = Not supported on Wayland.
settings-features-autocomplete-symbols = Autocomplete quotes, parentheses, and brackets
settings-features-error-underlining = Error underlining for commands
settings-features-syntax-highlighting = Syntax highlighting for commands
settings-features-completions-while-typing = Open completions menu as you type
settings-features-command-corrections = Suggest corrected commands
settings-features-expand-aliases = Expand aliases as you type
settings-features-middle-click-paste = Middle-click to paste
settings-features-vim-mode = Edit code and commands with Vim keybindings
settings-features-at-context-menu = Enable '@' context menu in terminal mode
settings-features-slash-commands-in-terminal = Enable slash commands in terminal mode
settings-features-outline-codebase-symbols = Outline codebase symbols for '@' context menu
settings-features-show-input-message-bar = Show terminal input message line
settings-features-show-autosuggestion-hint = Show autosuggestion keybinding hint
settings-features-show-autosuggestion-ignore = Show autosuggestion ignore button
settings-features-enable-mouse-reporting = Enable Mouse Reporting
settings-features-enable-scroll-reporting = Enable Scroll Reporting
settings-features-enable-focus-reporting = Enable Focus Reporting
settings-features-use-audible-bell = Use Audible Bell
settings-features-double-click-smart-selection = Double-click smart selection
settings-features-show-help-block-in-new-sessions = Show help block in new sessions
settings-features-copy-on-select = Copy on select
settings-features-show-global-workflows-in-command-search = Show Global Workflows in Command Search (ctrl-r)
settings-features-linux-selection-clipboard = Honor linux selection clipboard
settings-features-prefer-low-power-gpu = Prefer rendering new windows with integrated GPU (low power)
settings-features-use-wayland = Use Wayland for window management
settings-features-use-wayland-tooltip = Enables the use of Wayland
settings-features-ctrl-tab-behavior-label = Ctrl+Tab behavior:
settings-features-extra-meta-key-left-mac = Left Option key is Meta
settings-features-extra-meta-key-right-mac = Right Option key is Meta
settings-features-extra-meta-key-left-other = Left Alt key is Meta
settings-features-extra-meta-key-right-other = Right Alt key is Meta
settings-features-default-shell-header = Default shell for new sessions
settings-features-working-directory-header = Working directory for new sessions
settings-features-notify-agent-task-completed = Notify when an agent completes a task
settings-features-notify-needs-attention = Notify when a command or agent needs your attention to continue
settings-features-play-notification-sounds = Play notification sounds

# --- ANCHOR-SUB-TEAMS (agent-settings-teams) ---
# settings_view/teams_page.rs strings (P0 + P1)
# 命名前缀:settings-teams-*
settings-teams-page-title = Teams
settings-teams-create-page-subtitle = Create a team
settings-teams-create-description = When you create a team, you can collaborate on agent-driven development by sharing cloud agent runs, environments, automations, and artifacts. You can also create a shared knowledge store for teammates and agents alike.
settings-teams-create-button = Create
settings-teams-team-name-placeholder = Team name
settings-teams-rename-placeholder = Your new team name
settings-teams-leave-team-button = Leave team
settings-teams-delete-team-button = Delete team
settings-teams-emails-placeholder = Emails, comma separated
settings-teams-domains-placeholder = Domains, comma separated
settings-teams-set-button = Set
settings-teams-invite-button = Invite
settings-teams-join-button = Join
settings-teams-contact-admin-button = Contact Admin to request access
settings-teams-tab-link = Link
settings-teams-tab-email = Email
settings-teams-section-team-members = Team Members
settings-teams-section-team-members-pricing = Team members
settings-teams-section-invite-by-link = Invite by Link
settings-teams-section-invite-by-email = Invite by Email
settings-teams-section-restrict-by-domain = Restrict by domain
settings-teams-section-make-discoverable = Make team discoverable
settings-teams-section-plan-usage-free = Free plan usage limits
settings-teams-section-plan-usage = Plan usage limits
settings-teams-shared-notebooks = Shared Notebooks
settings-teams-shared-workflows = Shared Workflows
settings-teams-reset-links = Reset links
settings-teams-compare-plans = Compare plans
settings-teams-upgrade-build = Upgrade to Build
settings-teams-upgrade-turbo = Upgrade to Turbo plan
settings-teams-upgrade-lightspeed = Upgrade to Lightspeed plan
settings-teams-contact-support = Contact support
settings-teams-manage-billing = Manage billing
settings-teams-manage-plan = Manage plan
settings-teams-open-admin-panel = Open admin panel
settings-teams-or-join-existing = Or, join an existing team within your company
settings-teams-discovery-cta = Join this team and start collaborating on workflows, notebooks, and more.
settings-teams-discovery-1-teammate = 1 teammate
settings-teams-discovery-n-teammates = { $count } teammates
settings-teams-transfer-modal-title = Transfer team ownership?
settings-teams-action-cancel-invite = Cancel invite
settings-teams-action-transfer-ownership = Transfer ownership
settings-teams-action-demote-from-admin = Demote from admin
settings-teams-action-promote-to-admin = Promote to admin
settings-teams-action-remove-from-team = Remove from team
settings-teams-action-remove-domain = Remove domain
settings-teams-state-expired = EXPIRED
settings-teams-state-pending = PENDING
settings-teams-state-owner = OWNER
settings-teams-state-admin = ADMIN
settings-teams-badge-past-due = PAST DUE
settings-teams-badge-unpaid = UNPAID
settings-teams-offline = You are offline.
settings-teams-failed-load-invite-link = Failed to load invite link.
settings-teams-toast-link-copied = Link copied to clipboard!
settings-teams-toast-invite-sent-one = Your invite is on the way!
settings-teams-toast-invites-sent = Your { $count } invites are on the way!
settings-teams-toast-domain-added = Domain restrictions added: { $count }
settings-teams-toast-invalid-domains = Invalid domains: { $count }
settings-teams-toast-invalid-emails = Invalid emails: { $count }
settings-teams-toast-toggled-invite-links = Toggled invite links
settings-teams-toast-reset-invite-links = Reset invite links
settings-teams-toast-deleted-invite = Deleted invite
settings-teams-toast-toggled-discoverability = Toggled team discoverability
settings-teams-toast-joined-team = Successfully joined team
settings-teams-toast-joined-team-named = Successfully joined { $name }
settings-teams-toast-transferred-ownership = Successfully transferred team ownership
settings-teams-toast-updated-role = Successfully updated team member role
settings-teams-toast-left-team = Successfully left team
settings-teams-toast-renamed-team = Successfully renamed team
settings-teams-error-leave-team = Error leaving team
settings-teams-error-rename-team = Failed to rename team
settings-teams-error-send-invite = Failed to send invite
settings-teams-error-toggle-invite-links = Failed to toggle invite links
settings-teams-error-reset-invite-links = Failed to reset invite links
settings-teams-error-delete-invite = Failed to delete invite
settings-teams-error-add-domain = Failed to add domain restriction
settings-teams-error-delete-domain = Failed to delete domain restriction
settings-teams-error-upgrade-link = Failed to generate upgrade link. Please contact us at feedback@warp.dev
settings-teams-error-billing-link = Failed to generate billing link. Please contact us at feedback@warp.dev
settings-teams-error-toggle-discoverability = Failed to toggle team discoverability
settings-teams-error-join-team = Failed to join team
settings-teams-error-transfer-ownership = Failed to transfer team ownership
settings-teams-error-update-role = Failed to update team member role

# --- ANCHOR-SUB-SETTINGS-PAGE-NAV (agent-settings-page-nav) ---
# 此锚点下放 settings_view/{settings_page,nav,delete_environment_confirmation_dialog,directory_color_add_picker,pane_manager}.rs 字符串
# 命名前缀:settings-page-* / settings-nav-* / settings-confirm-* / settings-color-picker-*

# ---- settings_page.rs ----
settings-page-info-icon-tooltip = Click to learn more in docs
settings-page-local-only-icon-tooltip = This setting is not synced to your other devices
settings-page-reset-to-default = Reset to default

# ---- delete_environment_confirmation_dialog.rs ----
settings-confirm-cancel = Cancel
settings-confirm-delete-environment-button = Delete environment
settings-confirm-delete-environment-title = Delete environment?
settings-confirm-delete-environment-description = Are you sure you want to remove the { $name } environment?

# ---- directory_color_add_picker.rs ----
settings-color-picker-add-directory-footer = + Add directory…
settings-color-picker-add-directory-color = Add directory color

# ---- settings_file_footer.rs ----
settings-footer-open-file = Open settings file
settings-footer-alert-open-file = Open file
settings-footer-alert-fix-with-oz = Fix with Oz

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
settings-exec-profile-title-model = Title generation:
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
settings-agent-providers-description = Configure custom Agent providers across multiple protocols — OpenAI-compatible (DeepSeek, Zhipu GLM, Moonshot, DashScope, SiliconFlow, OpenRouter, etc.), Anthropic, Gemini, and local Ollama. You can add models manually (display name + model ID mapping) or fetch them automatically from the API. Provider metadata is stored in the local settings.toml; API keys are stored securely in the system keychain.
settings-agent-providers-empty = No providers configured yet. Click [+ Add provider] in the top-right to add one.
settings-agent-providers-add-button = + Add provider
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

# --- ANCHOR-SUB-RULES-PAGE (agent-rules-page) ---
# Manage Rules 页面(Warp Drive 中的 AI Fact Collection)。
rules-collection-name = Rules

# --- ANCHOR-SUB-KEYBINDING-DESC (agent-keybinding-descriptions) ---
# Description 文案 for keyboard binding entries shown in the Settings >
# Keyboard Shortcuts page and the command palette. Each key corresponds to
# a binding registered via `EditableBinding::new(name, description, action)`
# or `BindingDescription::new("…")`. The binding `name` (e.g.
# `workspace:open_settings_file`) is **not** translated — it is a protocol
# field used to persist user-customised shortcuts.

# Tabs / sessions
keybinding-desc-workspace-cycle-next-session = Switch to next tab
keybinding-desc-workspace-cycle-prev-session = Switch to previous tab
keybinding-desc-workspace-add-window = Create New Window
keybinding-desc-workspace-new-file = New File
keybinding-desc-workspace-zoom-in = Zoom In
keybinding-desc-workspace-zoom-out = Zoom Out
keybinding-desc-workspace-reset-zoom = Reset Zoom
keybinding-desc-workspace-increase-font-size = Increase font size
keybinding-desc-workspace-decrease-font-size = Decrease font size
keybinding-desc-workspace-reset-font-size = Reset font size to default
keybinding-desc-workspace-increase-zoom = Increase zoom level
keybinding-desc-workspace-decrease-zoom = Decrease zoom level
keybinding-desc-workspace-reset-zoom-level = Reset zoom level to default
keybinding-desc-workspace-save-launch-config = Save new launch configuration

# Project Explorer / panels
keybinding-desc-workspace-toggle-project-explorer = Toggle project explorer
keybinding-desc-workspace-toggle-project-explorer-menu = Project Explorer
keybinding-desc-workspace-show-theme-chooser = Open theme picker
keybinding-desc-workspace-toggle-tab-configs-menu = Open tab configs menu

# Switch to N-th tab
keybinding-desc-workspace-activate-1st-tab = Switch to 1st tab
keybinding-desc-workspace-activate-2nd-tab = Switch to 2nd tab
keybinding-desc-workspace-activate-3rd-tab = Switch to 3rd tab
keybinding-desc-workspace-activate-4th-tab = Switch to 4th tab
keybinding-desc-workspace-activate-5th-tab = Switch to 5th tab
keybinding-desc-workspace-activate-6th-tab = Switch to 6th tab
keybinding-desc-workspace-activate-7th-tab = Switch to 7th tab
keybinding-desc-workspace-activate-8th-tab = Switch to 8th tab
keybinding-desc-workspace-activate-last-tab = Switch to last tab
keybinding-desc-workspace-activate-prev-tab = Activate previous tab
keybinding-desc-workspace-activate-next-tab = Activate next tab

# Pane navigation
keybinding-desc-pane-group-navigate-prev = Activate previous pane
keybinding-desc-pane-group-navigate-next = Activate next pane

# Mouse / Notebooks / Workflows / Folders
keybinding-desc-workspace-toggle-mouse-reporting = Toggle Mouse Reporting
keybinding-desc-workspace-create-team-notebook = Create a new team notebook
keybinding-desc-workspace-create-team-notebook-menu = New Team Notebook
keybinding-desc-workspace-create-personal-notebook = Create a new personal notebook
keybinding-desc-workspace-create-personal-notebook-menu = New Personal Notebook
keybinding-desc-workspace-create-team-workflow = Create a new team workflow
keybinding-desc-workspace-create-team-workflow-menu = New Team Workflow
keybinding-desc-workspace-create-personal-workflow = Create a new personal workflow
keybinding-desc-workspace-create-personal-workflow-menu = New Personal Workflow
keybinding-desc-workspace-create-team-folder = Create a new team folder
keybinding-desc-workspace-create-team-folder-menu = New Team Folder
keybinding-desc-workspace-create-personal-folder = Create a new personal folder
keybinding-desc-workspace-create-personal-folder-menu = New Personal Folder

# New tab variants
keybinding-desc-workspace-new-tab = Create new tab
keybinding-desc-workspace-new-terminal-tab = New Terminal Tab
keybinding-desc-workspace-new-agent-tab = New Agent Tab
keybinding-desc-workspace-new-cloud-agent-tab = New Cloud Agent Tab

# Left / right panel toggles
keybinding-desc-workspace-toggle-left-panel = Open Left Panel
keybinding-desc-workspace-toggle-right-panel = Toggle code review
keybinding-desc-workspace-toggle-right-panel-menu = Toggle Code Review
keybinding-desc-workspace-toggle-vertical-tabs = Toggle vertical tabs panel
keybinding-desc-workspace-toggle-vertical-tabs-menu = Toggle Vertical Tabs Panel
keybinding-desc-workspace-left-panel-agent-conversations = Left Panel: Agent conversations
keybinding-desc-workspace-left-panel-project-explorer = Left Panel: Project explorer
keybinding-desc-workspace-left-panel-global-search = Left Panel: Global search
keybinding-desc-workspace-left-panel-warp-drive = Left Panel: Warp Drive
keybinding-desc-workspace-open-global-search = Open global search
keybinding-desc-workspace-open-global-search-menu = Global Search
keybinding-desc-workspace-toggle-warp-drive = Toggle Warp Drive
keybinding-desc-workspace-toggle-warp-drive-menu = Warp Drive
keybinding-desc-workspace-toggle-conversation-list-view = Toggle Agent conversation list view
keybinding-desc-workspace-toggle-conversation-list-view-menu = Agent conversation list view
keybinding-desc-workspace-close-panel = Close focused panel

# Command palette / navigation
keybinding-desc-workspace-toggle-command-palette = Toggle command palette
keybinding-desc-workspace-toggle-command-palette-menu = Command Palette
keybinding-desc-workspace-toggle-navigation-palette = Toggle navigation palette
keybinding-desc-workspace-toggle-navigation-palette-menu = Navigation Palette
keybinding-desc-workspace-toggle-launch-config-palette = Launch configuration palette
keybinding-desc-workspace-toggle-files-palette = Toggle Files Palette
keybinding-desc-workspace-search-drive = Search Warp Drive
keybinding-desc-workspace-move-tab-left = Move tab left
keybinding-desc-workspace-move-tab-up = move tab up
keybinding-desc-workspace-move-tab-right = Move tab right
keybinding-desc-workspace-move-tab-down = move tab down

# Keybindings settings
keybinding-desc-workspace-toggle-keybindings-page = Toggle keyboard shortcuts
keybinding-desc-workspace-show-keybinding-settings = Open keybindings editor
keybinding-desc-workspace-toggle-block-snackbar = Toggle sticky command header

# Window / tab close
keybinding-desc-workspace-rename-active-tab = Rename the current tab
keybinding-desc-workspace-terminate-app = Quit Warp
keybinding-desc-workspace-close-window = Close Window
keybinding-desc-workspace-close-active-tab = Close the current tab
keybinding-desc-workspace-close-other-tabs = Close other tabs
keybinding-desc-workspace-close-tabs-right = Close tabs to the right
keybinding-desc-workspace-close-tabs-below = close tabs below

# Notifications
keybinding-desc-workspace-toggle-notifications-on = Turn notifications on
keybinding-desc-workspace-toggle-notifications-off = Turn notifications off

# Updates / changelog
keybinding-desc-workspace-update-and-relaunch = Install update and relaunch
keybinding-desc-workspace-check-for-updates = Check for updates
keybinding-desc-workspace-view-changelog = View latest changelog

# Resource center / Drive export / CLI
keybinding-desc-workspace-toggle-resource-center = Toggle resource center
keybinding-desc-workspace-export-all-warp-drive-objects = Export all Warp Drive objects
keybinding-desc-workspace-install-cli = Install Oz CLI command
keybinding-desc-workspace-uninstall-cli = Uninstall Oz CLI command

# AI assistant / agents
keybinding-desc-workspace-toggle-ai-assistant = Toggle Warp AI

# Env vars / prompts
keybinding-desc-workspace-create-team-env-vars = Create new team environment variables
keybinding-desc-workspace-create-team-env-vars-menu = New Team Environment Variables
keybinding-desc-workspace-create-personal-env-vars = Create new personal environment variables
keybinding-desc-workspace-create-personal-env-vars-menu = New Personal Environment Variables
keybinding-desc-workspace-create-personal-ai-prompt = Create a new personal prompt
keybinding-desc-workspace-create-personal-ai-prompt-menu = New Personal Prompt
keybinding-desc-workspace-create-team-ai-prompt = Create a new team prompt
keybinding-desc-workspace-create-team-ai-prompt-menu = New Team Prompt

# Focus / import
keybinding-desc-workspace-shift-focus-left = Switch Focus to Left Panel
keybinding-desc-workspace-shift-focus-right = Switch Focus to Right Panel
keybinding-desc-workspace-import-to-personal-drive = Import To Personal Drive
keybinding-desc-workspace-import-to-team-drive = Import To Team Drive

# Drive / repository / AI rules / MCP
keybinding-desc-workspace-open-repository = Open repository
keybinding-desc-workspace-open-repository-menu = Open Repository
keybinding-desc-workspace-open-ai-fact-collection = Open AI Rules
keybinding-desc-workspace-open-mcp-servers = Open MCP Servers
keybinding-desc-workspace-jump-to-latest-toast = Jump to latest agent task
keybinding-desc-workspace-toggle-notification-mailbox = Toggle notification mailbox
keybinding-desc-workspace-toggle-agent-management-view = Toggle the agent management view

# Settings pages
keybinding-desc-workspace-show-settings = Open Settings
keybinding-desc-workspace-show-settings-menu = Settings
keybinding-desc-workspace-show-settings-account = Open Settings: Account
keybinding-desc-workspace-show-settings-appearance = Open Settings: Appearance
keybinding-desc-workspace-show-settings-appearance-menu = Appearance...
keybinding-desc-workspace-show-settings-features = Open Settings: Features
keybinding-desc-workspace-show-settings-shared-blocks = Open Settings: Shared Blocks
keybinding-desc-workspace-show-settings-shared-blocks-menu = View Shared Blocks...
keybinding-desc-workspace-show-settings-keyboard-shortcuts = Open Settings: Keyboard Shortcuts
keybinding-desc-workspace-show-settings-keyboard-shortcuts-menu = Configure Keyboard Shortcuts...
keybinding-desc-workspace-show-settings-about = Open Settings: About
keybinding-desc-workspace-show-settings-about-menu = About Warp
keybinding-desc-workspace-show-settings-teams = Open Settings: Teams
keybinding-desc-workspace-show-settings-teams-menu = Open Team Settings
keybinding-desc-workspace-show-settings-privacy = Open Settings: Privacy
keybinding-desc-workspace-show-settings-warpify = Open Settings: Warpify
keybinding-desc-workspace-show-settings-warpify-menu = Configure Warpify...
keybinding-desc-workspace-show-settings-ai = Open Settings: AI
keybinding-desc-workspace-show-settings-code = Open Settings: Code
keybinding-desc-workspace-show-settings-referrals = Open Settings: Referrals
keybinding-desc-workspace-show-settings-environments = Open Settings: Environments
keybinding-desc-workspace-show-settings-mcp-servers = Open Settings: MCP Servers
keybinding-desc-workspace-open-settings-file = Open settings file

# Overflow menu / external links
keybinding-desc-workspace-link-to-slack = Join our Slack community (opens external link)
keybinding-desc-workspace-link-to-user-docs = View user docs (opens external link)
keybinding-desc-workspace-send-feedback = Send feedback (opens external link)
keybinding-desc-workspace-send-feedback-oz = Send feedback with Oz
keybinding-desc-workspace-view-logs = View Warp logs
keybinding-desc-workspace-link-to-privacy-policy = View privacy policy (opens external link)

# Input / terminal / project bindings (registered outside workspace/mod.rs)
keybinding-desc-input-edit-prompt = Edit Prompt
keybinding-desc-terminal-attach-block-as-context = Attach Selected Block as Agent Context
keybinding-desc-terminal-attach-text-as-context = Attach Selected Text as Agent Context
keybinding-desc-terminal-attach-as-context-menu = Attach Selection as Agent Context
keybinding-desc-workspace-write-codebase-index = Write current codebase index snapshot
keybinding-desc-workspace-init-project = Initiate project for warp
keybinding-desc-workspace-add-current-folder = Add current folder as project

# Workspace debug / crash / sentry / heap profile bindings
keybinding-desc-workspace-crash-macos = Crash the app (for testing sentry-cocoa)
keybinding-desc-workspace-crash-other = Crash the app (for testing sentry-native)
keybinding-desc-workspace-log-review-comment-send-status = [Debug] Log review comment send status for active tab
keybinding-desc-workspace-panic = Trigger a panic (for testing sentry-rust)
keybinding-desc-workspace-open-view-tree-debugger = Open view tree debugger
keybinding-desc-workspace-view-first-time-user-experience = [Debug] View first-time user experience
keybinding-desc-workspace-open-build-plan-migration-modal = [Debug] Open Build Plan Migration Modal
keybinding-desc-workspace-reset-build-plan-migration-modal-state = [Debug] Reset Build Plan Migration Modal State
keybinding-desc-workspace-undismiss-aws-login-banner = [Debug] Un-dismiss AWS login banner
keybinding-desc-workspace-open-oz-launch-modal = [Debug] Open Oz Launch Modal
keybinding-desc-workspace-reset-oz-launch-modal-state = [Debug] Reset Oz Launch Modal State
keybinding-desc-workspace-open-openwarp-launch-modal = [Debug] Open OpenWarp Launch Modal
keybinding-desc-workspace-reset-openwarp-launch-modal-state = [Debug] Reset OpenWarp Launch Modal State
keybinding-desc-workspace-install-opencode-warp-plugin = [Debug] Install OpenCode Warp plugin
keybinding-desc-workspace-use-local-opencode-warp-plugin = [Debug] Use local OpenCode Warp plugin (testing only)
keybinding-desc-workspace-open-session-config-modal = [Debug] Open Session Config Modal
keybinding-desc-workspace-start-hoa-onboarding-flow = [Debug] Start HOA Onboarding Flow
keybinding-desc-workspace-sample-process = Sample Process
keybinding-desc-workspace-dump-heap-profile = Dump heap profile (can only be done once)

# Terminal input bindings
keybinding-desc-input-show-network-log = Show Warp network log
keybinding-desc-input-clear-screen = Clear screen
keybinding-desc-input-toggle-classic-completions = (Experimental) Toggle classic completions mode
keybinding-desc-input-command-search = Command Search
keybinding-desc-input-history-search = History Search
keybinding-desc-input-open-completions-menu = Open completions menu
keybinding-desc-input-workflows = Workflows
keybinding-desc-input-open-ai-command-suggestions = Open AI Command Suggestions
keybinding-desc-input-new-agent-conversation = New agent conversation
keybinding-desc-input-trigger-auto-detection = Trigger Auto Detection
keybinding-desc-input-clear-and-reset-ai-context-menu-query = Clear and reset AI context menu query

# Terminal view bindings
keybinding-desc-terminal-alternate-paste = Alternate terminal paste
keybinding-desc-terminal-toggle-cli-agent-rich-input = Toggle CLI Agent Rich Input
keybinding-desc-terminal-warpify-subshell = Warpify subshell
keybinding-desc-terminal-warpify-ssh-session = Warpify ssh session
keybinding-desc-terminal-accept-prompt-suggestion = Accept Prompt Suggestion
keybinding-desc-terminal-cancel-process-windows = Copy text or cancel active process
keybinding-desc-terminal-cancel-process = Cancel active process
keybinding-desc-terminal-focus-input = Focus terminal input
keybinding-desc-terminal-paste = Paste
keybinding-desc-terminal-copy = Copy
keybinding-desc-terminal-reinput-commands = Reinput selected commands
keybinding-desc-terminal-reinput-commands-sudo = Reinput selected commands as root
keybinding-desc-terminal-find = Find in Terminal
keybinding-desc-terminal-select-bookmark-up = Select the closest bookmark up
keybinding-desc-terminal-select-bookmark-down = Select the closest bookmark down
keybinding-desc-terminal-open-block-context-menu = Open block context menu
keybinding-desc-terminal-toggle-team-workflows-modal = Toggle team workflows modal
keybinding-desc-terminal-copy-git-branch = Copy git branch
keybinding-desc-terminal-clear-blocks = Clear Blocks
keybinding-desc-terminal-cursor-word-left = Move cursor one word to the left within an executing command
keybinding-desc-terminal-cursor-word-right = Move cursor one word to the right within an executing command
keybinding-desc-terminal-cursor-home = Move cursor home within an executing command
keybinding-desc-terminal-cursor-end = Move cursor end within an executing command
keybinding-desc-terminal-delete-word-left = Delete word left within an executing command
keybinding-desc-terminal-delete-line-start = Delete to line start within an executing command
keybinding-desc-terminal-delete-line-end = Delete to line end within an executing command
keybinding-desc-terminal-backward-tabulation = Backward tabulation within an executing command
keybinding-desc-terminal-select-previous-block = Select previous block
keybinding-desc-terminal-select-next-block = Select next block
keybinding-desc-terminal-share-selected-block = Share selected block
keybinding-desc-terminal-bookmark-selected-block = Bookmark selected block
keybinding-desc-terminal-find-within-selected-block = Find within selected block
keybinding-desc-terminal-copy-command-and-output = Copy command and output
keybinding-desc-terminal-copy-command-output = Copy command output
keybinding-desc-terminal-copy-command = Copy command
keybinding-desc-terminal-scroll-up-one-line = Scroll terminal output up one line
keybinding-desc-terminal-scroll-down-one-line = Scroll terminal output down one line
keybinding-desc-terminal-scroll-to-top-of-block = Scroll to top of selected block
keybinding-desc-terminal-scroll-to-bottom-of-block = Scroll to bottom of selected block
keybinding-desc-terminal-select-all-blocks = Select all blocks
keybinding-desc-terminal-expand-blocks-above = Expand selected blocks above
keybinding-desc-terminal-expand-blocks-below = Expand selected blocks below
keybinding-desc-terminal-insert-command-correction = Insert Command Correction
keybinding-desc-terminal-setup-guide = Setup Guide
keybinding-desc-terminal-onboarding-warp-input-terminal = [Debug] Onboarding Callout: WarpInput - Terminal
keybinding-desc-terminal-onboarding-warp-input-project = [Debug] Onboarding Callout: WarpInput - Project
keybinding-desc-terminal-onboarding-warp-input-no-project = [Debug] Onboarding Callout: WarpInput - No Project
keybinding-desc-terminal-onboarding-modality-project = [Debug] Onboarding Callout: Modality - Project
keybinding-desc-terminal-onboarding-modality-no-project = [Debug] Onboarding Callout: Modality - No Project
keybinding-desc-terminal-onboarding-modality-terminal = [Debug] Onboarding Callout: Modality - Terminal
keybinding-desc-terminal-import-external-settings = Import External Settings
keybinding-desc-terminal-share-current-session = Share current session
keybinding-desc-terminal-stop-sharing-current-session = Stop sharing current session
keybinding-desc-terminal-toggle-block-filter = Toggle block filter on selected or last block
keybinding-desc-terminal-toggle-sticky-command-header = Toggle Sticky Command Header in Active Pane
keybinding-desc-terminal-toggle-autoexecute-mode = Toggle Auto-execute Mode
keybinding-desc-terminal-toggle-queue-next-prompt = Toggle Queue Next Prompt
keybinding-desc-terminal-generate-codebase-index = [Debug] Generate codebase index

# Pane group bindings
keybinding-desc-pane-group-close-current-session = Close Current Session
keybinding-desc-pane-group-split-left = Split pane left
keybinding-desc-pane-group-split-up = Split pane up
keybinding-desc-pane-group-split-down = Split pane down
keybinding-desc-pane-group-split-right = Split pane right
keybinding-desc-pane-group-switch-left = Switch panes left
keybinding-desc-pane-group-switch-right = Switch panes right
keybinding-desc-pane-group-switch-up = Switch panes up
keybinding-desc-pane-group-switch-down = Switch panes down
keybinding-desc-pane-group-resize-left = Resize pane > Move divider left
keybinding-desc-pane-group-resize-right = Resize pane > Move divider right
keybinding-desc-pane-group-resize-up = Resize pane > Move divider up
keybinding-desc-pane-group-resize-down = Resize pane > Move divider down
keybinding-desc-pane-group-toggle-maximize = Toggle Maximize Active Pane

# Root view bindings
keybinding-desc-root-view-toggle-fullscreen = Toggle fullscreen
keybinding-desc-root-view-enter-onboarding-state = [Debug] Enter Onboarding State

# Workflow view bindings
keybinding-desc-workflow-view-save = Save workflow
keybinding-desc-workflow-view-close = Close

# Editor view binding desc (shared by editor/view/mod.rs, code/editor/view/actions.rs, notebooks/editor/view.rs)
keybinding-desc-editor-copy = Copy
keybinding-desc-editor-cut = Cut
keybinding-desc-editor-paste = Paste
keybinding-desc-editor-undo = Undo
keybinding-desc-editor-redo = Redo
keybinding-desc-editor-select-left-by-word = Select one word to the left
keybinding-desc-editor-select-right-by-word = Select one word to the right
keybinding-desc-editor-select-left = Select one character to the left
keybinding-desc-editor-select-right = Select one character to the right
keybinding-desc-editor-select-up = Select up
keybinding-desc-editor-select-down = Select down
keybinding-desc-editor-select-all = Select all
keybinding-desc-editor-select-to-line-start = Select to start of line
keybinding-desc-editor-select-to-line-end = Select to end of line
keybinding-desc-editor-select-to-line-start-cap = Select To Line Start
keybinding-desc-editor-select-to-line-end-cap = Select To Line End
keybinding-desc-editor-clear-and-copy-lines = Copy and clear selected lines
keybinding-desc-editor-add-next-occurrence = Add selection for next occurrence
keybinding-desc-editor-up = Move cursor up
keybinding-desc-editor-down = Move cursor down
keybinding-desc-editor-left = Move cursor left
keybinding-desc-editor-right = Move cursor right
keybinding-desc-editor-move-to-line-start = Move to start of line
keybinding-desc-editor-move-to-line-end = Move to end of line
keybinding-desc-editor-move-to-line-start-short = Move to line start
keybinding-desc-editor-move-to-line-end-short = Move to line end
keybinding-desc-editor-home = Home
keybinding-desc-editor-end = End
keybinding-desc-editor-cmd-down = Move cursor to the bottom
keybinding-desc-editor-cmd-up = Move cursor to the top
keybinding-desc-editor-move-to-and-select-buffer-start = Select and move to the top
keybinding-desc-editor-move-to-and-select-buffer-end = Select and move to the bottom
keybinding-desc-editor-move-forward-one-word = Move forward one word
keybinding-desc-editor-move-backward-one-word = Move backward one word
keybinding-desc-editor-move-forward-one-word-cap = Move Forward One Word
keybinding-desc-editor-move-backward-one-word-cap = Move Backward One Word
keybinding-desc-editor-move-to-paragraph-start = Move to the start of the paragraph
keybinding-desc-editor-move-to-paragraph-end = Move to the end of the paragraph
keybinding-desc-editor-move-to-paragraph-start-short = Move to start of paragraph
keybinding-desc-editor-move-to-paragraph-end-short = Move to end of paragraph
keybinding-desc-editor-move-to-buffer-start = Move to the start of the buffer
keybinding-desc-editor-move-to-buffer-end = Move to the end of the buffer
keybinding-desc-editor-cursor-at-buffer-start = Cursor at buffer start
keybinding-desc-editor-cursor-at-buffer-end = Cursor at buffer end
keybinding-desc-editor-backspace = Remove the previous character
keybinding-desc-editor-cut-word-left = Cut word left
keybinding-desc-editor-cut-word-right = Cut word right
keybinding-desc-editor-delete-word-left = Delete word left
keybinding-desc-editor-delete-word-right = Delete word right
keybinding-desc-editor-cut-all-left = Cut all left
keybinding-desc-editor-cut-all-right = Cut all right
keybinding-desc-editor-delete-all-left = Delete all left
keybinding-desc-editor-delete-all-right = Delete all right
keybinding-desc-editor-delete = Delete
keybinding-desc-editor-clear-lines = Clear selected lines
keybinding-desc-editor-insert-newline = Insert newline
keybinding-desc-editor-fold = Fold
keybinding-desc-editor-unfold = Unfold
keybinding-desc-editor-fold-selected-ranges = Fold selected ranges
keybinding-desc-editor-insert-last-word-prev-cmd = Insert last word of previous command
keybinding-desc-editor-move-backward-one-subword = Move Backward One Subword
keybinding-desc-editor-move-forward-one-subword = Move Forward One Subword
keybinding-desc-editor-select-left-by-subword = Select one subword to the left
keybinding-desc-editor-select-right-by-subword = Select one subword to the right
keybinding-desc-editor-accept-autosuggestion = Accept autosuggestion
keybinding-desc-editor-inspect-command = Inspect Command
keybinding-desc-editor-clear-buffer = Clear command editor
keybinding-desc-editor-add-cursor-above = Add cursor above
keybinding-desc-editor-add-cursor-below = Add cursor below
keybinding-desc-editor-insert-nonexpanding-space = Insert non-expanding space
keybinding-desc-editor-vim-exit-insert-mode = Exit Vim insert mode
keybinding-desc-editor-toggle-comment = Toggle comment
keybinding-desc-editor-go-to-line = Go to line
keybinding-desc-editor-find-in-code-editor = Find in code editor

# Code editor (Code) binding desc
keybinding-desc-code-save-as = Save file as
keybinding-desc-code-close-all-tabs = Close all tabs
keybinding-desc-code-close-saved-tabs = Close saved tabs

# Welcome view binding desc
keybinding-desc-welcome-terminal-session = Terminal session
keybinding-desc-welcome-add-repository = Add repository

# AI assistant panel binding desc
keybinding-desc-ai-assistant-close = Close Warp AI
keybinding-desc-ai-assistant-focus-terminal-input = Focus Terminal Input From Warp AI
keybinding-desc-ai-assistant-restart = Restart Warp AI

# Code review binding desc
keybinding-desc-code-review-save-all = Save all unsaved files in code review
keybinding-desc-code-review-show-find = Show find bar in code review

# Project buttons binding desc
keybinding-desc-project-buttons-open-repository = Open repository
keybinding-desc-project-buttons-create-new-project = Create new project

# Find view binding desc
keybinding-desc-find-next-occurrence = Find the next occurrence of your search query
keybinding-desc-find-prev-occurrence = Find the previous occurrence of your search query

# Notebook file / notebook binding desc
keybinding-desc-notebook-focus-terminal-input-from-file = Focus Terminal Input from File
keybinding-desc-notebook-reload-file = Reload file
keybinding-desc-notebook-increase-font-size = Increase notebook font size
keybinding-desc-notebook-decrease-font-size = Decrease notebook font size
keybinding-desc-notebook-reset-font-size = Reset notebook font size
keybinding-desc-notebook-focus-terminal-input = Focus Terminal Input from Notebook
keybinding-desc-notebook-fb-increase-font-size = Increase font size
keybinding-desc-notebook-fb-decrease-font-size = Decrease font size

# Notebook editor binding desc (extra to shared editor keys)
keybinding-desc-nbeditor-deselect-command = De-select shell commands
keybinding-desc-nbeditor-select-command = Select shell command at cursor
keybinding-desc-nbeditor-select-previous-command = Select previous command
keybinding-desc-nbeditor-select-next-command = Select next command
keybinding-desc-nbeditor-run-commands = Run selected commands
keybinding-desc-nbeditor-toggle-debug = Toggle rich-text debug mode
keybinding-desc-nbeditor-debug-copy-buffer = Copy rich-text buffer
keybinding-desc-nbeditor-debug-copy-selection = Copy rich-text selection
keybinding-desc-nbeditor-log-state = Log editor state
keybinding-desc-nbeditor-edit-link = Create or edit link
keybinding-desc-nbeditor-inline-code = Toggle inline code styling
keybinding-desc-nbeditor-strikethrough = Toggle strikethrough styling
keybinding-desc-nbeditor-underline = Toggle underline styling
keybinding-desc-nbeditor-find = Find in Notebook
keybinding-desc-nbeditor-next-find-match = Focus next match
keybinding-desc-nbeditor-previous-find-match = Focus previous match
keybinding-desc-nbeditor-toggle-regex-find = Toggle regular expression search
keybinding-desc-nbeditor-toggle-case-sensitive-find = Toggle case-sensitive search

# Pane group / undo close binding desc
keybinding-desc-get-started-terminal-session = Terminal session
keybinding-desc-undo-close-reopen-session = Reopen closed session
keybinding-desc-pane-share-pane = Share pane
keybinding-desc-right-panel-toggle-maximize-code-review = Toggle Maximize Code Review Panel

# Workspace sync inputs binding desc
keybinding-desc-workspace-disable-sync-inputs = Stop Synchronizing Any Panes
keybinding-desc-workspace-toggle-sync-inputs-tab = Toggle Synchronizing All Panes in Current Tab
keybinding-desc-workspace-toggle-sync-inputs-all-tabs = Toggle Synchronizing All Panes in All Tabs

# Workspace a11y / debug binding desc
keybinding-desc-workspace-a11y-concise = [a11y] Set concise accessibility announcements
keybinding-desc-workspace-a11y-verbose = [a11y] Set verbose accessibility announcements
keybinding-desc-workspace-copy-access-token = Copy access token to clipboard

# Env var collection binding desc
keybinding-desc-env-var-collection-close = Close

# Auth / share modal binding desc
keybinding-desc-share-block-copy = Copy
keybinding-desc-auth-paste-token = Paste
keybinding-desc-conversation-details-copy = Copy

# Terminal extras binding desc
keybinding-desc-terminal-show-history = Show History
keybinding-desc-terminal-ask-ai-selection = Ask Warp AI about Selection
keybinding-desc-terminal-ask-ai-last-block = Ask Warp AI about last block
keybinding-desc-terminal-ask-ai = Ask Warp AI
keybinding-desc-terminal-load-agent-conversation = Load agent mode conversation (from debug link in clipboard)
keybinding-desc-terminal-toggle-session-recording = Toggle PTY Recording for Session

# Notebook editor extra
keybinding-desc-nbeditor-select-to-paragraph-start = Select to start of paragraph
keybinding-desc-nbeditor-select-to-paragraph-end = Select to end of paragraph
