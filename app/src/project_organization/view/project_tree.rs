use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use pathfinder_color::ColorU;
use pathfinder_geometry::vector::vec2f;
use warp_core::ui::color::coloru_with_opacity;
use warp_core::ui::icons::Icon as WarpIcon;
use warp_core::ui::theme::WarpTheme;
use warpui::{
    assets::asset_cache::AssetSource,
    elements::{
        Border, CacheOption, ChildView, ClippedScrollStateHandle, ClippedScrollable,
        ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, DropShadow, Element, Empty,
        Fill, Flex, Hoverable, Image, MainAxisAlignment, MainAxisSize, MouseStateHandle,
        ParentElement, Radius, SavePosition, ScrollbarWidth, Shrinkable, Text,
    },
    platform::Cursor,
    text_layout::ClipConfig,
    ui_components::components::UiComponent,
    AppContext, Entity, ModelHandle, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle,
};

use crate::{
    appearance::Appearance,
    project_organization::{
        model::ProjectOrganizationModel,
        workspace_agent_activity::{
            workspace_activity_slot, WorkspaceActivitySlot, WorkspaceAgentActivity,
            WorkspaceAgentIdentity, WorkspaceAgentPhase,
        },
    },
    ui_components::{
        breathing_ring::{BreathingRing, BreathingStateHandle},
        buttons::icon_button,
        icon_with_status::{render_icon_with_status, IconWithStatusSizing, IconWithStatusVariant},
        icons,
    },
    view_components::action_button::{ActionButton, ButtonSize, SecondaryTheme},
};

use crate::project_organization::domain::{
    Repository, RepositoryId, RepositoryWorkspace, RepositoryWorkspaceId,
};
use crate::project_organization::project_tree_tab::{
    workspace_parent_activity_slot, ProjectTreeTabId, ProjectTreeTabNode, TabNodeActivity,
};

const WORKSPACE_RUNNING_DOT_SIZE: f32 = 6.;
const WORKSPACE_ACTIVITY_SLOT_SIZE: f32 = 16.;
const WORKSPACE_TREE_RAIL_WIDTH: f32 = 2.;
const WORKSPACE_GROUP_INDENT: f32 = 16.;
const REPOSITORY_GROUP_SPACING: f32 = 10.;
const WORKSPACE_AGENT_RING_WIDTH: f32 = 1.5;
const TREE_CHEVRON_SIZE: f32 = 16.;
const TREE_ROW_ICON_SIZE: f32 = 16.;
const TREE_ICON_GAP: f32 = 6.;
/// 页签图标与 workspace 分支图标对齐: 与 chevron 同宽,行内 Flex spacing 再补 gap。
const TAB_UNDER_WORKSPACE_INDENT: f32 = TREE_CHEVRON_SIZE;
const ITERM_PROMPT_ICON_PATH: &str = "bundled/svg/iterm-prompt.svg";

const WORKSPACE_AGENT_ICON_SIZING: IconWithStatusSizing = IconWithStatusSizing {
    icon_size: 8.,
    padding: 2.5,
    badge_icon_size: 8.,
    badge_padding: 1.,
    overall_size_override: Some(13.),
    badge_offset: (0., 0.),
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TabLayout {
    Horizontal,
    Vertical,
}

/// 解析项目组织模式下的页签布局。
///
/// 启用 repository workspaces 时强制使用水平 TabBar, 但不会修改用户原有的
/// Vertical Tabs 设置值。
pub fn resolved_project_organization_tab_layout(
    repository_workspaces_enabled: bool,
    vertical_tabs_enabled: bool,
) -> TabLayout {
    if repository_workspaces_enabled || !vertical_tabs_enabled {
        TabLayout::Horizontal
    } else {
        TabLayout::Vertical
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceTreeNode {
    pub workspace_id: RepositoryWorkspaceId,
    pub display_name: String,
    pub branch: String,
    pub tab_count: usize,
    pub expanded: bool,
    pub tabs: Vec<ProjectTreeTabNode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryTreeNode {
    pub repository_id: RepositoryId,
    pub display_name: String,
    pub expanded: bool,
    pub workspaces: Vec<WorkspaceTreeNode>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectTreeRow {
    Repository(RepositoryId),
    Workspace(RepositoryWorkspaceId),
    Tab {
        workspace_id: RepositoryWorkspaceId,
        tab_id: ProjectTreeTabId,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectTreeState {
    repositories: Vec<RepositoryTreeNode>,
    selected_workspace_id: Option<RepositoryWorkspaceId>,
}

impl ProjectTreeState {
    pub fn new(repositories: Vec<RepositoryTreeNode>) -> Self {
        Self {
            repositories,
            selected_workspace_id: None,
        }
    }

    pub fn repositories(&self) -> &[RepositoryTreeNode] {
        &self.repositories
    }

    pub fn from_records(
        repositories: Vec<Repository>,
        workspaces: Vec<RepositoryWorkspace>,
        tab_counts: &HashMap<RepositoryWorkspaceId, usize>,
        tabs: &HashMap<RepositoryWorkspaceId, Vec<ProjectTreeTabNode>>,
    ) -> Self {
        let mut workspaces_by_repository = HashMap::<RepositoryId, Vec<RepositoryWorkspace>>::new();
        for workspace in workspaces {
            workspaces_by_repository
                .entry(workspace.repository_id)
                .or_default()
                .push(workspace);
        }

        let mut repositories = repositories;
        repositories.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.display_name.cmp(&right.display_name))
        });
        let repositories = repositories
            .into_iter()
            .map(|repository| {
                let mut workspaces = workspaces_by_repository
                    .remove(&repository.id)
                    .unwrap_or_default();
                workspaces.sort_by(|left, right| {
                    left.created_at
                        .cmp(&right.created_at)
                        .then_with(|| left.display_name.cmp(&right.display_name))
                });
                let workspaces = workspaces
                    .into_iter()
                    .map(|workspace| WorkspaceTreeNode {
                        workspace_id: workspace.id,
                        display_name: workspace.display_name,
                        branch: workspace.branch,
                        tab_count: tab_counts.get(&workspace.id).copied().unwrap_or_default(),
                        expanded: true,
                        tabs: tabs.get(&workspace.id).cloned().unwrap_or_default(),
                    })
                    .collect();
                RepositoryTreeNode {
                    repository_id: repository.id,
                    display_name: repository.display_name,
                    expanded: true,
                    workspaces,
                }
            })
            .collect::<Vec<_>>();
        Self::new(repositories)
    }

    pub fn visible_rows(&self) -> Vec<ProjectTreeRow> {
        let mut rows = Vec::new();
        for repository in &self.repositories {
            rows.push(ProjectTreeRow::Repository(repository.repository_id));
            if !repository.expanded {
                continue;
            }
            for workspace in &repository.workspaces {
                rows.push(ProjectTreeRow::Workspace(workspace.workspace_id));
                if !workspace.expanded {
                    continue;
                }
                for tab in &workspace.tabs {
                    rows.push(ProjectTreeRow::Tab {
                        workspace_id: workspace.workspace_id,
                        tab_id: tab.id,
                    });
                }
            }
        }
        rows
    }

    pub fn toggle_repository(&mut self, repository_id: RepositoryId) -> bool {
        let Some(repository) = self
            .repositories
            .iter_mut()
            .find(|repository| repository.repository_id == repository_id)
        else {
            return false;
        };
        repository.expanded = !repository.expanded;
        true
    }

    pub fn select_workspace(&mut self, workspace_id: RepositoryWorkspaceId) -> bool {
        self.select_workspace_internal(workspace_id, true)
    }

    /// 点名称: 未选中或已收起则选中并展开; 已选中且已展开则收起。
    pub fn select_or_toggle_workspace(&mut self, workspace_id: RepositoryWorkspaceId) -> bool {
        let already_selected = self.selected_workspace_id == Some(workspace_id);
        let expanded = self.workspace_is_expanded(workspace_id);
        if already_selected && expanded {
            self.toggle_workspace_expanded(workspace_id)
        } else {
            self.select_workspace(workspace_id)
        }
    }

    fn workspace_is_expanded(&self, workspace_id: RepositoryWorkspaceId) -> bool {
        self.repositories.iter().any(|repository| {
            repository
                .workspaces
                .iter()
                .any(|workspace| workspace.workspace_id == workspace_id && workspace.expanded)
        })
    }

    fn select_workspace_internal(
        &mut self,
        workspace_id: RepositoryWorkspaceId,
        expand: bool,
    ) -> bool {
        let exists = self.repositories.iter().any(|repository| {
            repository
                .workspaces
                .iter()
                .any(|workspace| workspace.workspace_id == workspace_id)
        });
        if !exists {
            return false;
        }
        self.selected_workspace_id = Some(workspace_id);
        if expand {
            if let Some(workspace) = self.workspace_mut(workspace_id) {
                workspace.expanded = true;
            }
        }
        true
    }

    pub fn toggle_workspace_expanded(&mut self, workspace_id: RepositoryWorkspaceId) -> bool {
        let Some(workspace) = self.workspace_mut(workspace_id) else {
            return false;
        };
        workspace.expanded = !workspace.expanded;
        true
    }

    fn workspace_mut(
        &mut self,
        workspace_id: RepositoryWorkspaceId,
    ) -> Option<&mut WorkspaceTreeNode> {
        self.repositories.iter_mut().find_map(|repository| {
            repository
                .workspaces
                .iter_mut()
                .find(|workspace| workspace.workspace_id == workspace_id)
        })
    }

    pub fn set_active_workspace(&mut self, workspace_id: Option<RepositoryWorkspaceId>) {
        if let Some(workspace_id) = workspace_id {
            if self.select_workspace_internal(workspace_id, false) {
                return;
            }
        }
        self.selected_workspace_id = None;
    }

    pub fn selected_workspace_id(&self) -> Option<RepositoryWorkspaceId> {
        self.selected_workspace_id
    }
}

#[derive(Clone, Debug)]
pub enum ProjectTreeAction {
    AddRepository,
    CreateWorkspace {
        repository_id: RepositoryId,
    },
    DeleteWorkspace {
        workspace_id: RepositoryWorkspaceId,
    },
    ToggleRepository {
        repository_id: RepositoryId,
    },
    SelectWorkspace {
        workspace_id: Option<RepositoryWorkspaceId>,
    },
    ToggleWorkspace {
        workspace_id: RepositoryWorkspaceId,
    },
    SelectTab {
        workspace_id: RepositoryWorkspaceId,
        tab_id: ProjectTreeTabId,
    },
    CloseTab {
        workspace_id: RepositoryWorkspaceId,
        tab_id: ProjectTreeTabId,
    },
    NewTab {
        workspace_id: RepositoryWorkspaceId,
    },
    ReorderTabs {
        workspace_id: RepositoryWorkspaceId,
        from: usize,
        to: usize,
    },
}

#[derive(Clone, Debug)]
pub enum ProjectTreeEvent {
    AddRepositoryRequested,
    CreateWorkspaceRequested {
        repository_id: RepositoryId,
    },
    DeleteWorkspaceRequested {
        workspace_id: RepositoryWorkspaceId,
    },
    WorkspaceSelected {
        workspace_id: Option<RepositoryWorkspaceId>,
    },
    TabSelected {
        workspace_id: RepositoryWorkspaceId,
        tab_id: ProjectTreeTabId,
    },
    TabCloseRequested {
        workspace_id: RepositoryWorkspaceId,
        tab_id: ProjectTreeTabId,
    },
    NewTabRequested {
        workspace_id: RepositoryWorkspaceId,
    },
    TabsReordered {
        workspace_id: RepositoryWorkspaceId,
        from: usize,
        to: usize,
    },
}

fn repository_add_workspace_position_id(repository_id: RepositoryId) -> String {
    format!("project_tree:repository:{repository_id}:add_workspace")
}

fn should_show_workspace_hover_actions(workspace_row_hovered: bool) -> bool {
    workspace_row_hovered
}

fn workspace_row_is_selected(
    selected_workspace_id: Option<RepositoryWorkspaceId>,
    workspace_id: RepositoryWorkspaceId,
) -> bool {
    selected_workspace_id == Some(workspace_id)
}

fn workspace_row_shows_selection_accent(workspace: &WorkspaceTreeNode, is_selected: bool) -> bool {
    is_selected && !workspace.expanded
}

fn tree_row_icon(
    icon: icons::Icon,
    color: warp_core::ui::theme::Fill,
    size: f32,
) -> Box<dyn Element> {
    ConstrainedBox::new(icon.to_warpui_icon(color).finish())
        .with_width(size)
        .with_height(size)
        .finish()
}

fn tree_status_icon_offset() -> f32 {
    TREE_CHEVRON_SIZE + TREE_ICON_GAP
}

fn tab_status_icon_offset() -> f32 {
    TAB_UNDER_WORKSPACE_INDENT + TREE_ICON_GAP
}

fn tree_name_offset() -> f32 {
    tree_status_icon_offset() + TREE_ROW_ICON_SIZE + TREE_ICON_GAP
}

fn tab_name_offset() -> f32 {
    tab_status_icon_offset() + TREE_ROW_ICON_SIZE + TREE_ICON_GAP
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceVisualState {
    is_selected: bool,
    has_running_terminal: bool,
    agent_activity: Option<WorkspaceAgentActivity>,
}

impl WorkspaceVisualState {
    pub(crate) fn new(
        is_selected: bool,
        has_running_terminal: bool,
        agent_activity: Option<WorkspaceAgentActivity>,
    ) -> Self {
        Self {
            is_selected,
            has_running_terminal,
            agent_activity,
        }
    }

    pub(crate) fn should_render_selection_frame(&self) -> bool {
        false
    }

    pub(crate) fn should_render_selection_accent(&self) -> bool {
        self.is_selected
    }

    /// 活动槽类型: agent 在场时绿点让位给头像。
    pub(crate) fn activity_slot(&self) -> WorkspaceActivitySlot {
        workspace_activity_slot(self.agent_activity, self.has_running_terminal)
    }

    pub(crate) fn should_render_running_indicator(&self) -> bool {
        matches!(self.activity_slot(), WorkspaceActivitySlot::RunningDot)
    }

    pub(crate) fn should_breathe_agent_ring(&self) -> bool {
        self.agent_activity
            .is_some_and(WorkspaceAgentActivity::should_breathe)
    }

    pub(crate) fn should_fill_idle_row(&self) -> bool {
        false
    }
}

/// 品牌色过暗时换对比色, 避免深色侧栏上看不见环。
fn ring_color_contrasts_on_dark_brand(brand: ColorU, fallback: ColorU) -> ColorU {
    if (brand.r as u16) + (brand.g as u16) + (brand.b as u16) < 180 {
        fallback
    } else {
        brand
    }
}

/// 活动槽呼吸环颜色: Blocked 为黄, InProgress CLI 用 brand, Oz 用 accent。
fn agent_activity_ring_color(activity: WorkspaceAgentActivity, theme: &WarpTheme) -> ColorU {
    match activity.phase {
        WorkspaceAgentPhase::Blocked => theme.ansi_fg_yellow(),
        WorkspaceAgentPhase::InProgress => match activity.identity {
            WorkspaceAgentIdentity::Cli(agent) => {
                let brand = agent
                    .brand_color()
                    .unwrap_or_else(|| theme.accent().into_solid_bias_right_color());
                ring_color_contrasts_on_dark_brand(
                    brand,
                    theme
                        .main_text_color(theme.background())
                        .into_solid_bias_right_color(),
                )
            }
            WorkspaceAgentIdentity::Oz { ambient: true }
            | WorkspaceAgentIdentity::Oz { ambient: false } => {
                theme.accent().into_solid_bias_right_color()
            }
        },
    }
}

fn agent_activity_icon_variant(
    identity: WorkspaceAgentIdentity,
    theme: &WarpTheme,
) -> IconWithStatusVariant {
    match identity {
        WorkspaceAgentIdentity::Oz { ambient } => IconWithStatusVariant::OzAgent {
            status: None,
            is_ambient: ambient,
        },
        WorkspaceAgentIdentity::Cli(agent) => match agent.brand_color() {
            Some(_) => IconWithStatusVariant::CLIAgent {
                agent,
                status: None,
            },
            None => IconWithStatusVariant::Neutral {
                icon: WarpIcon::Terminal,
                icon_color: theme.sub_text_color(theme.background()),
            },
        },
    }
}

/// 固定 16×16 槽,把绿点 / 终端图标 / agent 头像都居中在和 workspace 分支图标同一列。
/// ConstrainedBox 只收紧子约束、不强制返回尺寸,所以用 Flex Max 把槽撑满。
fn tree_status_slot(child: Box<dyn Element>) -> Box<dyn Element> {
    ConstrainedBox::new(
        Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(child)
            .finish(),
    )
    .with_width(TREE_ROW_ICON_SIZE)
    .with_height(TREE_ROW_ICON_SIZE)
    .finish()
}

fn sized_activity_slot(child: Box<dyn Element>) -> Box<dyn Element> {
    tree_status_slot(child)
}

/// 显示名与真实分支相同时不再重复第二行; 分支信息仍由显示名本身承担。
fn workspace_shows_branch_subtitle(display_name: &str, branch: &str) -> bool {
    display_name != branch
}

fn workspace_count_pill_label(workspace_count: usize) -> String {
    workspace_count.to_string()
}

fn tab_count_badge_label(tab_count: usize) -> String {
    if tab_count > 99 {
        "99+".to_string()
    } else {
        tab_count.to_string()
    }
}

fn synchronize_mouse_states<Id>(mouse_states: &mut HashMap<Id, MouseStateHandle>, ids: &HashSet<Id>)
where
    Id: Copy + Eq + Hash,
{
    mouse_states.retain(|id, _| ids.contains(id));
    for id in ids {
        mouse_states.entry(*id).or_default();
    }
}

/// 左侧 repository/workspace 树。
///
/// 该视图只维护展示和选择状态。所有 Git、持久化和页签生命周期操作均通过
/// [`ProjectTreeEvent`] 交由窗口根处理，避免视图跨越领域边界。
pub struct ProjectTreePanel {
    project_organization_model: ModelHandle<ProjectOrganizationModel>,
    state: ProjectTreeState,
    clipped_scroll_state: ClippedScrollStateHandle,
    tab_counts: HashMap<RepositoryWorkspaceId, usize>,
    tab_nodes: HashMap<RepositoryWorkspaceId, Vec<ProjectTreeTabNode>>,
    running_workspace_ids: HashSet<RepositoryWorkspaceId>,
    tab_breathing_states: HashMap<ProjectTreeTabId, BreathingStateHandle>,
    repository_mouse_states: HashMap<RepositoryId, MouseStateHandle>,
    workspace_mouse_states: HashMap<RepositoryWorkspaceId, MouseStateHandle>,
    workspace_delete_mouse_states: HashMap<RepositoryWorkspaceId, MouseStateHandle>,
    workspace_add_tab_mouse_states: HashMap<RepositoryWorkspaceId, MouseStateHandle>,
    workspace_toggle_mouse_states: HashMap<RepositoryWorkspaceId, MouseStateHandle>,
    tab_mouse_states: HashMap<ProjectTreeTabId, MouseStateHandle>,
    tab_close_mouse_states: HashMap<ProjectTreeTabId, MouseStateHandle>,
    repository_add_workspace_mouse_states: HashMap<RepositoryId, MouseStateHandle>,
    unclassified_mouse_state: MouseStateHandle,
    add_repository_button: ViewHandle<ActionButton>,
}

impl ProjectTreePanel {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let project_organization_model = ProjectOrganizationModel::handle(ctx);
        let add_repository_button = ctx.add_view(|_| {
            ActionButton::new("Add repository", SecondaryTheme)
                .with_icon(icons::Icon::Plus)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| ctx.dispatch_typed_action(ProjectTreeAction::AddRepository))
        });

        let mut panel = Self {
            project_organization_model: project_organization_model.clone(),
            state: ProjectTreeState::default(),
            clipped_scroll_state: Default::default(),
            tab_counts: HashMap::new(),
            tab_nodes: HashMap::new(),
            running_workspace_ids: HashSet::new(),
            tab_breathing_states: HashMap::new(),
            repository_mouse_states: HashMap::new(),
            workspace_mouse_states: HashMap::new(),
            workspace_delete_mouse_states: HashMap::new(),
            workspace_add_tab_mouse_states: HashMap::new(),
            workspace_toggle_mouse_states: HashMap::new(),
            tab_mouse_states: HashMap::new(),
            tab_close_mouse_states: HashMap::new(),
            repository_add_workspace_mouse_states: HashMap::new(),
            unclassified_mouse_state: Default::default(),
            add_repository_button,
        };
        panel.refresh_tree(ctx);
        ctx.subscribe_to_model(&project_organization_model, |panel, _, _, ctx| {
            panel.refresh_tree(ctx);
        });
        panel
    }

    pub fn set_tab_counts(
        &mut self,
        tab_counts: HashMap<RepositoryWorkspaceId, usize>,
        ctx: &mut ViewContext<Self>,
    ) {
        if self.tab_counts == tab_counts {
            return;
        }
        self.tab_counts = tab_counts;
        self.refresh_tree(ctx);
    }

    pub fn set_running_workspaces(
        &mut self,
        running_workspace_ids: HashSet<RepositoryWorkspaceId>,
        ctx: &mut ViewContext<Self>,
    ) {
        if self.running_workspace_ids == running_workspace_ids {
            return;
        }
        self.running_workspace_ids = running_workspace_ids;
        ctx.notify();
    }

    /// 设置各 workspace 的页签子节点,并同步呼吸环状态。
    pub fn set_tab_nodes(
        &mut self,
        tab_nodes: HashMap<RepositoryWorkspaceId, Vec<ProjectTreeTabNode>>,
        ctx: &mut ViewContext<Self>,
    ) {
        if self.tab_nodes == tab_nodes {
            return;
        }
        self.tab_nodes = tab_nodes;
        self.refresh_tree(ctx);
    }

    fn sync_breathing_states(&mut self) {
        let breathing_ids = self
            .tab_nodes
            .values()
            .flatten()
            .filter(|tab| tab.activity.should_breathe())
            .map(|tab| tab.id)
            .collect::<HashSet<_>>();
        self.tab_breathing_states
            .retain(|id, _| breathing_ids.contains(id));
        for id in breathing_ids {
            self.tab_breathing_states.entry(id).or_default();
        }
    }

    pub fn set_active_workspace(
        &mut self,
        workspace_id: Option<RepositoryWorkspaceId>,
        ctx: &mut ViewContext<Self>,
    ) {
        if self.state.selected_workspace_id() == workspace_id {
            return;
        }
        self.state.set_active_workspace(workspace_id);
        ctx.notify();
    }

    fn refresh_tree(&mut self, ctx: &mut ViewContext<Self>) {
        let expanded_by_repository = self
            .state
            .repositories()
            .iter()
            .map(|repository| (repository.repository_id, repository.expanded))
            .collect::<HashMap<_, _>>();
        let expanded_by_workspace = self
            .state
            .repositories()
            .iter()
            .flat_map(|repository| repository.workspaces.iter())
            .map(|workspace| (workspace.workspace_id, workspace.expanded))
            .collect::<HashMap<_, _>>();
        let selected_workspace_id = self.state.selected_workspace_id();
        let repositories = self
            .project_organization_model
            .as_ref(ctx)
            .repositories()
            .cloned()
            .collect();
        let workspaces = self
            .project_organization_model
            .as_ref(ctx)
            .workspaces()
            .cloned()
            .collect();

        self.state = ProjectTreeState::from_records(
            repositories,
            workspaces,
            &self.tab_counts,
            &self.tab_nodes,
        );
        for repository in &mut self.state.repositories {
            if let Some(expanded) = expanded_by_repository.get(&repository.repository_id) {
                repository.expanded = *expanded;
            }
            for workspace in &mut repository.workspaces {
                if let Some(expanded) = expanded_by_workspace.get(&workspace.workspace_id) {
                    workspace.expanded = *expanded;
                }
            }
        }
        self.state.set_active_workspace(selected_workspace_id);

        let repository_ids = self
            .state
            .repositories()
            .iter()
            .map(|repository| repository.repository_id)
            .collect::<HashSet<_>>();
        let workspace_ids = self
            .state
            .repositories()
            .iter()
            .flat_map(|repository| repository.workspaces.iter())
            .map(|workspace| workspace.workspace_id)
            .collect::<HashSet<_>>();
        let tab_ids = self
            .tab_nodes
            .values()
            .flatten()
            .map(|tab| tab.id)
            .collect::<HashSet<_>>();
        synchronize_mouse_states(&mut self.repository_mouse_states, &repository_ids);
        synchronize_mouse_states(
            &mut self.repository_add_workspace_mouse_states,
            &repository_ids,
        );
        synchronize_mouse_states(&mut self.workspace_mouse_states, &workspace_ids);
        synchronize_mouse_states(&mut self.workspace_delete_mouse_states, &workspace_ids);
        synchronize_mouse_states(&mut self.workspace_add_tab_mouse_states, &workspace_ids);
        synchronize_mouse_states(&mut self.workspace_toggle_mouse_states, &workspace_ids);
        synchronize_mouse_states(&mut self.tab_mouse_states, &tab_ids);
        synchronize_mouse_states(&mut self.tab_close_mouse_states, &tab_ids);
        self.running_workspace_ids
            .retain(|workspace_id| workspace_ids.contains(workspace_id));
        self.tab_nodes
            .retain(|workspace_id, _| workspace_ids.contains(workspace_id));
        self.sync_breathing_states();
        ctx.notify();
    }

    fn render_header(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let title = Text::new_inline(
            "Repositories",
            appearance.ui_font_family(),
            appearance.ui_font_subheading(),
        )
        .with_color(theme.main_text_color(theme.background()).into())
        .finish();

        Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(title)
            .with_child(ChildView::new(&self.add_repository_button).finish())
            .finish()
    }

    fn render_repository_row(
        &self,
        repository: &RepositoryTreeNode,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let icon_color = if repository.expanded {
            theme.main_text_color(theme.background())
        } else {
            theme.sub_text_color(theme.background())
        };
        let chevron = if repository.expanded {
            icons::Icon::ChevronDown
        } else {
            icons::Icon::ChevronRight
        };
        let repository_id = repository.repository_id;
        let add_workspace_action = ProjectTreeAction::CreateWorkspace { repository_id };
        let add_workspace_tooltip = appearance
            .ui_builder()
            .tool_tip("Create workspace".to_string())
            .build()
            .finish();
        let add_workspace = icon_button(
            appearance,
            icons::Icon::Plus,
            false,
            self.repository_add_workspace_mouse_states
                .get(&repository_id)
                .expect("repository add-workspace mouse state should be initialized during tree refresh")
                .clone(),
        )
        .with_tooltip(move || add_workspace_tooltip)
        .build()
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(add_workspace_action.clone());
        })
        .with_cursor(Cursor::PointingHand)
        .finish();
        let add_workspace_position_id = repository_add_workspace_position_id(repository_id);
        let add_workspace = SavePosition::new(add_workspace, &add_workspace_position_id).finish();

        let workspace_count = Container::new(
            Text::new_inline(
                workspace_count_pill_label(repository.workspaces.len()),
                appearance.ui_font_family(),
                appearance.ui_font_footnote(),
            )
            .with_color(theme.sub_text_color(theme.background()).into())
            .finish(),
        )
        .with_horizontal_padding(5.)
        .with_vertical_padding(1.)
        .with_background(theme.surface_overlay_1())
        .with_border(Border::all(1.).with_border_fill(theme.surface_2()))
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)))
        .finish();

        let row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(TREE_ICON_GAP)
            .with_child(tree_row_icon(chevron, icon_color, TREE_CHEVRON_SIZE))
            .with_child(tree_row_icon(
                icons::Icon::Folder,
                icon_color,
                TREE_ROW_ICON_SIZE,
            ))
            .with_child(
                Shrinkable::new(
                    1.0,
                    Text::new_inline(
                        repository.display_name.clone(),
                        appearance.ui_font_family(),
                        appearance.ui_font_body(),
                    )
                    .with_clip(ClipConfig::ellipsis())
                    .with_color(theme.main_text_color(theme.background()).into())
                    .finish(),
                )
                .finish(),
            )
            .with_child(
                Container::new(workspace_count)
                    .with_margin_left(8.)
                    .with_margin_right(6.)
                    .finish(),
            )
            .with_child(add_workspace)
            .finish();
        let toggle_action = ProjectTreeAction::ToggleRepository { repository_id };

        Hoverable::new(
            self.repository_mouse_states
                .get(&repository_id)
                .expect("repository row mouse state should be initialized during tree refresh")
                .clone(),
            move |mouse_state| {
                let mut container = Container::new(row)
                    .with_horizontal_padding(8.)
                    .with_vertical_padding(6.)
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)));
                if mouse_state.is_hovered() {
                    container = container.with_background(theme.surface_overlay_1());
                }
                container.finish()
            },
        )
        .with_cursor(Cursor::PointingHand)
        .with_defer_events_to_children()
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(toggle_action.clone());
        })
        .finish()
    }

    fn render_workspace_running_dot(
        visual_state: WorkspaceVisualState,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let running_color: pathfinder_color::ColorU = theme.terminal_colors().normal.green.into();
        let dot = if visual_state.should_render_running_indicator() {
            Container::new(Empty::new().finish())
                .with_background(running_color)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(
                    WORKSPACE_RUNNING_DOT_SIZE / 2.,
                )))
                .with_drop_shadow(
                    DropShadow::new_with_standard_offset_and_spread(coloru_with_opacity(
                        running_color,
                        48,
                    ))
                    .with_offset(vec2f(0., 0.)),
                )
                .finish()
        } else {
            Empty::new().finish()
        };
        ConstrainedBox::new(dot)
            .with_width(WORKSPACE_RUNNING_DOT_SIZE)
            .with_height(WORKSPACE_RUNNING_DOT_SIZE)
            .finish()
    }

    fn render_workspace_activity_slot(
        &self,
        visual_state: WorkspaceVisualState,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        match visual_state.activity_slot() {
            WorkspaceActivitySlot::Empty | WorkspaceActivitySlot::Agent(_) => {
                sized_activity_slot(Empty::new().finish())
            }
            WorkspaceActivitySlot::RunningDot => {
                sized_activity_slot(Self::render_workspace_running_dot(visual_state, appearance))
            }
        }
    }

    fn render_tab_activity_slot(
        &self,
        tab: &ProjectTreeTabNode,
        visual_state: WorkspaceVisualState,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        match tab.activity {
            TabNodeActivity::Idle => tree_status_slot(
                ConstrainedBox::new(
                    Image::new(
                        AssetSource::Bundled {
                            path: ITERM_PROMPT_ICON_PATH,
                        },
                        CacheOption::BySize,
                    )
                    .finish(),
                )
                .with_width(TREE_ROW_ICON_SIZE)
                .with_height(TREE_ROW_ICON_SIZE)
                .finish(),
            ),
            TabNodeActivity::RunningDot => {
                tree_status_slot(Self::render_workspace_running_dot(visual_state, appearance))
            }
            TabNodeActivity::Agent(activity) => {
                tree_status_slot(self.render_tab_agent_avatar(tab.id, activity, appearance))
            }
        }
    }

    fn render_tab_agent_avatar(
        &self,
        tab_id: ProjectTreeTabId,
        activity: WorkspaceAgentActivity,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let avatar = render_icon_with_status(
            agent_activity_icon_variant(activity.identity, theme),
            &WORKSPACE_AGENT_ICON_SIZING,
            theme,
            theme.background(),
        );
        let ring_color = agent_activity_ring_color(activity, theme);
        let animate = activity.should_breathe();
        let handle = if animate {
            self.tab_breathing_states
                .get(&tab_id)
                .expect("InProgress agent 必须先由 sync_breathing_states 插入呼吸环 handle")
                .clone()
        } else {
            BreathingStateHandle::default()
        };
        // 透明度在 BreathingRing::paint 里按 elapsed 更新。
        // repaint_after 不会重跑 View::render,不能把 alpha 写死在 Container 上。
        let ringed_avatar = Box::new(BreathingRing::new(
            avatar,
            ring_color,
            WORKSPACE_AGENT_RING_WIDTH,
            animate,
            handle,
        ));
        // 直接把 max=16 传给带边框环; 不能包 Flex, 否则主轴 max 变无限,槽宽会变成 19。
        ConstrainedBox::new(ringed_avatar)
            .with_width(WORKSPACE_ACTIVITY_SLOT_SIZE)
            .with_height(WORKSPACE_ACTIVITY_SLOT_SIZE)
            .finish()
    }

    fn render_workspace_tab_count(
        tab_count: usize,
        visual_state: WorkspaceVisualState,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let color = if visual_state.should_render_selection_accent() {
            theme.accent().into_solid_bias_right_color()
        } else {
            theme
                .sub_text_color(theme.background())
                .into_solid_bias_right_color()
        };
        ConstrainedBox::new(
            Text::new_inline(
                tab_count_badge_label(tab_count),
                appearance.ui_font_family(),
                appearance.ui_font_footnote(),
            )
            .with_color(color.into())
            .finish(),
        )
        .with_min_width(14.)
        .finish()
    }

    fn render_workspace_row(
        &self,
        workspace: &WorkspaceTreeNode,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let selected =
            workspace_row_is_selected(self.state.selected_workspace_id(), workspace.workspace_id);
        let shows_selection_accent = workspace_row_shows_selection_accent(workspace, selected);
        let selection_accent = theme.accent();
        let selection_accent_color = selection_accent.into_solid_bias_right_color();
        let label_color = if shows_selection_accent {
            selection_accent_color
        } else {
            theme
                .main_text_color(theme.background())
                .into_solid_bias_right_color()
        };
        let metadata_color = theme.sub_text_color(theme.background());
        let workspace_id = workspace.workspace_id;
        let action = ProjectTreeAction::SelectWorkspace {
            workspace_id: Some(workspace_id),
        };
        let delete_action = ProjectTreeAction::DeleteWorkspace { workspace_id };
        let name = Text::new_inline(
            workspace.display_name.clone(),
            appearance.ui_font_family(),
            appearance.ui_font_body(),
        )
        .with_clip(ClipConfig::ellipsis())
        .with_color(label_color.into())
        .finish();
        let content = if workspace_shows_branch_subtitle(&workspace.display_name, &workspace.branch)
        {
            let branch = Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_spacing(3.)
                .with_child(
                    ConstrainedBox::new(
                        icons::Icon::GitBranch
                            .to_warpui_icon(metadata_color)
                            .finish(),
                    )
                    .with_width(12.)
                    .with_height(12.)
                    .finish(),
                )
                .with_child(
                    Shrinkable::new(
                        1.,
                        Text::new_inline(
                            workspace.branch.clone(),
                            appearance.ui_font_family(),
                            appearance.ui_font_footnote(),
                        )
                        .with_clip(ClipConfig::ellipsis())
                        .with_color(metadata_color.into())
                        .finish(),
                    )
                    .finish(),
                )
                .finish();
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Start)
                .with_spacing(1.)
                .with_child(name)
                .with_child(branch)
                .finish()
        } else {
            name
        };

        let any_child_busy = workspace.tabs.iter().any(|tab| tab.activity.is_busy());
        let parent_slot = workspace_parent_activity_slot(workspace.expanded, any_child_busy);
        let visual_state = WorkspaceVisualState::new(
            shows_selection_accent,
            matches!(parent_slot, WorkspaceActivitySlot::RunningDot),
            None,
        );
        let activity_slot = self.render_workspace_activity_slot(visual_state, appearance);
        let tab_count =
            Self::render_workspace_tab_count(workspace.tab_count, visual_state, appearance);
        let toggle_action = ProjectTreeAction::ToggleWorkspace { workspace_id };
        let chevron_icon = if workspace.expanded {
            icons::Icon::ChevronDown
        } else {
            icons::Icon::ChevronRight
        };
        let chevron = Hoverable::new(
            self.workspace_toggle_mouse_states
                .get(&workspace_id)
                .expect("workspace toggle mouse state should be initialized during tree refresh")
                .clone(),
            move |_| tree_row_icon(chevron_icon, metadata_color, TREE_CHEVRON_SIZE),
        )
        .with_cursor(Cursor::PointingHand)
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(toggle_action.clone());
        })
        .finish();
        let workspace_icon = if matches!(parent_slot, WorkspaceActivitySlot::RunningDot) {
            activity_slot
        } else {
            tree_row_icon(icons::Icon::GitBranch, metadata_color, TREE_ROW_ICON_SIZE)
        };
        let labeled_content = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(TREE_ICON_GAP)
            .with_child(chevron)
            .with_child(workspace_icon)
            .with_child(Shrinkable::new(1.0, content).finish())
            .finish();

        let delete_tooltip = appearance
            .ui_builder()
            .tool_tip("Remove workspace".to_string())
            .build()
            .finish();
        let delete = icon_button(
            appearance,
            icons::Icon::X,
            false,
            self.workspace_delete_mouse_states
                .get(&workspace_id)
                .expect("workspace delete mouse state should be initialized during tree refresh")
                .clone(),
        )
        .with_tooltip(move || delete_tooltip)
        .build()
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(delete_action.clone());
        })
        .with_cursor(Cursor::PointingHand)
        .finish();
        let delete_placeholder = ConstrainedBox::new(Empty::new().finish())
            .with_width(icons::ICON_DIMENSIONS)
            .with_height(icons::ICON_DIMENSIONS)
            .finish();
        let new_tab_action = ProjectTreeAction::NewTab { workspace_id };
        let new_tab_tooltip = appearance
            .ui_builder()
            .tool_tip("New tab".to_string())
            .build()
            .finish();
        let new_tab = icon_button(
            appearance,
            icons::Icon::Plus,
            false,
            self.workspace_add_tab_mouse_states
                .get(&workspace_id)
                .expect("workspace add-tab mouse state should be initialized during tree refresh")
                .clone(),
        )
        .with_tooltip(move || new_tab_tooltip)
        .build()
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(new_tab_action.clone());
        })
        .with_cursor(Cursor::PointingHand)
        .finish();
        let new_tab_placeholder = ConstrainedBox::new(Empty::new().finish())
            .with_width(icons::ICON_DIMENSIONS)
            .with_height(icons::ICON_DIMENSIONS)
            .finish();

        Hoverable::new(
            self.workspace_mouse_states
                .get(&workspace_id)
                .expect("workspace row mouse state should be initialized during tree refresh")
                .clone(),
            move |mouse_state| {
                let show_actions = should_show_workspace_hover_actions(mouse_state.is_hovered());
                let new_tab = if show_actions {
                    new_tab
                } else {
                    new_tab_placeholder
                };
                let delete = if show_actions {
                    delete
                } else {
                    delete_placeholder
                };
                let row_content = Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(Shrinkable::new(1.0, labeled_content).finish())
                    .with_child(
                        Flex::row()
                            .with_cross_axis_alignment(CrossAxisAlignment::Center)
                            .with_spacing(8.)
                            .with_child(tab_count)
                            .with_child(new_tab)
                            .with_child(delete)
                            .finish(),
                    )
                    .finish();
                let mut row_container = Container::new(row_content)
                    .with_horizontal_padding(8.)
                    .with_vertical_padding(5.)
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(5.)));
                if visual_state.should_render_selection_accent() {
                    row_container =
                        row_container.with_background(selection_accent.with_opacity(10));
                } else if mouse_state.is_hovered() {
                    row_container = row_container.with_background(theme.surface_overlay_2());
                } else if visual_state.should_fill_idle_row() {
                    row_container = row_container.with_background(theme.surface_overlay_1());
                }
                if visual_state.should_render_selection_frame() {
                    row_container = row_container
                        .with_border(Border::all(1.).with_border_fill(selection_accent));
                }

                row_container.finish()
            },
        )
        .with_cursor(Cursor::PointingHand)
        .with_defer_events_to_children()
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(action.clone());
        })
        .finish()
    }

    fn render_tab_row(
        &self,
        workspace_id: RepositoryWorkspaceId,
        tab: &ProjectTreeTabNode,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let selection_accent = theme.accent();
        let selection_accent_color = selection_accent.into_solid_bias_right_color();
        let visual_state = WorkspaceVisualState::new(
            tab.is_active,
            matches!(tab.activity, TabNodeActivity::RunningDot),
            tab.activity.agent(),
        );
        let label_color = if tab.is_active {
            selection_accent_color
        } else {
            theme
                .main_text_color(theme.background())
                .into_solid_bias_right_color()
        };
        let activity_slot = self.render_tab_activity_slot(tab, visual_state, appearance);
        let title = Text::new_inline(
            tab.title.clone(),
            appearance.ui_font_family(),
            appearance.ui_font_body(),
        )
        .with_clip(ClipConfig::ellipsis())
        .with_color(label_color.into())
        .finish();
        let labeled_content = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(TREE_ICON_GAP)
            .with_child(
                ConstrainedBox::new(Empty::new().finish())
                    .with_width(TAB_UNDER_WORKSPACE_INDENT)
                    .finish(),
            )
            .with_child(activity_slot)
            .with_child(Shrinkable::new(1.0, title).finish())
            .finish();

        let tab_id = tab.id;
        let select_action = ProjectTreeAction::SelectTab {
            workspace_id,
            tab_id,
        };
        let close_action = ProjectTreeAction::CloseTab {
            workspace_id,
            tab_id,
        };
        let close_tooltip = appearance
            .ui_builder()
            .tool_tip("Close tab".to_string())
            .build()
            .finish();
        let close = icon_button(
            appearance,
            icons::Icon::X,
            false,
            self.tab_close_mouse_states
                .get(&tab_id)
                .expect("tab close mouse state should be initialized during tree refresh")
                .clone(),
        )
        .with_tooltip(move || close_tooltip)
        .build()
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(close_action.clone());
        })
        .with_cursor(Cursor::PointingHand)
        .finish();
        let close_placeholder = ConstrainedBox::new(Empty::new().finish())
            .with_width(icons::ICON_DIMENSIONS)
            .with_height(icons::ICON_DIMENSIONS)
            .finish();

        Hoverable::new(
            self.tab_mouse_states
                .get(&tab_id)
                .expect("tab row mouse state should be initialized during tree refresh")
                .clone(),
            move |mouse_state| {
                let close = if mouse_state.is_hovered() {
                    close
                } else {
                    close_placeholder
                };
                let row_content = Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(Shrinkable::new(1.0, labeled_content).finish())
                    .with_child(close)
                    .finish();
                let mut row_container = Container::new(row_content)
                    .with_horizontal_padding(8.)
                    .with_vertical_padding(4.)
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(5.)));
                if visual_state.should_render_selection_accent() {
                    row_container =
                        row_container.with_background(selection_accent.with_opacity(10));
                } else if mouse_state.is_hovered() {
                    row_container = row_container.with_background(theme.surface_overlay_2());
                }

                Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(Shrinkable::new(1.0, row_container.finish()).finish())
                    .finish()
            },
        )
        .with_cursor(Cursor::PointingHand)
        .with_defer_events_to_children()
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(select_action.clone());
        })
        .finish()
    }

    fn render_unclassified_row(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let action = ProjectTreeAction::SelectWorkspace { workspace_id: None };
        let content = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                ConstrainedBox::new(
                    icons::Icon::Terminal
                        .to_warpui_icon(theme.sub_text_color(theme.background()))
                        .finish(),
                )
                .with_width(16.)
                .with_height(16.)
                .finish(),
            )
            .with_child(
                Container::new(
                    Text::new_inline(
                        "Unclassified tabs",
                        appearance.ui_font_family(),
                        appearance.ui_font_body(),
                    )
                    .with_color(theme.main_text_color(theme.background()).into())
                    .finish(),
                )
                .with_margin_left(8.)
                .finish(),
            )
            .finish();

        Hoverable::new(self.unclassified_mouse_state.clone(), move |mouse_state| {
            let mut container = Container::new(content)
                .with_horizontal_padding(8.)
                .with_vertical_padding(6.)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)));
            if mouse_state.is_hovered() {
                container = container.with_background(theme.surface_overlay_1());
            }
            container.finish()
        })
        .with_cursor(Cursor::PointingHand)
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(action.clone());
        })
        .finish()
    }
}

impl Entity for ProjectTreePanel {
    type Event = ProjectTreeEvent;
}

impl TypedActionView for ProjectTreePanel {
    type Action = ProjectTreeAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            ProjectTreeAction::AddRepository => ctx.emit(ProjectTreeEvent::AddRepositoryRequested),
            ProjectTreeAction::CreateWorkspace { repository_id } => {
                ctx.emit(ProjectTreeEvent::CreateWorkspaceRequested {
                    repository_id: *repository_id,
                });
            }
            ProjectTreeAction::DeleteWorkspace { workspace_id } => {
                ctx.emit(ProjectTreeEvent::DeleteWorkspaceRequested {
                    workspace_id: *workspace_id,
                });
            }
            ProjectTreeAction::ToggleRepository { repository_id } => {
                self.state.toggle_repository(*repository_id);
                ctx.notify();
            }
            ProjectTreeAction::SelectWorkspace { workspace_id } => {
                if let Some(workspace_id) = workspace_id {
                    self.state.select_or_toggle_workspace(*workspace_id);
                } else {
                    self.state.selected_workspace_id = None;
                }
                ctx.emit(ProjectTreeEvent::WorkspaceSelected {
                    workspace_id: *workspace_id,
                });
                ctx.notify();
            }
            ProjectTreeAction::ToggleWorkspace { workspace_id } => {
                self.state.toggle_workspace_expanded(*workspace_id);
                ctx.notify();
            }
            ProjectTreeAction::SelectTab {
                workspace_id,
                tab_id,
            } => {
                self.state.select_workspace(*workspace_id);
                ctx.emit(ProjectTreeEvent::TabSelected {
                    workspace_id: *workspace_id,
                    tab_id: *tab_id,
                });
                ctx.notify();
            }
            ProjectTreeAction::CloseTab {
                workspace_id,
                tab_id,
            } => {
                ctx.emit(ProjectTreeEvent::TabCloseRequested {
                    workspace_id: *workspace_id,
                    tab_id: *tab_id,
                });
            }
            ProjectTreeAction::NewTab { workspace_id } => {
                self.state.select_workspace(*workspace_id);
                ctx.emit(ProjectTreeEvent::NewTabRequested {
                    workspace_id: *workspace_id,
                });
                ctx.notify();
            }
            ProjectTreeAction::ReorderTabs {
                workspace_id,
                from,
                to,
            } => {
                ctx.emit(ProjectTreeEvent::TabsReordered {
                    workspace_id: *workspace_id,
                    from: *from,
                    to: *to,
                });
            }
        }
    }
}

impl View for ProjectTreePanel {
    fn ui_name() -> &'static str {
        "ProjectTreePanel"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let mut tree = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(REPOSITORY_GROUP_SPACING);
        for repository in self.state.repositories() {
            let mut repository_group = Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(self.render_repository_row(repository, appearance));
            if repository.expanded && !repository.workspaces.is_empty() {
                let mut workspaces = Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_spacing(0.);
                for workspace in &repository.workspaces {
                    workspaces.add_child(self.render_workspace_row(workspace, appearance));
                    if workspace.expanded {
                        for tab in &workspace.tabs {
                            workspaces.add_child(self.render_tab_row(
                                workspace.workspace_id,
                                tab,
                                appearance,
                            ));
                        }
                    }
                }
                repository_group.add_child(
                    Container::new(workspaces.finish())
                        .with_margin_left(WORKSPACE_GROUP_INDENT)
                        .with_margin_top(2.)
                        .finish(),
                );
            }
            tree.add_child(repository_group.finish());
        }

        let body: Box<dyn Element> = if self.state.repositories().is_empty() {
            Container::new(
                Text::new_inline(
                    "Add a local Git repository to create workspaces.",
                    appearance.ui_font_family(),
                    appearance.ui_font_body(),
                )
                .with_color(
                    appearance
                        .theme()
                        .sub_text_color(appearance.theme().background())
                        .into(),
                )
                .finish(),
            )
            .with_uniform_padding(8.)
            .finish()
        } else {
            Container::new(tree.finish())
                .with_horizontal_padding(4.)
                .with_vertical_padding(6.)
                .finish()
        };
        let scrollable_body = ClippedScrollable::vertical(
            self.clipped_scroll_state.clone(),
            body,
            ScrollbarWidth::Auto,
            theme.disabled_text_color(theme.background()).into(),
            theme.main_text_color(theme.background()).into(),
            Fill::None,
        )
        .finish();

        Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(
                Container::new(self.render_header(appearance))
                    .with_horizontal_padding(12.)
                    .with_vertical_padding(10.)
                    .with_border(Border::bottom(1.).with_border_fill(theme.surface_2()))
                    .finish(),
            )
            .with_child(Shrinkable::new(1.0, scrollable_body).finish())
            .with_child(
                Container::new(self.render_unclassified_row(appearance))
                    .with_uniform_padding(8.)
                    .with_border(Border::top(1.).with_border_fill(theme.surface_2()))
                    .finish(),
            )
            .finish()
    }
}

#[cfg(test)]
#[path = "project_tree_tests.rs"]
mod tests;
