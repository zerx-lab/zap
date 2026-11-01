//! Settings › Agents › 配置管理：三栏（Agent/类型 → 条目 → 详情/diff）。
//!
//! 视觉对齐 `_prototypes/agent-config-manager/index.html`：安静 Surface / Inset /
//! hairline，不用大块 accent 轨。

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

use warp_core::ui::theme::color::internal_colors;
use chrono::{Local, TimeZone};
use warpui::{
    color::ColorU,
    elements::{
        Align, Border, ChildView, Clipped, ClippedScrollStateHandle, ClippedScrollable, Fill,
        ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Element, Expanded,
        Flex, Hoverable, MainAxisAlignment, MainAxisSize, MouseStateHandle, Padding, ParentElement,
        Radius, ScrollbarWidth, Text,
    },
    fonts::{Properties, Weight},
    platform::Cursor,
    ui_components::{
        button::ButtonVariant,
        components::{Coords, UiComponent, UiComponentStyles},
    },
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use super::settings_page::{
    MatchData, SettingsPageEvent, SettingsPageMeta, SettingsPageViewHandle,
};
use super::SettingsSection;
use crate::ai::agent_config::{
    self, AgentConfigItem, AgentKind, BackupManifest, ConfigKind, ConfigScope, DocumentFormat,
    ManagedBy,
};
use crate::appearance::Appearance;
use crate::editor::{
    EditorOptions, EditorView, Event as EditorEvent, PropagateAndNoOpNavigationKeys,
    PropagateHorizontalNavigationKeys, TextColors, TextOptions,
};
use crate::terminal::CLIAgent;
use crate::ui_components::icon_with_status::render_cli_agent_logo;
use crate::ui_components::icons::Icon;
use crate::workspace::WorkspaceAction;

const RAIL_W: f32 = 220.0;
const LIST_W: f32 = 340.0;
const ROW_RADIUS: f32 = 7.0;
const CHIP_RADIUS: f32 = 6.0;
const CARD_RADIUS: f32 = 9.0;
const AGENT_LOGO_SIZE: f32 = 14.0;

#[derive(Debug, Clone)]
pub enum AgentConfigPageAction {
    SelectAgent(Option<AgentKind>),
    SelectKind(Option<ConfigKind>),
    SelectScope(Option<ConfigScope>),
    SelectItem(String),
    ClearSearch,
    SelectBackup(String),
    RestoreBackup(String),
    Refresh,
    UnlockEdit,
    PreviewDiff,
    ConfirmWrite,
    CancelDraft,
    RollbackLatest,
    RepairBridge,
    OpenInEditor,
    SetDetailTab(DetailTab),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DetailTab {
    Content,
    Diff,
    Chain,
    Health,
    History,
}

pub struct AgentConfigPageView {
    items: Vec<AgentConfigItem>,
    filter_agent: Option<AgentKind>,
    filter_kind: Option<ConfigKind>,
    filter_scope: Option<ConfigScope>,
    selected_id: Option<String>,
    selected_backup_id: Option<String>,
    unlocked: bool,
    detail_tab: DetailTab,
    diff_text: String,
    status_message: String,
    warn_message: String,
    editor: ViewHandle<EditorView>,
    query_editor: ViewHandle<EditorView>,
    list_scroll: ClippedScrollStateHandle,
    detail_scroll: ClippedScrollStateHandle,
    rail_scroll: ClippedScrollStateHandle,
    repo_root: Option<PathBuf>,
    btn_refresh: MouseStateHandle,
    btn_unlock: MouseStateHandle,
    btn_preview: MouseStateHandle,
    btn_save: MouseStateHandle,
    btn_cancel: MouseStateHandle,
    btn_rollback: MouseStateHandle,
    btn_bridge: MouseStateHandle,
    btn_open: MouseStateHandle,
    btn_clear_search: MouseStateHandle,
    filter_agent_mouse: RefCell<HashMap<Option<AgentKind>, MouseStateHandle>>,
    filter_kind_mouse: RefCell<HashMap<Option<ConfigKind>, MouseStateHandle>>,
    filter_scope_mouse: RefCell<HashMap<Option<ConfigScope>, MouseStateHandle>>,
    row_mouse: RefCell<HashMap<String, MouseStateHandle>>,
    tab_mouse: RefCell<HashMap<DetailTab, MouseStateHandle>>,
    history_row_mouse: RefCell<HashMap<String, MouseStateHandle>>,
    history_restore_mouse: RefCell<HashMap<String, MouseStateHandle>>,
}

impl AgentConfigPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let appearance = Appearance::as_ref(ctx);
        let font_size = appearance.ui_font_size();
        let mono = appearance.monospace_font_family();
        let default_color = appearance.theme().active_ui_text_color();
        let disabled_color = appearance.theme().disabled_ui_text_color();
        let editor = ctx.add_typed_action_view(move |ctx| {
            let options = EditorOptions {
                soft_wrap: true,
                autogrow: false,
                text: TextOptions {
                    font_size_override: Some(font_size),
                    font_family_override: Some(mono),
                    text_colors_override: Some(TextColors {
                        default_color,
                        disabled_color,
                        hint_color: disabled_color,
                    }),
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut editor = EditorView::new(options, ctx);
            editor.set_placeholder_text("选择一条配置…", ctx);
            editor
        });

        let query_editor = ctx.add_typed_action_view(|ctx| {
            let options = EditorOptions {
                text: TextOptions::ui_text(
                    Some(Appearance::as_ref(ctx).ui_font_subheading()),
                    Appearance::as_ref(ctx),
                ),
                propagate_and_no_op_vertical_navigation_keys:
                    PropagateAndNoOpNavigationKeys::AtBoundary,
                propagate_horizontal_navigation_keys: PropagateHorizontalNavigationKeys::Always,
                single_line: true,
                clear_selections_on_blur: true,
                convert_newline_to_space: true,
                ..Default::default()
            };
            let mut editor = EditorView::new(options, ctx);
            editor.set_placeholder_text("按名称 / 路径 / 类型过滤…", ctx);
            editor
        });
        ctx.subscribe_to_view(&query_editor, |_me, _, event, ctx| {
            if matches!(
                event,
                EditorEvent::Edited(_)
                    | EditorEvent::BufferReplaced
                    | EditorEvent::BufferReinitialized
            ) {
                ctx.notify();
            }
        });

        let mut page = Self {
            items: Vec::new(),
            filter_agent: None,
            filter_kind: None,
            filter_scope: None,
            selected_id: None,
            selected_backup_id: None,
            unlocked: false,
            detail_tab: DetailTab::Content,
            diff_text: String::new(),
            status_message: String::new(),
            warn_message: String::new(),
            editor,
            query_editor,
            list_scroll: ClippedScrollStateHandle::default(),
            detail_scroll: ClippedScrollStateHandle::default(),
            rail_scroll: ClippedScrollStateHandle::default(),
            repo_root: None,
            btn_refresh: MouseStateHandle::default(),
            btn_unlock: MouseStateHandle::default(),
            btn_preview: MouseStateHandle::default(),
            btn_save: MouseStateHandle::default(),
            btn_cancel: MouseStateHandle::default(),
            btn_rollback: MouseStateHandle::default(),
            btn_bridge: MouseStateHandle::default(),
            btn_open: MouseStateHandle::default(),
            btn_clear_search: MouseStateHandle::default(),
            filter_agent_mouse: RefCell::new(HashMap::new()),
            filter_kind_mouse: RefCell::new(HashMap::new()),
            filter_scope_mouse: RefCell::new(HashMap::new()),
            row_mouse: RefCell::new(HashMap::new()),
            tab_mouse: RefCell::new(HashMap::new()),
            history_row_mouse: RefCell::new(HashMap::new()),
            history_restore_mouse: RefCell::new(HashMap::new()),
        };
        page.reload(ctx);
        page
    }

    fn reload(&mut self, ctx: &mut ViewContext<Self>) {
        let _ = agent_config::ensure_backup_root();
        self.items = agent_config::discover(self.repo_root.clone());
        self.status_message = format!("已发现 {} 条配置", self.items.len());
        if let Some(id) = self.selected_id.clone() {
            if !self.items.iter().any(|i| i.id == id) {
                self.selected_id = None;
                self.clear_editor(ctx);
            } else {
                self.load_selected_into_editor(ctx);
            }
        }
        ctx.notify();
    }

    fn search_query(&self, app: &AppContext) -> String {
        self.query_editor.as_ref(app).buffer_text(app)
    }

    fn filtered_items<'a>(&'a self, app: &AppContext) -> Vec<&'a AgentConfigItem> {
        let query = self.search_query(app);
        self.items
            .iter()
            .filter(|i| self.filter_agent.map(|a| i.agent == a).unwrap_or(true))
            .filter(|i| self.filter_kind.map(|k| i.kind == k).unwrap_or(true))
            .filter(|i| self.filter_scope.map(|s| i.scope == s).unwrap_or(true))
            .filter(|i| item_matches_query(i, &query))
            .collect()
    }

    fn selected_item(&self) -> Option<&AgentConfigItem> {
        let id = self.selected_id.as_ref()?;
        self.items.iter().find(|i| &i.id == id)
    }

    fn backups_for_selected(&self) -> Vec<BackupManifest> {
        let Some(item) = self.selected_item() else {
            return Vec::new();
        };
        backups_touching_path(&item.source_path)
    }

    fn clear_editor(&mut self, ctx: &mut ViewContext<Self>) {
        self.diff_text.clear();
        self.warn_message.clear();
        self.unlocked = false;
        self.editor.update(ctx, |ed, ctx| {
            ed.set_buffer_text("", ctx);
            ed.set_placeholder_text("选择一条配置…", ctx);
        });
    }

    fn load_selected_into_editor(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(item) = self.selected_item().cloned() else {
            self.clear_editor(ctx);
            return;
        };
        match agent_config::read_item_display(&item) {
            Ok(text) => {
                self.editor.update(ctx, |ed, ctx| {
                    ed.set_buffer_text(&text, ctx);
                });
                self.warn_message = build_warn(&item);
            }
            Err(e) => {
                self.status_message = format!("读取失败: {e}");
                self.editor.update(ctx, |ed, ctx| {
                    ed.set_buffer_text("", ctx);
                });
            }
        }
        self.unlocked = false;
        self.diff_text.clear();
        self.selected_backup_id = None;
        self.detail_tab = DetailTab::Content;
    }

    fn current_editor_text(&self, ctx: &AppContext) -> String {
        self.editor.as_ref(ctx).buffer_text(ctx)
    }

    fn filter_agent_mouse_for(&self, agent: Option<AgentKind>) -> MouseStateHandle {
        self.filter_agent_mouse.borrow_mut().entry(agent).or_default().clone()
    }
    fn filter_kind_mouse_for(&self, kind: Option<ConfigKind>) -> MouseStateHandle {
        self.filter_kind_mouse.borrow_mut().entry(kind).or_default().clone()
    }
    fn filter_scope_mouse_for(&self, scope: Option<ConfigScope>) -> MouseStateHandle {
        self.filter_scope_mouse.borrow_mut().entry(scope).or_default().clone()
    }
    fn row_mouse_for(&self, id: &str) -> MouseStateHandle {
        self.row_mouse.borrow_mut().entry(id.to_string()).or_default().clone()
    }
    fn tab_mouse_for(&self, tab: DetailTab) -> MouseStateHandle {
        self.tab_mouse.borrow_mut().entry(tab).or_default().clone()
    }
    fn history_row_mouse_for(&self, id: &str) -> MouseStateHandle {
        self.history_row_mouse
            .borrow_mut()
            .entry(id.to_string())
            .or_default()
            .clone()
    }
    fn history_restore_mouse_for(&self, id: &str) -> MouseStateHandle {
        self.history_restore_mouse
            .borrow_mut()
            .entry(id.to_string())
            .or_default()
            .clone()
    }
}

fn build_warn(item: &AgentConfigItem) -> String {
    let mut parts = Vec::new();
    if item.read_only { parts.push("只读"); }
    if item.executable { parts.push("可执行配置：写入前会自动备份"); }
    if item.project_tracked { parts.push("项目级（可能被 git 提交；Zap 不自动 commit）"); }
    if item.needs_codex_retrust { parts.push("Codex 改 hooks 后需重新信任"); }
    if matches!(item.managed_by, ManagedBy::Bridge) { parts.push("由 zap-cli-agent-bridge 托管"); }
    parts.join(" · ")
}

impl Entity for AgentConfigPageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for AgentConfigPageView {
    type Action = AgentConfigPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            AgentConfigPageAction::SelectAgent(a) => {
                self.filter_agent = *a;
                ctx.notify();
            }
            AgentConfigPageAction::SelectKind(k) => {
                self.filter_kind = *k;
                ctx.notify();
            }
            AgentConfigPageAction::SelectScope(s) => {
                self.filter_scope = *s;
                ctx.notify();
            }
            AgentConfigPageAction::SelectItem(id) => {
                self.selected_id = Some(id.clone());
                self.load_selected_into_editor(ctx);
                ctx.notify();
            }
            AgentConfigPageAction::ClearSearch => {
                self.query_editor.update(ctx, |ed, ctx| {
                    ed.set_buffer_text("", ctx);
                });
                ctx.notify();
            }
            AgentConfigPageAction::SelectBackup(id) => {
                self.selected_backup_id = Some(id.clone());
                ctx.notify();
            }
            AgentConfigPageAction::RestoreBackup(id) => {
                match agent_config::restore_backup(id) {
                    Ok(manifest) => {
                        let backup_id = manifest.id.clone();
                        let msg = format!("已回滚到备份 {backup_id}");
                        self.reload(ctx);
                        self.detail_tab = DetailTab::History;
                        self.selected_backup_id = Some(backup_id);
                        self.status_message = msg;
                        ctx.notify();
                    }
                    Err(e) => {
                        self.status_message = format!("回滚失败: {e}");
                        ctx.notify();
                    }
                }
            }
            AgentConfigPageAction::Refresh => self.reload(ctx),
            AgentConfigPageAction::UnlockEdit => {
                if let Some(item) = self.selected_item() {
                    if item.read_only || item.format == DocumentFormat::Directory {
                        self.status_message = "此条目不可编辑".into();
                    } else {
                        self.unlocked = true;
                        self.status_message = "已解锁编辑".into();
                    }
                }
                ctx.notify();
            }
            AgentConfigPageAction::PreviewDiff => {
                let Some(item) = self.selected_item().cloned() else { return; };
                let text = self.current_editor_text(ctx);
                match agent_config::preview_write(&item, &text) {
                    Ok((diff, adds, dels)) => {
                        self.diff_text = diff;
                        self.detail_tab = DetailTab::Diff;
                        self.status_message = format!("diff +{adds} −{dels}");
                    }
                    Err(e) => self.status_message = format!("预览失败: {e}"),
                }
                ctx.notify();
            }
            AgentConfigPageAction::ConfirmWrite => {
                let Some(item) = self.selected_item().cloned() else { return; };
                let text = self.current_editor_text(ctx);
                match agent_config::commit_write(&item, &text, &format!("edit {}", item.id)) {
                    Ok(manifest) => {
                        self.unlocked = false;
                        self.status_message = format!("已写入并备份 {}", manifest.id);
                        if item.needs_codex_retrust {
                            self.warn_message = "已保存。请在 Codex 中重新信任受影响的 hooks。".into();
                        }
                        self.reload(ctx);
                    }
                    Err(e) => {
                        self.status_message = format!("写入失败: {e}");
                        ctx.notify();
                    }
                }
            }
            AgentConfigPageAction::CancelDraft => {
                self.unlocked = false;
                self.diff_text.clear();
                self.load_selected_into_editor(ctx);
                self.status_message = "已放弃草稿".into();
                ctx.notify();
            }
            AgentConfigPageAction::RollbackLatest => match agent_config::restore_latest() {
                Ok(manifest) => {
                    self.status_message = format!("已回滚 {}", manifest.id);
                    self.reload(ctx);
                }
                Err(e) => {
                    self.status_message = format!("回滚失败: {e}");
                    ctx.notify();
                }
            },
            AgentConfigPageAction::RepairBridge => match agent_config::run_bridge_install() {
                Ok(msg) => {
                    self.status_message = if msg.trim().is_empty() {
                        "bridge 接线已执行".into()
                    } else {
                        msg
                    };
                    self.reload(ctx);
                }
                Err(e) => {
                    self.status_message = format!("修复失败: {e}");
                    ctx.notify();
                }
            },
            AgentConfigPageAction::OpenInEditor => {
                if let Some(item) = self.selected_item() {
                    let path = item.source_path.clone();
                    ctx.dispatch_typed_action(&WorkspaceAction::OpenFilePath { path });
                }
            }
            AgentConfigPageAction::SetDetailTab(tab) => {
                self.detail_tab = *tab;
                ctx.notify();
            }
        }
    }
}

impl View for AgentConfigPageView {
    fn ui_name() -> &'static str {
        "AgentConfigPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let hair = Border::left(1.).with_border_color(internal_colors::neutral_4(theme));

        let body = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(
                ConstrainedBox::new(self.render_rail(appearance))
                    .with_max_width(RAIL_W)
                    .with_min_width(RAIL_W)
                    .finish(),
            )
            .with_child(
                Container::new(
                    ConstrainedBox::new(self.render_list_column(appearance, app))
                        .with_max_width(LIST_W)
                        .with_min_width(LIST_W)
                        .finish(),
                )
                .with_border(hair)
                .finish(),
            )
            .with_child(
                Expanded::new(
                    1.,
                    Container::new(self.render_detail(appearance, app))
                        .with_border(
                            Border::left(1.).with_border_color(internal_colors::neutral_4(theme)),
                        )
                        .finish(),
                )
                .finish(),
            )
            .finish();

        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(self.render_header(appearance))
            .with_child(Expanded::new(1., body).finish())
            .with_child(self.render_status_bar(appearance))
            .finish()
    }
}

impl AgentConfigPageView {
    fn render_header(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let title = Text::new(
            "配置管理",
            appearance.ui_font_family(),
            appearance.ui_font_size() + 3.,
        )
        .with_color(theme.active_ui_text_color().into())
        .with_style(Properties::default().weight(Weight::Semibold))
        .finish();
        let subtitle = Text::new(
            "一处看清 Rules · Hooks · Skills · Settings · MCP；编辑走 diff 确认与自动备份。",
            appearance.ui_font_family(),
            appearance.ui_font_size() - 1.,
        )
        .with_color(theme.nonactive_ui_text_color().into())
        .finish();
        let legend = Text::new(
            "琥珀 = 可执行（hook / statusLine / MCP）  ·  托管 = bridge 接线",
            appearance.ui_font_family(),
            appearance.ui_font_size() - 2.,
        )
        .with_color(theme.disabled_ui_text_color().into())
        .finish();
        let refresh = chrome_btn(
            appearance,
            self.btn_refresh.clone(),
            "刷新",
            ButtonVariant::Secondary,
            AgentConfigPageAction::Refresh,
        );

        Container::new(
            Flex::column()
                .with_child(
                    Flex::row()
                        .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_child(title)
                        .with_child(refresh)
                        .finish(),
                )
                .with_child(Container::new(subtitle).with_margin_top(6.).finish())
                .with_child(Container::new(legend).with_margin_top(6.).finish())
                .finish(),
        )
        .with_padding(Padding::uniform(0.).with_top(18.).with_right(20.).with_bottom(14.).with_left(20.))
        .with_border(Border::bottom(1.).with_border_color(internal_colors::neutral_4(theme)))
        .finish()
    }

    fn render_status_bar(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let status_owned = if self.status_message.is_empty() {
            " ".into()
        } else {
            self.status_message.clone()
        };
        Container::new(
            Text::new(
                status_owned,
                appearance.ui_font_family(),
                appearance.ui_font_size() - 1.,
            )
            .with_color(theme.nonactive_ui_text_color().into())
            .finish(),
        )
        .with_padding(Padding::uniform(0.).with_top(8.).with_right(20.).with_bottom(10.).with_left(20.))
        .with_background(internal_colors::fg_overlay_1(theme))
        .with_border(Border::top(1.).with_border_color(internal_colors::neutral_4(theme)))
        .finish()
    }

    fn render_rail(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let mut col = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);

        col.add_child(section_label(appearance, "AGENT"));
        col.add_child(self.rail_agent_row(appearance, None, "全部", None, self.items.len()));
        for agent in AgentKind::all() {
            let count = self.items.iter().filter(|i| i.agent == *agent).count();
            col.add_child(self.rail_agent_row(
                appearance,
                Some(*agent),
                agent.display_name(),
                Some(agent_logo(*agent, appearance)),
                count,
            ));
        }

        col.add_child(section_label(appearance, "类型"));
        let kind_all = self
            .items
            .iter()
            .filter(|i| self.filter_agent.map(|a| i.agent == a).unwrap_or(true))
            .count();
        col.add_child(self.rail_kind_row(appearance, None, "全部", "◎", kind_all));
        for kind in ConfigKind::all() {
            let count = self
                .items
                .iter()
                .filter(|i| self.filter_agent.map(|a| i.agent == a).unwrap_or(true))
                .filter(|i| i.kind == *kind)
                .count();
            col.add_child(self.rail_kind_row(
                appearance,
                Some(*kind),
                kind.display_name(),
                kind_glyph(*kind),
                count,
            ));
        }

        col.add_child(section_label(appearance, "作用域"));
        col.add_child(self.render_scope_bar(appearance));

        col.add_child(section_label(appearance, "维护"));
        col.add_child(self.rail_action_row(
            appearance,
            self.btn_bridge.clone(),
            "✓  修复 bridge 接线",
            AgentConfigPageAction::RepairBridge,
        ));
        col.add_child(self.rail_action_row(
            appearance,
            self.btn_rollback.clone(),
            "↺  回滚上次修改",
            AgentConfigPageAction::RollbackLatest,
        ));

        ClippedScrollable::vertical(
            self.rail_scroll.clone(),
            Container::new(col.finish())
                .with_padding(Padding::uniform(0.).with_top(10.).with_right(8.).with_bottom(12.).with_left(8.))
                .finish(),
            ScrollbarWidth::Custom(5.),
            theme.nonactive_ui_detail().into(),
            theme.active_ui_detail().into(),
            Fill::None,
        )
        .finish()
    }

    fn render_scope_bar(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let mut row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(4.);
        for (label, scope) in [
            ("全部", None),
            ("全局", Some(ConfigScope::User)),
            ("项目", Some(ConfigScope::Project)),
            ("本地", Some(ConfigScope::Local)),
        ] {
            let active = self.filter_scope == scope;
            let handle = self.filter_scope_mouse_for(scope);
            let label_s = label.to_string();
            row.add_child(
                Expanded::new(
                    1.,
                    Hoverable::new(handle, move |mouse| {
                        let mut chip = Container::new(
                            Align::new(
                                Text::new(
                                    label_s.clone(),
                                    appearance.ui_font_family(),
                                    appearance.ui_font_size() - 2.,
                                )
                                .with_color(if active {
                                    theme.active_ui_text_color().into()
                                } else {
                                    theme.disabled_ui_text_color().into()
                                })
                                .finish(),
                            )
                            .top_center()
                            .finish(),
                        )
                        .with_padding(Padding::uniform(0.).with_top(5.).with_bottom(5.))
                        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(5.)))
                        .with_border(
                            Border::all(1.).with_border_color(if active {
                                internal_colors::neutral_5(theme)
                            } else {
                                internal_colors::neutral_4(theme)
                            }),
                        );
                        if active {
                            chip = chip.with_background(internal_colors::fg_overlay_3(theme));
                        } else if mouse.is_hovered() {
                            chip = chip.with_background(internal_colors::fg_overlay_2(theme));
                        }
                        chip.finish()
                    })
                    .with_cursor(Cursor::PointingHand)
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(AgentConfigPageAction::SelectScope(scope));
                    })
                    .finish(),
                )
                .finish(),
            );
        }
        Container::new(row.finish())
            .with_padding(Padding::uniform(0.).with_left(6.).with_right(6.).with_top(2.))
            .finish()
    }

    fn rail_agent_row(
        &self,
        appearance: &Appearance,
        agent: Option<AgentKind>,
        label: &str,
        logo: Option<Box<dyn Element>>,
        count: usize,
    ) -> Box<dyn Element> {
        let active = self.filter_agent == agent;
        let handle = self.filter_agent_mouse_for(agent);
        rail_row(
            appearance,
            handle,
            active,
            label,
            count,
            logo,
            None,
            move |ctx| {
                ctx.dispatch_typed_action(AgentConfigPageAction::SelectAgent(agent));
            },
        )
    }

    fn rail_kind_row(
        &self,
        appearance: &Appearance,
        kind: Option<ConfigKind>,
        label: &str,
        glyph: &str,
        count: usize,
    ) -> Box<dyn Element> {
        let active = self.filter_kind == kind;
        let handle = self.filter_kind_mouse_for(kind);
        rail_row(
            appearance,
            handle,
            active,
            label,
            count,
            None,
            Some(glyph),
            move |ctx| {
                ctx.dispatch_typed_action(AgentConfigPageAction::SelectKind(kind));
            },
        )
    }

    fn rail_action_row(
        &self,
        appearance: &Appearance,
        handle: MouseStateHandle,
        label: &str,
        action: AgentConfigPageAction,
    ) -> Box<dyn Element> {
        rail_row(appearance, handle, false, label, 0, None, None, move |ctx| {
            ctx.dispatch_typed_action(action.clone());
        })
    }
}

impl AgentConfigPageView {
    fn render_list_column(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(self.render_search(appearance, app))
            .with_child(Expanded::new(1., self.render_list(appearance, app)).finish())
            .finish()
    }

    fn render_search(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        let theme = appearance.theme();
        let query = self.search_query(app);
        let has_query = !query.trim().is_empty();

        let mut row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.)
            .with_child(
                Text::new(
                    String::from("⌕"),
                    appearance.ui_font_family(),
                    appearance.ui_font_size() - 1.,
                )
                .with_color(theme.disabled_ui_text_color().into())
                .finish(),
            )
            .with_child(
                Expanded::new(
                    1.,
                    Clipped::new(ChildView::new(&self.query_editor).finish()).finish(),
                )
                .finish(),
            );

        if has_query {
            let clear_handle = self.btn_clear_search.clone();
            row.add_child(
                Hoverable::new(clear_handle, move |mouse| {
                    Text::new(
                        String::from("×"),
                        appearance.ui_font_family(),
                        appearance.ui_font_size() + 1.,
                    )
                    .with_color(if mouse.is_hovered() {
                        theme.active_ui_text_color().into()
                    } else {
                        theme.disabled_ui_text_color().into()
                    })
                    .finish()
                })
                .with_cursor(Cursor::PointingHand)
                .on_click(|ctx, _, _| {
                    ctx.dispatch_typed_action(AgentConfigPageAction::ClearSearch);
                })
                .finish(),
            );
        }

        Container::new(
            Container::new(row.finish())
                .with_padding(
                    Padding::uniform(0.)
                        .with_top(7.)
                        .with_right(10.)
                        .with_bottom(7.)
                        .with_left(10.),
                )
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(ROW_RADIUS)))
                .with_border(Border::all(1.).with_border_color(internal_colors::neutral_4(theme)))
                .with_background(internal_colors::fg_overlay_1(theme))
                .finish(),
        )
        .with_padding(
            Padding::uniform(0.)
                .with_top(10.)
                .with_right(10.)
                .with_bottom(4.)
                .with_left(10.),
        )
        .finish()
    }

    fn render_list(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        let theme = appearance.theme();
        let items = self.filtered_items(app);
        let mut col = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);

        if items.is_empty() {
            let hint = if self.search_query(app).trim().is_empty() {
                "调整左侧 Agent / 类型 / 作用域筛选，或点刷新重新扫描。"
            } else {
                "没有匹配当前搜索的条目；可清空搜索框或放宽筛选。"
            };
            col.add_child(empty_state(appearance, "无匹配项", hint));
        }

        let mut group_count_cache: HashMap<(ConfigScope, String), usize> = HashMap::new();
        for item in &items {
            let key = (item.scope, short_path(&item.source_path));
            *group_count_cache.entry(key).or_insert(0) += 1;
        }

        let mut last_key = None;
        for item in items {
            let path = short_path(&item.source_path);
            let gkey = (item.scope, item.source_path.clone());
            if last_key.as_ref() != Some(&gkey) {
                last_key = Some(gkey);
                let count = group_count_cache
                    .get(&(item.scope, path.clone()))
                    .copied()
                    .unwrap_or(0);
                col.add_child(group_header(
                    appearance,
                    &format!("{} · {}", item.scope.display_name(), path),
                    count,
                ));
            }

            let selected = self.selected_id.as_deref() == Some(item.id.as_str());
            let id = item.id.clone();
            let title = item.title.clone();
            let summary = item.summary.clone();
            let executable = item.executable;
            let bridged = matches!(item.managed_by, ManagedBy::Bridge);
            let read_only = item.read_only;
            let glyph = kind_glyph(item.kind).to_string();
            let handle = self.row_mouse_for(&id);

            let row = Hoverable::new(handle, move |mouse| {
                let mut title_row = Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_spacing(8.)
                    .with_child(
                        Text::new(
                            glyph.clone(),
                            appearance.ui_font_family(),
                            appearance.ui_font_size() - 1.,
                        )
                        .with_color(theme.disabled_ui_text_color().into())
                        .finish(),
                    )
                    .with_child(
                        Expanded::new(
                            1.,
                            Text::new(
                                title.clone(),
                                appearance.ui_font_family(),
                                appearance.ui_font_size(),
                            )
                            .with_color(theme.active_ui_text_color().into())
                            .with_style(Properties::default().weight(Weight::Medium))
                            .finish(),
                        )
                        .finish(),
                    );
                if executable {
                    title_row.add_child(pill_tag(appearance, "exec", TagTone::Warn));
                }
                if bridged {
                    title_row.add_child(pill_tag(appearance, "托管", TagTone::Muted));
                }
                if read_only {
                    title_row.add_child(pill_tag(appearance, "只读", TagTone::Muted));
                }

                let body = Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_spacing(3.)
                    .with_child(title_row.finish())
                    .with_child(
                        Text::new(
                            summary.clone(),
                            appearance.monospace_font_family(),
                            appearance.ui_font_size() - 2.,
                        )
                        .with_color(theme.disabled_ui_text_color().into())
                        .finish(),
                    )
                    .finish();

                let mut row = Container::new(body)
                    .with_padding(
                        Padding::uniform(0.)
                            .with_top(9.)
                            .with_right(10.)
                            .with_bottom(9.)
                            .with_left(10.),
                    )
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(ROW_RADIUS)))
                    .with_margin_bottom(2.);
                if selected {
                    row = row
                        .with_background(internal_colors::fg_overlay_3(theme))
                        .with_border(
                            Border::all(1.)
                                .with_border_color(internal_colors::neutral_5(theme)),
                        );
                } else if mouse.is_hovered() {
                    row = row.with_background(internal_colors::fg_overlay_2(theme));
                }
                row.finish()
            })
            .with_cursor(Cursor::PointingHand)
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(AgentConfigPageAction::SelectItem(id.clone()));
            })
            .finish();
            col.add_child(row);
        }

        ClippedScrollable::vertical(
            self.list_scroll.clone(),
            Container::new(col.finish())
                .with_padding(
                    Padding::uniform(0.)
                        .with_top(8.)
                        .with_right(8.)
                        .with_bottom(10.)
                        .with_left(8.),
                )
                .finish(),
            ScrollbarWidth::Custom(6.),
            theme.nonactive_ui_detail().into(),
            theme.active_ui_detail().into(),
            Fill::None,
        )
        .finish()
    }
}

impl AgentConfigPageView {
    fn render_history_tab(
        &self,
        appearance: &Appearance,
        _item: &AgentConfigItem,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let backups = self.backups_for_selected();
        let mut col = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(8.);

        col.add_child(
            Text::new(
                format!(
                    "Zap 数据目录 agent-config/backups · 默认保留 {} 份 · 仅列出含本文件的备份",
                    agent_config::DEFAULT_KEEP
                ),
                appearance.ui_font_family(),
                appearance.ui_font_size() - 2.,
            )
            .with_color(theme.disabled_ui_text_color().into())
            .finish(),
        );

        if backups.is_empty() {
            col.add_child(empty_state(
                appearance,
                "暂无相关备份",
                "写入可编辑配置时会自动备份到 Zap 数据目录；回滚后可在此查看与恢复。",
            ));
            return inset_card(appearance, "历史", col.finish());
        }

        for backup in backups {
            let id = backup.id.clone();
            let selected = self.selected_backup_id.as_deref() == Some(id.as_str());
            let row_handle = self.history_row_mouse_for(&id);
            let restore_handle = self.history_restore_mouse_for(&id);
            let time_label = format_backup_time(backup.created_unix_ms);
            let note = if backup.note.trim().is_empty() {
                "（无说明）".to_string()
            } else {
                backup.note.clone()
            };
            let file_count = backup.files.len();
            let restore_id = id.clone();
            let select_id = id.clone();
            let time_for_row = time_label.clone();
            let note_for_row = note.clone();
            let id_for_row = id.clone();

            let row = Hoverable::new(row_handle, move |mouse| {
                let restore_btn = Hoverable::new(restore_handle.clone(), move |mouse| {
                    let mut btn = Container::new(
                        Text::new(
                            String::from("回滚到此"),
                            appearance.ui_font_family(),
                            appearance.ui_font_size() - 2.,
                        )
                        .with_color(if mouse.is_hovered() {
                            theme.active_ui_text_color().into()
                        } else {
                            theme.nonactive_ui_text_color().into()
                        })
                        .finish(),
                    )
                    .with_padding(
                        Padding::uniform(0.)
                            .with_top(4.)
                            .with_right(8.)
                            .with_bottom(4.)
                            .with_left(8.),
                    )
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(CHIP_RADIUS)))
                    .with_border(
                        Border::all(1.).with_border_color(internal_colors::neutral_5(theme)),
                    );
                    if mouse.is_hovered() {
                        btn = btn.with_background(internal_colors::fg_overlay_2(theme));
                    }
                    btn.finish()
                })
                .with_cursor(Cursor::PointingHand)
                .on_click({
                    let restore_id = restore_id.clone();
                    move |ctx, _, _| {
                        ctx.dispatch_typed_action(AgentConfigPageAction::RestoreBackup(
                            restore_id.clone(),
                        ));
                    }
                })
                .finish();

                let body = Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_spacing(10.)
                    .with_child(
                        Expanded::new(
                            1.,
                            Flex::column()
                                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                                .with_spacing(3.)
                                .with_child(
                                    Text::new(
                                        time_for_row.clone(),
                                        appearance.ui_font_family(),
                                        appearance.ui_font_size() - 1.,
                                    )
                                    .with_color(theme.active_ui_text_color().into())
                                    .with_style(Properties::default().weight(Weight::Medium))
                                    .finish(),
                                )
                                .with_child(
                                    Text::new(
                                        note_for_row.clone(),
                                        appearance.ui_font_family(),
                                        appearance.ui_font_size() - 2.,
                                    )
                                    .with_color(theme.nonactive_ui_text_color().into())
                                    .finish(),
                                )
                                .with_child(
                                    Text::new(
                                        format!("id {id_for_row} · {file_count} 个文件"),
                                        appearance.monospace_font_family(),
                                        appearance.ui_font_size() - 3.,
                                    )
                                    .with_color(theme.disabled_ui_text_color().into())
                                    .finish(),
                                )
                                .finish(),
                        )
                        .finish(),
                    )
                    .with_child(restore_btn)
                    .finish();

                let mut box_el = Container::new(body)
                    .with_padding(Padding::uniform(10.))
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(7.)));
                if selected {
                    box_el = box_el
                        .with_background(internal_colors::fg_overlay_3(theme))
                        .with_border(
                            Border::all(1.).with_border_color(internal_colors::neutral_5(theme)),
                        );
                } else if mouse.is_hovered() {
                    box_el = box_el.with_background(internal_colors::fg_overlay_2(theme));
                } else {
                    box_el = box_el.with_background(internal_colors::fg_overlay_1(theme));
                }
                box_el.finish()
            })
            .with_cursor(Cursor::PointingHand)
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(AgentConfigPageAction::SelectBackup(select_id.clone()));
            })
            .finish();

            col.add_child(row);
        }

        inset_card(appearance, "历史", col.finish())
    }

    fn render_detail(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        let theme = appearance.theme();
        let Some(item) = self.selected_item() else {
            return Container::new(empty_state(
                appearance,
                "从左侧选择一条配置",
                "筛选 Agent / 类型后，点中栏条目查看内容、diff、生效链、体检与历史备份。",
            ))
            .with_padding(Padding::uniform(24.))
            .finish();
        };

        let mut col = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);

        let mut title_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(10.)
            .with_child(agent_logo(item.agent, appearance))
            .with_child(
                Text::new(
                    format!("{} · {}", item.agent.display_name(), item.title),
                    appearance.ui_font_family(),
                    appearance.ui_font_size() + 2.,
                )
                .with_color(theme.active_ui_text_color().into())
                .with_style(Properties::default().weight(Weight::Semibold))
                .finish(),
            );
        if item.executable {
            title_row.add_child(pill_tag(appearance, "可执行", TagTone::Warn));
        }
        if matches!(item.managed_by, ManagedBy::Bridge) {
            title_row.add_child(pill_tag(appearance, "托管", TagTone::Accent));
        }
        if item.read_only {
            title_row.add_child(pill_tag(appearance, "只读", TagTone::Muted));
        }

        let meta = Text::new(
            format!(
                "来源  {}{}  ·  作用域  {}  ·  {}",
                short_path(&item.source_path),
                if item.pointer.is_empty() {
                    String::new()
                } else {
                    format!(" › {}", item.pointer)
                },
                item.scope.display_name(),
                item.kind.display_name(),
            ),
            appearance.ui_font_family(),
            appearance.ui_font_size() - 1.,
        )
        .with_color(theme.nonactive_ui_text_color().into())
        .finish();

        col.add_child(
            Container::new(
                Flex::column()
                    .with_child(title_row.finish())
                    .with_child(Container::new(meta).with_margin_top(6.).finish())
                    .finish(),
            )
            .with_padding(
                Padding::uniform(0.)
                    .with_top(14.)
                    .with_right(18.)
                    .with_bottom(10.)
                    .with_left(18.),
            )
            .with_border(Border::bottom(1.).with_border_color(internal_colors::neutral_4(theme)))
            .finish(),
        );

        let mut tabs = Flex::row()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::End)
            .with_spacing(2.);
        for (tab, label) in [
            (DetailTab::Content, "内容"),
            (DetailTab::Diff, "变更 diff"),
            (DetailTab::Chain, "生效链"),
            (DetailTab::Health, "体检"),
            (DetailTab::History, "历史"),
        ] {
            let active = self.detail_tab == tab;
            let handle = self.tab_mouse_for(tab);
            let label = label.to_string();
            tabs.add_child(
                Hoverable::new(handle, move |mouse| {
                    let mut tab_el = Container::new(
                        Text::new(
                            label.clone(),
                            appearance.ui_font_family(),
                            appearance.ui_font_size() - 1.,
                        )
                        .with_color(if active {
                            theme.active_ui_text_color().into()
                        } else if mouse.is_hovered() {
                            theme.nonactive_ui_text_color().into()
                        } else {
                            theme.disabled_ui_text_color().into()
                        })
                        .finish(),
                    )
                    .with_padding(
                        Padding::uniform(0.)
                            .with_top(8.)
                            .with_right(12.)
                            .with_bottom(8.)
                            .with_left(12.),
                    );
                    if active {
                        tab_el = tab_el.with_border(
                            Border::bottom(2.).with_border_color(theme.accent().into_solid()),
                        );
                    } else {
                        tab_el = tab_el.with_border(
                            Border::bottom(2.).with_border_color(ColorU::transparent_black()),
                        );
                    }
                    tab_el.finish()
                })
                .with_cursor(Cursor::PointingHand)
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(AgentConfigPageAction::SetDetailTab(tab));
                })
                .finish(),
            );
        }
        col.add_child(
            Container::new(tabs.finish())
                .with_padding(Padding::uniform(0.).with_left(10.).with_right(10.))
                .with_border(Border::bottom(1.).with_border_color(internal_colors::neutral_4(theme)))
                .finish(),
        );

        let mut body_col = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(12.);
        if !self.warn_message.is_empty() {
            body_col.add_child(guard_banner(appearance, &self.warn_message));
        }

        let body_inner: Box<dyn Element> = match self.detail_tab {
            DetailTab::Content => {
                let format_label = match item.format {
                    DocumentFormat::Json => "JSON",
                    DocumentFormat::Toml => "TOML",
                    DocumentFormat::Markdown => "Markdown",
                    DocumentFormat::Directory => "目录",
                };
                inset_card(
                    appearance,
                    &format!("{format_label} · 原文"),
                    ConstrainedBox::new(ChildView::new(&self.editor).finish())
                        .with_min_height(260.)
                        .finish(),
                )
            }
            DetailTab::Diff => inset_card(
                appearance,
                "变更 diff",
                Text::new(
                    if self.diff_text.is_empty() {
                        "解锁编辑并点「预览 diff」后，这里会显示逐行差异。".into()
                    } else {
                        self.diff_text.clone()
                    },
                    appearance.monospace_font_family(),
                    appearance.ui_font_size() - 1.,
                )
                .with_color(theme.active_ui_text_color().into())
                .finish(),
            ),
            DetailTab::Chain => {
                let layers =
                    agent_config::claude_settings_effective_chain(self.repo_root.as_deref());
                let mut c = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
                for (idx, layer) in layers.iter().enumerate() {
                    let mark = if layer.wins {
                        "生效"
                    } else if layer.exists {
                        "存在"
                    } else {
                        "不存在"
                    };
                    let mut row = Container::new(
                        Flex::row()
                            .with_main_axis_size(MainAxisSize::Max)
                            .with_cross_axis_alignment(CrossAxisAlignment::Center)
                            .with_spacing(10.)
                            .with_child(
                                ConstrainedBox::new(
                                    Text::new(
                                        layer.scope.display_name().to_string(),
                                        appearance.ui_font_family(),
                                        appearance.ui_font_size() - 1.,
                                    )
                                    .with_color(theme.disabled_ui_text_color().into())
                                    .finish(),
                                )
                                .with_width(48.)
                                .finish(),
                            )
                            .with_child(
                                Expanded::new(
                                    1.,
                                    Text::new(
                                        short_path(&layer.path),
                                        appearance.monospace_font_family(),
                                        appearance.ui_font_size() - 1.,
                                    )
                                    .with_color(if layer.wins {
                                        theme.active_ui_text_color().into()
                                    } else {
                                        theme.nonactive_ui_text_color().into()
                                    })
                                    .finish(),
                                )
                                .finish(),
                            )
                            .with_child(pill_tag(
                                appearance,
                                mark,
                                if layer.wins {
                                    TagTone::Accent
                                } else {
                                    TagTone::Muted
                                },
                            ))
                            .finish(),
                    )
                    .with_padding(
                        Padding::uniform(0.)
                            .with_top(8.)
                            .with_right(12.)
                            .with_bottom(8.)
                            .with_left(12.),
                    );
                    if idx > 0 {
                        row = row.with_border(
                            Border::top(1.).with_border_color(internal_colors::neutral_4(theme)),
                        );
                    }
                    if layer.wins {
                        row = row.with_background(internal_colors::fg_overlay_2(theme));
                    }
                    c.add_child(row.finish());
                }
                inset_card(appearance, "生效链 · 高 → 低", c.finish())
            }
            DetailTab::Health => {
                let findings = agent_config::run_health_check(&self.items);
                let mut c = Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_spacing(8.);
                if findings.is_empty() {
                    c.add_child(
                        Text::new(
                            "未发现问题",
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_color(theme.nonactive_ui_text_color().into())
                        .finish(),
                    );
                }
                for f in findings {
                    c.add_child(
                        Container::new(
                            Flex::column()
                                .with_child(
                                    Text::new(
                                        f.title.clone(),
                                        appearance.ui_font_family(),
                                        appearance.ui_font_size() - 1.,
                                    )
                                    .with_color(theme.active_ui_text_color().into())
                                    .with_style(Properties::default().weight(Weight::Medium))
                                    .finish(),
                                )
                                .with_child(
                                    Container::new(
                                        Text::new(
                                            f.detail.clone(),
                                            appearance.ui_font_family(),
                                            appearance.ui_font_size() - 2.,
                                        )
                                        .with_color(theme.nonactive_ui_text_color().into())
                                        .finish(),
                                    )
                                    .with_margin_top(3.)
                                    .finish(),
                                )
                                .finish(),
                        )
                        .with_padding(Padding::uniform(10.))
                        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(7.)))
                        .with_background(internal_colors::fg_overlay_2(theme))
                        .finish(),
                    );
                }
                inset_card(appearance, "体检", c.finish())
            }
            DetailTab::History => self.render_history_tab(appearance, item),
        };
        body_col.add_child(body_inner);

        col.add_child(
            Expanded::new(
                1.,
                ClippedScrollable::vertical(
                    self.detail_scroll.clone(),
                    Container::new(body_col.finish())
                        .with_padding(
                            Padding::uniform(0.)
                                .with_top(14.)
                                .with_right(18.)
                                .with_bottom(12.)
                                .with_left(18.),
                        )
                        .finish(),
                    ScrollbarWidth::Custom(6.),
                    theme.nonactive_ui_detail().into(),
                    theme.active_ui_detail().into(),
                    Fill::None,
                )
                .finish(),
            )
            .finish(),
        );

        let mut actions = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.);
        actions.add_child(
            Expanded::new(
                1.,
                Text::new(
                    if self.unlocked {
                        String::from("已解锁 — 预览 diff 后再写入；写入前自动备份")
                    } else {
                        String::from("env / token 已打码，不会进入备份差异视图")
                    },
                    appearance.ui_font_family(),
                    appearance.ui_font_size() - 2.,
                )
                .with_color(theme.disabled_ui_text_color().into())
                .finish(),
            )
            .finish(),
        );
        actions.add_child(chrome_btn(
            appearance,
            self.btn_open.clone(),
            "在编辑器打开",
            ButtonVariant::Secondary,
            AgentConfigPageAction::OpenInEditor,
        ));
        if !item.read_only && item.format != DocumentFormat::Directory {
            if self.unlocked {
                actions.add_child(chrome_btn(
                    appearance,
                    self.btn_cancel.clone(),
                    "放弃",
                    ButtonVariant::Secondary,
                    AgentConfigPageAction::CancelDraft,
                ));
                actions.add_child(chrome_btn(
                    appearance,
                    self.btn_preview.clone(),
                    "预览 diff",
                    ButtonVariant::Secondary,
                    AgentConfigPageAction::PreviewDiff,
                ));
                actions.add_child(chrome_btn(
                    appearance,
                    self.btn_save.clone(),
                    "备份并写入",
                    ButtonVariant::Accent,
                    AgentConfigPageAction::ConfirmWrite,
                ));
            } else {
                actions.add_child(chrome_btn(
                    appearance,
                    self.btn_unlock.clone(),
                    "解锁编辑",
                    ButtonVariant::Accent,
                    AgentConfigPageAction::UnlockEdit,
                ));
            }
        }

        col.add_child(
            Container::new(actions.finish())
                .with_padding(
                    Padding::uniform(0.)
                        .with_top(10.)
                        .with_right(18.)
                        .with_bottom(12.)
                        .with_left(18.),
                )
                .with_background(internal_colors::fg_overlay_1(theme))
                .with_border(Border::top(1.).with_border_color(internal_colors::neutral_4(theme)))
                .finish(),
        );

        let _ = app;
        col.finish()
    }
}

fn section_label(appearance: &Appearance, text: &str) -> Box<dyn Element> {
    Container::new(
        Text::new(
            text.to_string(),
            appearance.ui_font_family(),
            appearance.ui_font_size() - 2.,
        )
        .with_color(appearance.theme().disabled_ui_text_color().into())
        .with_style(Properties::default().weight(Weight::Medium))
        .finish(),
    )
    .with_padding(Padding::uniform(0.).with_top(12.).with_right(10.).with_bottom(6.).with_left(10.))
    .finish()
}

fn group_header(appearance: &Appearance, label: &str, count: usize) -> Box<dyn Element> {
    let theme = appearance.theme();
    Container::new(
        Flex::row()
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Text::new(
                    label.to_string(),
                    appearance.ui_font_family(),
                    appearance.ui_font_size() - 2.,
                )
                .with_color(theme.disabled_ui_text_color().into())
                .finish(),
            )
            .with_child(
                Text::new(
                    count.to_string(),
                    appearance.monospace_font_family(),
                    appearance.ui_font_size() - 2.,
                )
                .with_color(theme.disabled_ui_text_color().into())
                .finish(),
            )
            .finish(),
    )
    .with_padding(Padding::uniform(0.).with_top(12.).with_right(10.).with_bottom(5.).with_left(10.))
    .finish()
}

fn rail_row<F>(
    appearance: &Appearance,
    handle: MouseStateHandle,
    active: bool,
    label: &str,
    count: usize,
    logo: Option<Box<dyn Element>>,
    glyph: Option<&str>,
    on_click: F,
) -> Box<dyn Element>
where
    F: 'static + Fn(&mut warpui::EventContext),
{
    let theme = appearance.theme();
    let label = label.to_string();
    let glyph = glyph.map(|g| g.to_string());
    Hoverable::new(handle, move |mouse| {
        let mut lead = Flex::row()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.);
        if let Some(logo) = logo {
            lead.add_child(logo);
        } else if let Some(g) = glyph.clone() {
            lead.add_child(
                ConstrainedBox::new(
                    Text::new(g, appearance.ui_font_family(), appearance.ui_font_size() - 1.)
                        .with_color(theme.disabled_ui_text_color().into())
                        .finish(),
                )
                .with_width(16.)
                .finish(),
            );
        }
        lead.add_child(
            Text::new(
                label.clone(),
                appearance.ui_font_family(),
                appearance.ui_font_size() - 1.,
            )
            .with_color(if active {
                theme.active_ui_text_color().into()
            } else {
                theme.nonactive_ui_text_color().into()
            })
            .finish(),
        );

        let mut row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Expanded::new(1., lead.finish()).finish());
        if count > 0 {
            row.add_child(
                Text::new(
                    count.to_string(),
                    appearance.monospace_font_family(),
                    appearance.ui_font_size() - 2.,
                )
                .with_color(theme.disabled_ui_text_color().into())
                .finish(),
            );
        }

        let mut chip = Container::new(row.finish())
            .with_padding(
                Padding::uniform(0.)
                    .with_top(6.)
                    .with_right(10.)
                    .with_bottom(6.)
                    .with_left(10.),
            )
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(CHIP_RADIUS)))
            .with_margin_bottom(1.);
        if active {
            chip = chip.with_background(internal_colors::fg_overlay_3(theme));
        } else if mouse.is_hovered() {
            chip = chip.with_background(internal_colors::fg_overlay_2(theme));
        }
        chip.finish()
    })
    .with_cursor(Cursor::PointingHand)
    .on_click(move |ctx, _, _| on_click(ctx))
    .finish()
}

#[derive(Clone, Copy)]
enum TagTone {
    Warn,
    Accent,
    Muted,
}

fn pill_tag(appearance: &Appearance, label: &str, tone: TagTone) -> Box<dyn Element> {
    let theme = appearance.theme();
    let (fg, bg, border) = match tone {
        TagTone::Warn => (
            theme.ui_yellow_color(),
            warp_core::ui::theme::Fill::Solid(theme.ui_yellow_color()).with_opacity(28),
            ColorU { r: 230, g: 180, b: 80, a: 90 },
        ),
        TagTone::Accent => (
            theme.accent().into_solid(),
            theme.accent().with_opacity(36),
            ColorU { r: 138, g: 169, b: 255, a: 90 },
        ),
        TagTone::Muted => (
            theme.disabled_ui_text_color().into_solid(),
            internal_colors::fg_overlay_2(theme),
            internal_colors::neutral_4(theme),
        ),
    };
    Container::new(
        Text::new(
            label.to_string(),
            appearance.ui_font_family(),
            appearance.ui_font_size() - 3.,
        )
        .with_color(fg.into())
        .with_style(Properties::default().weight(Weight::Medium))
        .finish(),
    )
    .with_padding(Padding::uniform(0.).with_top(2.).with_right(6.).with_bottom(2.).with_left(6.))
    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
    .with_background(bg)
    .with_border(Border::all(1.).with_border_color(border))
    .finish()
}

fn inset_card(appearance: &Appearance, title: &str, body: Box<dyn Element>) -> Box<dyn Element> {
    let theme = appearance.theme();
    Container::new(
        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(
                Container::new(
                    Text::new(
                        title.to_string(),
                        appearance.ui_font_family(),
                        appearance.ui_font_size() - 1.,
                    )
                    .with_color(theme.nonactive_ui_text_color().into())
                    .finish(),
                )
                .with_padding(
                    Padding::uniform(0.)
                        .with_top(8.)
                        .with_right(12.)
                        .with_bottom(8.)
                        .with_left(12.),
                )
                .with_border(Border::bottom(1.).with_border_color(internal_colors::neutral_4(theme)))
                .finish(),
            )
            .with_child(Container::new(body).with_padding(Padding::uniform(10.)).finish())
            .finish(),
    )
    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(CARD_RADIUS)))
    .with_border(Border::all(1.).with_border_color(internal_colors::neutral_4(theme)))
    .with_background(internal_colors::fg_overlay_1(theme))
    .finish()
}

fn guard_banner(appearance: &Appearance, message: &str) -> Box<dyn Element> {
    let theme = appearance.theme();
    Container::new(
        Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Start)
            .with_spacing(10.)
            .with_child(
                Text::new(String::from("⚠"), appearance.ui_font_family(), appearance.ui_font_size())
                    .with_color(theme.ui_yellow_color().into())
                    .finish(),
            )
            .with_child(
                Expanded::new(
                    1.,
                    Text::new(
                        message.to_string(),
                        appearance.ui_font_family(),
                        appearance.ui_font_size() - 1.,
                    )
                    .with_color(theme.ui_yellow_color().into())
                    .finish(),
                )
                .finish(),
            )
            .finish(),
    )
    .with_padding(Padding::uniform(11.))
    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(CARD_RADIUS)))
    .with_background(warp_core::ui::theme::Fill::Solid(theme.ui_yellow_color()).with_opacity(28))
    .with_border(Border::all(1.).with_border_color(ColorU { r: 230, g: 180, b: 80, a: 70 }))
    .finish()
}

fn empty_state(appearance: &Appearance, title: &str, hint: &str) -> Box<dyn Element> {
    let theme = appearance.theme();
    Container::new(
        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Start)
            .with_spacing(6.)
            .with_child(
                Text::new(
                    title.to_string(),
                    appearance.ui_font_family(),
                    appearance.ui_font_size() + 1.,
                )
                .with_color(theme.nonactive_ui_text_color().into())
                .with_style(Properties::default().weight(Weight::Medium))
                .finish(),
            )
            .with_child(
                Text::new(
                    hint.to_string(),
                    appearance.ui_font_family(),
                    appearance.ui_font_size() - 1.,
                )
                .with_color(theme.disabled_ui_text_color().into())
                .finish(),
            )
            .finish(),
    )
    .with_padding(Padding::uniform(16.))
    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(CARD_RADIUS)))
    .with_border(Border::all(1.).with_border_color(internal_colors::neutral_4(theme)))
    .with_background(internal_colors::fg_overlay_1(theme))
    .finish()
}

fn chrome_btn(
    appearance: &Appearance,
    handle: MouseStateHandle,
    label: &str,
    variant: ButtonVariant,
    action: AgentConfigPageAction,
) -> Box<dyn Element> {
    appearance
        .ui_builder()
        .button(variant, handle)
        .with_centered_text_label(label.to_string())
        .with_style(UiComponentStyles {
            padding: Some(Coords::uniform(7.)),
            ..Default::default()
        })
        .build()
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(action.clone());
        })
        .finish()
}

/// Same logo set as session tabs / vertical tabs (`render_cli_agent_logo`).
fn agent_logo(agent: AgentKind, appearance: &Appearance) -> Box<dyn Element> {
    let theme = appearance.theme();
    let main = theme.main_text_color(theme.background());
    let muted = theme.sub_text_color(theme.background());
    let icon = match agent_to_cli(agent) {
        Some(cli) => render_cli_agent_logo(cli, main, muted),
        None => {
            // .agents：与 SkillProvider::Agents 一致，安静默认标，不用彩色点。
            Icon::WarpLogoLight.to_warpui_icon(muted).finish()
        }
    };
    ConstrainedBox::new(icon)
        .with_width(AGENT_LOGO_SIZE)
        .with_height(AGENT_LOGO_SIZE)
        .finish()
}

fn agent_to_cli(agent: AgentKind) -> Option<CLIAgent> {
    match agent {
        AgentKind::Claude => Some(CLIAgent::Claude),
        AgentKind::Codex => Some(CLIAgent::Codex),
        AgentKind::Grok => Some(CLIAgent::Grok),
        AgentKind::Gemini => Some(CLIAgent::Gemini),
        AgentKind::Cursor => Some(CLIAgent::CursorCli),
        AgentKind::Agents => None,
    }
}

fn kind_glyph(kind: ConfigKind) -> &'static str {
    match kind {
        ConfigKind::Rules => "¶",
        ConfigKind::Hooks => "↯",
        ConfigKind::Skills => "◇",
        ConfigKind::Commands => "⌘",
        ConfigKind::Settings => "⚙",
        ConfigKind::Mcp => "⇄",
    }
}

fn short_path(path: &std::path::Path) -> String {
    let s = path.to_string_lossy();
    if let Some(home) = dirs::home_dir() {
        let hs = home.to_string_lossy();
        if let Some(rest) = s.strip_prefix(&*hs) {
            return format!("~{rest}");
        }
    }
    s.into_owned()
}


fn item_matches_query(item: &AgentConfigItem, query: &str) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return true;
    }
    let q = q.to_ascii_lowercase();
    let path = item.source_path.to_string_lossy();
    let short = short_path(&item.source_path);
    item.title.to_ascii_lowercase().contains(&q)
        || item.summary.to_ascii_lowercase().contains(&q)
        || item.pointer.to_ascii_lowercase().contains(&q)
        || item.kind.display_name().to_ascii_lowercase().contains(&q)
        || item.agent.display_name().to_ascii_lowercase().contains(&q)
        || item.scope.display_name().to_ascii_lowercase().contains(&q)
        || path.to_ascii_lowercase().contains(&q)
        || short.to_ascii_lowercase().contains(&q)
}

fn backups_touching_path(path: &std::path::Path) -> Vec<BackupManifest> {
    match agent_config::list_backups() {
        Ok(list) => list
            .into_iter()
            .filter(|m| m.files.iter().any(|f| f.original == path))
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn format_backup_time(created_unix_ms: u128) -> String {
    let secs = (created_unix_ms / 1000) as i64;
    let nsecs = ((created_unix_ms % 1000) * 1_000_000) as u32;
    match Local.timestamp_opt(secs, nsecs) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => format!("{created_unix_ms}"),
    }
}

impl SettingsPageMeta for AgentConfigPageView {
    fn section() -> SettingsSection {
        SettingsSection::AgentConfig
    }

    fn on_page_selected(&mut self, _allow_steal_focus: bool, ctx: &mut ViewContext<Self>) {
        self.reload(ctx);
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn update_filter(&mut self, query: &str, _ctx: &mut ViewContext<Self>) -> MatchData {
        if query.is_empty() {
            return MatchData::Uncounted(true);
        }
        let q = query.to_ascii_lowercase();
        let hit = self.items.iter().any(|i| {
            i.title.to_ascii_lowercase().contains(&q)
                || i.summary.to_ascii_lowercase().contains(&q)
                || i.agent.display_name().to_ascii_lowercase().contains(&q)
                || i.kind.display_name().to_ascii_lowercase().contains(&q)
                || i.source_path
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .contains(&q)
        }) || "配置管理".contains(query)
            || "agent config".contains(&q);
        MatchData::Uncounted(hit)
    }

    fn scroll_to_widget(&mut self, _widget_id: &'static str) {}
    fn clear_highlighted_widget(&mut self) {}
}

impl From<ViewHandle<AgentConfigPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<AgentConfigPageView>) -> Self {
        SettingsPageViewHandle::AgentConfig(view_handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::agent_config::{BackupFileEntry, ConfigScope, DocumentFormat, ManagedBy};
    use std::path::PathBuf;

    fn sample_item(title: &str, path: &str, kind: ConfigKind) -> AgentConfigItem {
        AgentConfigItem {
            id: title.into(),
            agent: AgentKind::Claude,
            kind,
            scope: ConfigScope::User,
            title: title.into(),
            source_path: PathBuf::from(path),
            pointer: "hooks.Stop".into(),
            format: DocumentFormat::Json,
            executable: true,
            managed_by: ManagedBy::None,
            project_tracked: false,
            needs_codex_retrust: false,
            summary: "zap-agent-status".into(),
            read_only: false,
        }
    }

    #[test]
    fn search_matches_name_path_and_kind() {
        let item = sample_item("PostToolUse", "/Users/x/.claude/settings.json", ConfigKind::Hooks);
        assert!(item_matches_query(&item, "post"));
        assert!(item_matches_query(&item, "settings.json"));
        assert!(item_matches_query(&item, "hooks"));
        assert!(item_matches_query(&item, "zap-agent"));
        assert!(!item_matches_query(&item, "gemini"));
        assert!(item_matches_query(&item, "  "));
    }

    #[test]
    fn backup_path_filter_keeps_matching_files() {
        let path = PathBuf::from("/tmp/a.json");
        let hit = BackupManifest {
            id: "1".into(),
            created_unix_ms: 1,
            note: "edit".into(),
            files: vec![BackupFileEntry {
                original: path.clone(),
                stored_as: "a.json".into(),
            }],
        };
        let miss = BackupManifest {
            id: "2".into(),
            created_unix_ms: 2,
            note: "other".into(),
            files: vec![BackupFileEntry {
                original: PathBuf::from("/tmp/b.json"),
                stored_as: "b.json".into(),
            }],
        };
        let filtered: Vec<_> = [hit, miss]
            .into_iter()
            .filter(|m| m.files.iter().any(|f| f.original == path))
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "1");
    }
}
