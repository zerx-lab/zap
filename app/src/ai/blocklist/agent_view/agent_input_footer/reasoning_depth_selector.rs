//! 输入框右下角"思考深度"picker(BYOP 模式)。
//!
//! 与 `EnvironmentSelector` 同模板,简化为:
//! - 数据源:`LLMPreferences::get_reasoning_effort(...)` + 当前选中模型的 variants 表
//! - 状态:`LLMPreferences::reasoning_effort_per_terminal`(session-only)
//! - 不写 settings.toml,不发 telemetry,不接 cloud
//! - 当前模型不支持 reasoning(variants 为空)→ 整个组件渲染空,picker 自然消失

use pathfinder_color::ColorU;
use pathfinder_geometry::vector::vec2f;
use warp_core::ui::color::blend::Blend;
use warp_core::ui::theme::Fill;
use warpui::{
    elements::{
        ChildAnchor, ChildView, ConstrainedBox, OffsetPositioning, ParentAnchor, ParentElement,
        ParentOffsetBounds, Stack,
    },
    AppContext, Element, Entity, EntityId, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle,
};

use std::sync::Arc;

use crate::{
    ai::{
        agent_providers::reasoning::model_reasoning_variants,
        llms::{LLMPreferences, LLMPreferencesEvent},
    },
    appearance::Appearance,
    context_chips::display_menu::{
        ChipMenuType, DisplayChipMenu, GenericMenuItem, PromptDisplayMenuEvent,
    },
    settings::{AgentProviderApiType, ReasoningEffortSetting},
    terminal::input::{MenuPositioning, MenuPositioningProvider},
    ui_components::icons::Icon,
    view_components::action_button::{ActionButton, ActionButtonTheme, ButtonSize},
};

use super::AgentInputButtonTheme;

/// 输入框 toolbar 的"Reasoning Depth"选择器。
pub struct ReasoningDepthSelector {
    button: ViewHandle<ActionButton>,
    dropdown: ViewHandle<DisplayChipMenu>,
    is_menu_open: bool,
    menu_positioning_provider: Arc<dyn MenuPositioningProvider>,
    terminal_view_id: EntityId,
    /// 当前 picker 关联的 (api_type, model_id);随 LLMPreferences 变化刷新。
    current_target: Option<(AgentProviderApiType, String)>,
}

pub enum ReasoningDepthSelectorEvent {
    MenuVisibilityChanged { open: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReasoningDepthSelectorAction {
    ToggleMenu,
}

#[derive(Debug, Clone)]
struct ReasoningDepthMenuItem {
    effort: ReasoningEffortSetting,
    is_selected: bool,
}

const ITEM_CHECK_ICON_SIZE: f32 = 16.;

impl GenericMenuItem for ReasoningDepthMenuItem {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> String {
        self.effort.display_name().to_owned()
    }

    fn icon(&self, _app: &AppContext) -> Option<Icon> {
        None
    }

    fn action_data(&self) -> String {
        format!("{:?}", self.effort)
    }

    fn right_side_element(&self, app: &AppContext) -> Option<Box<dyn Element>> {
        if !self.is_selected {
            return None;
        }
        let theme = Appearance::as_ref(app).theme();
        let color = theme.main_text_color(theme.surface_2()).into_solid();
        Some(
            ConstrainedBox::new(Icon::Check.to_warpui_icon(Fill::Solid(color)).finish())
                .with_width(ITEM_CHECK_ICON_SIZE)
                .with_height(ITEM_CHECK_ICON_SIZE)
                .finish(),
        )
    }
}

impl ReasoningDepthSelector {
    pub fn new(
        menu_positioning_provider: Arc<dyn MenuPositioningProvider>,
        terminal_view_id: EntityId,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        let button = ctx.add_typed_action_view(|_ctx| {
            ActionButton::new("", AgentInputButtonTheme)
                .with_icon(Icon::Stars)
                .with_tooltip("Reasoning depth")
                .with_size(ButtonSize::AgentInputButton)
                .with_disabled_theme(DisabledTheme)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(ReasoningDepthSelectorAction::ToggleMenu);
                })
        });

        let dropdown = ctx.add_typed_action_view(move |ctx| {
            // 用 CodeReview 风格(无 search input、无 environment sidecar、视觉紧凑)。
            // 不能用 Environments — 它的 sidecar 路径会把 action_data() 喂给
            // `ServerId::from_string_lossy`,我们的 effort 名字(如 "Auto"/"Off")
            // 不是 22 字符,debug build 触发 panic。
            DisplayChipMenu::new(
                Vec::<ReasoningDepthMenuItem>::new(),
                None,
                ChipMenuType::CodeReview,
                ctx,
            )
        });

        ctx.subscribe_to_view(&dropdown, |me, _, event, ctx| match event {
            PromptDisplayMenuEvent::MenuAction(generic_event) => {
                if let Some(item) = generic_event
                    .action_item
                    .as_any()
                    .downcast_ref::<ReasoningDepthMenuItem>()
                {
                    me.apply_selection(item.effort, ctx);
                    me.set_menu_visibility(false, ctx);
                }
            }
            PromptDisplayMenuEvent::CloseMenu => {
                me.set_menu_visibility(false, ctx);
            }
        });

        ctx.subscribe_to_model(
            &LLMPreferences::handle(ctx),
            |me, _, event, ctx| match event {
                LLMPreferencesEvent::UpdatedAvailableLLMs
                | LLMPreferencesEvent::UpdatedActiveAgentModeLLM
                | LLMPreferencesEvent::UpdatedReasoningEffort => {
                    me.refresh(ctx);
                }
                LLMPreferencesEvent::UpdatedActiveCodingLLM => {}
            },
        );

        let mut me = Self {
            button,
            dropdown,
            is_menu_open: false,
            menu_positioning_provider,
            terminal_view_id,
            current_target: None,
        };
        me.refresh(ctx);
        me
    }

    pub fn is_menu_open(&self) -> bool {
        self.is_menu_open
    }

    fn set_menu_visibility(&mut self, is_open: bool, ctx: &mut ViewContext<Self>) {
        if self.is_menu_open == is_open {
            return;
        }
        self.is_menu_open = is_open;
        if is_open {
            ctx.focus(&self.dropdown);
        }
        ctx.emit(ReasoningDepthSelectorEvent::MenuVisibilityChanged { open: is_open });
        ctx.notify();
    }

    fn apply_selection(&mut self, effort: ReasoningEffortSetting, ctx: &mut ViewContext<Self>) {
        let Some((api_type, model_id)) = self.current_target.clone() else {
            return;
        };
        let terminal_view_id = self.terminal_view_id;
        LLMPreferences::handle(ctx).update(ctx, |prefs, ctx| {
            prefs.set_reasoning_effort(terminal_view_id, api_type, &model_id, effort, ctx);
        });
    }

    /// 解析当前选中模型 → 若是 BYOP 模型解出 (api_type, model_id),否则 None。
    fn resolve_current_target(&self, ctx: &AppContext) -> Option<(AgentProviderApiType, String)> {
        let prefs = LLMPreferences::as_ref(ctx);
        let llm_id = prefs
            .get_active_base_model(ctx, Some(self.terminal_view_id))
            .id
            .clone();
        let (provider, _api_key, model_id) = crate::ai::agent_providers::lookup_byop(ctx, &llm_id)?;
        Some((provider.api_type, model_id))
    }

    fn refresh(&mut self, ctx: &mut ViewContext<Self>) {
        let target = self.resolve_current_target(ctx);
        self.current_target = target.clone();

        // Variants 为空 → 整个组件后续 render 走空
        let (variants, current_effort) = match target.as_ref() {
            Some((api_type, model_id)) => {
                let v = model_reasoning_variants(*api_type, model_id);
                let cur = LLMPreferences::as_ref(ctx).get_reasoning_effort(
                    Some(self.terminal_view_id),
                    *api_type,
                    model_id,
                );
                (v, cur)
            }
            None => (Vec::new(), ReasoningEffortSetting::Auto),
        };

        // 刷新 dropdown items
        let menu_items: Vec<ReasoningDepthMenuItem> = variants
            .iter()
            .map(|v| ReasoningDepthMenuItem {
                effort: *v,
                is_selected: *v == current_effort,
            })
            .collect();
        self.dropdown.update(ctx, |menu, ctx| {
            menu.update_menu_items(menu_items, ctx);
        });

        // 刷新按钮 label / disabled
        let label = if variants.is_empty() {
            String::new()
        } else {
            current_effort.display_name().to_owned()
        };
        let disabled = variants.is_empty();
        self.button.update(ctx, |button, ctx| {
            button.set_label(label, ctx);
            button.set_disabled(disabled, ctx);
        });

        ctx.notify();
    }

    /// 当前是否应渲染(模型支持 reasoning)。
    fn is_visible(&self) -> bool {
        match &self.current_target {
            Some((api_type, model_id)) => !model_reasoning_variants(*api_type, model_id).is_empty(),
            None => false,
        }
    }

    fn get_menu_positioning(&self, app: &AppContext) -> OffsetPositioning {
        match self.menu_positioning_provider.menu_position(app) {
            MenuPositioning::BelowInputBox => OffsetPositioning::offset_from_parent(
                vec2f(0., 4.),
                ParentOffsetBounds::WindowByPosition,
                ParentAnchor::BottomLeft,
                ChildAnchor::TopLeft,
            ),
            MenuPositioning::AboveInputBox => OffsetPositioning::offset_from_parent(
                vec2f(0., -4.),
                ParentOffsetBounds::WindowByPosition,
                ParentAnchor::TopLeft,
                ChildAnchor::BottomLeft,
            ),
        }
    }
}

impl TypedActionView for ReasoningDepthSelector {
    type Action = ReasoningDepthSelectorAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            ReasoningDepthSelectorAction::ToggleMenu => {
                if self.is_visible() {
                    self.set_menu_visibility(!self.is_menu_open, ctx);
                }
            }
        }
    }
}

impl View for ReasoningDepthSelector {
    fn ui_name() -> &'static str {
        "ReasoningDepthSelector"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        if !self.is_visible() {
            // 不支持 reasoning 的模型 → 渲染空
            return Stack::new().finish();
        }

        let mut stack = Stack::new();
        stack.add_child(ChildView::new(&self.button).finish());

        if self.is_menu_open {
            let menu = ChildView::new(&self.dropdown).finish();
            let positioning = self.get_menu_positioning(app);
            stack.add_positioned_overlay_child(menu, positioning);
        }

        stack.finish()
    }
}

impl Entity for ReasoningDepthSelector {
    type Event = ReasoningDepthSelectorEvent;
}

struct DisabledTheme;

impl ActionButtonTheme for DisabledTheme {
    fn background(&self, hovered: bool, appearance: &Appearance) -> Option<Fill> {
        AgentInputButtonTheme.background(hovered, appearance)
    }

    fn text_color(
        &self,
        _hovered: bool,
        background: Option<Fill>,
        appearance: &Appearance,
    ) -> ColorU {
        let base_bg = appearance.theme().surface_1();
        let effective_bg = match background {
            Some(overlay) => base_bg.blend(&overlay),
            None => base_bg,
        };
        appearance
            .theme()
            .disabled_text_color(effective_bg)
            .into_solid()
    }

    fn border(&self, appearance: &Appearance) -> Option<ColorU> {
        AgentInputButtonTheme.border(appearance)
    }

    fn should_opt_out_of_contrast_adjustment(&self) -> bool {
        AgentInputButtonTheme.should_opt_out_of_contrast_adjustment()
    }
}
