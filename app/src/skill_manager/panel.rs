use std::collections::HashSet;
use std::path::PathBuf;

use ai::skills::SkillProvider;
use warp_core::ui::appearance::Appearance;
use warp_core::ui::theme::color::internal_colors;
use warpui::{
    elements::{
        Border, ChildView, ClippedScrollStateHandle, ClippedScrollable, ConstrainedBox, Container,
        CornerRadius, CrossAxisAlignment, Element, Fill as ElementFill, Flex, Hoverable,
        MainAxisSize, MouseStateHandle, ParentElement, Radius, ScrollbarWidth, Shrinkable, Text,
    },
    platform::Cursor,
    text_layout::ClipConfig,
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use crate::ai::skills::{
    SkillInventoryDuplicate, SkillInventoryItem, SkillManager, SkillManagerEvent,
};
use crate::editor::{
    EditorOptions, EditorView, Event as EditorEvent, PropagateAndNoOpNavigationKeys,
    PropagateHorizontalNavigationKeys, TextOptions,
};

const PANEL_PADDING: f32 = 8.0;
const ROW_PADDING_VERTICAL: f32 = 5.0;
const ROW_PADDING_HORIZONTAL: f32 = 8.0;
const FONT_SIZE: f32 = 13.0;
const META_FONT_SIZE: f32 = 11.0;
const FILTER_BUTTON_HEIGHT: f32 = 24.0;
const PREVIEW_MIN_HEIGHT: f32 = 180.0;
const FILTER_LABEL_WIDTH: f32 = 44.0;
const FILTER_BUTTON_MIN_WIDTH: f32 = 52.0;

#[derive(Clone, Debug)]
pub enum SkillManagerPanelAction {
    SelectSkill(PathBuf),
    SelectProviderFilter(Option<SkillProvider>),
    EditSkill(PathBuf),
    EditSelected,
}

#[derive(Clone, Debug)]
pub enum SkillManagerPanelEvent {
    OpenSkillFile { path: PathBuf },
}

pub struct SkillManagerPanel {
    selected_path: Option<PathBuf>,
    provider_filter: Option<SkillProvider>,
    query_editor: ViewHandle<EditorView>,
    list_scroll_state: ClippedScrollStateHandle,
    preview_scroll_state: ClippedScrollStateHandle,
}

impl SkillManagerPanel {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let query_editor = ctx.add_typed_action_view(|ctx| {
            let options = EditorOptions {
                text: TextOptions::ui_text(Some(FONT_SIZE), Appearance::as_ref(ctx)),
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

        ctx.subscribe_to_view(&query_editor, |me, _handle, event, ctx| {
            if matches!(
                event,
                EditorEvent::Edited(_)
                    | EditorEvent::BufferReplaced
                    | EditorEvent::BufferReinitialized
            ) {
                me.selected_path = None;
                ctx.notify();
            }
        });

        let skill_manager = SkillManager::handle(ctx);
        ctx.subscribe_to_model(&skill_manager, |me, _manager, event, ctx| match event {
            SkillManagerEvent::InventoryChanged => {
                me.selected_path = None;
                ctx.notify();
            }
        });

        Self {
            selected_path: None,
            provider_filter: None,
            query_editor,
            list_scroll_state: ClippedScrollStateHandle::default(),
            preview_scroll_state: ClippedScrollStateHandle::default(),
        }
    }

    fn query(&self, app: &AppContext) -> String {
        self.query_editor
            .as_ref(app)
            .buffer_text(app)
            .trim()
            .to_lowercase()
    }

    fn filtered_items(&self, app: &AppContext) -> Vec<SkillInventoryItem> {
        let query = self.query(app);
        SkillManager::as_ref(app)
            .list_skill_inventory(app)
            .into_iter()
            .filter_map(|item| {
                let duplicates = item
                    .duplicates
                    .into_iter()
                    .filter(|duplicate| {
                        self.provider_filter
                            .is_none_or(|provider| duplicate.provider == provider)
                            && (query.is_empty()
                                || duplicate.name.to_lowercase().contains(&query)
                                || duplicate.description.to_lowercase().contains(&query)
                                || duplicate
                                    .path
                                    .display()
                                    .to_string()
                                    .to_lowercase()
                                    .contains(&query))
                    })
                    .collect::<Vec<_>>();

                let default_skill = duplicates.first()?.clone();
                Some(SkillInventoryItem {
                    name: item.name,
                    default_skill,
                    duplicates,
                })
            })
            .collect()
    }

    fn selected_duplicate(&self, app: &AppContext) -> Option<SkillInventoryDuplicate> {
        let selected_path = self.selected_path.as_ref()?;
        SkillManager::as_ref(app)
            .list_skill_inventory(app)
            .into_iter()
            .flat_map(|item| item.duplicates.into_iter())
            .find(|duplicate| &duplicate.path == selected_path)
    }

    fn fallback_duplicate(&self, app: &AppContext) -> Option<SkillInventoryDuplicate> {
        self.filtered_items(app)
            .into_iter()
            .flat_map(|item| item.duplicates.into_iter())
            .next()
    }

    fn active_duplicate(&self, app: &AppContext) -> Option<SkillInventoryDuplicate> {
        self.selected_duplicate(app)
            .or_else(|| self.fallback_duplicate(app))
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

    fn render_filter_button(
        label: String,
        is_active: bool,
        state: MouseStateHandle,
        action: SkillManagerPanelAction,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let text_color = if is_active {
            theme.main_text_color(theme.background())
        } else {
            theme.sub_text_color(theme.background())
        };
        let background = is_active.then(|| internal_colors::fg_overlay_3(theme));
        let label_el = Self::render_label(label, appearance, META_FONT_SIZE, text_color);

        Hoverable::new(state, move |mouse| {
            let mut button = Container::new(label_el)
                .with_padding_left(8.0)
                .with_padding_right(8.0)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)));
            if let Some(background) = background {
                button = button.with_background(background);
            }
            if mouse.is_hovered() && !is_active {
                button = button.with_background(internal_colors::fg_overlay_2(theme));
            }
            ConstrainedBox::new(button.finish())
                .with_height(FILTER_BUTTON_HEIGHT)
                .with_min_width(FILTER_BUTTON_MIN_WIDTH)
                .finish()
        })
        .with_cursor(Cursor::PointingHand)
        .on_mouse_down(move |ctx, _, _| {
            ctx.dispatch_typed_action(action.clone());
        })
        .finish()
    }

    fn render_filter_group<T>(
        label: String,
        values: Vec<T>,
        is_all_active: bool,
        is_active: impl Fn(T) -> bool,
        action_for: impl Fn(Option<T>) -> SkillManagerPanelAction,
        appearance: &Appearance,
    ) -> Box<dyn Element>
    where
        T: Copy + ToString + 'static,
    {
        let theme = appearance.theme();
        let mut row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(4.0)
            .with_child(
                ConstrainedBox::new(Self::render_label(
                    label,
                    appearance,
                    META_FONT_SIZE,
                    theme.sub_text_color(theme.background()),
                ))
                .with_width(FILTER_LABEL_WIDTH)
                .finish(),
            )
            .with_child(Self::render_filter_button(
                crate::t!("skill-manager-filter-all"),
                is_all_active,
                MouseStateHandle::default(),
                action_for(None),
                appearance,
            ));

        for value in values {
            row.add_child(Self::render_filter_button(
                value.to_string(),
                is_active(value),
                MouseStateHandle::default(),
                action_for(Some(value)),
                appearance,
            ));
        }

        row.finish()
    }

    fn render_filter_rows(&self, app: &AppContext, appearance: &Appearance) -> Box<dyn Element> {
        let inventory = SkillManager::as_ref(app).list_skill_inventory(app);
        let mut providers = inventory
            .iter()
            .flat_map(|item| item.duplicates.iter().map(|duplicate| duplicate.provider))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        providers.sort_by_key(|provider| provider.to_string());

        Self::render_filter_group(
            crate::t!("skill-manager-filter-provider"),
            providers,
            self.provider_filter.is_none(),
            |provider| self.provider_filter == Some(provider),
            SkillManagerPanelAction::SelectProviderFilter,
            appearance,
        )
    }

    fn render_skill_row(
        &self,
        duplicate: &SkillInventoryDuplicate,
        is_selected: bool,
        is_default: bool,
        has_duplicates: bool,
        state: MouseStateHandle,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let path = duplicate.path.display().to_string();
        let mut meta = format!("{} · {}", duplicate.provider, duplicate.scope);
        if has_duplicates {
            if is_default {
                meta.push_str(" · ");
                meta.push_str(&crate::t!("skill-manager-meta-default"));
            } else {
                meta.push_str(" · ");
                meta.push_str(&crate::t!("skill-manager-meta-duplicate"));
            }
        }

        let title = Self::render_label(
            duplicate.name.clone(),
            appearance,
            FONT_SIZE,
            theme.main_text_color(theme.background()),
        );
        let description = Self::render_label(
            duplicate.description.clone(),
            appearance,
            META_FONT_SIZE,
            theme.sub_text_color(theme.background()),
        );
        let meta = Self::render_label(
            meta,
            appearance,
            META_FONT_SIZE,
            theme.sub_text_color(theme.background()),
        );
        let path = Self::render_label(
            path,
            appearance,
            META_FONT_SIZE,
            theme.sub_text_color(theme.background()),
        );

        let action = SkillManagerPanelAction::EditSkill(duplicate.path.clone());
        Hoverable::new(state, move |mouse| {
            let background = if is_selected {
                Some(internal_colors::fg_overlay_3(theme))
            } else if mouse.is_hovered() {
                Some(internal_colors::fg_overlay_2(theme))
            } else {
                None
            };
            let mut row = Container::new(
                Flex::column()
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_spacing(2.0)
                    .with_child(title)
                    .with_child(description)
                    .with_child(meta)
                    .with_child(path)
                    .finish(),
            )
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
        .with_cursor(Cursor::PointingHand)
        .on_mouse_down(move |ctx, _, _| {
            ctx.dispatch_typed_action(action.clone());
        })
        .finish()
    }

    fn render_skill_list(
        &self,
        items: &[SkillInventoryItem],
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        if items.is_empty() {
            return Container::new(Self::render_label(
                crate::t!("skill-manager-empty"),
                appearance,
                FONT_SIZE,
                appearance
                    .theme()
                    .sub_text_color(appearance.theme().background()),
            ))
            .with_uniform_padding(12.0)
            .finish();
        }

        let mut rows = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(2.0);
        for item in items {
            let has_duplicates = item.has_duplicates();
            for duplicate in &item.duplicates {
                let is_default = duplicate.path == item.default_skill.path;
                let is_selected = self
                    .selected_path
                    .as_ref()
                    .is_some_and(|path| path == &duplicate.path);
                rows.add_child(self.render_skill_row(
                    duplicate,
                    is_selected,
                    is_default,
                    has_duplicates,
                    MouseStateHandle::default(),
                    appearance,
                ));
            }
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

    fn render_preview(
        &self,
        duplicate: Option<&SkillInventoryDuplicate>,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let Some(duplicate) = duplicate else {
            return Container::new(Self::render_label(
                crate::t!("skill-manager-preview-empty"),
                appearance,
                FONT_SIZE,
                theme.sub_text_color(theme.background()),
            ))
            .with_uniform_padding(12.0)
            .finish();
        };

        let header = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Shrinkable::new(
                    1.0,
                    Self::render_label(
                        duplicate.path.display().to_string(),
                        appearance,
                        META_FONT_SIZE,
                        theme.sub_text_color(theme.background()),
                    ),
                )
                .finish(),
            )
            .finish();

        let preview_text = Text::new_inline(
            duplicate.content.clone(),
            appearance.monospace_font_family(),
            12.0,
        )
        .with_color(theme.main_text_color(theme.background()).into())
        .finish();
        let preview_body = Container::new(preview_text)
            .with_uniform_padding(8.0)
            .with_background(theme.surface_2())
            .with_border(Border::all(1.0).with_border_color(theme.surface_3().into()))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)))
            .finish();
        let preview_scroll = ClippedScrollable::vertical(
            self.preview_scroll_state.clone(),
            preview_body,
            ScrollbarWidth::Auto,
            theme.disabled_text_color(theme.background()).into(),
            theme.main_text_color(theme.background()).into(),
            ElementFill::None,
        )
        .with_overlayed_scrollbar()
        .finish();

        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(6.0)
            .with_child(header)
            .with_child(
                ConstrainedBox::new(preview_scroll)
                    .with_height(PREVIEW_MIN_HEIGHT)
                    .finish(),
            )
            .finish()
    }
}

impl TypedActionView for SkillManagerPanel {
    type Action = SkillManagerPanelAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            SkillManagerPanelAction::SelectSkill(path) => {
                self.selected_path = Some(path.clone());
                ctx.notify();
            }
            SkillManagerPanelAction::SelectProviderFilter(provider) => {
                self.provider_filter = *provider;
                self.selected_path = None;
                ctx.notify();
            }
            SkillManagerPanelAction::EditSkill(path) => {
                self.selected_path = Some(path.clone());
                ctx.emit(SkillManagerPanelEvent::OpenSkillFile { path: path.clone() });
                ctx.notify();
            }
            SkillManagerPanelAction::EditSelected => {
                if let Some(path) = &self.selected_path {
                    ctx.emit(SkillManagerPanelEvent::OpenSkillFile { path: path.clone() });
                }
            }
        }
    }
}

impl View for SkillManagerPanel {
    fn ui_name() -> &'static str {
        "SkillManagerPanel"
    }

    fn on_focus(&mut self, _focus_ctx: &warpui::FocusContext, ctx: &mut ViewContext<Self>) {
        ctx.focus(&self.query_editor);
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let items = self.filtered_items(app);
        let active_duplicate = self.active_duplicate(app);
        let search = Container::new(ChildView::new(&self.query_editor).finish())
            .with_uniform_padding(6.0)
            .with_background(appearance.theme().surface_2())
            .with_border(Border::all(1.0).with_border_color(appearance.theme().surface_3().into()))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)))
            .finish();

        Container::new(
            Flex::column()
                .with_main_axis_size(MainAxisSize::Max)
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_spacing(8.0)
                .with_child(search)
                .with_child(self.render_filter_rows(app, appearance))
                .with_child(
                    Shrinkable::new(1.0, self.render_skill_list(&items, appearance)).finish(),
                )
                .with_child(self.render_preview(active_duplicate.as_ref(), appearance))
                .finish(),
        )
        .with_uniform_padding(PANEL_PADDING)
        .finish()
    }
}

impl Entity for SkillManagerPanel {
    type Event = SkillManagerPanelEvent;
}
