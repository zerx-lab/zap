use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ai::skills::SkillProvider;
use warp_core::ui::appearance::Appearance;
use warp_core::ui::color::CLAUDE_ORANGE;
use warp_core::ui::icons::Icon as WarpIcon;
use warp_core::ui::theme::color::internal_colors;
use warp_core::ui::theme::WarpTheme;
use instant::Instant;
use warpui::{
    clipboard::ClipboardContent,
    color::ColorU,
    elements::{
        Border, ChildAnchor, ChildView, Clipped, ClippedScrollStateHandle, ClippedScrollable,
        Align, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Element, Empty, Expanded,
        Fill as ElementFill, Flex, Hoverable, MainAxisSize, MouseStateHandle, OffsetPositioning,
        Padding, ParentAnchor, ParentElement, ParentOffsetBounds, Radius, SavePosition,
        ScrollTarget, ScrollToPositionMode, ScrollbarWidth, Shrinkable, Stack, Text, Wrap,
    },
    geometry::vector::vec2f,
    platform::Cursor,
    r#async::Timer,
    text_layout::ClipConfig,
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use crate::ai::skills::{
    SkillInventoryDuplicate, SkillInventoryItem, SkillManager, SkillManagerEvent,
};
use crate::editor::{
    EnterAction, EnterSettings, EditorOptions, EditorView, Event as EditorEvent,
    PropagateAndNoOpNavigationKeys, PropagateHorizontalNavigationKeys, TextOptions,
};
use crate::settings_view::SettingsSection;
use crate::skill_manager::generate::{
    build_create_skill_user_query, can_advance, create_skill_symlinks, decide_create_skill_launch,
    display_skill_md_path, draft_skill_md_path, find_name_collision,
    is_symlink_disabled, is_valid_skill_slug, planned_symlink_targets, slugify_skill_name,
    symlink_candidate_providers, target_path_exists, CreateSkillLaunchMode, SkillGenerateDraft,
    SkillGenerateIntentKind, SkillGenerateLocation, SkillGeneratePhase, SkillGenerateStep,
};
use crate::view_components::copyable_text_field::COPY_FEEDBACK_DURATION;
use crate::view_components::dropdown::{Dropdown, DropdownItem};
use crate::workspace::WorkspaceAction;

const PANEL_PADDING: f32 = 8.0;
const ROW_PADDING_VERTICAL: f32 = 5.0;
const ROW_PADDING_HORIZONTAL: f32 = 8.0;
// filter tab:保持紧凑高度,用 12px 字号 + 强对比激活态撑起可读性,
// 避免被外壳 ClippedScrollable 拦截点击事件。
const FILTER_BUTTON_HEIGHT: f32 = 24.0;
const FILTER_BUTTON_HORIZONTAL_PADDING: f32 = 8.0;
const FILTER_BUTTON_CORNER_RADIUS: f32 = 4.0;
// 来源 tab 平铺显示的最大数量;超过则折叠为「全部 + 来源下拉」。
// 取 4:典型侧栏宽度下「全部」加约 4 个短来源名仍可平铺。布局引擎是单遍保留树,
// 无法按实测像素宽度自适应折叠,故以来源数量作为折叠阈值。
const FILTER_TABS_INLINE_MAX: usize = 4;
const SOURCE_TAG_HEIGHT: f32 = 20.0;
const SOURCE_TAG_PIP: f32 = 6.0;
const SOURCE_TAG_CHECK: f32 = 10.0;

#[derive(Clone, Debug)]
pub enum SkillManagerPanelAction {
    SelectProviderFilter(Option<SkillProvider>),
    EditSkill(PathBuf),
    /// 点来源标签. `open` 为 true 时同时打开该文件.
    PinSkillSource {
        name: String,
        path: PathBuf,
        open: bool,
    },
    OpenGenerateWizard,
    CloseGenerateWizard,
    GenerateSetIntentKind(SkillGenerateIntentKind),
    GenerateSetLocation(SkillGenerateLocation),
    GenerateToggleSymlink(SkillProvider),
    GenerateFocusCustomPath,
    GeneratePrevStep,
    GenerateNextStep,
    GenerateConfirmOverwriteAndLaunch,
    GenerateCancelOverwrite,
    GenerateOpenDraftedSkill,
    GenerateConfirmOverwriteAndOpen,
    GenerateJumpToDrafting,
    /// 点击目标/触发/命名输入框容器时聚焦对应编辑器。
    GenerateFocusGoal,
    GenerateFocusTrigger,
    GenerateFocusName,
    /// 空目标时点示例：填入目标编辑器。
    GenerateApplyExample(String),
    /// CLI 兜底：复制 create-skill Prompt 到剪贴板。
    GenerateCopyCliPrompt,
    /// CLI 兜底：回 Launch 收集步。
    GenerateBackFromCliFallback,
    /// CLI 兜底：进入审阅 / 打开 SKILL.md 路径。
    GenerateOpenFromCliFallback,
    /// CLI 兜底（可选）：打开设置 → Agent Providers。
    GenerateOpenByopSettings,
}

#[derive(Clone, Debug)]
pub enum SkillManagerPanelEvent {
    OpenSkillFile { path: PathBuf },
}

pub struct SkillManagerPanel {
    selected_path: Option<PathBuf>,
    /// 每个 skill 名字当前要打开的那一份. 缺省时用生效的那份.
    pinned_paths: HashMap<String, PathBuf>,
    provider_filter: Option<SkillProvider>,
    query_editor: ViewHandle<EditorView>,
    // 折叠态(来源数 > FILTER_TABS_INLINE_MAX)收纳各来源 tab 的「来源」下拉。
    source_dropdown: ViewHandle<Dropdown<SkillManagerPanelAction>>,
    filter_mouse_states: RefCell<HashMap<Option<SkillProvider>, MouseStateHandle>>,
    row_mouse_states: RefCell<HashMap<PathBuf, MouseStateHandle>>,
    tag_mouse_states: RefCell<HashMap<PathBuf, MouseStateHandle>>,
    list_scroll_state: ClippedScrollStateHandle,
    /// 当前活动仓库根（由 WorkspaceView 同步），Agents 落点需要。
    generate_repo_root: Option<PathBuf>,
    generate_phase: SkillGeneratePhase,
    generate_draft: SkillGenerateDraft,
    generate_goal_editor: ViewHandle<EditorView>,
    generate_trigger_editor: ViewHandle<EditorView>,
    generate_name_editor: ViewHandle<EditorView>,
    generate_custom_path_editor: ViewHandle<EditorView>,
    generate_button_mouse: MouseStateHandle,
    generate_cta_mouse: MouseStateHandle,
    generate_nav_mouse: RefCell<HashMap<&'static str, MouseStateHandle>>,
    generate_choice_mouse: RefCell<HashMap<String, MouseStateHandle>>,
    /// CLI 兜底页「复制 Prompt」后的轻反馈时间。
    generate_cli_copied_at: Option<Instant>,
}

impl SkillManagerPanel {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
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
            editor.set_placeholder_text(crate::t!("skill-manager-search-placeholder"), ctx);
            editor
        });

        ctx.subscribe_to_view(&query_editor, |me, _, event, ctx| {
            if matches!(
                event,
                EditorEvent::Edited(_)
                    | EditorEvent::BufferReplaced
                    | EditorEvent::BufferReinitialized
            ) {
                me.scroll_selected_path_into_view_with_ctx(ctx);
                ctx.notify();
            }
        });

        ctx.subscribe_to_model(&Appearance::handle(ctx), |_, _, _, ctx| {
            ctx.notify();
        });

        let skill_manager = SkillManager::handle(ctx);
        ctx.subscribe_to_model(&skill_manager, |me, _manager, event, ctx| match event {
            SkillManagerEvent::InventoryChanged => {
                let inventory = SkillManager::as_ref(ctx).list_skill_inventory(ctx);
                if me.provider_filter.is_some_and(|provider| {
                    !Self::providers_in_inventory(&inventory).contains(&provider)
                }) {
                    me.provider_filter = None;
                }
                let query = me.query(ctx);
                let items = Self::filter_inventory(&inventory, &query, me.provider_filter);
                me.scroll_selected_path_into_view(&items);
                // 来源集合随 inventory 变化,重建折叠态下拉的选项与选中态。
                me.rebuild_source_dropdown(ctx);
                ctx.notify();
            }
        });

        // 折叠态使用的「来源」下拉。由本面板的 ViewContext 创建,其派发的
        // SelectProviderFilter 会冒泡回本面板的 handle_action。
        let source_dropdown = ctx.add_typed_action_view(|ctx| {
            let mut dropdown = Dropdown::new(ctx);
            // 下拉头紧贴文本宽度,与左侧「全部」按钮并排,不撑满整行。
            dropdown.set_main_axis_size(MainAxisSize::Min, ctx);
            // 选中「全部」项时,闭合态显示「来源」占位,避免与左侧独立「全部」按钮重复。
            let all_label = crate::t!("skill-manager-filter-all").to_string();
            let source_label = crate::t!("skill-manager-filter-provider").to_string();
            dropdown.set_menu_header_text_override(move |label| {
                if label == all_label {
                    source_label.clone()
                } else {
                    label.to_string()
                }
            });
            dropdown
        });

        let generate_goal_editor = Self::make_multiline_editor(
            crate::t!("skill-generate-goal-placeholder").as_str(),
            ctx,
        );
        let generate_trigger_editor = Self::make_multiline_editor(
            crate::t!("skill-generate-trigger-placeholder").as_str(),
            ctx,
        );
        let generate_name_editor = Self::make_single_line_editor(
            crate::t!("skill-generate-name-placeholder").as_str(),
            ctx,
        );
        let generate_custom_path_editor = Self::make_single_line_editor(
            crate::t!("skill-generate-location-custom-placeholder").as_str(),
            ctx,
        );

        for editor in [
            &generate_goal_editor,
            &generate_trigger_editor,
            &generate_name_editor,
            &generate_custom_path_editor,
        ] {
            ctx.subscribe_to_view(editor, |me, _, event, ctx| {
                match event {
                    EditorEvent::Edited(_)
                    | EditorEvent::BufferReplaced
                    | EditorEvent::BufferReinitialized => {
                        me.sync_generate_draft_from_editors(ctx);
                        ctx.notify();
                    }
                    // 单行 Enter / 多行 Cmd·Ctrl-Enter：可前进时推进向导。
                    EditorEvent::Enter
                    | EditorEvent::CmdEnter
                    | EditorEvent::CtrlEnter => {
                        me.try_advance_generate_from_submit(ctx);
                    }
                    _ => {}
                }
            });
        }

        let mut me = Self {
            selected_path: None,
            pinned_paths: HashMap::new(),
            provider_filter: None,
            query_editor,
            source_dropdown,
            filter_mouse_states: RefCell::new(HashMap::new()),
            row_mouse_states: RefCell::new(HashMap::new()),
            tag_mouse_states: RefCell::new(HashMap::new()),
            list_scroll_state: ClippedScrollStateHandle::default(),
            generate_repo_root: None,
            generate_phase: SkillGeneratePhase::Closed,
            generate_draft: SkillGenerateDraft::default(),
            generate_goal_editor,
            generate_trigger_editor,
            generate_name_editor,
            generate_custom_path_editor,
            generate_button_mouse: MouseStateHandle::default(),
            generate_cta_mouse: MouseStateHandle::default(),
            generate_nav_mouse: RefCell::new(HashMap::new()),
            generate_choice_mouse: RefCell::new(HashMap::new()),
            generate_cli_copied_at: None,
        };
        me.rebuild_source_dropdown(ctx);
        me
    }

    fn make_single_line_editor(
        placeholder: &str,
        ctx: &mut ViewContext<Self>,
    ) -> ViewHandle<EditorView> {
        let placeholder = placeholder.to_string();
        ctx.add_typed_action_view(move |ctx| {
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
            editor.set_placeholder_text(placeholder.clone(), ctx);
            editor
        })
    }

    fn make_multiline_editor(
        placeholder: &str,
        ctx: &mut ViewContext<Self>,
    ) -> ViewHandle<EditorView> {
        let placeholder = placeholder.to_string();
        ctx.add_typed_action_view(move |ctx| {
            let options = EditorOptions {
                text: TextOptions::ui_text(
                    Some(Appearance::as_ref(ctx).ui_font_subheading()),
                    Appearance::as_ref(ctx),
                ),
                propagate_and_no_op_vertical_navigation_keys:
                    PropagateAndNoOpNavigationKeys::AtBoundary,
                propagate_horizontal_navigation_keys: PropagateHorizontalNavigationKeys::Always,
                single_line: false,
                clear_selections_on_blur: true,
                soft_wrap: true,
                // Enter 换行；Cmd/Ctrl-Enter 交给父级推进向导。
                enter_settings: EnterSettings {
                    enter: EnterAction::InsertNewLineIfMultiLine,
                    shift_enter: EnterAction::InsertNewLineIfMultiLine,
                    alt_enter: EnterAction::InsertNewLineIfMultiLine,
                    ctrl_enter: EnterAction::Emit,
                },
                ..Default::default()
            };
            let mut editor = EditorView::new(options, ctx);
            editor.set_placeholder_text(placeholder.clone(), ctx);
            editor
        })
    }

    /// Workspace 同步当前活动仓库根，供 Agents 落点解析。
    pub fn set_generate_repo_root(&mut self, repo_root: Option<PathBuf>, ctx: &mut ViewContext<Self>) {
        if self.generate_repo_root == repo_root {
            return;
        }
        self.generate_repo_root = repo_root;
        ctx.notify();
    }

    fn sync_generate_draft_from_editors(&mut self, ctx: &AppContext) {
        self.generate_draft.goal = self
            .generate_goal_editor
            .as_ref(ctx)
            .buffer_text(ctx)
            .trim()
            .to_string();
        self.generate_draft.trigger_description = self
            .generate_trigger_editor
            .as_ref(ctx)
            .buffer_text(ctx)
            .trim()
            .to_string();
        let raw_name = self.generate_name_editor.as_ref(ctx).buffer_text(ctx);
        self.generate_draft.name = slugify_skill_name(&raw_name);
        self.generate_draft.custom_path = self
            .generate_custom_path_editor
            .as_ref(ctx)
            .buffer_text(ctx)
            .trim()
            .to_string();
    }

    /// 编辑器提交键：同步草稿后，若当前步可前进则派发下一步。
    fn try_advance_generate_from_submit(&mut self, ctx: &mut ViewContext<Self>) {
        self.sync_generate_draft_from_editors(ctx);
        let SkillGeneratePhase::Collecting { step, .. } = self.generate_phase else {
            return;
        };
        if can_advance(step, &self.generate_draft, self.generate_repo_root.as_deref()) {
            ctx.dispatch_typed_action(&SkillManagerPanelAction::GenerateNextStep);
        }
    }

    fn apply_generate_goal_example(&mut self, text: &str, ctx: &mut ViewContext<Self>) {
        self.generate_goal_editor.update(ctx, |editor, ctx| {
            editor.set_buffer_text(text, ctx);
        });
        self.sync_generate_draft_from_editors(ctx);
        ctx.focus(&self.generate_goal_editor);
        ctx.notify();
    }

    fn generate_nav_mouse_for(&self, key: &'static str) -> MouseStateHandle {
        self.generate_nav_mouse
            .borrow_mut()
            .entry(key)
            .or_default()
            .clone()
    }

    fn generate_choice_mouse_for(&self, key: impl Into<String>) -> MouseStateHandle {
        self.generate_choice_mouse
            .borrow_mut()
            .entry(key.into())
            .or_default()
            .clone()
    }

    fn open_generate_wizard(&mut self, ctx: &mut ViewContext<Self>) {
        self.generate_draft = SkillGenerateDraft::default();
        for editor in [
            &self.generate_goal_editor,
            &self.generate_trigger_editor,
            &self.generate_name_editor,
            &self.generate_custom_path_editor,
        ] {
            editor.update(ctx, |editor, ctx| {
                editor.clear_buffer_and_reset_undo_stack(ctx);
            });
        }
        self.generate_phase = SkillGeneratePhase::Collecting {
            step: SkillGenerateStep::Intent,
            awaiting_overwrite_confirm: false,
        };
        ctx.focus(&self.generate_goal_editor);
        ctx.notify();
    }

    fn close_generate_wizard(&mut self, ctx: &mut ViewContext<Self>) {
        self.generate_phase = SkillGeneratePhase::Closed;
        ctx.notify();
    }

    /// 收集步骤里把焦点放到对应编辑器；Location/Launch 无编辑器则不动。
    fn focus_generate_collecting_editor(
        &self,
        step: SkillGenerateStep,
        ctx: &mut ViewContext<Self>,
    ) {
        match step {
            SkillGenerateStep::Intent => ctx.focus(&self.generate_goal_editor),
            SkillGenerateStep::Trigger => ctx.focus(&self.generate_trigger_editor),
            SkillGenerateStep::Name => ctx.focus(&self.generate_name_editor),
            SkillGenerateStep::Location => {
                if self.generate_draft.location == SkillGenerateLocation::Custom {
                    ctx.focus(&self.generate_custom_path_editor);
                }
            }
            SkillGenerateStep::Launch => {}
        }
    }

    fn launch_create_skill(&mut self, ctx: &mut ViewContext<Self>) {
        self.sync_generate_draft_from_editors(ctx);
        let Some(path) = draft_skill_md_path(
            &self.generate_draft,
            self.generate_repo_root.as_deref(),
        ) else {
            return;
        };
        if target_path_exists(&path) && !self.generate_draft.allow_overwrite {
            if let SkillGeneratePhase::Collecting { step, .. } = self.generate_phase {
                self.generate_phase = SkillGeneratePhase::Collecting {
                    step,
                    awaiting_overwrite_confirm: true,
                };
                ctx.notify();
                return;
            }
        }
        match decide_create_skill_launch(
            crate::ai::agent_providers::has_configured_byop_provider(ctx),
        ) {
            CreateSkillLaunchMode::ZapAgent => {
                let query = build_create_skill_user_query(&self.generate_draft, &path);
                ctx.dispatch_typed_action(&WorkspaceAction::CreateSkillWithOz { user_query: query });
                self.generate_phase = SkillGeneratePhase::Drafting;
            }
            CreateSkillLaunchMode::CliFallback => {
                self.generate_cli_copied_at = None;
                self.generate_phase = SkillGeneratePhase::CliFallback;
            }
        }
        ctx.notify();
    }

    fn current_create_skill_prompt(&self) -> Option<String> {
        let path = draft_skill_md_path(
            &self.generate_draft,
            self.generate_repo_root.as_deref(),
        )?;
        Some(build_create_skill_user_query(&self.generate_draft, &path))
    }

    fn copy_cli_prompt(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(prompt) = self.current_create_skill_prompt() else {
            return;
        };
        ctx.clipboard()
            .write(ClipboardContent::plain_text(prompt));
        self.generate_cli_copied_at = Some(Instant::now());
        let duration = COPY_FEEDBACK_DURATION;
        ctx.spawn(
            async move {
                Timer::after(duration).await;
            },
            move |me, _, ctx| {
                if me
                    .generate_cli_copied_at
                    .is_some_and(|t| t.elapsed() >= duration)
                {
                    me.generate_cli_copied_at = None;
                }
                ctx.notify();
            },
        );
        ctx.notify();
    }

    fn enter_review_phase(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(path) = draft_skill_md_path(
            &self.generate_draft,
            self.generate_repo_root.as_deref(),
        ) else {
            return;
        };
        let exists = target_path_exists(&path);
        self.generate_phase = SkillGeneratePhase::Review {
            path,
            path_exists: exists,
            awaiting_overwrite_confirm: false,
        };
        ctx.notify();
    }

    /// 重建「来源」下拉的选项与选中态。
    ///
    /// 首项「全部」对应清空过滤(index 0),其余项依次对应当前 inventory 中存在的
    /// 各 provider。选中态由 `provider_filter` 驱动,与左侧「全部」按钮保持一致。
    fn rebuild_source_dropdown(&mut self, ctx: &mut ViewContext<Self>) {
        let inventory = SkillManager::as_ref(ctx).list_skill_inventory(ctx);
        let providers = Self::providers_in_inventory(&inventory);

        let mut items = vec![DropdownItem::new(
            crate::t!("skill-manager-filter-all"),
            SkillManagerPanelAction::SelectProviderFilter(None),
        )];
        for provider in &providers {
            items.push(DropdownItem::new(
                provider.to_string(),
                SkillManagerPanelAction::SelectProviderFilter(Some(*provider)),
            ));
        }

        // 首项「全部」占据 index 0,故命中的 provider 需 +1。
        let selected_index = match self.provider_filter {
            None => 0,
            Some(filter) => providers
                .iter()
                .position(|provider| *provider == filter)
                .map_or(0, |position| position + 1),
        };

        self.source_dropdown.update(ctx, |dropdown, ctx| {
            dropdown.set_items(items, ctx);
            dropdown.set_selected_by_index(selected_index, ctx);
        });
    }

    fn query(&self, app: &AppContext) -> String {
        self.query_editor
            .as_ref(app)
            .buffer_text(app)
            .trim()
            .to_lowercase()
    }

    fn providers_in_inventory(inventory: &[SkillInventoryItem]) -> Vec<SkillProvider> {
        let mut providers = inventory
            .iter()
            .flat_map(|item| item.duplicates.iter().map(|duplicate| duplicate.provider))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        providers.sort_by_key(|provider| provider.to_string());
        providers
    }

    fn filter_inventory(
        inventory: &[SkillInventoryItem],
        query: &str,
        provider_filter: Option<SkillProvider>,
    ) -> Vec<SkillInventoryItem> {
        inventory
            .iter()
            .filter_map(|item| {
                let duplicates = item
                    .duplicates
                    .iter()
                    .filter(|duplicate| {
                        provider_filter.is_none_or(|provider| duplicate.provider == provider)
                            && (query.is_empty()
                                || duplicate.name.to_lowercase().contains(query)
                                || duplicate.description.to_lowercase().contains(query)
                                || duplicate
                                    .path
                                    .display()
                                    .to_string()
                                    .to_lowercase()
                                    .contains(query))
                    })
                    .cloned()
                    .collect::<Vec<_>>();

                if duplicates.is_empty() {
                    return None;
                }
                // 过滤后仍保留真正生效的那份, 避免「默认」被换成剩余列表的第一项.
                let default_skill = duplicates
                    .iter()
                    .find(|duplicate| duplicate.path == item.default_skill.path)
                    .cloned()
                    .unwrap_or_else(|| duplicates[0].clone());
                Some(SkillInventoryItem {
                    name: item.name.clone(),
                    default_skill,
                    duplicates,
                })
            })
            .collect()
    }

    /// 悬停中的来源优先, 其次是点选记住的那份, 最后是生效的那份.
    fn resolve_shown_duplicate<'a>(
        item: &'a SkillInventoryItem,
        pinned_path: Option<&Path>,
        hovered_path: Option<&Path>,
    ) -> &'a SkillInventoryDuplicate {
        if let Some(path) = hovered_path {
            if let Some(duplicate) = item
                .duplicates
                .iter()
                .find(|duplicate| duplicate.path.as_path() == path)
            {
                return duplicate;
            }
        }
        if let Some(path) = pinned_path {
            if let Some(duplicate) = item
                .duplicates
                .iter()
                .find(|duplicate| duplicate.path.as_path() == path)
            {
                return duplicate;
            }
        }
        item.duplicates
            .iter()
            .find(|duplicate| duplicate.path == item.default_skill.path)
            .unwrap_or(&item.duplicates[0])
    }

    fn source_content_differs(
        duplicate: &SkillInventoryDuplicate,
        default_skill: &SkillInventoryDuplicate,
    ) -> bool {
        duplicate.path != default_skill.path && duplicate.content != default_skill.content
    }

    fn provider_pip_color(provider: SkillProvider, theme: &WarpTheme) -> ColorU {
        match provider {
            SkillProvider::Agents | SkillProvider::Github => {
                theme.main_text_color(theme.background()).into_solid()
            }
            SkillProvider::Zap => theme.accent().into_solid(),
            SkillProvider::Claude => CLAUDE_ORANGE,
            SkillProvider::Codex => theme.ansi_fg_green(),
            SkillProvider::Cursor => theme.ansi_fg_cyan(),
            SkillProvider::Gemini => theme.ansi_fg_blue(),
            SkillProvider::Copilot => theme.ansi_fg_magenta(),
            SkillProvider::Droid => theme.ansi_fg_yellow(),
            SkillProvider::OpenCode => theme.ansi_fg_red(),
            SkillProvider::Grok => ColorU::from_u32(0xffffffff),
        }
    }

    fn skill_row_position_id(name: &str) -> String {
        format!("skill-manager-row:{name}")
    }

    fn scroll_selected_path_into_view(&self, items: &[SkillInventoryItem]) {
        let Some(path) = self.selected_path.as_deref() else {
            return;
        };
        let Some(item) = items.iter().find(|item| {
            item.duplicates
                .iter()
                .any(|duplicate| duplicate.path.as_path() == path)
        }) else {
            return;
        };

        self.list_scroll_state.scroll_to_position(ScrollTarget {
            position_id: Self::skill_row_position_id(&item.name),
            mode: ScrollToPositionMode::FullyIntoView,
        });
    }

    fn scroll_selected_path_into_view_with_ctx(&self, ctx: &AppContext) {
        let inventory = SkillManager::as_ref(ctx).list_skill_inventory(ctx);
        let query = self.query(ctx);
        let items = Self::filter_inventory(&inventory, &query, self.provider_filter);
        self.scroll_selected_path_into_view(&items);
    }

    fn filter_mouse_state_for(&self, provider: Option<SkillProvider>) -> MouseStateHandle {
        self.filter_mouse_states
            .borrow_mut()
            .entry(provider)
            .or_default()
            .clone()
    }

    fn row_mouse_state_for(&self, path: &Path) -> MouseStateHandle {
        self.row_mouse_states
            .borrow_mut()
            .entry(path.to_path_buf())
            .or_default()
            .clone()
    }

    fn tag_mouse_state_for(&self, path: &Path) -> MouseStateHandle {
        self.tag_mouse_states
            .borrow_mut()
            .entry(path.to_path_buf())
            .or_default()
            .clone()
    }

    fn hovered_tag_path(&self, item: &SkillInventoryItem) -> Option<PathBuf> {
        for duplicate in &item.duplicates {
            let hovered = self
                .tag_mouse_states
                .borrow()
                .get(&duplicate.path)
                .and_then(|state| state.lock().ok().map(|mouse| mouse.is_hovered()))
                .unwrap_or(false);
            if hovered {
                return Some(duplicate.path.clone());
            }
        }
        None
    }

    fn render_dot(color: ColorU, diameter: f32) -> Box<dyn Element> {
        Container::new(
            ConstrainedBox::new(Empty::new().finish())
                .with_width(diameter)
                .with_height(diameter)
                .finish(),
        )
        .with_corner_radius(CornerRadius::with_all(Radius::Percentage(50.)))
        .with_background_color(color)
        .finish()
    }

    fn with_diff_dot(tag: Box<dyn Element>, color: ColorU) -> Box<dyn Element> {
        Stack::new()
            .with_child(tag)
            .with_positioned_child(
                Self::render_dot(color, SOURCE_TAG_PIP),
                OffsetPositioning::offset_from_parent(
                    vec2f(SOURCE_TAG_PIP / 2.0, -(SOURCE_TAG_PIP / 2.0)),
                    ParentOffsetBounds::Unbounded,
                    ParentAnchor::TopRight,
                    ChildAnchor::TopRight,
                ),
            )
            .finish()
    }

    fn render_label(
        text: impl Into<String>,
        appearance: &Appearance,
        font_size: f32,
        color: impl Into<pathfinder_color::ColorU>,
    ) -> Box<dyn Element> {
        Text::new_inline(text.into(), appearance.ui_font_family(), font_size)
            .with_color(color.into())
            .with_clip(ClipConfig::ellipsis())
            .finish()
    }

    /// Multi-line soft-wrapped label for instructional / help copy.
    /// Unlike [`Self::render_label`], does not ellipsis-truncate to a single line.
    fn render_label_wrap(
        text: impl Into<String>,
        appearance: &Appearance,
        font_size: f32,
        color: impl Into<pathfinder_color::ColorU>,
    ) -> Box<dyn Element> {
        Text::new(text.into(), appearance.ui_font_family(), font_size)
            .with_color(color.into())
            .soft_wrap(true)
            .finish()
    }

    fn render_filter_button(
        &self,
        label: String,
        is_active: bool,
        provider: Option<SkillProvider>,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let text_color = if is_active {
            theme.main_text_color(theme.background())
        } else {
            theme.sub_text_color(theme.background())
        };
        let state = self.filter_mouse_state_for(provider);
        let action = SkillManagerPanelAction::SelectProviderFilter(provider);

        Hoverable::new(state, move |mouse| {
            // 文字用 Flex::row + CrossAxisAlignment::Center 垂直居中于固定行高里。
            let label_row = Flex::row()
                .with_main_axis_size(MainAxisSize::Min)
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(Self::render_label(
                    label.clone(),
                    appearance,
                    appearance.ui_font_body(),
                    text_color,
                ))
                .finish();
            let mut button = Container::new(label_row)
                .with_padding_left(FILTER_BUTTON_HORIZONTAL_PADDING)
                .with_padding_right(FILTER_BUTTON_HORIZONTAL_PADDING)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(
                    FILTER_BUTTON_CORNER_RADIUS,
                )));
            if is_active {
                // 激活态使用 fg_overlay_4 让对比更强,避免快速切换时看不出选中变化。
                button = button.with_background(internal_colors::fg_overlay_4(theme));
            } else if mouse.is_hovered() {
                button = button.with_background(internal_colors::fg_overlay_2(theme));
            }
            ConstrainedBox::new(button.finish())
                .with_height(FILTER_BUTTON_HEIGHT)
                .finish()
        })
        .with_cursor(Cursor::PointingHand)
        .on_mouse_down(move |ctx, _, _| {
            ctx.dispatch_typed_action(action.clone());
        })
        .finish()
    }

    fn render_filter_rows(
        &self,
        providers: &[SkillProvider],
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        // 来源过多时折叠为「全部 + 来源下拉」,避免窄侧栏放不下被裁断。
        if providers.len() > FILTER_TABS_INLINE_MAX {
            return self.render_collapsed_filter_rows(appearance);
        }

        let active_filter = self.provider_filter;

        // 使用 MainAxisSize::Max 让过滤标签行填满面板宽度,消除右侧留白。
        let mut filter_buttons = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(4.0)
            .with_child(self.render_filter_button(
                crate::t!("skill-manager-filter-all").into(),
                active_filter.is_none(),
                None,
                appearance,
            ));

        for provider in providers {
            filter_buttons.add_child(self.render_filter_button(
                provider.to_string(),
                active_filter == Some(*provider),
                Some(*provider),
                appearance,
            ));
        }

        // 不再外套 ClippedScrollable / 外壳容器:
        // 1. 横向滚动容器会拦截 mouse-down 用于拖拽判定,导致快速切换 tab 时点击响应延迟。
        // 2. 平铺态 provider 数 ≤ FILTER_TABS_INLINE_MAX,侧栏宽度可容纳;
        //    更多来源走折叠下拉分支。
        Clipped::new(filter_buttons.finish()).finish()
    }

    /// 折叠态过滤行:左侧「全部」快捷按钮 + 右侧「来源 ▾」下拉。
    ///
    /// 绝不外套 `Clipped`:下拉展开的菜单是定位 overlay,被裁剪会丢失。
    fn render_collapsed_filter_rows(&self, appearance: &Appearance) -> Box<dyn Element> {
        Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(4.0)
            .with_child(self.render_filter_button(
                crate::t!("skill-manager-filter-all").into(),
                self.provider_filter.is_none(),
                None,
                appearance,
            ))
            .with_child(ChildView::new(&self.source_dropdown).finish())
            .finish()
    }

    fn render_search_input(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        // 不再内嵌 Shrinkable：父级 Flex（搜索行 Expanded / 旧的整行）会给出有限宽度。
        // Shrinkable 只有作为 Flex 直接子节点时才有意义；包在 Container 里会丢失 flex parent_data。
        Container::new(Clipped::new(ChildView::new(&self.query_editor).finish()).finish())
            .with_padding(Padding::uniform(6.0).with_left(12.0).with_right(12.0))
            .with_border(Border::all(1.0).with_border_fill(theme.surface_3()))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)))
            .finish()
    }

    fn render_source_tag(
        &self,
        item: &SkillInventoryItem,
        duplicate: &SkillInventoryDuplicate,
        pinned_path: &Path,
        any_tag_hovered: bool,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let tag_path = duplicate.path.clone();
        let pinned_path = pinned_path.to_path_buf();
        let skill_name = item.name.clone();
        let label = duplicate.provider.to_string();
        let is_effective = duplicate.path == item.default_skill.path;
        let content_differs = Self::source_content_differs(duplicate, &item.default_skill);
        let pip_color = Self::provider_pip_color(duplicate.provider, theme);
        let diff_color = theme.ui_yellow_color();
        let state = self.tag_mouse_state_for(&duplicate.path);

        let tag = Hoverable::new(state, move |mouse| {
            let is_shown = if any_tag_hovered {
                mouse.is_hovered()
            } else {
                tag_path == pinned_path
            };
            let text_color = if is_shown {
                theme.main_text_color(theme.background())
            } else {
                theme.sub_text_color(theme.background())
            };
            let mut row = Flex::row()
                .with_main_axis_size(MainAxisSize::Min)
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_spacing(4.0)
                .with_child(Self::render_dot(pip_color, SOURCE_TAG_PIP))
                .with_child(Self::render_label(
                    label.clone(),
                    appearance,
                    appearance.ui_font_footnote(),
                    text_color,
                ));
            if is_effective {
                row.add_child(
                    ConstrainedBox::new(WarpIcon::Check.to_warpui_icon(text_color).finish())
                        .with_width(SOURCE_TAG_CHECK)
                        .with_height(SOURCE_TAG_CHECK)
                        .finish(),
                );
            }
            let mut tag = Container::new(row.finish())
                .with_padding_left(6.0)
                .with_padding_right(6.0)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)));
            if is_shown {
                tag = tag.with_background(internal_colors::fg_overlay_3(theme));
            } else {
                tag = tag.with_border(Border::all(1.0).with_border_fill(theme.surface_3()));
            }
            let tag = ConstrainedBox::new(tag.finish())
                .with_height(SOURCE_TAG_HEIGHT)
                .finish();
            if content_differs {
                Self::with_diff_dot(tag, diff_color)
            } else {
                tag
            }
        })
        .with_cursor(Cursor::PointingHand)
        .on_click({
            let name = skill_name.clone();
            let path = duplicate.path.clone();
            move |ctx, _, _| {
                ctx.dispatch_typed_action(SkillManagerPanelAction::PinSkillSource {
                    name: name.clone(),
                    path: path.clone(),
                    open: false,
                });
            }
        })
        .on_double_click({
            let path = duplicate.path.clone();
            move |ctx, _, _| {
                ctx.dispatch_typed_action(SkillManagerPanelAction::PinSkillSource {
                    name: skill_name.clone(),
                    path: path.clone(),
                    open: true,
                });
            }
        })
        .finish();
        tag
    }

    fn render_skill_row(
        &self,
        item: &SkillInventoryItem,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let pinned = Self::resolve_shown_duplicate(
            item,
            self.pinned_paths.get(&item.name).map(PathBuf::as_path),
            None,
        );
        let hovered_path = self.hovered_tag_path(item);
        let shown = Self::resolve_shown_duplicate(
            item,
            Some(pinned.path.as_path()),
            hovered_path.as_deref(),
        );
        let is_selected = self.selected_path.as_deref().is_some_and(|path| {
            item.duplicates
                .iter()
                .any(|duplicate| duplicate.path.as_path() == path)
        });
        let description_color = if shown.description != item.default_skill.description {
            theme.ui_yellow_color()
        } else {
            theme.sub_text_color(theme.background()).into_solid()
        };

        let title = Self::render_label(
            item.name.clone(),
            appearance,
            appearance.ui_font_body_large(),
            theme.main_text_color(theme.background()),
        );
        let description = Self::render_label(
            shown.description.clone(),
            appearance,
            appearance.ui_font_footnote(),
            description_color,
        );
        let path_label = Self::render_label(
            shown.path.display().to_string(),
            appearance,
            appearance.ui_font_footnote(),
            theme.sub_text_color(theme.background()),
        );

        let mut column = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(2.0)
            .with_child(title)
            .with_child(description);
        if item.has_duplicates() {
            let mut tags = Wrap::row()
                .with_spacing(4.0)
                .with_run_spacing(4.0)
                .with_cross_axis_alignment(CrossAxisAlignment::Center);
            for duplicate in &item.duplicates {
                tags.extend(std::iter::once(self.render_source_tag(
                    item,
                    duplicate,
                    pinned.path.as_path(),
                    hovered_path.is_some(),
                    appearance,
                )));
            }
            column.add_child(
                Container::new(tags.finish())
                    .with_padding_top(4.0)
                    .with_padding_bottom(2.0)
                    .finish(),
            );
        }
        column.add_child(path_label);

        let action = SkillManagerPanelAction::EditSkill(pinned.path.clone());
        let position_id = Self::skill_row_position_id(&item.name);
        let state = self.row_mouse_state_for(&item.default_skill.path);
        // 标签自己处理点击. 行的按下只在点到标签以外时打开当前记住的那份.
        let row = Hoverable::new(state, move |mouse| {
            let background = if is_selected && mouse.is_hovered() {
                Some(internal_colors::fg_overlay_4(theme))
            } else if is_selected {
                Some(internal_colors::fg_overlay_3(theme))
            } else if mouse.is_hovered() {
                Some(internal_colors::fg_overlay_2(theme))
            } else {
                None
            };
            // 外层 Flex::row 使用 MainAxisSize::Max 填满面板宽度,
            // 避免 Container 收缩到内容自然宽度导致高亮区域右侧留白。
            // 内层 column 必须包 Expanded：否则非 flex 子节点在横轴拿到无限 max，
            // Stretch 会把 min=max=∞ 传给子节点；Text 会因此变成无限宽，带 Wrap 的
            // 来源标签 Container 在 paint 时触发 scene.rs 的 is_infinite 断言闪退。
            let content = Flex::row()
                .with_main_axis_size(MainAxisSize::Max)
                .with_cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(Expanded::new(1.0, column.finish()).finish())
                .finish();
            let mut row = Container::new(content)
                .with_padding_top(ROW_PADDING_VERTICAL)
                .with_padding_bottom(ROW_PADDING_VERTICAL)
                .with_padding_left(ROW_PADDING_HORIZONTAL)
                .with_padding_right(ROW_PADDING_HORIZONTAL)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)));
            if let Some(background) = background {
                row = row.with_background(background);
            }
            row.finish()
        })
        .with_defer_events_to_children()
        .with_cursor(Cursor::PointingHand)
        .on_mouse_down(move |ctx, _, _| {
            ctx.dispatch_typed_action(action.clone());
        })
        .finish();

        SavePosition::new(row, &position_id).finish()
    }

    fn render_skill_list(
        &self,
        items: &[SkillInventoryItem],
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        if items.is_empty() {
            let empty_label = Self::render_label(
                crate::t!("skill-manager-empty"),
                appearance,
                appearance.ui_font_body_large(),
                appearance
                    .theme()
                    .sub_text_color(appearance.theme().background()),
            );
            let cta = self.render_generate_cta_button(appearance);
            return Container::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Start)
                    .with_spacing(10.0)
                    .with_child(empty_label)
                    .with_child(Self::render_label(
                        crate::t!("skill-generate-empty-cta-hint"),
                        appearance,
                        appearance.ui_font_footnote(),
                        appearance
                            .theme()
                            .sub_text_color(appearance.theme().background()),
                    ))
                    .with_child(cta)
                    .finish(),
            )
            .with_uniform_padding(12.0)
            .finish();
        }

        let mut rows = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(2.0);
        for item in items {
            rows.add_child(self.render_skill_row(item, appearance));
        }

        let theme = appearance.theme();
        ClippedScrollable::vertical(
            self.list_scroll_state.clone(),
            rows.finish(),
            ScrollbarWidth::Auto,
            theme.disabled_text_color(theme.background()).into(),
            theme.main_text_color(theme.background()).into(),
            ElementFill::None,
        )
        .with_overlayed_scrollbar()
        .finish()
    }
}


impl SkillManagerPanel {
    fn render_generate_header_button(&self, appearance: &Appearance, ai_enabled: bool) -> Box<dyn Element> {
        let theme = appearance.theme();
        let state = self.generate_button_mouse.clone();
        let label = crate::t!("skill-generate-button").to_string();
        Hoverable::new(state, move |mouse| {
            let hovered = mouse.is_hovered() && ai_enabled;
            let bg = if !ai_enabled {
                internal_colors::fg_overlay_2(theme)
            } else if hovered {
                theme.accent()
            } else {
                internal_colors::fg_overlay_3(theme)
            };
            let text_color = if ai_enabled {
                theme.main_text_color(theme.background())
            } else {
                theme.disabled_text_color(theme.background())
            };
            Container::new(Self::render_label(
                label.clone(),
                appearance,
                appearance.ui_font_footnote(),
                text_color,
            ))
            .with_padding(Padding::uniform(6.0).with_left(10.0).with_right(10.0))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)))
            .with_background(bg)
            .finish()
        })
        .with_cursor(if ai_enabled {
            Cursor::PointingHand
        } else {
            Cursor::Arrow
        })
        .on_click(move |ctx, _, _| {
            if ai_enabled {
                ctx.dispatch_typed_action(SkillManagerPanelAction::OpenGenerateWizard);
            }
        })
        .finish()
    }

    fn render_generate_cta_button(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let state = self.generate_cta_mouse.clone();
        let label = crate::t!("skill-generate-empty-cta").to_string();
        Hoverable::new(state, move |mouse| {
            let bg = if mouse.is_hovered() {
                theme.accent()
            } else {
                internal_colors::fg_overlay_3(theme)
            };
            Container::new(Self::render_label(
                label.clone(),
                appearance,
                appearance.ui_font_footnote(),
                theme.main_text_color(theme.background()),
            ))
            .with_padding(Padding::uniform(6.0).with_left(10.0).with_right(10.0))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)))
            .with_background(bg)
            .finish()
        })
        .with_cursor(Cursor::PointingHand)
        .on_click(|ctx, _, _| {
            ctx.dispatch_typed_action(SkillManagerPanelAction::OpenGenerateWizard);
        })
        .finish()
    }

    fn render_simple_button(
        &self,
        key: &'static str,
        label: String,
        primary: bool,
        action: SkillManagerPanelAction,
        appearance: &Appearance,
        enabled: bool,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let state = self.generate_nav_mouse_for(key);
        Hoverable::new(state, move |mouse| {
            let hovered = mouse.is_hovered() && enabled;
            let bg = if !enabled {
                internal_colors::fg_overlay_2(theme)
            } else if primary && hovered {
                theme.accent()
            } else if primary {
                internal_colors::fg_overlay_4(theme)
            } else if hovered {
                internal_colors::fg_overlay_3(theme)
            } else {
                internal_colors::fg_overlay_2(theme)
            };
            let text_color = if enabled {
                theme.main_text_color(theme.background())
            } else {
                theme.disabled_text_color(theme.background())
            };
            Container::new(Self::render_label(
                label.clone(),
                appearance,
                appearance.ui_font_footnote(),
                text_color,
            ))
            .with_padding(Padding::uniform(6.0).with_left(10.0).with_right(10.0))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)))
            .with_background(bg)
            .finish()
        })
        .with_cursor(if enabled {
            Cursor::PointingHand
        } else {
            Cursor::Arrow
        })
        .on_click(move |ctx, _, _| {
            if enabled {
                ctx.dispatch_typed_action(action.clone());
            }
        })
        .finish()
    }


    /// 主落点紧凑 chip（HomeAgents 默认在前）。
    fn render_location_chips(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let repo_ok = self.generate_repo_root.is_some();
        let mut wrap = Wrap::row()
            .with_spacing(6.0)
            .with_run_spacing(6.0)
            .with_cross_axis_alignment(CrossAxisAlignment::Center);
        for location in SkillGenerateLocation::all() {
            if location == SkillGenerateLocation::RepoAgents && !repo_ok {
                continue;
            }
            let title = match location {
                SkillGenerateLocation::HomeAgents => {
                    crate::t!("skill-generate-location-home-agents").to_string()
                }
                SkillGenerateLocation::RepoAgents => {
                    crate::t!("skill-generate-location-agents").to_string()
                }
                SkillGenerateLocation::PersonalZap => {
                    crate::t!("skill-generate-location-personal").to_string()
                }
                SkillGenerateLocation::Custom => {
                    crate::t!("skill-generate-location-custom").to_string()
                }
            };
            let selected = self.generate_draft.location == location;
            let action = SkillManagerPanelAction::GenerateSetLocation(location);
            let state = self.generate_choice_mouse_for(format!("loc-chip-{location:?}"));
            wrap.extend(std::iter::once(
                Hoverable::new(state, move |mouse| {
                    let hovered = mouse.is_hovered();
                    let text_color = if selected {
                        theme.main_text_color(theme.background())
                    } else {
                        theme.sub_text_color(theme.background())
                    };
                    let mut chip = Container::new(Self::render_label(
                        title.clone(),
                        appearance,
                        appearance.ui_font_footnote(),
                        text_color,
                    ))
                    .with_padding(Padding::uniform(4.0).with_left(10.0).with_right(10.0))
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.0)));
                    // 不用大圆角+描边：GPUI 在矮 chip 上会画出菱形伪影。
                    if selected {
                        chip = chip.with_background(internal_colors::fg_overlay_4(theme));
                    } else if hovered {
                        chip = chip.with_background(internal_colors::fg_overlay_3(theme));
                    }
                    chip.finish()
                })
                .with_cursor(Cursor::PointingHand)
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(action.clone());
                })
                .finish(),
            ));
        }
        wrap.finish()
    }

    fn symlink_provider_label(provider: SkillProvider) -> String {
        match provider {
            SkillProvider::Claude => crate::t!("skill-generate-symlink-claude").to_string(),
            SkillProvider::Codex => crate::t!("skill-generate-symlink-codex").to_string(),
            SkillProvider::Cursor => crate::t!("skill-generate-symlink-cursor").to_string(),
            SkillProvider::Zap => crate::t!("skill-generate-symlink-personal").to_string(),
            other => other.to_string(),
        }
    }

    fn symlink_provider_hint(provider: SkillProvider, name: &str) -> String {
        let slug = if name.is_empty() { "<name>" } else { name };
        let home = match provider {
            SkillProvider::Claude => "~/.claude/skills",
            SkillProvider::Codex => "~/.codex/skills",
            SkillProvider::Cursor => "~/.cursor/skills",
            SkillProvider::Zap => "~/.warp/skills",
            SkillProvider::Agents => "~/.agents/skills",
            _ => "~/.?/skills",
        };
        if let Some(real) = ai::skills::home_skills_path(provider) {
            return format!("{}/{slug}", real.display());
        }
        format!("{home}/{slug}")
    }

    /// 14×14 勾选方块：用 Icon::Check，避免纯文字 + 矮容器被压成蓝条。
    fn render_symlink_check_box(checked: bool, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        const SIZE: f32 = 14.0;
        const ICON: f32 = 10.0;
        let inner = if checked {
            Align::new(
                ConstrainedBox::new(
                    WarpIcon::Check
                        .to_warpui_icon(theme.font_color(theme.accent()))
                        .finish(),
                )
                .with_width(ICON)
                .with_height(ICON)
                .finish(),
            )
            .finish()
        } else {
            Empty::new().finish()
        };
        let mut box_el = Container::new(
            ConstrainedBox::new(inner)
                .with_width(SIZE)
                .with_height(SIZE)
                .finish(),
        )
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(3.0)));
        if checked {
            box_el = box_el
                .with_background(theme.accent())
                .with_border(Border::all(1.0).with_border_fill(theme.accent()));
        } else {
            box_el = box_el.with_border(Border::all(1.0).with_border_fill(theme.surface_3()));
        }
        ConstrainedBox::new(box_el.finish())
            .with_width(SIZE)
            .with_height(SIZE)
            .finish()
    }

    /// 可选软链多选行。
    fn render_symlink_checkboxes(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let name = if self.generate_draft.name.is_empty() {
            "<name>"
        } else {
            self.generate_draft.name.as_str()
        };
        let mut col = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(4.0);
        for provider in symlink_candidate_providers() {
            let disabled = is_symlink_disabled(
                provider,
                &self.generate_draft,
                self.generate_repo_root.as_deref(),
            );
            let checked = !disabled
                && self
                    .generate_draft
                    .symlink_targets
                    .contains(&provider);
            let title = Self::symlink_provider_label(provider);
            let subtitle = Self::symlink_provider_hint(provider, name);
            let action = SkillManagerPanelAction::GenerateToggleSymlink(provider);
            let row_state = self.generate_choice_mouse_for(format!("symlink-{provider:?}"));
            col.add_child(
                Hoverable::new(row_state, move |mouse| {
                    let hovered = mouse.is_hovered() && !disabled;
                    let check_box = Self::render_symlink_check_box(checked && !disabled, appearance);
                    let text_col = Flex::column()
                        .with_cross_axis_alignment(CrossAxisAlignment::Start)
                        .with_spacing(2.0)
                        .with_child(Self::render_label(
                            title.clone(),
                            appearance,
                            appearance.ui_font_footnote(),
                            if disabled {
                                theme.sub_text_color(theme.background())
                            } else {
                                theme.main_text_color(theme.background())
                            },
                        ))
                        .with_child(Self::render_label(
                            subtitle.clone(),
                            appearance,
                            appearance.ui_font_overline(),
                            theme.sub_text_color(theme.background()),
                        ))
                        .finish();
                    let row = Flex::row()
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_spacing(10.0)
                        .with_child(check_box)
                        .with_child(Expanded::new(1.0, text_col).finish())
                        .finish();
                    let mut card = Container::new(row)
                        .with_uniform_padding(8.0)
                        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.0)));
                    if checked && !disabled {
                        card = card
                            .with_background(internal_colors::fg_overlay_3(theme))
                            .with_border(Border::all(1.0).with_border_fill(theme.accent()));
                    } else if hovered {
                        card = card.with_background(internal_colors::fg_overlay_2(theme));
                    } else {
                        card = card.with_background(internal_colors::fg_overlay_1(theme));
                    }
                    card.finish()
                })
                .with_cursor(if disabled {
                    Cursor::Arrow
                } else {
                    Cursor::PointingHand
                })
                .on_click(move |ctx, _, _| {
                    if !disabled {
                        ctx.dispatch_typed_action(action.clone());
                    }
                })
                .finish(),
            );
        }
        col.finish()
    }

    fn render_choice_card(
        &self,
        key: String,
        title: String,
        subtitle: String,
        selected: bool,
        action: SkillManagerPanelAction,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let state = self.generate_choice_mouse_for(key);
        Hoverable::new(state, move |mouse| {
            let hovered = mouse.is_hovered();
            let mut card = Container::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Start)
                    .with_spacing(2.0)
                    .with_child(Self::render_label(
                        title.clone(),
                        appearance,
                        appearance.ui_font_body_large(),
                        theme.main_text_color(theme.background()),
                    ))
                    .with_child(Self::render_label(
                        subtitle.clone(),
                        appearance,
                        appearance.ui_font_footnote(),
                        theme.sub_text_color(theme.background()),
                    ))
                    .finish(),
            )
            .with_uniform_padding(10.0)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.0)));
            if selected {
                card = card
                    .with_background(internal_colors::fg_overlay_3(theme))
                    .with_border(Border::all(1.0).with_border_fill(theme.accent()));
            } else if hovered {
                card = card.with_background(internal_colors::fg_overlay_2(theme));
            } else {
                card = card.with_border(Border::all(1.0).with_border_fill(theme.surface_3()));
            }
            card.finish()
        })
        .with_cursor(Cursor::PointingHand)
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(action.clone());
        })
        .finish()
    }

    /// 细进度条 +「第 N 步 · 共 5 步」元信息（替代粗编号 pills）。
    fn render_generate_progress(
        &self,
        current: SkillGenerateStep,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let filled = (current.index() + 1) as f32;
        let rest = (SkillGenerateStep::COUNT - current.index() - 1) as f32;
        let mut bar_row = Flex::row().with_main_axis_size(MainAxisSize::Max);
        bar_row.add_child(
            Expanded::new(
                filled,
                Container::new(Empty::new().finish())
                    .with_background(theme.accent())
                    .finish(),
            )
            .finish(),
        );
        if rest > 0.0 {
            bar_row.add_child(Expanded::new(rest, Empty::new().finish()).finish());
        }
        let track = ConstrainedBox::new(
            Container::new(bar_row.finish())
                .with_background(internal_colors::fg_overlay_2(theme))
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(99.0)))
                .finish(),
        )
        .with_height(2.0)
        .finish();

        let step_n = current.index() + 1;
        let step_total = SkillGenerateStep::COUNT;
        let meta = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Self::render_label(
                crate::t!(
                    "skill-generate-step-meta",
                    n = step_n,
                    total = step_total
                ),
                appearance,
                appearance.ui_font_overline(),
                theme.sub_text_color(theme.background()),
            ))
            .with_child(Shrinkable::new(1.0, Empty::new().finish()).finish())
            .with_child(Self::render_label(
                if current == SkillGenerateStep::Intent {
                    crate::t!("skill-generate-step-eta")
                } else {
                    match current {
                        SkillGenerateStep::Trigger => crate::t!("skill-generate-step-trigger"),
                        SkillGenerateStep::Location => crate::t!("skill-generate-step-location"),
                        SkillGenerateStep::Name => crate::t!("skill-generate-step-name"),
                        SkillGenerateStep::Launch => crate::t!("skill-generate-step-launch"),
                        SkillGenerateStep::Intent => crate::t!("skill-generate-step-intent"),
                    }
                },
                appearance,
                appearance.ui_font_overline(),
                theme.sub_text_color(theme.background()),
            ))
            .finish();

        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(6.0)
            .with_child(track)
            .with_child(meta)
            .finish()
    }

    /// Intent 步类型分段（紧凑 chip，不是大卡片）。
    fn render_intent_kind_segment(
        &self,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let mut seg = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(2.0);
        for kind in SkillGenerateIntentKind::all() {
            let title = match kind {
                SkillGenerateIntentKind::Workflow => {
                    crate::t!("skill-generate-intent-workflow")
                }
                SkillGenerateIntentKind::Review => crate::t!("skill-generate-intent-review"),
                SkillGenerateIntentKind::Docs => crate::t!("skill-generate-intent-docs"),
                SkillGenerateIntentKind::Other => crate::t!("skill-generate-intent-other"),
            };
            let selected = self.generate_draft.intent_kind == kind;
            let action = SkillManagerPanelAction::GenerateSetIntentKind(kind);
            let state = self.generate_choice_mouse_for(format!("intent-seg-{}", kind.as_str()));
            seg.add_child(
                Hoverable::new(state, move |mouse| {
                    let hovered = mouse.is_hovered();
                    let text_color = if selected {
                        theme.main_text_color(theme.background())
                    } else {
                        theme.sub_text_color(theme.background())
                    };
                    let mut chip = Container::new(Self::render_label(
                        title.clone(),
                        appearance,
                        appearance.ui_font_footnote(),
                        text_color,
                    ))
                    .with_padding(Padding::uniform(4.0).with_left(8.0).with_right(8.0))
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.0)));
                    if selected {
                        chip = chip.with_background(internal_colors::fg_overlay_4(theme));
                    } else if hovered {
                        chip = chip.with_background(internal_colors::fg_overlay_3(theme));
                    }
                    chip.finish()
                })
                .with_cursor(Cursor::PointingHand)
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(action.clone());
                })
                .finish(),
            );
        }
        Container::new(seg.finish())
            .with_padding(Padding::uniform(3.0))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.0)))
            .with_background(internal_colors::fg_overlay_2(theme))
            .finish()
    }

    /// 空目标时的示例列表（点选填入；「自己写…」只聚焦）。
    fn render_goal_examples(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let examples = [
            (
                crate::t!("skill-generate-example-1"),
                Some(crate::t!("skill-generate-example-1")),
            ),
            (
                crate::t!("skill-generate-example-2"),
                Some(crate::t!("skill-generate-example-2")),
            ),
            (
                crate::t!("skill-generate-example-3"),
                Some(crate::t!("skill-generate-example-3")),
            ),
            (crate::t!("skill-generate-example-write-own"), None),
        ];
        let mut col = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(2.0);
        for (index, (label, fill)) in examples.into_iter().enumerate() {
            let state = self.generate_choice_mouse_for(format!("goal-ex-{index}"));
            let action = match fill {
                Some(text) => SkillManagerPanelAction::GenerateApplyExample(text),
                None => SkillManagerPanelAction::GenerateFocusGoal,
            };
            let display = format!("↗ {label}");
            col.add_child(
                Hoverable::new(state, move |mouse| {
                    let hovered = mouse.is_hovered();
                    let text_color = if hovered {
                        theme.main_text_color(theme.background())
                    } else {
                        theme.sub_text_color(theme.background())
                    };
                    let mut row = Container::new(Self::render_label(
                        display.clone(),
                        appearance,
                        appearance.ui_font_footnote(),
                        text_color,
                    ))
                    .with_padding(Padding::uniform(6.0).with_left(8.0).with_right(8.0))
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.0)));
                    if hovered {
                        row = row.with_background(internal_colors::fg_overlay_2(theme));
                    }
                    row.finish()
                })
                .with_cursor(Cursor::PointingHand)
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(action.clone());
                })
                .finish(),
            );
        }
        col.finish()
    }

    /// 带边框的生成向导编辑器；点击容器（含内边距）会聚焦编辑器。
    fn render_generate_editor_box(
        &self,
        mouse_key: &'static str,
        editor: ViewHandle<EditorView>,
        focus_action: SkillManagerPanelAction,
        appearance: &Appearance,
        min_height: Option<f32>,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let state = self.generate_nav_mouse_for(mouse_key);
        Hoverable::new(state, move |_mouse| {
            let content = Clipped::new(ChildView::new(&editor).finish()).finish();
            let content = if let Some(height) = min_height {
                ConstrainedBox::new(content).with_min_height(height).finish()
            } else {
                content
            };
            Container::new(content)
                .with_border(Border::all(1.0).with_border_fill(theme.surface_3()))
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)))
                .with_uniform_padding(6.0)
                .finish()
        })
        .with_cursor(Cursor::IBeam)
        .with_defer_events_to_children()
        .on_mouse_down(move |ctx, _, _| {
            ctx.dispatch_typed_action(focus_action.clone());
        })
        .finish()
    }

    fn render_generate_collecting(
        &self,
        step: SkillGenerateStep,
        awaiting_overwrite: bool,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let inventory = SkillManager::as_ref(app).list_skill_inventory(app);
        let collision = find_name_collision(&inventory, &self.generate_draft.name);
        let path_preview = display_skill_md_path(
            self.generate_draft.location,
            &self.generate_draft.name,
            self.generate_repo_root.as_deref(),
            &self.generate_draft.custom_path,
        );

        let body: Box<dyn Element> = match step {
            SkillGenerateStep::Intent => {
                let goal_empty = self.generate_draft.goal.trim().is_empty();
                let mut col = Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_spacing(10.0)
                    .with_child(Self::render_label(
                        crate::t!("skill-generate-goal-label"),
                        appearance,
                        appearance.ui_font_heading_2(),
                        theme.main_text_color(theme.background()),
                    ))
                    .with_child(Self::render_label(
                        crate::t!("skill-generate-goal-help"),
                        appearance,
                        appearance.ui_font_footnote(),
                        theme.sub_text_color(theme.background()),
                    ))
                    .with_child(self.render_generate_editor_box(
                        "goal-editor",
                        self.generate_goal_editor.clone(),
                        SkillManagerPanelAction::GenerateFocusGoal,
                        appearance,
                        Some(112.0),
                    ));
                if goal_empty {
                    col.add_child(self.render_goal_examples(appearance));
                }
                col.add_child(Self::render_label(
                    crate::t!("skill-generate-type-optional"),
                    appearance,
                    appearance.ui_font_overline(),
                    theme.sub_text_color(theme.background()),
                ));
                col.add_child(self.render_intent_kind_segment(appearance));
                col.finish()
            }
            SkillGenerateStep::Trigger => Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_spacing(8.0)
                .with_child(Self::render_label(
                    crate::t!("skill-generate-trigger-label"),
                    appearance,
                    appearance.ui_font_body_large(),
                    theme.main_text_color(theme.background()),
                ))
                .with_child(Self::render_label(
                    crate::t!("skill-generate-trigger-help"),
                    appearance,
                    appearance.ui_font_footnote(),
                    theme.sub_text_color(theme.background()),
                ))
                .with_child(self.render_generate_editor_box(
                    "trigger-editor",
                    self.generate_trigger_editor.clone(),
                    SkillManagerPanelAction::GenerateFocusTrigger,
                    appearance,
                    Some(96.0),
                ))
                .finish(),
            SkillGenerateStep::Location => {
                let preview_name = if self.generate_draft.name.is_empty() {
                    "name"
                } else {
                    self.generate_draft.name.as_str()
                };
                let path_line = display_skill_md_path(
                    self.generate_draft.location,
                    preview_name,
                    self.generate_repo_root.as_deref(),
                    &self.generate_draft.custom_path,
                );
                // 预览偏向 …/<name>/（去掉尾部 SKILL.md）。
                let path_dir_preview = path_line
                    .strip_suffix("SKILL.md")
                    .map(|s| s.to_string())
                    .unwrap_or(path_line);
                let mut col = Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_spacing(8.0)
                    .with_child(Self::render_label(
                        crate::t!("skill-generate-location-label"),
                        appearance,
                        appearance.ui_font_body_large(),
                        theme.main_text_color(theme.background()),
                    ))
                    .with_child(Self::render_label(
                        crate::t!("skill-generate-location-sub"),
                        appearance,
                        appearance.ui_font_footnote(),
                        theme.sub_text_color(theme.background()),
                    ))
                    .with_child(Self::render_label(
                        crate::t!("skill-generate-location-path-preview"),
                        appearance,
                        appearance.ui_font_overline(),
                        theme.sub_text_color(theme.background()),
                    ))
                    .with_child(self.render_location_chips(appearance))
                    .with_child(
                        Flex::row()
                            .with_main_axis_size(MainAxisSize::Max)
                            .with_child(Expanded::new(
                                1.0,
                                Self::render_label(
                                    path_dir_preview,
                                    appearance,
                                    appearance.ui_font_footnote(),
                                    theme.sub_text_color(theme.background()),
                                ),
                            ).finish())
                            .finish(),
                    );
                if self.generate_draft.location == SkillGenerateLocation::Custom {
                    col.add_child(self.render_generate_editor_box(
                        "custom-path-editor",
                        self.generate_custom_path_editor.clone(),
                        SkillManagerPanelAction::GenerateFocusCustomPath,
                        appearance,
                        None,
                    ));
                }
                if self.generate_draft.location == SkillGenerateLocation::RepoAgents
                    && self.generate_repo_root.is_none()
                {
                    col.add_child(Self::render_label(
                        crate::t!("skill-generate-location-agents-disabled"),
                        appearance,
                        appearance.ui_font_footnote(),
                        theme.sub_text_color(theme.background()),
                    ));
                }
                col.add_child(Self::render_label(
                    crate::t!("skill-generate-symlink-section"),
                    appearance,
                    appearance.ui_font_overline(),
                    theme.sub_text_color(theme.background()),
                ));
                col.add_child(self.render_symlink_checkboxes(appearance));
                col.add_child(Self::render_label(
                    crate::t!("skill-generate-symlink-note"),
                    appearance,
                    appearance.ui_font_footnote(),
                    theme.sub_text_color(theme.background()),
                ));
                col.finish()
            }
            SkillGenerateStep::Name => {
                let mut col = Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_spacing(8.0)
                    .with_child(Self::render_label(
                        crate::t!("skill-generate-name-label"),
                        appearance,
                        appearance.ui_font_body_large(),
                        theme.main_text_color(theme.background()),
                    ))
                    .with_child(self.render_generate_editor_box(
                        "name-editor",
                        self.generate_name_editor.clone(),
                        SkillManagerPanelAction::GenerateFocusName,
                        appearance,
                        None,
                    ))
                    .with_child(Self::render_label(
                        format!(
                            "{} {}",
                            crate::t!("skill-generate-path-preview"),
                            path_preview
                        ),
                        appearance,
                        appearance.ui_font_footnote(),
                        theme.sub_text_color(theme.background()),
                    ));
                if let Some(hit) = collision {
                    col.add_child(Self::render_label(
                        format!(
                            "{} {} ({})",
                            crate::t!("skill-generate-collision-warn"),
                            hit.name,
                            hit.effective.provider
                        ),
                        appearance,
                        appearance.ui_font_footnote(),
                        theme.ui_yellow_color(),
                    ));
                    col.add_child(Self::render_label(
                        hit.effective.path.display().to_string(),
                        appearance,
                        appearance.ui_font_footnote(),
                        theme.sub_text_color(theme.background()),
                    ));
                } else if is_valid_skill_slug(&self.generate_draft.name) {
                    col.add_child(Self::render_label(
                        crate::t!("skill-generate-name-ok"),
                        appearance,
                        appearance.ui_font_footnote(),
                        theme.ansi_fg_green(),
                    ));
                }
                col.finish()
            }
            SkillGenerateStep::Launch => {
                let byop_ready =
                    crate::ai::agent_providers::has_configured_byop_provider(app);
                let mut col = Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_spacing(8.0)
                    .with_child(Self::render_label(
                        if byop_ready {
                            crate::t!("skill-generate-launch-title")
                        } else {
                            crate::t!("skill-generate-cli-title")
                        },
                        appearance,
                        appearance.ui_font_body_large(),
                        theme.main_text_color(theme.background()),
                    ))
                    .with_child(Self::render_label_wrap(
                        if byop_ready {
                            crate::t!("skill-generate-launch-help")
                        } else {
                            crate::t!("skill-generate-cli-help")
                        },
                        appearance,
                        appearance.ui_font_footnote(),
                        theme.sub_text_color(theme.background()),
                    ))
                    .with_child(Self::render_label(
                        format!(
                            "name: {}\npath: {}\ngoal: {}",
                            self.generate_draft.name,
                            path_preview,
                            self.generate_draft.goal
                        ),
                        appearance,
                        appearance.ui_font_footnote(),
                        theme.sub_text_color(theme.background()),
                    ));
                col.finish()
            }
        };

        let can_next = can_advance(step, &self.generate_draft, self.generate_repo_root.as_deref());
        let byop_ready = crate::ai::agent_providers::has_configured_byop_provider(app);
        let next_label = if step == SkillGenerateStep::Launch {
            if byop_ready {
                crate::t!("skill-generate-launch-button").to_string()
            } else {
                crate::t!("skill-generate-cli-launch-button").to_string()
            }
        } else if step == SkillGenerateStep::Intent {
            crate::t!("skill-generate-continue").to_string()
        } else {
            crate::t!("skill-generate-next").to_string()
        };

        let mut foot = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.0);
        if step.prev().is_some() {
            foot.add_child(self.render_simple_button(
                "prev",
                crate::t!("skill-generate-prev").to_string(),
                false,
                SkillManagerPanelAction::GeneratePrevStep,
                appearance,
                true,
            ));
        } else {
            // Intent 第一步：取消关闭向导，而不是灰掉的「上一步」。
            foot.add_child(self.render_simple_button(
                "cancel",
                crate::t!("common-cancel").to_string(),
                false,
                SkillManagerPanelAction::CloseGenerateWizard,
                appearance,
                true,
            ));
        }
        foot.add_child(Shrinkable::new(1.0, Empty::new().finish()).finish());
        if awaiting_overwrite {
            foot.add_child(self.render_simple_button(
                "cancel-ow",
                crate::t!("skill-generate-overwrite-cancel").to_string(),
                false,
                SkillManagerPanelAction::GenerateCancelOverwrite,
                appearance,
                true,
            ));
            foot.add_child(self.render_simple_button(
                "confirm-ow",
                crate::t!("skill-generate-overwrite-confirm").to_string(),
                true,
                SkillManagerPanelAction::GenerateConfirmOverwriteAndLaunch,
                appearance,
                true,
            ));
        } else {
            foot.add_child(self.render_simple_button(
                "next",
                next_label,
                step == SkillGenerateStep::Launch || (step == SkillGenerateStep::Intent && can_next),
                SkillManagerPanelAction::GenerateNextStep,
                appearance,
                can_next,
            ));
        }

        let mut column = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(10.0)
            .with_child(self.render_generate_wizard_header(appearance))
            .with_child(self.render_generate_progress(step, appearance))
            .with_child(Shrinkable::new(1.0, body).finish());
        if awaiting_overwrite {
            column.add_child(Self::render_label(
                crate::t!("skill-generate-overwrite-prompt"),
                appearance,
                appearance.ui_font_footnote(),
                theme.ui_yellow_color(),
            ));
        }
        column.add_child(foot.finish());
        // 软提示放在页脚下方（非黄条）：仅 Intent 且目标为空时。
        if !awaiting_overwrite && step == SkillGenerateStep::Intent && !can_next {
            column.add_child(
                Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_child(Shrinkable::new(1.0, Empty::new().finish()).finish())
                    .with_child(Self::render_label(
                        crate::t!("skill-generate-goal-required-hint"),
                        appearance,
                        appearance.ui_font_overline(),
                        theme.sub_text_color(theme.background()),
                    ))
                    .finish(),
            );
        }
        column.finish()
    }

    fn render_generate_wizard_header(&self, appearance: &Appearance) -> Box<dyn Element> {
        Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.0)
            .with_child(self.render_simple_button(
                "back",
                crate::t!("skill-generate-back").to_string(),
                false,
                SkillManagerPanelAction::CloseGenerateWizard,
                appearance,
                true,
            ))
            .with_child(Self::render_label(
                crate::t!("skill-generate-title"),
                appearance,
                appearance.ui_font_body_large(),
                appearance
                    .theme()
                    .main_text_color(appearance.theme().background()),
            ))
            .finish()
    }

    fn render_generate_drafting(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        let theme = appearance.theme();
        let path_preview = display_skill_md_path(
            self.generate_draft.location,
            &self.generate_draft.name,
            self.generate_repo_root.as_deref(),
            &self.generate_draft.custom_path,
        );
        let byop_ready = crate::ai::agent_providers::has_configured_byop_provider(app);
        let mut column = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(10.0)
            .with_child(self.render_generate_wizard_header(appearance))
            .with_child(self.render_generate_progress(SkillGenerateStep::Launch, appearance))
            .with_child(Self::render_label(
                crate::t!("skill-generate-drafting-title"),
                appearance,
                appearance.ui_font_body_large(),
                theme.main_text_color(theme.background()),
            ))
            .with_child(Self::render_label_wrap(
                crate::t!("skill-generate-drafting-help"),
                appearance,
                appearance.ui_font_footnote(),
                theme.sub_text_color(theme.background()),
            ))
            .with_child(Self::render_label(
                format!(
                    "skill: create-skill\nname: {}\npath: {}",
                    self.generate_draft.name, path_preview
                ),
                appearance,
                appearance.ui_font_footnote(),
                theme.sub_text_color(theme.background()),
            ));
        if !byop_ready {
            column.add_child(Self::render_label(
                crate::t!("skill-generate-drafting-cli-hint"),
                appearance,
                appearance.ui_font_footnote(),
                theme.ui_yellow_color(),
            ));
            column.add_child(self.render_simple_button(
                "draft-copy-cli",
                crate::t!("skill-generate-cli-copy").to_string(),
                false,
                SkillManagerPanelAction::GenerateCopyCliPrompt,
                appearance,
                true,
            ));
            if self
                .generate_cli_copied_at
                .is_some_and(|t| t.elapsed() < COPY_FEEDBACK_DURATION)
            {
                column.add_child(Self::render_label(
                    crate::t!("skill-generate-cli-copied"),
                    appearance,
                    appearance.ui_font_footnote(),
                    theme.ansi_fg_green(),
                ));
            }
        }
        column.add_child(self.render_simple_button(
            "to-review",
            crate::t!("skill-generate-drafting-ready").to_string(),
            true,
            SkillManagerPanelAction::GenerateNextStep,
            appearance,
            true,
        ));
        column.finish()
    }

    fn render_generate_cli_fallback(
        &self,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let prompt = self
            .current_create_skill_prompt()
            .unwrap_or_else(|| crate::t!("skill-generate-cli-prompt-missing").to_string());
        let copied = self
            .generate_cli_copied_at
            .is_some_and(|t| t.elapsed() < COPY_FEEDBACK_DURATION);

        let mut body = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(10.0)
            .with_child(Self::render_label(
                crate::t!("skill-generate-cli-title"),
                appearance,
                appearance.ui_font_body_large(),
                theme.main_text_color(theme.background()),
            ))
            .with_child(Self::render_label_wrap(
                crate::t!("skill-generate-cli-help"),
                appearance,
                appearance.ui_font_footnote(),
                theme.sub_text_color(theme.background()),
            ))
            .with_child(
                Container::new(Self::render_label_wrap(
                    crate::t!("skill-generate-cli-banner"),
                    appearance,
                    appearance.ui_font_footnote(),
                    theme.main_text_color(theme.background()),
                ))
                .with_uniform_padding(10.0)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.0)))
                .with_background(internal_colors::fg_overlay_2(theme))
                .finish(),
            )
            .with_child(
                Container::new(Self::render_label(
                    prompt,
                    appearance,
                    appearance.ui_font_overline(),
                    theme.sub_text_color(theme.background()),
                ))
                .with_uniform_padding(10.0)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.0)))
                .with_background(internal_colors::fg_overlay_3(theme))
                .with_border(Border::all(1.0).with_border_fill(theme.surface_3()))
                .finish(),
            )
            .with_child(Self::render_label_wrap(
                crate::t!("skill-generate-cli-steps"),
                appearance,
                appearance.ui_font_footnote(),
                theme.sub_text_color(theme.background()),
            ));
        if copied {
            body.add_child(Self::render_label(
                crate::t!("skill-generate-cli-copied"),
                appearance,
                appearance.ui_font_footnote(),
                theme.ansi_fg_green(),
            ));
        }
        body.add_child(self.render_simple_button(
            "cli-settings",
            crate::t!("skill-generate-cli-open-settings").to_string(),
            false,
            SkillManagerPanelAction::GenerateOpenByopSettings,
            appearance,
            true,
        ));

        let foot = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.0)
            .with_child(self.render_simple_button(
                "cli-back",
                crate::t!("skill-generate-prev").to_string(),
                false,
                SkillManagerPanelAction::GenerateBackFromCliFallback,
                appearance,
                true,
            ))
            .with_child(Shrinkable::new(1.0, Empty::new().finish()).finish())
            .with_child(self.render_simple_button(
                "cli-open",
                crate::t!("skill-generate-cli-open-skill").to_string(),
                false,
                SkillManagerPanelAction::GenerateOpenFromCliFallback,
                appearance,
                true,
            ))
            .with_child(self.render_simple_button(
                "cli-copy",
                crate::t!("skill-generate-cli-copy").to_string(),
                true,
                SkillManagerPanelAction::GenerateCopyCliPrompt,
                appearance,
                true,
            ))
            .finish();

        Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(10.0)
            .with_child(self.render_generate_wizard_header(appearance))
            .with_child(self.render_generate_progress(SkillGenerateStep::Launch, appearance))
            .with_child(Self::render_label(
                crate::t!("skill-generate-cli-meta"),
                appearance,
                appearance.ui_font_overline(),
                theme.sub_text_color(theme.background()),
            ))
            .with_child(Shrinkable::new(1.0, body.finish()).finish())
            .with_child(foot)
            .finish()
    }

    fn render_generate_review(
        &self,
        path: &Path,
        path_exists: bool,
        awaiting_overwrite: bool,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let mut column = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(10.0)
            .with_child(self.render_generate_wizard_header(appearance))
            .with_child(Self::render_label(
                crate::t!("skill-generate-review-title"),
                appearance,
                appearance.ui_font_body_large(),
                theme.main_text_color(theme.background()),
            ))
            .with_child(Self::render_label(
                crate::t!("skill-generate-review-help"),
                appearance,
                appearance.ui_font_footnote(),
                theme.sub_text_color(theme.background()),
            ))
            .with_child(Self::render_label(
                path.display().to_string(),
                appearance,
                appearance.ui_font_footnote(),
                theme.sub_text_color(theme.background()),
            ));
        if path_exists {
            column.add_child(Self::render_label(
                crate::t!("skill-generate-review-exists"),
                appearance,
                appearance.ui_font_footnote(),
                theme.ui_yellow_color(),
            ));
        } else {
            column.add_child(Self::render_label(
                crate::t!("skill-generate-review-missing"),
                appearance,
                appearance.ui_font_footnote(),
                theme.sub_text_color(theme.background()),
            ));
        }
        let mut foot = Flex::row()
            .with_spacing(8.0)
            .with_child(self.render_simple_button(
                "back-draft",
                crate::t!("skill-generate-review-back-agent").to_string(),
                false,
                SkillManagerPanelAction::GenerateJumpToDrafting,
                appearance,
                true,
            ));
        if awaiting_overwrite {
            foot.add_child(self.render_simple_button(
                "cancel-ow2",
                crate::t!("skill-generate-overwrite-cancel").to_string(),
                false,
                SkillManagerPanelAction::GenerateCancelOverwrite,
                appearance,
                true,
            ));
            foot.add_child(self.render_simple_button(
                "confirm-open",
                crate::t!("skill-generate-overwrite-open").to_string(),
                true,
                SkillManagerPanelAction::GenerateConfirmOverwriteAndOpen,
                appearance,
                true,
            ));
        } else {
            foot.add_child(self.render_simple_button(
                "open-skill",
                crate::t!("skill-generate-open-skill").to_string(),
                true,
                SkillManagerPanelAction::GenerateOpenDraftedSkill,
                appearance,
                true,
            ));
        }
        column.add_child(foot.finish());
        column.finish()
    }

    fn render_generate_wizard(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        match self.generate_phase.clone() {
            SkillGeneratePhase::Closed => Empty::new().finish(),
            SkillGeneratePhase::Collecting {
                step,
                awaiting_overwrite_confirm,
            } => self.render_generate_collecting(
                step,
                awaiting_overwrite_confirm,
                appearance,
                app,
            ),
            SkillGeneratePhase::Drafting => self.render_generate_drafting(appearance, app),
            SkillGeneratePhase::CliFallback => self.render_generate_cli_fallback(appearance, app),
            SkillGeneratePhase::Review {
                path,
                path_exists,
                awaiting_overwrite_confirm,
            } => self.render_generate_review(
                &path,
                path_exists,
                awaiting_overwrite_confirm,
                appearance,
            ),
        }
    }

}

impl TypedActionView for SkillManagerPanel {
    type Action = SkillManagerPanelAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            SkillManagerPanelAction::SelectProviderFilter(provider) => {
                if self.provider_filter == *provider {
                    return;
                }
                self.provider_filter = *provider;
                // 同步折叠态下拉的选中项:点击左侧「全部」按钮时下拉并不知情。
                self.rebuild_source_dropdown(ctx);
                ctx.notify();
            }
            SkillManagerPanelAction::EditSkill(path) => {
                self.open_skill_file(path.clone(), ctx);
                ctx.notify();
            }
            SkillManagerPanelAction::PinSkillSource { name, path, open } => {
                self.pinned_paths.insert(name.clone(), path.clone());
                if *open {
                    self.open_skill_file(path.clone(), ctx);
                }
                ctx.notify();
            }
            SkillManagerPanelAction::OpenGenerateWizard => {
                // 无 BYOP / AI 关闭时仍可打开向导，最终走 CLI 复制兜底。
                self.open_generate_wizard(ctx);
            }
            SkillManagerPanelAction::CloseGenerateWizard => {
                self.close_generate_wizard(ctx);
            }
            SkillManagerPanelAction::GenerateSetIntentKind(kind) => {
                self.generate_draft.intent_kind = *kind;
                ctx.notify();
            }
            SkillManagerPanelAction::GenerateSetLocation(location) => {
                self.generate_draft.location = *location;
                // 切到个人 Zap 时去掉 Zap 自链勾选。
                if *location == SkillGenerateLocation::PersonalZap {
                    self.generate_draft
                        .symlink_targets
                        .retain(|p| *p != SkillProvider::Zap);
                }
                if *location == SkillGenerateLocation::Custom {
                    ctx.focus(&self.generate_custom_path_editor);
                }
                ctx.notify();
            }
            SkillManagerPanelAction::GenerateToggleSymlink(provider) => {
                if is_symlink_disabled(
                    *provider,
                    &self.generate_draft,
                    self.generate_repo_root.as_deref(),
                ) {
                    return;
                }
                let targets = &mut self.generate_draft.symlink_targets;
                if let Some(idx) = targets.iter().position(|p| p == provider) {
                    targets.remove(idx);
                } else {
                    targets.push(*provider);
                }
                ctx.notify();
            }
            SkillManagerPanelAction::GenerateFocusCustomPath => {
                ctx.focus(&self.generate_custom_path_editor);
            }
            SkillManagerPanelAction::GeneratePrevStep => {
                self.sync_generate_draft_from_editors(ctx);
                if let SkillGeneratePhase::Collecting { step, .. } = self.generate_phase {
                    if let Some(prev) = step.prev() {
                        self.generate_phase = SkillGeneratePhase::Collecting {
                            step: prev,
                            awaiting_overwrite_confirm: false,
                        };
                        self.focus_generate_collecting_editor(prev, ctx);
                        ctx.notify();
                    }
                }
            }
            SkillManagerPanelAction::GenerateNextStep => {
                self.sync_generate_draft_from_editors(ctx);
                match self.generate_phase.clone() {
                    SkillGeneratePhase::Collecting { step, .. } => {
                        if !can_advance(step, &self.generate_draft, self.generate_repo_root.as_deref())
                        {
                            return;
                        }
                        if step == SkillGenerateStep::Launch {
                            self.launch_create_skill(ctx);
                            return;
                        }
                        if let Some(next) = step.next() {
                            self.generate_phase = SkillGeneratePhase::Collecting {
                                step: next,
                                awaiting_overwrite_confirm: false,
                            };
                            self.focus_generate_collecting_editor(next, ctx);
                            ctx.notify();
                        }
                    }
                    SkillGeneratePhase::Drafting => {
                        self.enter_review_phase(ctx);
                    }
                    _ => {}
                }
            }
            SkillManagerPanelAction::GenerateConfirmOverwriteAndLaunch => {
                self.generate_draft.allow_overwrite = true;
                if let SkillGeneratePhase::Collecting { step, .. } = self.generate_phase {
                    self.generate_phase = SkillGeneratePhase::Collecting {
                        step,
                        awaiting_overwrite_confirm: false,
                    };
                }
                self.launch_create_skill(ctx);
            }
            SkillManagerPanelAction::GenerateCancelOverwrite => {
                match &mut self.generate_phase {
                    SkillGeneratePhase::Collecting {
                        awaiting_overwrite_confirm,
                        ..
                    }
                    | SkillGeneratePhase::Review {
                        awaiting_overwrite_confirm,
                        ..
                    } => {
                        *awaiting_overwrite_confirm = false;
                        self.generate_draft.allow_overwrite = false;
                        ctx.notify();
                    }
                    _ => {}
                }
            }
            SkillManagerPanelAction::GenerateOpenDraftedSkill => {
                let SkillGeneratePhase::Review {
                    path,
                    path_exists,
                    awaiting_overwrite_confirm,
                } = self.generate_phase.clone()
                else {
                    return;
                };
                if path_exists && !self.generate_draft.allow_overwrite && !awaiting_overwrite_confirm
                {
                    self.generate_phase = SkillGeneratePhase::Review {
                        path,
                        path_exists,
                        awaiting_overwrite_confirm: true,
                    };
                    ctx.notify();
                    return;
                }
                self.create_symlinks_then_open(path, ctx);
            }
            SkillManagerPanelAction::GenerateConfirmOverwriteAndOpen => {
                self.generate_draft.allow_overwrite = true;
                if let SkillGeneratePhase::Review { path, path_exists, .. } =
                    self.generate_phase.clone()
                {
                    self.generate_phase = SkillGeneratePhase::Review {
                        path: path.clone(),
                        path_exists,
                        awaiting_overwrite_confirm: false,
                    };
                    self.create_symlinks_then_open(path, ctx);
                }
            }
            SkillManagerPanelAction::GenerateJumpToDrafting => {
                // 无 BYOP 时回到 CLI 兜底页，避免再进会失败的 Drafting。
                if crate::ai::agent_providers::has_configured_byop_provider(ctx) {
                    self.generate_phase = SkillGeneratePhase::Drafting;
                } else {
                    self.generate_phase = SkillGeneratePhase::CliFallback;
                }
                ctx.notify();
            }
            SkillManagerPanelAction::GenerateFocusGoal => {
                ctx.focus(&self.generate_goal_editor);
            }
            SkillManagerPanelAction::GenerateFocusTrigger => {
                ctx.focus(&self.generate_trigger_editor);
            }
            SkillManagerPanelAction::GenerateFocusName => {
                ctx.focus(&self.generate_name_editor);
            }
            SkillManagerPanelAction::GenerateApplyExample(text) => {
                self.apply_generate_goal_example(text, ctx);
            }
            SkillManagerPanelAction::GenerateCopyCliPrompt => {
                self.copy_cli_prompt(ctx);
            }
            SkillManagerPanelAction::GenerateBackFromCliFallback => {
                self.generate_cli_copied_at = None;
                self.generate_phase = SkillGeneratePhase::Collecting {
                    step: SkillGenerateStep::Launch,
                    awaiting_overwrite_confirm: false,
                };
                ctx.notify();
            }
            SkillManagerPanelAction::GenerateOpenFromCliFallback => {
                self.enter_review_phase(ctx);
            }
            SkillManagerPanelAction::GenerateOpenByopSettings => {
                ctx.dispatch_typed_action(&WorkspaceAction::ShowSettingsPage(
                    SettingsSection::AgentProviders,
                ));
            }
        }
    }
}

impl SkillManagerPanel {
    /// Review 确认打开：先按草稿创建软链（主 SKILL.md 已存在时），再打开主文件。
    fn create_symlinks_then_open(&mut self, path: PathBuf, ctx: &mut ViewContext<Self>) {
        if let Some(primary_dir) = path.parent() {
            if primary_dir.exists() {
                let targets =
                    planned_symlink_targets(&self.generate_draft, self.generate_repo_root.as_deref());
                if !targets.is_empty() {
                    if let Err(err) = create_skill_symlinks(primary_dir, &targets) {
                        log::warn!(
                            "{}: {err}",
                            crate::t!("skill-generate-symlink-error")
                        );
                    }
                }
            }
        }
        self.open_skill_file(path, ctx);
        self.generate_phase = SkillGeneratePhase::Closed;
        ctx.notify();
    }

    fn open_skill_file(&mut self, path: PathBuf, ctx: &mut ViewContext<Self>) {
        self.selected_path = Some(path.clone());
        self.scroll_selected_path_into_view_with_ctx(ctx);
        ctx.emit(SkillManagerPanelEvent::OpenSkillFile { path });
    }
}

impl View for SkillManagerPanel {
    fn ui_name() -> &'static str {
        "SkillManagerPanel"
    }

    fn on_focus(&mut self, _focus_ctx: &warpui::FocusContext, ctx: &mut ViewContext<Self>) {
        // 向导打开时绝不能把焦点抢回隐藏的搜索框，否则目标/触发/命名无法输入。
        match self.generate_phase.clone() {
            SkillGeneratePhase::Closed => {
                ctx.focus(&self.query_editor);
                self.scroll_selected_path_into_view_with_ctx(ctx);
            }
            SkillGeneratePhase::Collecting { step, .. } => {
                self.focus_generate_collecting_editor(step, ctx);
            }
            SkillGeneratePhase::Drafting
            | SkillGeneratePhase::CliFallback
            | SkillGeneratePhase::Review { .. } => {
                // 草拟 / CLI 兜底 / 审阅阶段不聚焦搜索框。
            }
        }
        ctx.notify();
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        if !matches!(self.generate_phase, SkillGeneratePhase::Closed) {
            return Container::new(self.render_generate_wizard(appearance, app))
                .with_uniform_padding(PANEL_PADDING)
                .finish();
        }

        let inventory = SkillManager::as_ref(app).list_skill_inventory(app);
        let providers = Self::providers_in_inventory(&inventory);
        let query = self.query(app);
        let items = Self::filter_inventory(&inventory, &query, self.provider_filter);
        // 向导在无 BYOP 时走 CLI 兜底，入口始终可用。
        let search_row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.0)
            .with_child(Expanded::new(1.0, self.render_search_input(appearance)).finish())
            .with_child(self.render_generate_header_button(appearance, true))
            .finish();

        Container::new(
            Flex::column()
                .with_main_axis_size(MainAxisSize::Max)
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_spacing(8.0)
                .with_child(search_row)
                .with_child(self.render_filter_rows(&providers, appearance))
                .with_child(
                    Shrinkable::new(1.0, self.render_skill_list(&items, appearance)).finish(),
                )
                .finish(),
        )
        .with_uniform_padding(PANEL_PADDING)
        .finish()
    }
}

impl Entity for SkillManagerPanel {
    type Event = SkillManagerPanelEvent;
}

#[cfg(test)]
#[path = "panel_tests.rs"]
mod tests;
