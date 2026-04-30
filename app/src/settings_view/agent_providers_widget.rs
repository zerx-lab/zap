//! 自定义 Agent Provider 设置面板 widget。
//!
//! UI 形态:
//! - Sub-header + 简短说明
//! - 每条 provider 一张卡片,卡片内含:
//!     · `Name` / `Base URL` / `API Key` 三个输入框 (失焦/Enter 时保存)
//!     · 模型列表区: 表头 `显示名 | 模型 ID`,每行两个输入框 + `×` 删除按钮
//!     · 底部按钮行: `+ 添加模型` `Fetch from API` `Remove` (provider)
//! - 底部 "Add OpenAI-compatible provider" 整宽按钮
//!
//! 当 provider 列表大小或某条 provider 的 models 数量变化时,
//! `AISettingsPageView::rebuild_current_page` 会被触发以重建整个 widget,
//! 从而让新增/删除的条目获得自己的 EditorView handle。
//! `rebuild_current_page` 内部会复用旧 PageType 的 vertical scroll handle,
//! 滚动位置不会被重置。
//!
//! provider 元数据(name/base_url/models) 走 `settings.toml`,
//! `api_key` 走 OS keychain (`AgentProviderSecrets`)。

use std::cell::RefCell;
use std::collections::HashMap;

use settings::Setting;
use warpui::elements::{
    ChildView, Container, CornerRadius, CrossAxisAlignment, Expanded, Flex, MainAxisAlignment,
    MouseStateHandle, ParentElement, Radius, Text, Wrap,
};
use warpui::ui_components::{
    button::ButtonVariant,
    components::{Coords, UiComponent, UiComponentStyles},
};
use warpui::{AppContext, Element, SingletonEntity, ViewContext, ViewHandle};

use crate::ai::agent_providers::AgentProviderSecrets;
use crate::appearance::Appearance;
use crate::editor::{
    EditorView, Event as EditorEvent, SingleLineEditorOptions, TextColors, TextOptions,
};
use crate::settings::{AISettings, AgentProvider, AgentProviderModel};

use super::ai_page::{AISettingsPageAction, AISettingsPageView};
use super::settings_page::{
    build_sub_header, render_full_pane_width_ai_button, SettingsWidget, HEADER_PADDING,
};

const CARD_BUTTON_FONT_SIZE: f32 = 12.0;
const CARD_BUTTON_PADDING: f32 = 6.0;
const FIELD_LABEL_MARGIN_TOP: f32 = 6.0;
const FIELD_LABEL_MARGIN_BOTTOM: f32 = 2.0;
const MODEL_ROW_GAP: f32 = 6.0;

/// 一条模型条目(name + id + context + output)的可编辑 view handle。
struct ModelRow {
    name_editor: ViewHandle<EditorView>,
    id_editor: ViewHandle<EditorView>,
    context_editor: ViewHandle<EditorView>,
    output_editor: ViewHandle<EditorView>,
    remove_button_state: MouseStateHandle,
}

/// 一条 provider 行的所有可编辑 view handle。
struct ProviderRow {
    name_editor: ViewHandle<EditorView>,
    base_url_editor: ViewHandle<EditorView>,
    api_key_editor: ViewHandle<EditorView>,
    fetch_button_state: MouseStateHandle,
    sync_models_dev_button_state: MouseStateHandle,
    remove_button_state: MouseStateHandle,
    add_model_button_state: MouseStateHandle,
    model_rows: Vec<ModelRow>,
}

/// 自定义 Agent Provider 设置 widget。
pub(super) struct AgentProvidersWidget {
    add_button_state: MouseStateHandle,
    refresh_catalog_button_state: MouseStateHandle,
    expand_chips_button_state: MouseStateHandle,
    /// 快速添加 chip 行的搜索框。
    search_editor: ViewHandle<EditorView>,
    /// 每个 catalog provider id 一个按钮 state — chip 行使用。
    quick_add_button_states: RefCell<HashMap<String, MouseStateHandle>>,
    rows: RefCell<HashMap<String, ProviderRow>>,
}

impl AgentProvidersWidget {
    pub(super) fn new(ctx: &mut ViewContext<AISettingsPageView>) -> Self {
        let providers = AISettings::as_ref(ctx).agent_providers.value().clone();
        let mut rows = HashMap::with_capacity(providers.len());
        for provider in &providers {
            let row = Self::build_row(provider, ctx);
            rows.insert(provider.id.clone(), row);
        }

        // 进入页面即触发一次目录加载(磁盘缓存 + 必要时网络)。
        ctx.dispatch_typed_action_deferred(AISettingsPageAction::EnsureModelsDevLoaded);

        // ---- 搜索框 ----
        let initial_query =
            crate::ai::agent_providers::models_dev::search_query();
        let search_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("搜索提供商…", ctx);
            if !initial_query.is_empty() {
                editor.set_buffer_text(&initial_query, ctx);
            }
            editor
        });
        ctx.subscribe_to_view(&search_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Edited(_)) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                ctx.dispatch_typed_action_deferred(
                    AISettingsPageAction::SetModelsDevSearchQuery(buffer_text),
                );
            }
        });

        Self {
            add_button_state: MouseStateHandle::default(),
            refresh_catalog_button_state: MouseStateHandle::default(),
            expand_chips_button_state: MouseStateHandle::default(),
            search_editor,
            quick_add_button_states: RefCell::new(HashMap::new()),
            rows: RefCell::new(rows),
        }
    }

    /// 构造单条模型行的 EditorView 与订阅。
    fn build_model_row(
        provider_id: &str,
        model_index: usize,
        model: &AgentProviderModel,
        ctx: &mut ViewContext<AISettingsPageView>,
    ) -> ModelRow {
        // ---- name 编辑器 ----
        let initial_name = model.name.clone();
        let name_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("显示名(例如: DS-V3 通用)", ctx);
            if !initial_name.is_empty() {
                editor.set_buffer_text(&initial_name, ctx);
            }
            editor
        });
        let provider_id_for_name = provider_id.to_owned();
        ctx.subscribe_to_view(&name_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                ctx.dispatch_typed_action_deferred(
                    AISettingsPageAction::UpdateAgentProviderModelName {
                        provider_id: provider_id_for_name.clone(),
                        model_index,
                        name: buffer_text,
                    },
                );
            }
        });

        // ---- id 编辑器 ----
        let initial_id = model.id.clone();
        let id_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("模型 ID(发给 API 的 model 字段, 如: deepseek-chat)", ctx);
            if !initial_id.is_empty() {
                editor.set_buffer_text(&initial_id, ctx);
            }
            editor
        });
        let provider_id_for_id = provider_id.to_owned();
        ctx.subscribe_to_view(&id_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                ctx.dispatch_typed_action_deferred(
                    AISettingsPageAction::UpdateAgentProviderModelId {
                        provider_id: provider_id_for_id.clone(),
                        model_index,
                        id: buffer_text,
                    },
                );
            }
        });

        // ---- context_window 编辑器(数字,空 = 0 = 未指定) ----
        let initial_context = if model.context_window == 0 {
            String::new()
        } else {
            model.context_window.to_string()
        };
        let context_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("上下文 (tokens)", ctx);
            if !initial_context.is_empty() {
                editor.set_buffer_text(&initial_context, ctx);
            }
            editor
        });
        let provider_id_for_ctx = provider_id.to_owned();
        ctx.subscribe_to_view(&context_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                let value = parse_token_count(&buffer_text);
                ctx.dispatch_typed_action_deferred(
                    AISettingsPageAction::UpdateAgentProviderModelContextWindow {
                        provider_id: provider_id_for_ctx.clone(),
                        model_index,
                        context_window: value,
                    },
                );
            }
        });

        // ---- max_output_tokens 编辑器 ----
        let initial_output = if model.max_output_tokens == 0 {
            String::new()
        } else {
            model.max_output_tokens.to_string()
        };
        let output_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("输出 (tokens)", ctx);
            if !initial_output.is_empty() {
                editor.set_buffer_text(&initial_output, ctx);
            }
            editor
        });
        let provider_id_for_out = provider_id.to_owned();
        ctx.subscribe_to_view(&output_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                let value = parse_token_count(&buffer_text);
                ctx.dispatch_typed_action_deferred(
                    AISettingsPageAction::UpdateAgentProviderModelMaxOutput {
                        provider_id: provider_id_for_out.clone(),
                        model_index,
                        max_output_tokens: value,
                    },
                );
            }
        });

        ModelRow {
            name_editor,
            id_editor,
            context_editor,
            output_editor,
            remove_button_state: MouseStateHandle::default(),
        }
    }

    /// 为一条 provider 构造它的所有 view handle 与按钮 mouse state。
    fn build_row(
        provider: &AgentProvider,
        ctx: &mut ViewContext<AISettingsPageView>,
    ) -> ProviderRow {
        let provider_id = provider.id.clone();

        // ---- Name 编辑器 ----
        let initial_name = provider.name.clone();
        let name_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("自定义提供商名称(例如: DeepSeek、本地 Ollama)", ctx);
            if !initial_name.is_empty() {
                editor.set_buffer_text(&initial_name, ctx);
            }
            editor
        });
        let provider_id_for_name = provider_id.clone();
        ctx.subscribe_to_view(&name_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                ctx.dispatch_typed_action_deferred(AISettingsPageAction::UpdateAgentProviderName {
                    provider_id: provider_id_for_name.clone(),
                    name: buffer_text,
                });
            }
        });

        // ---- Base URL 编辑器 ----
        let initial_base_url = provider.base_url.clone();
        let base_url_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, false);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("https://api.deepseek.com/v1", ctx);
            if !initial_base_url.is_empty() {
                editor.set_buffer_text(&initial_base_url, ctx);
            }
            editor
        });
        let provider_id_for_url = provider_id.clone();
        ctx.subscribe_to_view(&base_url_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                ctx.dispatch_typed_action_deferred(AISettingsPageAction::UpdateAgentProviderBaseUrl {
                    provider_id: provider_id_for_url.clone(),
                    base_url: buffer_text,
                });
            }
        });

        // ---- API Key 编辑器(密码模式) ----
        let initial_api_key = AgentProviderSecrets::as_ref(ctx)
            .get(&provider_id)
            .map(str::to_owned)
            .unwrap_or_default();
        let api_key_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = single_line_editor_options(&appearance, true);
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("sk-... (失焦或按 Enter 保存到系统 keychain)", ctx);
            if !initial_api_key.is_empty() {
                editor.set_buffer_text(&initial_api_key, ctx);
            }
            editor
        });
        let provider_id_for_key = provider_id.clone();
        ctx.subscribe_to_view(&api_key_editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                ctx.dispatch_typed_action_deferred(AISettingsPageAction::UpdateAgentProviderApiKey {
                    provider_id: provider_id_for_key.clone(),
                    api_key: buffer_text,
                });
            }
        });

        // ---- 模型行 ----
        let model_rows: Vec<ModelRow> = provider
            .models
            .iter()
            .enumerate()
            .map(|(idx, m)| Self::build_model_row(&provider_id, idx, m, ctx))
            .collect();

        ProviderRow {
            name_editor,
            base_url_editor,
            api_key_editor,
            fetch_button_state: MouseStateHandle::default(),
            sync_models_dev_button_state: MouseStateHandle::default(),
            remove_button_state: MouseStateHandle::default(),
            add_model_button_state: MouseStateHandle::default(),
            model_rows,
        }
    }

    fn render_card_button(
        label: impl Into<String>,
        mouse_state: MouseStateHandle,
        action: AISettingsPageAction,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, mouse_state)
            .with_style(UiComponentStyles {
                font_size: Some(CARD_BUTTON_FONT_SIZE),
                padding: Some(Coords::uniform(CARD_BUTTON_PADDING)),
                ..Default::default()
            })
            .with_centered_text_label(label.into())
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(action.clone());
            })
            .finish()
    }

    fn render_model_row(
        provider_id: &str,
        index: usize,
        row: &ModelRow,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let remove_button = Self::render_card_button(
            "×",
            row.remove_button_state.clone(),
            AISettingsPageAction::RemoveAgentProviderModel {
                provider_id: provider_id.to_owned(),
                model_index: index,
            },
            appearance,
        );

        let cell = |flex: f32, view: &ViewHandle<EditorView>| -> Box<dyn Element> {
            Expanded::new(
                flex,
                Container::new(ChildView::new(view).finish())
                    .with_margin_right(MODEL_ROW_GAP)
                    .finish(),
            )
            .finish()
        };

        Container::new(
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(cell(2., &row.name_editor))
                .with_child(cell(2., &row.id_editor))
                .with_child(cell(1., &row.context_editor))
                .with_child(cell(1., &row.output_editor))
                .with_child(remove_button)
                .finish(),
        )
        .with_margin_bottom(MODEL_ROW_GAP)
        .finish()
    }

    fn render_provider_card(
        &self,
        provider: &AgentProvider,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let is_any_ai_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);
        let label_color = if is_any_ai_enabled {
            appearance.theme().active_ui_text_color()
        } else {
            appearance.theme().disabled_ui_text_color()
        };
        let detail_color = if is_any_ai_enabled {
            appearance.theme().foreground()
        } else {
            appearance.theme().disabled_ui_text_color()
        };

        let rows = self.rows.borrow();
        let row = match rows.get(&provider.id) {
            Some(row) => row,
            None => {
                return Container::new(
                    Text::new(
                        format!("(此 provider 还未关联编辑器: {})", provider.id),
                        appearance.ui_font_family(),
                        appearance.ui_font_size(),
                    )
                    .with_color(detail_color.into())
                    .finish(),
                )
                .with_margin_bottom(8.)
                .finish();
            }
        };

        let name_field = field_block(
            "Name",
            ChildView::new(&row.name_editor).finish(),
            label_color,
            appearance,
        );
        let base_url_field = field_block(
            "Base URL",
            ChildView::new(&row.base_url_editor).finish(),
            label_color,
            appearance,
        );
        let api_key_field = field_block(
            "API Key",
            ChildView::new(&row.api_key_editor).finish(),
            label_color,
            appearance,
        );

        // ---- 模型列表区 ----
        let models_label = Container::new(
            Text::new(
                format!("模型列表 ({} 个)", provider.models.len()),
                appearance.ui_font_family(),
                appearance.ui_font_size(),
            )
            .with_color(label_color.into())
            .finish(),
        )
        .with_margin_top(FIELD_LABEL_MARGIN_TOP)
        .with_margin_bottom(FIELD_LABEL_MARGIN_BOTTOM)
        .finish();

        let mut models_column = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(models_label);

        if provider.models.is_empty() {
            let empty_hint = Container::new(
                Text::new(
                    "还未配置模型。点 [+ 添加模型] 手动添加,或点 [Fetch from API] 自动抓取。",
                    appearance.ui_font_family(),
                    appearance.ui_font_size(),
                )
                .with_color(appearance.theme().disabled_ui_text_color().into())
                .soft_wrap(true)
                .finish(),
            )
            .with_margin_bottom(MODEL_ROW_GAP)
            .finish();
            models_column.add_child(empty_hint);
        } else {
            // 表头: 显示名 | 模型 ID | 上下文 | 输出
            let dim = appearance.theme().disabled_ui_text_color();
            let header_cell = |flex: f32, label: &str| -> Box<dyn Element> {
                Expanded::new(
                    flex,
                    Container::new(
                        Text::new(
                            label.to_string(),
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_color(dim.into())
                        .finish(),
                    )
                    .with_margin_right(MODEL_ROW_GAP)
                    .finish(),
                )
                .finish()
            };
            let header = Container::new(
                Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(header_cell(2., "显示名"))
                    .with_child(header_cell(2., "模型 ID"))
                    .with_child(header_cell(1., "上下文 (tok)"))
                    .with_child(header_cell(1., "输出 (tok)"))
                    // 占位,与下方 × 按钮对齐
                    .with_child(
                        Text::new(
                            "  ".to_string(),
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_color(dim.into())
                        .finish(),
                    )
                    .finish(),
            )
            .with_margin_bottom(2.)
            .finish();
            models_column.add_child(header);

            for (idx, m_row) in row.model_rows.iter().enumerate() {
                models_column.add_child(Self::render_model_row(
                    &provider.id,
                    idx,
                    m_row,
                    appearance,
                ));
            }
        }

        // ---- 底部按钮行 ----
        let add_model_button = Self::render_card_button(
            "+ 添加模型",
            row.add_model_button_state.clone(),
            AISettingsPageAction::AddAgentProviderModel {
                provider_id: provider.id.clone(),
            },
            appearance,
        );
        let fetch_button = Self::render_card_button(
            "Fetch from API",
            row.fetch_button_state.clone(),
            AISettingsPageAction::FetchAgentProviderModels {
                provider_id: provider.id.clone(),
            },
            appearance,
        );
        let sync_models_dev_button = Self::render_card_button(
            "Sync from models.dev",
            row.sync_models_dev_button_state.clone(),
            AISettingsPageAction::SyncProviderModelsFromModelsDev {
                provider_id: provider.id.clone(),
            },
            appearance,
        );
        let remove_button = Self::render_card_button(
            "Remove",
            row.remove_button_state.clone(),
            AISettingsPageAction::RemoveAgentProvider {
                provider_id: provider.id.clone(),
            },
            appearance,
        );

        let bottom_row = Flex::row()
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(Container::new(add_model_button).with_margin_right(8.).finish())
                    .with_child(Container::new(fetch_button).with_margin_right(8.).finish())
                    .with_child(sync_models_dev_button)
                    .finish(),
            )
            .with_child(remove_button)
            .finish();

        // 用透明 detail_color 触发它被读取(避免 unused 警告);仅用于潜在配色。
        let _ = detail_color;

        Container::new(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(name_field)
                .with_child(base_url_field)
                .with_child(api_key_field)
                .with_child(Container::new(models_column.finish()).with_margin_top(8.).finish())
                .with_child(Container::new(bottom_row).with_margin_top(10.).finish())
                .finish(),
        )
        .with_background(appearance.theme().surface_1())
        .with_uniform_padding(12.)
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
        .with_margin_bottom(8.)
        .finish()
    }
}

/// 把用户输入解析成 token 数。容忍 `128k` / `128K` / `128 000` / `128,000` / 空白,
/// 解析失败一律返回 0(语义:未指定)。
fn parse_token_count(input: &str) -> u32 {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',' && *c != '_')
        .collect();
    if cleaned.is_empty() {
        return 0;
    }
    let lower = cleaned.to_lowercase();
    let (num_part, multiplier): (&str, u64) = if let Some(stripped) = lower.strip_suffix('k') {
        (stripped, 1_000)
    } else if let Some(stripped) = lower.strip_suffix('m') {
        (stripped, 1_000_000)
    } else {
        (lower.as_str(), 1)
    };
    num_part
        .parse::<f64>()
        .ok()
        .map(|n| (n * multiplier as f64).round() as u64)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0)
}

fn single_line_editor_options(
    appearance: &Appearance,
    is_password: bool,
) -> SingleLineEditorOptions {
    SingleLineEditorOptions {
        is_password,
        text: TextOptions {
            font_size_override: Some(appearance.ui_font_size()),
            font_family_override: Some(appearance.monospace_font_family()),
            text_colors_override: Some(TextColors {
                default_color: appearance.theme().active_ui_text_color(),
                disabled_color: appearance.theme().disabled_ui_text_color(),
                hint_color: appearance.theme().disabled_ui_text_color(),
            }),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn field_block(
    label: &str,
    editor_element: Box<dyn Element>,
    label_color: warp_core::ui::theme::Fill,
    appearance: &Appearance,
) -> Box<dyn Element> {
    let label_text = Container::new(
        Text::new(
            label.to_string(),
            appearance.ui_font_family(),
            appearance.ui_font_size(),
        )
        .with_color(label_color.into())
        .finish(),
    )
    .with_margin_top(FIELD_LABEL_MARGIN_TOP)
    .with_margin_bottom(FIELD_LABEL_MARGIN_BOTTOM)
    .finish();

    Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_child(label_text)
        .with_child(editor_element)
        .finish()
}

impl AgentProvidersWidget {
    /// 渲染 "来自 models.dev 的已知 provider 快速添加" 区:
    /// - 标题 + "刷新目录" 按钮
    /// - 一行 chip(每个对应一个 catalog provider id),点击即新建本地 provider 并预填模型
    /// - 目录尚未加载时,显示 "正在拉取..."
    fn render_models_dev_section(
        &self,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        use crate::ai::agent_providers::models_dev;

        let label_color = appearance.theme().active_ui_text_color();
        let dim_color = appearance.theme().disabled_ui_text_color();

        let title = Text::new(
            "快速添加".to_string(),
            appearance.ui_font_family(),
            appearance.ui_font_size(),
        )
        .with_color(label_color.into())
        .finish();

        let refresh_button = Self::render_card_button(
            "刷新目录",
            self.refresh_catalog_button_state.clone(),
            AISettingsPageAction::RefreshModelsDev,
            appearance,
        );

        let search_box = Container::new(ChildView::new(&self.search_editor).finish())
            .with_margin_left(8.)
            .with_margin_right(8.)
            .finish();

        let header_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(title)
            .with_child(Expanded::new(1., search_box).finish())
            .with_child(refresh_button)
            .finish();

        let mut body = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
        body.add_child(header_row);

        // 收起时显示前 N 个(够撑约 1 行 — 实际换行交给 Wrap layout 处理)。
        const COLLAPSED_LIMIT: usize = 8;
        let expanded = models_dev::chips_expanded();

        match models_dev::cached() {
            None => {
                body.add_child(
                    Container::new(
                        Text::new(
                            "正在拉取 models.dev 目录…(第一次可能需要几秒)".to_string(),
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_color(dim_color.into())
                        .finish(),
                    )
                    .with_margin_top(4.)
                    .finish(),
                );
            }
            Some(catalog) if catalog.is_empty() => {
                body.add_child(
                    Container::new(
                        Text::new(
                            "models.dev 目录为空,点 [刷新目录] 重试。".to_string(),
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_color(dim_color.into())
                        .finish(),
                    )
                    .with_margin_top(4.)
                    .finish(),
                );
            }
            Some(catalog) => {
                // 按搜索 query 过滤;空 query → 全部条目顺序。
                let query = models_dev::search_query();
                let filtered = models_dev::filter_catalog(&catalog, &query);
                let total = filtered.len();
                let has_query = !query.trim().is_empty();
                // 搜索激活时一律展开全部匹配,不做折叠(否则结果数 ≤ 折叠上限就看不全)。
                let visible_count = if expanded || has_query {
                    total
                } else {
                    COLLAPSED_LIMIT.min(total)
                };

                let mut wrap = Wrap::row()
                    .with_spacing(6.)
                    .with_run_spacing(6.)
                    .with_cross_axis_alignment(CrossAxisAlignment::Center);
                {
                    let mut states = self.quick_add_button_states.borrow_mut();
                    for (cat_id, cat_provider) in filtered.iter().take(visible_count) {
                        let label = if cat_provider.name.is_empty() {
                            cat_id.clone()
                        } else {
                            cat_provider.name.clone()
                        };
                        let state = states
                            .entry(cat_id.clone())
                            .or_insert_with(MouseStateHandle::default)
                            .clone();
                        let model_count = cat_provider.models.len();
                        let display_label = format!("+ {label} ({model_count})");
                        let chip = Self::render_card_button(
                            display_label,
                            state,
                            AISettingsPageAction::AddProviderFromModelsDev {
                                catalog_provider_id: cat_id.clone(),
                            },
                            appearance,
                        );
                        wrap = wrap.with_child(chip);
                    }
                }
                body.add_child(Container::new(wrap.finish()).with_margin_top(4.).finish());

                if has_query && total == 0 {
                    body.add_child(
                        Container::new(
                            Text::new(
                                format!("无匹配 \"{query}\""),
                                appearance.ui_font_family(),
                                appearance.ui_font_size(),
                            )
                            .with_color(dim_color.into())
                            .finish(),
                        )
                        .with_margin_top(4.)
                        .finish(),
                    );
                }

                // 展开/收起按钮(只在无搜索 + catalog 比折叠上限多时才展示)。
                if !has_query && total > COLLAPSED_LIMIT {
                    let toggle_label = if expanded {
                        "收起 ▲".to_string()
                    } else {
                        format!("展开剩余 {} 个 ▼", total - COLLAPSED_LIMIT)
                    };
                    let toggle_button = Self::render_card_button(
                        toggle_label,
                        self.expand_chips_button_state.clone(),
                        AISettingsPageAction::ToggleModelsDevChipsExpanded,
                        appearance,
                    );
                    body.add_child(
                        Container::new(
                            Flex::row()
                                .with_main_axis_alignment(MainAxisAlignment::Start)
                                .with_child(toggle_button)
                                .finish(),
                        )
                        .with_margin_top(6.)
                        .finish(),
                    );
                }
            }
        }

        Container::new(body.finish())
            .with_background(appearance.theme().surface_1())
            .with_uniform_padding(10.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .with_margin_bottom(10.)
            .finish()
    }
}

impl SettingsWidget for AgentProvidersWidget {
    type View = AISettingsPageView;

    fn search_terms(&self) -> &str {
        "agent provider providers custom openai compatible deepseek glm moonshot dashscope qwen ollama base url api key models 提供商 自定义 模型"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let is_any_ai_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);
        let providers = AISettings::as_ref(app).agent_providers.value().clone();

        let header = build_sub_header(
            appearance,
            "Agent 提供商",
            Some(if is_any_ai_enabled {
                appearance.theme().active_ui_text_color()
            } else {
                appearance.theme().disabled_ui_text_color()
            }),
        )
        .with_padding_bottom(HEADER_PADDING)
        .finish();

        let description_text =
            "配置自定义 OpenAI 兼容的 Agent 提供商(如 DeepSeek、智谱 GLM、Moonshot、\
            通义千问 DashScope、SiliconFlow、OpenRouter、本地 Ollama 等)。\
            可以手动添加模型(显示名 + 模型 ID 映射),也可以从 API 自动抓取。\
            提供商元数据存储在本地 settings.toml,API key 安全存储在系统密钥库。";
        let description = Container::new(
            Text::new(
                description_text,
                appearance.ui_font_family(),
                appearance.ui_font_size(),
            )
            .with_color(if is_any_ai_enabled {
                appearance.theme().foreground().into()
            } else {
                appearance.theme().disabled_ui_text_color().into()
            })
            .soft_wrap(true)
            .finish(),
        )
        .with_margin_bottom(12.)
        .finish();

        let mut column = Flex::column().with_child(header).with_child(description);

        // ---- 来自 models.dev 的快速添加 chip 行 ----
        column.add_child(self.render_models_dev_section(appearance, app));

        if providers.is_empty() {
            let empty = Container::new(
                Text::new(
                    "尚未配置任何提供商。点击下面按钮添加。",
                    appearance.ui_font_family(),
                    appearance.ui_font_size(),
                )
                .with_color(appearance.theme().disabled_ui_text_color().into())
                .finish(),
            )
            .with_margin_bottom(12.)
            .finish();
            column.add_child(empty);
        } else {
            for provider in &providers {
                column.add_child(self.render_provider_card(provider, appearance, app));
            }
        }

        let add_button = render_full_pane_width_ai_button(
            "+ 添加 OpenAI 兼容提供商",
            is_any_ai_enabled,
            self.add_button_state.clone(),
            AISettingsPageAction::AddAgentProvider,
            appearance,
        );
        column.add_child(add_button);

        Container::new(column.finish())
            .with_margin_bottom(HEADER_PADDING)
            .finish()
    }
}
