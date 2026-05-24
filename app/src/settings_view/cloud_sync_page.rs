use super::{
    settings_page::{
        render_body_item, render_sub_header, AdditionalInfo, MatchData, PageType, SettingsPageMeta,
        SettingsWidget,
    },
    LocalOnlyIconState, SettingsSection, ToggleState,
};
use crate::{appearance::Appearance, settings::CloudSyncSettings};
use settings::Setting as _;
use warp_ssh_manager::{with_conn, SshRepository};
use warpui::{
    elements::{Element, Flex, MouseStateHandle, ParentElement, Text},
    ui_components::{
        button::ButtonVariant,
        components::UiComponent,
    },
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext,
};

/// 云同步设置页面的操作
#[derive(Debug, Clone)]
pub enum CloudSyncPageAction {
    /// 更新 GitHub Token
    UpdateGithubToken(String),
    /// 更新 Gitee Token
    UpdateGiteeToken(String),
    /// 上传同步到 GitHub Gist
    UploadToGithub,
    /// 上传同步到 Gitee Gist
    UploadToGitee,
    /// 从 GitHub Gist 下载同步
    DownloadFromGithub,
    /// 从 Gitee Gist 下载同步
    DownloadFromGitee,
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
                    Box::new(SyncControlWidget::default()),
                    Box::new(SyncStatusWidget::default()),
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
            CloudSyncPageAction::UploadToGithub => {
                log::info!("Cloud sync: upload to GitHub triggered");
                // TODO: 调用 zap_sync 上传逻辑
            }
            CloudSyncPageAction::UploadToGitee => {
                log::info!("Cloud sync: upload to Gitee triggered");
                // TODO: 调用 zap_sync 上传逻辑
            }
            CloudSyncPageAction::DownloadFromGithub => {
                log::info!("Cloud sync: download from GitHub triggered");
                // TODO: 调用 zap_sync 下载逻辑
            }
            CloudSyncPageAction::DownloadFromGitee => {
                log::info!("Cloud sync: download from Gitee triggered");
                // TODO: 调用 zap_sync 下载逻辑
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

/// 同步操作按钮区域小部件
#[derive(Default)]
struct SyncControlWidget {
    upload_github_mouse: MouseStateHandle,
    upload_gitee_mouse: MouseStateHandle,
    download_github_mouse: MouseStateHandle,
    download_gitee_mouse: MouseStateHandle,
}

impl SyncControlWidget {
    /// 创建一个同步按钮
    fn render_sync_button(
        &self,
        label: impl Into<String>,
        mouse_state: MouseStateHandle,
        action: CloudSyncPageAction,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        appearance
            .ui_builder()
            .button(ButtonVariant::Secondary, mouse_state)
            .with_text_label(label.into())
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(action.clone());
            })
            .finish()
    }
}

impl SettingsWidget for SyncControlWidget {
    type View = CloudSyncPageView;

    fn search_terms(&self) -> &str {
        "sync upload download backup restore github gitee cloud button"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();

        let sub_header = render_sub_header(appearance, "Sync Operations", None);

        let upload_label = Text::new_inline(
            "Upload".to_string(),
            appearance.ui_font_family(),
            appearance.ui_font_body(),
        )
        .with_color(theme.nonactive_ui_text_color().into())
        .finish();

        let download_label = Text::new_inline(
            "Download".to_string(),
            appearance.ui_font_family(),
            appearance.ui_font_body(),
        )
        .with_color(theme.nonactive_ui_text_color().into())
        .finish();

        let upload_github_btn = self.render_sync_button(
            "Upload to GitHub",
            self.upload_github_mouse.clone(),
            CloudSyncPageAction::UploadToGithub,
            appearance,
        );

        let upload_gitee_btn = self.render_sync_button(
            "Upload to Gitee",
            self.upload_gitee_mouse.clone(),
            CloudSyncPageAction::UploadToGitee,
            appearance,
        );

        let download_github_btn = self.render_sync_button(
            "Download from GitHub",
            self.download_github_mouse.clone(),
            CloudSyncPageAction::DownloadFromGithub,
            appearance,
        );

        let download_gitee_btn = self.render_sync_button(
            "Download from Gitee",
            self.download_gitee_mouse.clone(),
            CloudSyncPageAction::DownloadFromGitee,
            appearance,
        );

        Flex::column()
            .with_child(sub_header)
            .with_child(upload_label)
            .with_child(
                Flex::row()
                    .with_child(upload_github_btn)
                    .with_child(upload_gitee_btn)
                    .finish(),
            )
            .with_child(download_label)
            .with_child(
                Flex::row()
                    .with_child(download_github_btn)
                    .with_child(download_gitee_btn)
                    .finish(),
            )
            .finish()
    }
}

/// 同步状态显示小部件
#[derive(Default)]
struct SyncStatusWidget {}

impl SettingsWidget for SyncStatusWidget {
    type View = CloudSyncPageView;

    fn search_terms(&self) -> &str {
        "sync status version last time platform cloud"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();

        let sub_header = render_sub_header(appearance, "Sync Status", None);

        let version = with_conn(|c| Ok(SshRepository::get_sync_version(c)?))
            .map(|v| v.to_string())
            .unwrap_or_else(|_| "N/A".to_string());

        let last_sync_time = with_conn(|c| Ok(SshRepository::get_last_sync_time(c)?))
            .unwrap_or_else(|e| {
                log::debug!("Failed to get last sync time: {e}");
                "Never".to_string()
            });

        let last_sync_platform = with_conn(|c| Ok(SshRepository::get_last_sync_platform(c)?))
            .unwrap_or_else(|e| {
                log::debug!("Failed to get last sync platform: {e}");
                "N/A".to_string()
            });

        let info_color = theme.nonactive_ui_text_color();

        let version_text = Text::new_inline(
            format!("Local version: {version}"),
            appearance.ui_font_family(),
            appearance.ui_font_body(),
        )
        .with_color(info_color.into())
        .finish();

        let time_text = Text::new_inline(
            format!("Last sync time: {last_sync_time}"),
            appearance.ui_font_family(),
            appearance.ui_font_body(),
        )
        .with_color(info_color.into())
        .finish();

        let platform_text = Text::new_inline(
            format!("Last sync platform: {last_sync_platform}"),
            appearance.ui_font_family(),
            appearance.ui_font_body(),
        )
        .with_color(info_color.into())
        .finish();

        Flex::column()
            .with_child(sub_header)
            .with_child(version_text)
            .with_child(time_text)
            .with_child(platform_text)
            .finish()
    }
}
