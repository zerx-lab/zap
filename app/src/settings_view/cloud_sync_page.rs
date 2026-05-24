use super::{
    settings_page::{
        render_body_item, AdditionalInfo, MatchData, PageType, SettingsPageMeta, SettingsWidget,
    },
    LocalOnlyIconState, SettingsSection, ToggleState,
};
use crate::{appearance::Appearance, settings::CloudSyncSettings};
use settings::Setting as _;
use warpui::{
    elements::{Element, MouseStateHandle},
    ui_components::components::UiComponent,
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext,
};

/// 云同步设置页面的操作
#[derive(Debug, Clone)]
pub enum CloudSyncPageAction {
    /// 更新 GitHub Token
    UpdateGithubToken(String),
    /// 更新 Gitee Token
    UpdateGiteeToken(String),
}

/// 云同步设置页面视图
pub struct CloudSyncPageView {
    page: PageType<Self>,
}

impl CloudSyncPageView {
    /// 创建云同步设置页面
    pub fn new(_ctx: &mut ViewContext<Self>) -> Self {
        Self {
            page: PageType::new_uncategorized(
                vec![
                    Box::new(CloudSyncHeaderWidget::default()),
                    Box::new(GithubTokenWidget::default()),
                    Box::new(GiteeTokenWidget::default()),
                ],
                Some("Cloud Sync"),
            ),
        }
    }
}

impl Entity for CloudSyncPageView {
    type Event = ();
}

impl TypedActionView for CloudSyncPageView {
    type Action = CloudSyncPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            CloudSyncPageAction::UpdateGithubToken(token) => {
                CloudSyncSettings::handle(ctx).update(ctx, |settings, ctx| {
                    let _ = settings.github_token.set_value(token.clone(), ctx);
                });
                ctx.notify();
            }
            CloudSyncPageAction::UpdateGiteeToken(token) => {
                CloudSyncSettings::handle(ctx).update(ctx, |settings, ctx| {
                    let _ = settings.gitee_token.set_value(token.clone(), ctx);
                });
                ctx.notify();
            }
        }
    }
}

impl View for CloudSyncPageView {
    fn ui_name() -> &'static str {
        "CloudSyncPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

impl SettingsPageMeta for CloudSyncPageView {
    fn section() -> SettingsSection {
        SettingsSection::CloudSync
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

/// 页面标题和说明小部件
#[derive(Default)]
struct CloudSyncHeaderWidget {}

impl SettingsWidget for CloudSyncHeaderWidget {
    type View = CloudSyncPageView;

    fn search_terms(&self) -> &str {
        "cloud sync gist github gitee backup token"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        warpui::elements::Container::new(
            warpui::elements::Text::new(
                "Configure cloud synchronization via GitHub Gist or Gitee Gist. Your settings will be encrypted and stored as a secret Gist.".to_string(),
                appearance.ui_font_family(),
                appearance.ui_font_body(),
            )
            .with_color(theme.nonactive_ui_text_color().into())
            .finish(),
        )
        .with_padding_bottom(super::settings_page::HEADER_PADDING)
        .finish()
    }
}

/// GitHub Token 配置小部件
#[derive(Default)]
struct GithubTokenWidget {
    mouse_state: MouseStateHandle,
}

impl SettingsWidget for GithubTokenWidget {
    type View = CloudSyncPageView;

    fn search_terms(&self) -> &str {
        "github token gist api cloud sync backup"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let settings = CloudSyncSettings::as_ref(app);
        let token_value = settings.github_token.value();
        let has_token = !token_value.is_empty();
        let display_text = if has_token {
            let len = token_value.len();
            if len > 8 {
                format!("{}...{}", &token_value[..4], &token_value[len - 4..])
            } else {
                "*".repeat(len)
            }
        } else {
            "Not configured".to_string()
        };

        render_body_item::<CloudSyncPageAction>(
            "GitHub Token".into(),
            Some(AdditionalInfo {
                mouse_state: self.mouse_state.clone(),
                on_click_action: None,
                secondary_text: Some(display_text),
                tooltip_override_text: Some(
                    "Personal access token with gist scope for GitHub Gist API".to_string(),
                ),
            }),
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            appearance
                .ui_builder()
                .button(
                    warpui::ui_components::button::ButtonVariant::Secondary,
                    self.mouse_state.clone(),
                )
                .with_text_label("Configure".to_string())
                .build()
                .finish(),
            Some("Enter your GitHub Personal Access Token with gist scope to sync settings via GitHub Gist.".into()),
        )
    }
}

/// Gitee Token 配置小部件
#[derive(Default)]
struct GiteeTokenWidget {
    mouse_state: MouseStateHandle,
}

impl SettingsWidget for GiteeTokenWidget {
    type View = CloudSyncPageView;

    fn search_terms(&self) -> &str {
        "gitee token gist api cloud sync backup"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let settings = CloudSyncSettings::as_ref(app);
        let token_value = settings.gitee_token.value();
        let has_token = !token_value.is_empty();
        let display_text = if has_token {
            let len = token_value.len();
            if len > 8 {
                format!("{}...{}", &token_value[..4], &token_value[len - 4..])
            } else {
                "*".repeat(len)
            }
        } else {
            "Not configured".to_string()
        };

        render_body_item::<CloudSyncPageAction>(
            "Gitee Token".into(),
            Some(AdditionalInfo {
                mouse_state: self.mouse_state.clone(),
                on_click_action: None,
                secondary_text: Some(display_text),
                tooltip_override_text: Some(
                    "Personal access token with gist scope for Gitee Gist API".to_string(),
                ),
            }),
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            appearance
                .ui_builder()
                .button(
                    warpui::ui_components::button::ButtonVariant::Secondary,
                    self.mouse_state.clone(),
                )
                .with_text_label("Configure".to_string())
                .build()
                .finish(),
            Some("Enter your Gitee Personal Access Token with gist scope to sync settings via Gitee Gist.".into()),
        )
    }
}
