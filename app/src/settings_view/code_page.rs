#[cfg(feature = "local_fs")]
use super::features::external_editor::ExternalEditorView;
use super::{
    settings_page::{
        build_sub_header, render_body_item, render_separator, Category, MatchData, PageType,
        SettingsPageMeta, SettingsPageViewHandle, SettingsWidget, HEADER_PADDING,
    },
    LocalOnlyIconState, SettingsAction, SettingsSection, ToggleState,
};
use crate::{
    ai::persisted_workspace::{
        EnablementState, LspRepoStatus, PersistedWorkspace, PersistedWorkspaceEvent,
    },
    appearance::Appearance,
    code::lsp_telemetry::{LspControlActionType, LspEnablementSource, LspTelemetryEvent},
    send_telemetry_from_ctx,
    settings::CodeSettings,
    terminal::general_settings::GeneralSettings,
    ui_components::{
        avatar::{Avatar, AvatarContent, StatusElementTypes},
        buttons::icon_button,
        icons::Icon,
    },
    workspace::tab_settings::TabSettings,
    workspaces::update_manager::TeamUpdateManager,
    TelemetryEvent,
};

use ai::project_context::model::{ProjectContextModel, ProjectContextModelEvent};

use lsp::supported_servers::LSPServerType;
use lsp::{LspManagerModel, LspManagerModelEvent, LspServerModel, LspState};
use pathfinder_color::ColorU;
use settings::Setting as _;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use warp_core::{
    features::FeatureFlag, report_if_error, settings::ToggleableSetting as _,
    ui::theme::AnsiColorIdentifier,
};
use warp_util::path::user_friendly_path;
use warpui::{
    elements::{
        ChildView, Container, CornerRadius, CrossAxisAlignment, Element, Empty, Expanded, Fill,
        Flex, MainAxisAlignment, MainAxisSize, MouseStateHandle, ParentElement, Radius, Shrinkable,
    },
    fonts::Weight,
    keymap::ContextPredicate,
    platform::Cursor,
    ui_components::{
        button::ButtonVariant,
        components::{Coords, UiComponent, UiComponentStyles},
        switch::SwitchStateHandle,
    },
    Action, AppContext, Entity, ModelHandle, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle,
};

const MAIN_SECTION_MARGIN: f32 = 12.;
const SUB_SECTION_MARGIN: f32 = 8.;

const LSP_STATUS_INDICATOR_SIZE: f32 = 8.;

/// Identifies which subpage of the Code settings the user is viewing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeSubpage {
    /// Codebase indexing and initialization settings.
    Indexing,
    /// External editor, code review panel, and project explorer settings.
    EditorAndCodeReview,
}

impl CodeSubpage {
    pub fn from_section(section: SettingsSection) -> Option<Self> {
        match section {
            SettingsSection::CodeIndexing => Some(Self::Indexing),
            SettingsSection::EditorAndCodeReview => Some(Self::EditorAndCodeReview),
            _ => None,
        }
    }

    pub fn title(&self) -> String {
        match self {
            Self::Indexing => crate::t!("settings-code-subpage-indexing-title"),
            Self::EditorAndCodeReview => crate::t!("settings-code-subpage-editor-review-title"),
        }
    }
}

#[derive(Clone, Default)]
struct LspServerRowMouseStates {
    restart: MouseStateHandle,
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    view_logs: MouseStateHandle,
    install: MouseStateHandle,
    uninstall: MouseStateHandle,
}

#[derive(Clone)]
struct InitializedFoldersMouseStates {
    lsp_rows: Vec<LspServerRowMouseStates>,
    open_project_rules: Vec<MouseStateHandle>,
}

pub struct CodeSettingsPageView {
    page: PageType<Self>,
    active_subpage: Option<CodeSubpage>,
    /// Mouse states for LSP server row buttons.
    /// This is kept separate from the codebase mouse states because each workspace/folder
    /// can have 0 to multiple LSP servers, so the count doesn't match 1:1 with workspaces.
    /// The states are flattened into a single Vec, indexed by iterating through workspaces
    /// and their enabled servers in order.
    lsp_row_mouse_states: Vec<LspServerRowMouseStates>,
    open_project_rules_mouse_states: Vec<MouseStateHandle>,
    /// Tracks installation status for suggested LSP servers so the UI can decide
    /// whether to show "Available for download" vs "Installed" and whether the
    /// "+" button should trigger install or just enable.
    suggested_server_statuses: HashMap<(PathBuf, LSPServerType), LspRepoStatus>,
    #[cfg(feature = "local_fs")]
    external_editor_view: Option<ViewHandle<ExternalEditorView>>,
}

impl CodeSettingsPageView {
    fn set_global_lsp_server_enabled(
        server_type: LSPServerType,
        enabled: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        CodeSettings::handle(ctx).update(ctx, |settings, ctx| {
            let mut enabled_servers = settings.enabled_lsp_servers.value().clone();
            if enabled {
                if !enabled_servers.contains(&server_type) {
                    enabled_servers.push(server_type);
                }
            } else {
                enabled_servers.retain(|s| *s != server_type);
            }
            report_if_error!(settings.enabled_lsp_servers.set_value(enabled_servers, ctx));
        });
    }

    pub fn new(ctx: &mut ViewContext<CodeSettingsPageView>) -> Self {
        // LSP enablement is global in openWarp: render one row per supported server type.
        let lsp_server_count = LSPServerType::all().count();

        // Subscribe to LSP manager events for real-time status updates
        ctx.subscribe_to_model(
            &LspManagerModel::handle(ctx),
            |me, _, event, ctx| match event {
                LspManagerModelEvent::ServerStarted(_)
                | LspManagerModelEvent::ServerStopped(_)
                | LspManagerModelEvent::ServerRemoved { .. } => {
                    // Recalculate LSP server count and resize mouse states if needed
                    let new_count = LSPServerType::all().count();
                    if me.lsp_row_mouse_states.len() != new_count {
                        me.lsp_row_mouse_states
                            .resize_with(new_count, Default::default);
                    }

                    me.resize_workspace_mouse_states(ctx);

                    ctx.notify();
                }
            },
        );

        // Subscribe to PersistedWorkspaceEvent to handle suggested server detection
        // and installation status updates. We don't scan for suggested servers
        // here — PersistedWorkspace::new() already kicks off detection at startup
        // and emits AvailableServersDetected for each workspace, which the
        // subscription below handles.
        let persisted = PersistedWorkspace::handle(ctx);

        ctx.subscribe_to_model(&persisted, move |me, _model, event, ctx| match event {
            PersistedWorkspaceEvent::AvailableServersDetected {
                workspace_path,
                servers,
            } => {
                // New suggested servers detected — kick off install detection
                // and resize mouse states.
                for &server_type in servers {
                    #[cfg(feature = "local_fs")]
                    let status = PersistedWorkspace::handle(ctx).update(ctx, |model, ctx| {
                        model.detect_lsp_workspace_status(workspace_path.clone(), server_type, ctx)
                    });
                    #[cfg(not(feature = "local_fs"))]
                    let status = LspRepoStatus::CheckingForInstallation;
                    me.suggested_server_statuses
                        .insert((workspace_path.clone(), server_type), status);
                }
                let new_count = LSPServerType::all().count();
                if me.lsp_row_mouse_states.len() != new_count {
                    me.lsp_row_mouse_states
                        .resize_with(new_count, Default::default);
                }
                me.resize_workspace_mouse_states(ctx);
                ctx.notify();
            }
            PersistedWorkspaceEvent::InstallStatusUpdate {
                server_type,
                status,
            } => {
                let new_status = LspRepoStatus::from_installation_status(status, *server_type);
                for ((_, st), repo_status) in &mut me.suggested_server_statuses {
                    if *st == *server_type {
                        *repo_status = new_status.clone();
                    }
                }
                ctx.notify();
            }
            PersistedWorkspaceEvent::InstallationSucceeded
            | PersistedWorkspaceEvent::InstallationFailed
            | PersistedWorkspaceEvent::WorkspaceAdded { .. } => {
                ctx.notify();
            }
        });

        // Re-render when project rules are added or removed so the
        // "Open project rules" button visibility stays up to date.
        ctx.subscribe_to_model(&ProjectContextModel::handle(ctx), |_me, _, event, ctx| {
            if matches!(event, ProjectContextModelEvent::KnownRulesChanged(_)) {
                ctx.notify();
            }
        });

        let code_page_widget = CodePageWidget;

        let workspace_count = PersistedWorkspace::as_ref(ctx).workspaces().count();

        #[cfg(feature = "local_fs")]
        let external_editor_view;
        let page = if FeatureFlag::OpenWarpNewSettingsModes.is_enabled() {
            #[cfg(feature = "local_fs")]
            {
                external_editor_view = Some(ctx.add_typed_action_view(ExternalEditorView::new));
            }

            let codebase_indexing_widgets: Vec<Box<dyn SettingsWidget<View = Self>>> =
                vec![Box::new(CodebaseIndexingCategorizedWidget {
                    inner: code_page_widget,
                })];
            #[cfg(feature = "local_fs")]
            let mut code_editor_review_widgets: Vec<
                Box<dyn SettingsWidget<View = Self>>,
            > = vec![Box::new(ExternalEditorCodeWidget)];
            #[cfg(not(feature = "local_fs"))]
            let mut code_editor_review_widgets: Vec<
                Box<dyn SettingsWidget<View = Self>>,
            > = vec![];
            code_editor_review_widgets.extend([
                Box::new(AutoOpenCodeReviewPaneCodeWidget::default())
                    as Box<dyn SettingsWidget<View = Self>>,
                Box::new(CodeReviewPanelToggleWidget::default()),
                Box::new(CodeReviewDiffStatsToggleWidget::default()),
                Box::new(ProjectExplorerToggleWidget::default()),
                Box::new(GlobalSearchToggleWidget::default()),
            ]);
            let categories = vec![
                Category::new(
                    &*Box::leak(
                        crate::t!("settings-code-category-codebase-indexing").into_boxed_str(),
                    ),
                    codebase_indexing_widgets,
                ),
                Category::new(
                    &*Box::leak(crate::t!("settings-code-category-editor-review").into_boxed_str()),
                    code_editor_review_widgets,
                ),
            ];
            PageType::new_categorized(categories, None)
        } else {
            #[cfg(feature = "local_fs")]
            {
                external_editor_view = None;
            }
            let widgets: Vec<Box<dyn SettingsWidget<View = Self>>> =
                vec![Box::new(code_page_widget)];
            PageType::new_uncategorized(widgets, None)
        };

        Self {
            page,
            active_subpage: None,

            lsp_row_mouse_states: (0..lsp_server_count).map(|_| Default::default()).collect(),
            open_project_rules_mouse_states: (0..workspace_count)
                .map(|_| Default::default())
                .collect(),
            suggested_server_statuses: HashMap::new(),
            #[cfg(feature = "local_fs")]
            external_editor_view,
        }
    }

    /// Set the active subpage and rebuild the page to show only the relevant widgets.
    pub fn set_active_subpage(
        &mut self,
        subpage: Option<CodeSubpage>,
        ctx: &mut ViewContext<Self>,
    ) {
        if self.active_subpage != subpage {
            self.active_subpage = subpage;
            // Rebuild the page with the relevant widgets for the selected subpage,
            // or the full categorized page when subpage is None.
            if let Some(subpage) = subpage {
                let mut widgets: Vec<Box<dyn SettingsWidget<View = Self>>> =
                    vec![Box::new(CodeSubpageHeaderWidget {
                        title: subpage.title(),
                    })];
                match subpage {
                    CodeSubpage::Indexing => {
                        widgets.push(Box::new(CodebaseIndexingCategorizedWidget {
                            inner: CodePageWidget,
                        }));
                    }
                    CodeSubpage::EditorAndCodeReview => {
                        #[cfg(feature = "local_fs")]
                        widgets.push(Box::new(ExternalEditorCodeWidget));
                        widgets.extend([
                            Box::new(AutoOpenCodeReviewPaneCodeWidget::default())
                                as Box<dyn SettingsWidget<View = Self>>,
                            Box::new(CodeReviewPanelToggleWidget::default()),
                            Box::new(CodeReviewDiffStatsToggleWidget::default()),
                            Box::new(ProjectExplorerToggleWidget::default()),
                            Box::new(GlobalSearchToggleWidget::default()),
                        ]);
                    }
                }
                // Subpage widgets render their own subheader-sized titles,
                // so we don't pass a page-level title.
                self.page = PageType::new_uncategorized(widgets, None);
            } else {
                // None: rebuild the full categorized page (all widgets).
                self.page = Self::build_full_page(ctx);
            }
            ctx.notify();
        }
    }

    /// Builds the full categorized page with all Code widgets.
    /// Used for the default/legacy view and when resetting to all-widgets mode for search.
    fn build_full_page(_ctx: &mut ViewContext<Self>) -> PageType<Self> {
        if FeatureFlag::OpenWarpNewSettingsModes.is_enabled() {
            let code_page_widget = CodePageWidget;
            let codebase_indexing_widgets: Vec<Box<dyn SettingsWidget<View = Self>>> =
                vec![Box::new(CodebaseIndexingCategorizedWidget {
                    inner: code_page_widget,
                })];
            #[cfg(feature = "local_fs")]
            let mut code_editor_review_widgets: Vec<
                Box<dyn SettingsWidget<View = Self>>,
            > = vec![Box::new(ExternalEditorCodeWidget)];
            #[cfg(not(feature = "local_fs"))]
            let mut code_editor_review_widgets: Vec<
                Box<dyn SettingsWidget<View = Self>>,
            > = vec![];
            code_editor_review_widgets.extend([
                Box::new(AutoOpenCodeReviewPaneCodeWidget::default())
                    as Box<dyn SettingsWidget<View = Self>>,
                Box::new(CodeReviewPanelToggleWidget::default()),
                Box::new(CodeReviewDiffStatsToggleWidget::default()),
                Box::new(ProjectExplorerToggleWidget::default()),
                Box::new(GlobalSearchToggleWidget::default()),
            ]);
            let categories = vec![
                Category::new(
                    &*Box::leak(
                        crate::t!("settings-code-category-codebase-indexing").into_boxed_str(),
                    ),
                    codebase_indexing_widgets,
                ),
                Category::new(
                    &*Box::leak(crate::t!("settings-code-category-editor-review").into_boxed_str()),
                    code_editor_review_widgets,
                ),
            ];
            PageType::new_categorized(categories, None)
        } else {
            let widgets: Vec<Box<dyn SettingsWidget<View = Self>>> = vec![Box::new(CodePageWidget)];
            PageType::new_uncategorized(widgets, None)
        }
    }

    /// Resize `open_project_rules_mouse_states` to match the current workspace count.
    fn resize_workspace_mouse_states(&mut self, ctx: &AppContext) {
        let workspace_count = PersistedWorkspace::as_ref(ctx).workspaces().count();
        if self.open_project_rules_mouse_states.len() != workspace_count {
            self.open_project_rules_mouse_states
                .resize_with(workspace_count, Default::default);
        }
    }
}

impl Entity for CodeSettingsPageView {
    type Event = CodeSettingsPageEvent;
}

impl View for CodeSettingsPageView {
    fn ui_name() -> &'static str {
        "CodePage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

#[derive(Debug, Clone)]
pub enum CodeSettingsPageEvent {
    SignupAnonymousUser,
    OpenLspLogs { log_path: PathBuf },
    OpenProjectRules { rule_paths: Vec<PathBuf> },
}

// Define the code page actions.
#[derive(Debug, Clone)]
pub enum CodeSettingsPageAction {
    SignupAnonymousUser,
    /// Uninstall a managed LSP server and disable it globally.
    UninstallLspServer {
        server_type: LSPServerType,
    },
    RestartLspServer {
        server: ModelHandle<LspServerModel>,
    },
    OpenLspLogs {
        log_path: PathBuf,
    },
    OpenProjectRules {
        rule_paths: Vec<PathBuf>,
    },
    ToggleCodeReviewPanel,
    ToggleShowCodeReviewDiffStats,
    ToggleAutoOpenCodeReviewPane,
    ToggleProjectExplorer,
    ToggleGlobalSearch,
    /// Install (if needed) and enable a suggested LSP server.
    InstallAndEnableLspServer {
        workspace_path: PathBuf,
        server_type: LSPServerType,
    },
    /// Enable a suggested LSP server that is already installed.
    EnableSuggestedLspServer {
        workspace_path: PathBuf,
        server_type: LSPServerType,
    },
}

impl TypedActionView for CodeSettingsPageView {
    type Action = CodeSettingsPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            CodeSettingsPageAction::SignupAnonymousUser => {
                ctx.emit(CodeSettingsPageEvent::SignupAnonymousUser);
            }
            CodeSettingsPageAction::UninstallLspServer { server_type } => {
                Self::set_global_lsp_server_enabled(*server_type, false, ctx);
                send_telemetry_from_ctx!(
                    LspTelemetryEvent::ServerRemoved {
                        server_type: server_type.binary_name().to_string(),
                        source: LspEnablementSource::Settings,
                    },
                    ctx
                );
                let roots: Vec<_> = LspManagerModel::as_ref(ctx)
                    .workspace_roots()
                    .cloned()
                    .collect();
                LspManagerModel::handle(ctx).update(ctx, |manager, ctx| {
                    for root in roots {
                        manager.remove_server(&root, *server_type, ctx);
                    }
                });

                #[cfg(feature = "local_fs")]
                {
                    let server_type = *server_type;
                    let install_dir = server_type.managed_install_dir();
                    ctx.spawn(
                        async move {
                            if install_dir.exists() {
                                std::fs::remove_dir_all(&install_dir)?;
                            }
                            Ok::<_, std::io::Error>(())
                        },
                        move |_me, result, ctx| {
                            if let Err(err) = result {
                                log::info!(
                                    "Failed to remove managed LSP installation for {}: {err}",
                                    server_type.binary_name()
                                );
                            }
                            PersistedWorkspace::handle(ctx).update(ctx, |workspace, ctx| {
                                workspace.mark_lsp_server_uninstalled(server_type, ctx);
                            });
                            ctx.notify();
                        },
                    );
                }

                ctx.notify();
            }
            CodeSettingsPageAction::RestartLspServer { server } => {
                let server_name = server.as_ref(ctx).server_name();
                send_telemetry_from_ctx!(
                    LspTelemetryEvent::ControlAction {
                        action: LspControlActionType::Restart,
                        server_type: Some(server_name),
                    },
                    ctx
                );
                server.update(ctx, |server, ctx| {
                    server.restart(ctx);
                });
            }
            CodeSettingsPageAction::OpenLspLogs { log_path } => {
                send_telemetry_from_ctx!(
                    LspTelemetryEvent::ControlAction {
                        action: LspControlActionType::OpenLogs,
                        server_type: None,
                    },
                    ctx
                );
                #[cfg(not(target_family = "wasm"))]
                {
                    let log_directory = if log_path.extension().is_some() {
                        log_path.parent()
                    } else {
                        Some(log_path.as_path())
                    };
                    if let Some(log_directory) = log_directory {
                        if let Err(err) = std::fs::create_dir_all(log_directory) {
                            log::info!(
                                "Failed to create LSP log directory {}: {err}",
                                log_directory.display()
                            );
                        }
                    }
                }
                ctx.emit(CodeSettingsPageEvent::OpenLspLogs {
                    log_path: log_path.clone(),
                });
            }
            CodeSettingsPageAction::OpenProjectRules { rule_paths } => {
                ctx.emit(CodeSettingsPageEvent::OpenProjectRules {
                    rule_paths: rule_paths.clone(),
                });
            }
            CodeSettingsPageAction::ToggleCodeReviewPanel => {
                TabSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings.show_code_review_button.toggle_and_save_value(ctx));
                });
                ctx.notify();
            }
            CodeSettingsPageAction::ToggleShowCodeReviewDiffStats => {
                TabSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings
                        .show_code_review_diff_stats
                        .toggle_and_save_value(ctx));
                });
                ctx.notify();
            }
            CodeSettingsPageAction::ToggleProjectExplorer => {
                CodeSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings.show_project_explorer.toggle_and_save_value(ctx));
                });
                ctx.notify();
            }
            CodeSettingsPageAction::ToggleGlobalSearch => {
                CodeSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings.show_global_search.toggle_and_save_value(ctx));
                });
                ctx.notify();
            }
            CodeSettingsPageAction::ToggleAutoOpenCodeReviewPane => {
                GeneralSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings
                        .auto_open_code_review_pane_on_first_agent_change
                        .toggle_and_save_value(ctx));
                });
                send_telemetry_from_ctx!(
                    TelemetryEvent::FeaturesPageAction {
                        action: "ToggleAutoOpenCodeReviewPane".to_string(),
                        value: format!(
                            "{}",
                            *GeneralSettings::as_ref(ctx)
                                .auto_open_code_review_pane_on_first_agent_change
                        )
                    },
                    ctx
                );
                ctx.notify();
            }
            CodeSettingsPageAction::InstallAndEnableLspServer {
                workspace_path,
                server_type,
            } => {
                Self::set_global_lsp_server_enabled(*server_type, true, ctx);
                send_telemetry_from_ctx!(
                    LspTelemetryEvent::ServerEnabled {
                        server_type: server_type.binary_name().to_string(),
                        source: LspEnablementSource::Settings,
                        needed_install: true,
                    },
                    ctx
                );
                #[cfg(feature = "local_fs")]
                {
                    let workspace_path = workspace_path.clone();
                    let server_type = *server_type;
                    PersistedWorkspace::handle(ctx).update(ctx, |workspace, _ctx| {
                        workspace.execute_lsp_task(
                            crate::ai::persisted_workspace::LspTask::Install {
                                file_path: workspace_path.clone(),
                                repo_root: workspace_path,
                                server_type,
                            },
                            _ctx,
                        );
                    });
                }
                #[cfg(not(feature = "local_fs"))]
                let _ = workspace_path;
                ctx.notify();
            }
            CodeSettingsPageAction::EnableSuggestedLspServer {
                workspace_path,
                server_type,
            } => {
                Self::set_global_lsp_server_enabled(*server_type, true, ctx);
                send_telemetry_from_ctx!(
                    LspTelemetryEvent::ServerEnabled {
                        server_type: server_type.binary_name().to_string(),
                        source: LspEnablementSource::Settings,
                        needed_install: false,
                    },
                    ctx
                );
                #[cfg(feature = "local_fs")]
                if !workspace_path.as_os_str().is_empty() {
                    let file_path = workspace_path.clone();
                    PersistedWorkspace::handle(ctx).update(ctx, |workspace, ctx| {
                        workspace.execute_lsp_task(
                            crate::ai::persisted_workspace::LspTask::Spawn {
                                file_path,
                                server_type: Some(*server_type),
                            },
                            ctx,
                        );
                    });
                }
                #[cfg(not(feature = "local_fs"))]
                let _ = workspace_path;
                ctx.notify();
            }
        }
    }
}

pub fn init_actions_from_parent_view<T: Action + Clone>(
    _app: &mut AppContext,
    _context: &ContextPredicate,
    _builder: fn(SettingsAction) -> T,
) {
}

struct CodePageWidget;

impl SettingsWidget for CodePageWidget {
    type View = CodeSettingsPageView;

    fn search_terms(&self) -> &str {
        "code coding codebase repository index indexing indices context path lsp language server"
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let mut content = Flex::column();

        content.add_child(self.render_code_header(appearance));
        content.add_child(render_separator(appearance));
        let mouse_states = InitializedFoldersMouseStates {
            lsp_rows: view.lsp_row_mouse_states.clone(),
            open_project_rules: view.open_project_rules_mouse_states.clone(),
        };

        content.add_child(self.render_initialized_folders(
            mouse_states,
            &view.suggested_server_statuses,
            appearance,
            app,
        ));

        Container::new(content.finish())
            .with_uniform_padding(24.0)
            .finish()
    }
}

impl CodePageWidget {
    /// Renders the main "Code" header.
    fn render_code_header(&self, appearance: &Appearance) -> Box<dyn Element> {
        let ui_builder = appearance.ui_builder();
        let theme = appearance.theme();

        Container::new(
            ui_builder
                .span(crate::t!("settings-code-feature-name"))
                .with_style(UiComponentStyles {
                    font_size: Some(24.0),
                    font_weight: Some(Weight::Bold),
                    font_color: Some(theme.active_ui_text_color().into()),
                    ..Default::default()
                })
                .build()
                .finish(),
        )
        .with_padding_bottom(15.)
        .finish()
    }

    /// Renders the "Initialized / indexed folders" section.
    fn render_initialized_folders(
        &self,
        mouse_states: InitializedFoldersMouseStates,
        _suggested_server_statuses: &HashMap<(PathBuf, LSPServerType), LspRepoStatus>,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ui_builder = appearance.ui_builder();
        let theme = appearance.theme();

        let InitializedFoldersMouseStates {
            lsp_rows: lsp_row_mouse_states,
            open_project_rules: _open_project_rules_mouse_states,
        } = mouse_states;

        let mut content = Flex::column();

        content.add_child(
            Container::new(
                ui_builder
                    .span(crate::t!("settings-code-initialized-folders-header"))
                    .with_style(UiComponentStyles {
                        font_size: Some(16.0),
                        font_weight: Some(Weight::Semibold),
                        font_color: Some(theme.active_ui_text_color().into()),
                        ..Default::default()
                    })
                    .build()
                    .finish(),
            )
            .with_margin_top(8.)
            .with_margin_bottom(12.)
            .finish(),
        );

        let lsp_manager = LspManagerModel::as_ref(app);
        let enabled_servers = CodeSettings::as_ref(app)
            .enabled_lsp_servers
            .value()
            .clone();

        for (idx, server_type) in LSPServerType::all().enumerate() {
            let mouse_states = lsp_row_mouse_states.get(idx).cloned().unwrap_or_default();

            if enabled_servers.contains(&server_type) {
                let server_model = lsp_manager.workspace_roots().find_map(|root| {
                    lsp_manager.servers_for_workspace(root).and_then(|servers| {
                        servers
                            .iter()
                            .find(|server| server.as_ref(app).server_type() == server_type)
                    })
                });
                content.add_child(self.render_lsp_server_row(
                    server_type,
                    server_model,
                    mouse_states,
                    appearance,
                    app,
                ));
            } else {
                content.add_child(self.render_suggested_lsp_server_row(
                    &PathBuf::new(),
                    server_type,
                    None,
                    mouse_states,
                    appearance,
                ));
            }
        }

        content.finish()
    }

    /// Renders a single workspace row with its LSP servers.
    #[allow(clippy::too_many_arguments)]
    fn render_workspace_row(
        &self,
        workspace_path: &Path,
        all_servers: &[(LSPServerType, EnablementState)],
        lsp_manager: &LspManagerModel,
        lsp_mouse_states: Vec<LspServerRowMouseStates>,
        open_rules_mouse: MouseStateHandle,
        suggested_server_statuses: &HashMap<(PathBuf, LSPServerType), LspRepoStatus>,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ui_builder = appearance.ui_builder();
        let theme = appearance.theme();

        let mut workspace_content = Flex::column().with_spacing(MAIN_SECTION_MARGIN);

        // Workspace path header with "Open project rules" button
        let home_dir =
            dirs::home_dir().and_then(|home_dir| home_dir.to_str().map(|s| s.to_owned()));
        let user_friendly = user_friendly_path(
            workspace_path.to_string_lossy().as_ref(),
            home_dir.as_deref(),
        )
        .to_string();

        // Query ProjectContextModel for rules under this workspace
        let workspace_rule_paths =
            ProjectContextModel::as_ref(app).rules_for_workspace(workspace_path);

        let workspace_header_label = Shrinkable::new(
            1.,
            ui_builder
                .span(user_friendly)
                .with_style(UiComponentStyles {
                    font_family_id: Some(appearance.monospace_font_family()),
                    font_size: Some(appearance.ui_font_size()),
                    font_weight: Some(Weight::Bold),
                    font_color: Some(theme.active_ui_text_color().into()),
                    ..Default::default()
                })
                .build()
                .finish(),
        )
        .finish();

        let mut header_row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::Center);
        header_row.add_child(Expanded::new(1., workspace_header_label).finish());

        // Only show "Open project rules" button if rules exist for this workspace
        if !workspace_rule_paths.is_empty() {
            let open_rules_button = ui_builder
                .button(ButtonVariant::Secondary, open_rules_mouse)
                .with_style(UiComponentStyles {
                    font_size: Some(12.),
                    padding: Some(Coords {
                        top: 4.,
                        bottom: 4.,
                        left: 8.,
                        right: 8.,
                    }),
                    ..Default::default()
                })
                .with_hovered_styles(UiComponentStyles {
                    background: Some(theme.surface_3().into()),
                    ..Default::default()
                })
                .with_text_and_icon_label(
                    warpui::ui_components::button::TextAndIcon::new(
                        warpui::ui_components::button::TextAndIconAlignment::IconFirst,
                        crate::t!("settings-code-open-project-rules"),
                        warpui::elements::Icon::new(
                            "bundled/svg/file-code-02.svg",
                            theme.foreground(),
                        ),
                        warpui::elements::MainAxisSize::Min,
                        warpui::elements::MainAxisAlignment::Center,
                        pathfinder_geometry::vector::vec2f(14., 14.),
                    )
                    .with_inner_padding(4.),
                )
                .build()
                .with_cursor(Cursor::PointingHand)
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(CodeSettingsPageAction::OpenProjectRules {
                        rule_paths: workspace_rule_paths.clone(),
                    });
                })
                .finish();
            header_row.add_child(open_rules_button);
        }

        workspace_content.add_child(header_row.finish());

        // LSP Servers section (if any servers known)
        if !all_servers.is_empty() {
            workspace_content.add_child(self.render_lsp_servers_subsection(
                workspace_path,
                all_servers,
                lsp_manager,
                lsp_mouse_states,
                suggested_server_statuses,
                appearance,
                app,
            ));
        }

        Container::new(workspace_content.finish())
            .with_uniform_padding(MAIN_SECTION_MARGIN)
            .with_background(theme.surface_1())
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
            .with_margin_bottom(MAIN_SECTION_MARGIN)
            .finish()
    }

    /// Renders the LSP servers subsection within a workspace row.
    #[allow(clippy::too_many_arguments)]
    fn render_lsp_servers_subsection(
        &self,
        workspace_path: &Path,
        all_servers: &[(LSPServerType, EnablementState)],
        lsp_manager: &LspManagerModel,
        lsp_mouse_states: Vec<LspServerRowMouseStates>,
        suggested_server_statuses: &HashMap<(PathBuf, LSPServerType), LspRepoStatus>,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ui_builder = appearance.ui_builder();
        let theme = appearance.theme();

        let mut content = Flex::column().with_spacing(SUB_SECTION_MARGIN);

        // "LSP SERVERS" label
        content.add_child(
            ui_builder
                .span(crate::t!("settings-code-lsp-section-label"))
                .with_style(UiComponentStyles {
                    font_size: Some(11.0),
                    font_weight: Some(Weight::Semibold),
                    font_color: Some(theme.disabled_ui_text_color().into()),
                    ..Default::default()
                })
                .build()
                .finish(),
        );

        // Get the actual server models for this workspace
        let server_models = lsp_manager.servers_for_workspace(workspace_path);

        for (idx, (server_type, enablement_state)) in all_servers.iter().enumerate() {
            let mouse_states = lsp_mouse_states.get(idx).cloned().unwrap_or_default();

            if *enablement_state == EnablementState::Suggested {
                // Render the "available for download" suggested server row.
                let repo_status = suggested_server_statuses
                    .get(&(workspace_path.to_path_buf(), *server_type))
                    .cloned();
                content.add_child(self.render_suggested_lsp_server_row(
                    workspace_path,
                    *server_type,
                    repo_status,
                    mouse_states,
                    appearance,
                ));
            } else {
                // Find the corresponding server model (only exists if enabled and running)
                let server_model = server_models.and_then(|servers| {
                    servers
                        .iter()
                        .find(|s| s.as_ref(app).server_type() == *server_type)
                });

                content.add_child(self.render_lsp_server_row(
                    *server_type,
                    server_model,
                    mouse_states,
                    appearance,
                    app,
                ));
            }
        }

        content.finish()
    }

    /// Renders a suggested LSP server row with "+" install/enable button.
    fn render_suggested_lsp_server_row(
        &self,
        workspace_path: &Path,
        server_type: LSPServerType,
        repo_status: Option<LspRepoStatus>,
        mouse_states: LspServerRowMouseStates,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let ui_builder = appearance.ui_builder();

        let mut row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::Center);

        // Left side: language initial badge + name/description column
        let mut left_content = Flex::row().with_cross_axis_alignment(CrossAxisAlignment::Center);

        // Language initial badge (no status dot for suggested servers)
        let badge_size = 36.0;
        let avatar = Avatar::new(
            AvatarContent::DisplayName(server_type.binary_name().to_string()),
            UiComponentStyles {
                width: Some(badge_size),
                height: Some(badge_size),
                border_radius: Some(CornerRadius::with_all(Radius::Percentage(50.))),
                font_family_id: Some(appearance.ui_font_family()),
                font_weight: Some(Weight::Bold),
                background: Some(theme.surface_3().into()),
                font_size: Some(16.),
                font_color: Some(theme.active_ui_text_color().into()),
                ..Default::default()
            },
        );

        left_content.add_child(
            Container::new(avatar.build().finish())
                .with_margin_right(8.)
                .finish(),
        );

        // Name + description
        let mut name_desc_column = Flex::column().with_spacing(4.);

        name_desc_column.add_child(
            ui_builder
                .span(server_type.binary_name())
                .with_style(UiComponentStyles {
                    font_size: Some(12.0),
                    font_color: Some(theme.active_ui_text_color().into()),
                    ..Default::default()
                })
                .build()
                .finish(),
        );

        let (description, is_installing) = match &repo_status {
            Some(LspRepoStatus::DisabledAndInstalled { .. }) => {
                (crate::t!("settings-code-lsp-installed"), false)
            }
            Some(LspRepoStatus::Installing { .. }) => {
                (crate::t!("settings-code-lsp-installing"), true)
            }
            Some(LspRepoStatus::CheckingForInstallation) => {
                (crate::t!("settings-code-lsp-checking"), true)
            }
            _ => (crate::t!("settings-code-lsp-available-for-download"), false),
        };

        name_desc_column.add_child(
            ui_builder
                .label(description)
                .with_style(UiComponentStyles {
                    font_color: Some(theme.disabled_ui_text_color().into()),
                    font_size: Some(12.),
                    ..Default::default()
                })
                .build()
                .finish(),
        );

        left_content.add_child(name_desc_column.finish());
        row.add_child(left_content.finish());

        // Right side: "+" button to install/enable
        if !is_installing {
            let workspace_path_clone = workspace_path.to_path_buf();
            let needs_install = matches!(
                &repo_status,
                None | Some(LspRepoStatus::DisabledAndNotInstalled { .. })
            );
            let install_button = icon_button(appearance, Icon::Plus, false, mouse_states.install)
                .with_style(UiComponentStyles {
                    border_width: Some(1.),
                    border_color: Some(theme.surface_3().into()),
                    ..Default::default()
                })
                .build()
                .with_cursor(Cursor::PointingHand)
                .on_click(move |ctx, _, _| {
                    if needs_install {
                        ctx.dispatch_typed_action(
                            CodeSettingsPageAction::InstallAndEnableLspServer {
                                workspace_path: workspace_path_clone.clone(),
                                server_type,
                            },
                        );
                    } else {
                        ctx.dispatch_typed_action(
                            CodeSettingsPageAction::EnableSuggestedLspServer {
                                workspace_path: workspace_path_clone.clone(),
                                server_type,
                            },
                        );
                    }
                })
                .finish();

            row.add_child(install_button);
        }

        Container::new(row.finish())
            .with_uniform_padding(12.)
            .with_background(theme.surface_2())
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
            .finish()
    }

    /// Renders a single LSP server row with language initial icon, status, and actions.
    fn render_lsp_server_row(
        &self,
        server_type: LSPServerType,
        server_model: Option<&warpui::ModelHandle<LspServerModel>>,
        mouse_states: LspServerRowMouseStates,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let ui_builder = appearance.ui_builder();

        let mut row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::Center);

        // Left side: language initial badge + name/status column
        let mut left_content = Flex::row().with_cross_axis_alignment(CrossAxisAlignment::Center);

        // Language initial badge with status dot overlay (using Avatar component)
        let (status_color, status_text) = self.get_lsp_status_info(server_model, app, theme);
        let is_failed = server_model
            .is_some_and(|model| matches!(model.as_ref(app).state(), LspState::Failed { .. }));

        // Language initial badge with status dot overlay (using Avatar component)
        let badge_size = 36.0;
        let mut avatar = Avatar::new(
            AvatarContent::DisplayName(server_type.binary_name().to_string()),
            UiComponentStyles {
                width: Some(badge_size),
                height: Some(badge_size),
                border_radius: Some(CornerRadius::with_all(Radius::Percentage(50.))),
                font_family_id: Some(appearance.ui_font_family()),
                font_weight: Some(Weight::Bold),
                background: Some(theme.surface_3().into()),
                font_size: Some(16.),
                font_color: Some(theme.active_ui_text_color().into()),
                ..Default::default()
            },
        );

        avatar = avatar.with_status_element_with_offset(
            StatusElementTypes::Circle,
            UiComponentStyles {
                width: Some(LSP_STATUS_INDICATOR_SIZE),
                height: Some(LSP_STATUS_INDICATOR_SIZE),
                border_radius: Some(CornerRadius::with_all(Radius::Percentage(50.))),
                background: Some(Fill::Solid(status_color)),
                ..Default::default()
            },
            -5.,
            5.,
        );

        left_content.add_child(
            Container::new(avatar.build().finish())
                .with_margin_right(8.)
                .finish(),
        );

        // Name + status on separate lines
        let mut name_status_column = Flex::column().with_spacing(4.);

        // Server name
        name_status_column.add_child(
            ui_builder
                .span(server_type.binary_name())
                .with_style(UiComponentStyles {
                    font_size: Some(12.0),
                    font_color: Some(theme.active_ui_text_color().into()),
                    ..Default::default()
                })
                .build()
                .finish(),
        );

        // Status text
        let status_text_color = if is_failed {
            Some(status_color)
        } else {
            Some(theme.disabled_ui_text_color().into())
        };

        name_status_column.add_child(
            ui_builder
                .label(status_text)
                .with_style(UiComponentStyles {
                    font_color: status_text_color,
                    font_size: Some(12.),
                    ..Default::default()
                })
                .build()
                .finish(),
        );

        left_content.add_child(name_status_column.finish());
        row.add_child(left_content.finish());

        // Right side: restart/logs/uninstall actions.
        let mut right_content = Flex::row()
            .with_spacing(8.)
            .with_cross_axis_alignment(CrossAxisAlignment::Center);

        if is_failed {
            if let Some(server_handle) = server_model.cloned() {
                let server_for_action = server_handle.clone();
                let restart_button = ui_builder
                    .button(ButtonVariant::Secondary, mouse_states.restart)
                    .with_style(UiComponentStyles {
                        font_size: Some(12.),
                        ..Default::default()
                    })
                    .with_hovered_styles(UiComponentStyles {
                        background: Some(theme.surface_3().into()),
                        ..Default::default()
                    })
                    .with_text_label(crate::t!("settings-code-lsp-restart-server"))
                    .build()
                    .with_cursor(Cursor::PointingHand)
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(CodeSettingsPageAction::RestartLspServer {
                            server: server_for_action.clone(),
                        });
                    })
                    .finish();

                right_content.add_child(restart_button);
            }
        }

        #[cfg(not(target_family = "wasm"))]
        {
            let log_path = server_model
                .map(|model| {
                    crate::code::lsp_logs::log_file_path(
                        server_type,
                        model.as_ref(app).initial_workspace(),
                    )
                })
                .unwrap_or_else(|| crate::code::lsp_logs::log_directory_path(server_type));
            let view_logs_button = ui_builder
                .button(ButtonVariant::Secondary, mouse_states.view_logs)
                .with_style(UiComponentStyles {
                    font_size: Some(12.),
                    ..Default::default()
                })
                .with_hovered_styles(UiComponentStyles {
                    background: Some(theme.surface_3().into()),
                    ..Default::default()
                })
                .with_text_label(crate::t!("settings-code-lsp-view-logs"))
                .build()
                .with_cursor(Cursor::PointingHand)
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(CodeSettingsPageAction::OpenLspLogs {
                        log_path: log_path.clone(),
                    });
                })
                .finish();

            right_content.add_child(view_logs_button);
        }

        let uninstall_button = ui_builder
            .button(ButtonVariant::Secondary, mouse_states.uninstall)
            .with_style(UiComponentStyles {
                font_size: Some(12.),
                ..Default::default()
            })
            .with_hovered_styles(UiComponentStyles {
                background: Some(theme.surface_3().into()),
                ..Default::default()
            })
            .with_text_label(crate::i18n::t_or("settings-code-lsp-uninstall", "卸载"))
            .build()
            .with_cursor(Cursor::PointingHand)
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(CodeSettingsPageAction::UninstallLspServer {
                    server_type,
                });
            })
            .finish();

        right_content.add_child(uninstall_button);

        row.add_child(right_content.finish());

        Container::new(row.finish())
            .with_uniform_padding(12.)
            .with_background(theme.surface_2())
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
            .finish()
    }

    /// Gets the status color and text for an LSP server.
    fn get_lsp_status_info(
        &self,
        server_model: Option<&warpui::ModelHandle<LspServerModel>>,
        app: &AppContext,
        theme: &warp_core::ui::theme::WarpTheme,
    ) -> (ColorU, String) {
        match server_model {
            Some(model) => {
                let server = model.as_ref(app);
                match server.state() {
                    LspState::Available { .. } if !server.has_pending_tasks() => (
                        AnsiColorIdentifier::Green
                            .to_ansi_color(&theme.terminal_colors().normal)
                            .into(),
                        crate::t!("settings-code-lsp-status-available"),
                    ),
                    LspState::Starting | LspState::Available { .. } => (
                        AnsiColorIdentifier::Yellow
                            .to_ansi_color(&theme.terminal_colors().normal)
                            .into(),
                        crate::t!("settings-code-lsp-status-busy"),
                    ),
                    LspState::Failed { .. } => (
                        AnsiColorIdentifier::Red
                            .to_ansi_color(&theme.terminal_colors().normal)
                            .into(),
                        crate::t!("settings-code-lsp-status-failed"),
                    ),
                    LspState::Stopped { .. } | LspState::Stopping { .. } => (
                        theme.disabled_ui_text_color().into_solid(),
                        crate::t!("settings-code-lsp-status-stopped"),
                    ),
                }
            }
            None => (
                theme.disabled_ui_text_color().into_solid(),
                crate::t!("settings-code-lsp-status-not-running"),
            ),
        }
    }
}

/// A simple widget that renders a subheader title for a Code subpage.
struct CodeSubpageHeaderWidget {
    title: String,
}

impl SettingsWidget for CodeSubpageHeaderWidget {
    type View = CodeSettingsPageView;

    fn search_terms(&self) -> &str {
        &self.title
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        build_sub_header(appearance, self.title.clone(), None)
            .with_padding_bottom(HEADER_PADDING)
            .finish()
    }
}

struct CodebaseIndexingCategorizedWidget {
    inner: CodePageWidget,
}

impl SettingsWidget for CodebaseIndexingCategorizedWidget {
    type View = CodeSettingsPageView;

    fn search_terms(&self) -> &str {
        "repository code path lsp language server project rules"
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let mut content = Flex::column();
        let mouse_states = InitializedFoldersMouseStates {
            lsp_rows: view.lsp_row_mouse_states.clone(),
            open_project_rules: view.open_project_rules_mouse_states.clone(),
        };
        content.add_child(self.inner.render_initialized_folders(
            mouse_states,
            &view.suggested_server_statuses,
            appearance,
            app,
        ));

        content.finish()
    }
}

#[cfg(feature = "local_fs")]
struct ExternalEditorCodeWidget;

#[cfg(feature = "local_fs")]
impl SettingsWidget for ExternalEditorCodeWidget {
    type View = CodeSettingsPageView;

    fn search_terms(&self) -> &str {
        "code editor open files markdown AI conversations layout pane tab"
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        if let Some(editor_view) = &view.external_editor_view {
            ChildView::new(editor_view).finish()
        } else {
            Empty::new().finish()
        }
    }
}

#[derive(Default)]
struct AutoOpenCodeReviewPaneCodeWidget {
    switch_state: SwitchStateHandle,
}

impl SettingsWidget for AutoOpenCodeReviewPaneCodeWidget {
    type View = CodeSettingsPageView;

    fn search_terms(&self) -> &str {
        "oz auto open code review pane panel agent mode change first time accepted diff view conversation"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let general_settings = GeneralSettings::as_ref(app);
        render_body_item::<CodeSettingsPageAction>(
            crate::t!("settings-code-auto-open-review-panel").into(),
            None,
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            appearance
                .ui_builder()
                .switch(self.switch_state.clone())
                .check(*general_settings.auto_open_code_review_pane_on_first_agent_change)
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(CodeSettingsPageAction::ToggleAutoOpenCodeReviewPane);
                })
                .finish(),
            Some(crate::t!("settings-code-auto-open-review-panel-desc").into()),
        )
    }
}

impl SettingsPageMeta for CodeSettingsPageView {
    fn section() -> SettingsSection {
        SettingsSection::Code
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        FeatureFlag::OpenWarpNewSettingsModes.is_enabled()
    }

    fn on_page_selected(&mut self, _: bool, ctx: &mut ViewContext<Self>) {
        // We want to immediately see if the user is part of a workspace rather than wait for the next poll.
        std::mem::drop(
            TeamUpdateManager::handle(ctx)
                .update(ctx, |manager, ctx| manager.refresh_workspace_metadata(ctx)),
        );
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<CodeSettingsPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<CodeSettingsPageView>) -> Self {
        SettingsPageViewHandle::Code(view_handle)
    }
}

#[derive(Default)]
struct CodeReviewPanelToggleWidget {
    switch_state: SwitchStateHandle,
}

impl SettingsWidget for CodeReviewPanelToggleWidget {
    type View = CodeSettingsPageView;

    fn search_terms(&self) -> &str {
        "code review panel right side diff git"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let tab_settings = TabSettings::as_ref(app);

        render_body_item::<CodeSettingsPageAction>(
            crate::t!("settings-code-show-code-review-button").into(),
            None,
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            appearance
                .ui_builder()
                .switch(self.switch_state.clone())
                .check(*tab_settings.show_code_review_button)
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(CodeSettingsPageAction::ToggleCodeReviewPanel);
                })
                .finish(),
            Some(crate::t!("settings-code-show-code-review-button-desc").into()),
        )
    }
}

#[derive(Default)]
struct CodeReviewDiffStatsToggleWidget {
    switch_state: SwitchStateHandle,
}

impl SettingsWidget for CodeReviewDiffStatsToggleWidget {
    type View = CodeSettingsPageView;

    fn search_terms(&self) -> &str {
        "code review diff stats lines added removed counts"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let tab_settings = TabSettings::as_ref(app);

        render_body_item::<CodeSettingsPageAction>(
            crate::t!("settings-code-show-diff-stats").into(),
            None,
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            appearance
                .ui_builder()
                .switch(self.switch_state.clone())
                .check(*tab_settings.show_code_review_diff_stats)
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(
                        CodeSettingsPageAction::ToggleShowCodeReviewDiffStats,
                    );
                })
                .finish(),
            Some(crate::t!("settings-code-show-diff-stats-desc").into()),
        )
    }
}

#[derive(Default)]
struct ProjectExplorerToggleWidget {
    switch_state: SwitchStateHandle,
}

impl SettingsWidget for ProjectExplorerToggleWidget {
    type View = CodeSettingsPageView;

    fn search_terms(&self) -> &str {
        "project explorer file tree left panel tools"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let code_settings = CodeSettings::as_ref(app);

        render_body_item::<CodeSettingsPageAction>(
            crate::t!("settings-code-project-explorer").into(),
            None,
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            appearance
                .ui_builder()
                .switch(self.switch_state.clone())
                .check(*code_settings.show_project_explorer)
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(CodeSettingsPageAction::ToggleProjectExplorer);
                })
                .finish(),
            Some(crate::t!("settings-code-project-explorer-desc").into()),
        )
    }
}

#[derive(Default)]
struct GlobalSearchToggleWidget {
    switch_state: SwitchStateHandle,
}

impl SettingsWidget for GlobalSearchToggleWidget {
    type View = CodeSettingsPageView;

    fn search_terms(&self) -> &str {
        "global search file search left panel tools"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let code_settings = CodeSettings::as_ref(app);

        render_body_item::<CodeSettingsPageAction>(
            crate::t!("settings-code-global-search").into(),
            None,
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            appearance
                .ui_builder()
                .switch(self.switch_state.clone())
                .check(*code_settings.show_global_search)
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(CodeSettingsPageAction::ToggleGlobalSearch);
                })
                .finish(),
            Some(crate::t!("settings-code-global-search-desc").into()),
        )
    }
}
