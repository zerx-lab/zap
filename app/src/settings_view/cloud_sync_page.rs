//! 云同步设置页面 — 平台选择、Token 配置、同步操作、状态显示
//!
// author: logic
// date: 2026-05-25

use settings::Setting;
use warpui::{
    elements::{
        ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Dismiss, Element, Flex,
        MainAxisSize, MouseStateHandle, ParentElement, Radius, Text,
    },
    ui_components::{
        button::ButtonVariant,
        components::{Coords, UiComponent, UiComponentStyles},
    },
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use super::settings_page::{
    render_body_item, AdditionalInfo, LocalOnlyIconState, MatchData, PageType,
    SettingsPageEvent, SettingsPageMeta, SettingsWidget, ToggleState,
};
use super::SettingsSection;
use crate::appearance::Appearance;
use crate::editor::{EditorView, SingleLineEditorOptions, TextOptions};
use crate::settings::SyncPlatformSetting;
use crate::settings::CloudSyncSettings;
use crate::ssh_manager::{SshTreeChangedEvent, SshTreeChangedNotifier};
use crate::view_components::dropdown::{Dropdown, DropdownItem};
use crate::ui_components::icons::Icon;
use warp_ssh_manager::{with_conn, DbVersionStore, SshRepository, SshSyncProvider};
use zap_sync::{GistClient, SyncEngine, SyncPlatform, SyncResult};

const INPUT_AREA_MAX_WIDTH: f32 = 420.0;
const BUTTON_PADDING: f32 = 6.0;
const DIALOG_WIDTH: f32 = 450.0;

/// 同步方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    Upload,
    Download,
}

/// 同步状态
#[derive(Debug, Clone, Default)]
enum SyncState {
    #[default]
    Idle,
    Validating,
    TokenValid {
        username: String,
    },
    Syncing {
        platform: SyncPlatform,
        direction: SyncDirection,
    },
    Success {
        platform: SyncPlatform,
        direction: SyncDirection,
        version: i64,
    },
    Failed {
        message: String,
    },
    Conflict {
        local_version: i64,
        remote_version: i64,
        direction: SyncDirection,
        platform: SyncPlatform,
    },
}

/// 云同步设置页面的操作
#[derive(Debug, Clone)]
pub enum CloudSyncPageAction {
    /// 切换同步平台
    SetPlatform(SyncPlatformSetting),
    /// 保存当前平台的 Token
    SaveToken,
    /// 清除当前平台的 Token
    ClearToken,
    /// Token 验证完成
    TokenValidated {
        result: Result<String, String>,
    },
    /// 上传同步（使用当前选中平台）
    Upload,
    /// 下载同步（使用当前选中平台）
    Download,
    /// 异步同步完成回调
    SyncComplete {
        platform: SyncPlatform,
        direction: SyncDirection,
        result: Result<SyncResult, String>,
    },
    /// 强制上传（覆盖远程）
    ForceUpload {
        platform: SyncPlatform,
    },
    /// 取消冲突弹窗
    CancelConflict,
    /// 确认下载
    ConfirmDownload { platform: SyncPlatform },
    /// 取消下载确认
    CancelDownloadConfirm,
}

/// 云同步设置页面视图
pub struct CloudSyncPageView {
    page: PageType<Self>,
    platform_dropdown: ViewHandle<Dropdown<CloudSyncPageAction>>,
    token_editor: ViewHandle<EditorView>,
    save_state: MouseStateHandle,
    clear_state: MouseStateHandle,
    upload_mouse: MouseStateHandle,
    download_mouse: MouseStateHandle,
    conflict_force_mouse: MouseStateHandle,
    conflict_cancel_mouse: MouseStateHandle,
    sync_state: SyncState,
    conflict_visible: bool,
    conflict_local_version: i64,
    conflict_remote_version: i64,
    conflict_platform: SyncPlatform,
    download_confirm_visible: bool,
    download_confirm_platform: SyncPlatform,
    download_confirm_mouse: MouseStateHandle,
    download_confirm_cancel_mouse: MouseStateHandle,
}

/// 构造 Token 密码编辑器
fn build_token_editor(
    ctx: &mut ViewContext<CloudSyncPageView>,
    placeholder: &str,
) -> ViewHandle<EditorView> {
    let placeholder = placeholder.to_string();
    ctx.add_typed_action_view(move |ctx| {
        let appearance = Appearance::as_ref(ctx);
        let options = SingleLineEditorOptions {
            is_password: true,
            text: TextOptions {
                font_size_override: Some(appearance.ui_font_size()),
                ..Default::default()
            },
            ..Default::default()
        };
        let mut editor = EditorView::single_line(options, ctx);
        editor.set_placeholder_text(placeholder.clone(), ctx);
        editor
    })
}

/// 从 CloudSyncSettings 同步 Dropdown 和编辑器内容
fn sync_from_settings(me: &mut CloudSyncPageView, ctx: &mut ViewContext<CloudSyncPageView>) {
    let (platform, token) = {
        let settings = CloudSyncSettings::as_ref(ctx);
        let platform = *settings.sync_platform.value();
        let token = match platform {
            SyncPlatformSetting::GitHub => settings.github_token.value().clone(),
            SyncPlatformSetting::Gitee => settings.gitee_token.value().clone(),
        };
        (platform, token)
    };

    // 同步 Dropdown 选中状态
    let label = platform.label().to_string();
    me.platform_dropdown.update(ctx, |dropdown, ctx| {
        dropdown.set_selected_by_name(&label, ctx);
    });

    // 同步 Token 编辑器内容
    me.token_editor.update(ctx, |editor, ctx| {
        if editor.buffer_text(ctx) != token {
            editor.set_buffer_text(&token, ctx);
        }
    });
}

/// 获取当前选中平台对应的 Token
fn current_token(ctx: &AppContext) -> String {
    let settings = CloudSyncSettings::as_ref(ctx);
    let platform = *settings.sync_platform.value();
    match platform {
        SyncPlatformSetting::GitHub => settings.github_token.value().clone(),
        SyncPlatformSetting::Gitee => settings.gitee_token.value().clone(),
    }
}

impl CloudSyncPageView {
    /// 创建云同步设置页面
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let platform_dropdown = ctx.add_typed_action_view(Dropdown::<CloudSyncPageAction>::new);
        platform_dropdown.update(ctx, |dropdown, ctx| {
            dropdown.set_items(
                vec![
                    DropdownItem::new(
                        SyncPlatformSetting::GitHub.label(),
                        CloudSyncPageAction::SetPlatform(SyncPlatformSetting::GitHub),
                    ),
                    DropdownItem::new(
                        SyncPlatformSetting::Gitee.label(),
                        CloudSyncPageAction::SetPlatform(SyncPlatformSetting::Gitee),
                    ),
                ],
                ctx,
            );
        });

        let token_editor = build_token_editor(ctx, &crate::t!("settings-cloud-sync-token-placeholder"));

        ctx.subscribe_to_model(
            &CloudSyncSettings::handle(ctx),
            |me: &mut Self, _, _, ctx| {
                sync_from_settings(me, ctx);
                ctx.notify();
            },
        );

        let mut me = Self {
            page: PageType::new_monolith(CloudSyncPageWidget::default(), None, false),
            platform_dropdown,
            token_editor,
            save_state: MouseStateHandle::default(),
            clear_state: MouseStateHandle::default(),
            upload_mouse: MouseStateHandle::default(),
            download_mouse: MouseStateHandle::default(),
            conflict_force_mouse: MouseStateHandle::default(),
            conflict_cancel_mouse: MouseStateHandle::default(),
            sync_state: SyncState::Idle,
            conflict_visible: false,
            conflict_local_version: 0,
            conflict_remote_version: 0,
            conflict_platform: SyncPlatform::GitHub,
            download_confirm_visible: false,
            download_confirm_platform: SyncPlatform::GitHub,
            download_confirm_mouse: MouseStateHandle::default(),
            download_confirm_cancel_mouse: MouseStateHandle::default(),
        };

        sync_from_settings(&mut me, ctx);
        me
    }

    /// 启动上传同步
    fn spawn_upload(&mut self, platform: SyncPlatform, ctx: &mut ViewContext<Self>) {
        let token = current_token(ctx);
        if token.is_empty() {
            let label = platform.label();
            self.sync_state = SyncState::Failed {
                message: crate::t!("settings-cloud-sync-token-not-configured", platform = label.to_string()),
            };
            ctx.notify();
            return;
        }

        self.sync_state = SyncState::Syncing {
            platform,
            direction: SyncDirection::Upload,
        };
        ctx.notify();

        ctx.spawn(
            async move {
                let engine = SyncEngine::new();
                let provider = SshSyncProvider::new();
                let version_store = DbVersionStore;
                engine
                    .upload(platform, &token, &[&provider], &version_store)
                    .await
                    .map_err(|e| e.to_string())
            },
            move |view, result, ctx| {
                view.handle_action(
                    &CloudSyncPageAction::SyncComplete {
                        platform,
                        direction: SyncDirection::Upload,
                        result,
                    },
                    ctx,
                );
            },
        );
    }

    /// 启动下载同步
    fn spawn_download(&mut self, platform: SyncPlatform, ctx: &mut ViewContext<Self>) {
        let token = current_token(ctx);
        if token.is_empty() {
            let label = platform.label();
            self.sync_state = SyncState::Failed {
                message: crate::t!("settings-cloud-sync-token-not-configured", platform = label.to_string()),
            };
            ctx.notify();
            return;
        }

        self.sync_state = SyncState::Syncing {
            platform,
            direction: SyncDirection::Download,
        };
        ctx.notify();

        ctx.spawn(
            async move {
                let engine = SyncEngine::new();
                let provider = SshSyncProvider::new();
                let version_store = DbVersionStore;
                engine
                    .download(platform, &token, &[&provider], &version_store)
                    .await
                    .map_err(|e| e.to_string())
            },
            move |view, result, ctx| {
                view.handle_action(
                    &CloudSyncPageAction::SyncComplete {
                        platform,
                        direction: SyncDirection::Download,
                        result,
                    },
                    ctx,
                );
            },
        );
    }

    /// 启动强制上传同步（覆盖远程）
    fn spawn_force_upload(&mut self, platform: SyncPlatform, ctx: &mut ViewContext<Self>) {
        let token = current_token(ctx);
        if token.is_empty() {
            let label = platform.label();
            self.sync_state = SyncState::Failed {
                message: crate::t!("settings-cloud-sync-token-not-configured", platform = label.to_string()),
            };
            ctx.notify();
            return;
        }

        self.sync_state = SyncState::Syncing {
            platform,
            direction: SyncDirection::Upload,
        };
        ctx.notify();

        ctx.spawn(
            async move {
                let engine = SyncEngine::new();
                let provider = SshSyncProvider::new();
                let version_store = DbVersionStore;
                engine
                    .force_upload(platform, &token, &[&provider], &version_store)
                    .await
                    .map_err(|e| e.to_string())
            },
            move |view, result, ctx| {
                view.handle_action(
                    &CloudSyncPageAction::SyncComplete {
                        platform,
                        direction: SyncDirection::Upload,
                        result,
                    },
                    ctx,
                );
            },
        );
    }
}

impl Entity for CloudSyncPageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for CloudSyncPageView {
    type Action = CloudSyncPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            CloudSyncPageAction::SetPlatform(platform) => {
                let platform = *platform;
                CloudSyncSettings::handle(ctx).update(ctx, |settings, ctx| {
                    let _ = settings.sync_platform.set_value(platform, ctx);
                });
                // sync_from_settings 会通过 settings 订阅自动触发
            }
            CloudSyncPageAction::SaveToken => {
                let value = self.token_editor.as_ref(ctx).buffer_text(ctx);
                let platform = CloudSyncSettings::as_ref(ctx)
                    .sync_platform
                    .value()
                    .to_sync_platform();
                if value.is_empty() {
                    ctx.notify();
                    return;
                }
                self.sync_state = SyncState::Validating;
                ctx.notify();

                let token = value.clone();
                ctx.spawn(
                    async move {
                        let client = GistClient::new();
                        client
                            .validate_token(platform, &token)
                            .await
                            .map_err(|e| e.to_string())
                    },
                    move |view, result, ctx| {
                        view.handle_action(
                            &CloudSyncPageAction::TokenValidated { result },
                            ctx,
                        );
                    },
                );
            }
            CloudSyncPageAction::TokenValidated { result } => {
                match result {
                    Ok(username) => {
                        let username = username.clone();
                        // 验证成功，保存 Token
                        let value = self.token_editor.as_ref(ctx).buffer_text(ctx);
                        let platform = *CloudSyncSettings::as_ref(ctx).sync_platform.value();
                        CloudSyncSettings::handle(ctx).update(ctx, |settings, ctx| {
                            match platform {
                                SyncPlatformSetting::GitHub => {
                                    let _ = settings.github_token.set_value(value, ctx);
                                }
                                SyncPlatformSetting::Gitee => {
                                    let _ = settings.gitee_token.set_value(value, ctx);
                                }
                            }
                        });
                        self.sync_state = SyncState::TokenValid { username };
                    }
                    Err(e) => {
                        self.sync_state = SyncState::Failed {
                            message: e.clone(),
                        };
                    }
                }
                ctx.notify();
            }
            CloudSyncPageAction::ClearToken => {
                let platform = *CloudSyncSettings::as_ref(ctx).sync_platform.value();
                CloudSyncSettings::handle(ctx).update(ctx, |settings, ctx| {
                    match platform {
                        SyncPlatformSetting::GitHub => {
                            let _ = settings.github_token.set_value(String::new(), ctx);
                        }
                        SyncPlatformSetting::Gitee => {
                            let _ = settings.gitee_token.set_value(String::new(), ctx);
                        }
                    }
                });
                self.token_editor.update(ctx, |editor, ctx| {
                    editor.set_buffer_text("", ctx);
                });
                ctx.notify();
            }
            CloudSyncPageAction::Upload => {
                let platform = CloudSyncSettings::as_ref(ctx)
                    .sync_platform
                    .value()
                    .to_sync_platform();
                self.spawn_upload(platform, ctx);
            }
            CloudSyncPageAction::Download => {
                let platform = CloudSyncSettings::as_ref(ctx)
                    .sync_platform
                    .value()
                    .to_sync_platform();
                self.download_confirm_visible = true;
                self.download_confirm_platform = platform;
                ctx.notify();
            }
            CloudSyncPageAction::SyncComplete {
                platform,
                direction,
                result,
            } => {
                let platform = *platform;
                let direction = *direction;
                match result {
                    Ok(SyncResult::Success { version, .. }) => {
                        self.sync_state = SyncState::Success {
                            platform,
                            direction,
                            version: *version,
                        };
                        if direction == SyncDirection::Download {
                            SshTreeChangedNotifier::handle(ctx).update(ctx, |_, ctx| {
                                ctx.emit(SshTreeChangedEvent::TreeChanged);
                            });
                        }
                    }
                    Ok(SyncResult::Conflict {
                        local_version,
                        remote_version,
                    }) => {
                        self.sync_state = SyncState::Conflict {
                            local_version: *local_version,
                            remote_version: *remote_version,
                            direction,
                            platform,
                        };
                        self.conflict_visible = true;
                        self.conflict_local_version = *local_version;
                        self.conflict_remote_version = *remote_version;
                        self.conflict_platform = platform;
                    }
                    Ok(SyncResult::AlreadyUpToDate { version }) => {
                        self.sync_state = SyncState::Success {
                            platform,
                            direction,
                            version: *version,
                        };
                        if direction == SyncDirection::Download {
                            SshTreeChangedNotifier::handle(ctx).update(ctx, |_, ctx| {
                                ctx.emit(SshTreeChangedEvent::TreeChanged);
                            });
                        }
                    }
                    Err(e) => {
                        self.sync_state = SyncState::Failed {
                            message: e.clone(),
                        };
                    }
                }
                ctx.notify();
            }
            CloudSyncPageAction::ForceUpload { platform } => {
                let platform = *platform;
                self.conflict_visible = false;
                self.spawn_force_upload(platform, ctx);
            }
            CloudSyncPageAction::CancelConflict => {
                self.conflict_visible = false;
                self.sync_state = SyncState::Idle;
                ctx.notify();
            }
            CloudSyncPageAction::ConfirmDownload { platform } => {
                let platform = *platform;
                self.download_confirm_visible = false;
                ctx.notify();
                self.spawn_download(platform, ctx);
            }
            CloudSyncPageAction::CancelDownloadConfirm => {
                self.download_confirm_visible = false;
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
        self.page.scroll_to_widget(widget_id);
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

#[derive(Default)]
struct CloudSyncPageWidget;

impl SettingsWidget for CloudSyncPageWidget {
    type View = CloudSyncPageView;

    fn search_terms(&self) -> &str {
        "cloud sync gist github gitee backup token upload download"
    }

    fn render(
        &self,
        view: &CloudSyncPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();

        let mut content = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(
                Text::new(
                    crate::t!("settings-cloud-sync-description"),
                    appearance.ui_font_family(),
                    appearance.ui_font_body(),
                )
                .with_color(theme.nonactive_ui_text_color().into())
                .finish(),
            );

        // 同步范围说明
        let scope_icon = Container::new(
            ConstrainedBox::new(
                Icon::AlertCircle
                    .to_warpui_icon(theme.active_ui_text_color())
                    .finish(),
            )
            .with_width(16.)
            .with_height(16.)
            .finish(),
        )
        .with_margin_right(8.)
        .finish();
        let scope_text = Text::new(
            crate::t!("settings-cloud-sync-scope-note"),
            appearance.ui_font_family(),
            appearance.ui_font_footnote(),
        )
        .with_color(theme.active_ui_text_color().into())
        .finish();
        content.add_child(
            Container::new(
                Flex::row()
                    .with_child(scope_icon)
                    .with_child(scope_text)
                    .with_main_axis_size(MainAxisSize::Max)
                    .finish(),
            )
            .with_background_color(theme.accent_overlay().into())
            .with_uniform_padding(12.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
            .with_margin_top(8.)
            .finish(),
        );

        // 平台选择 Dropdown
        let dropdown_element = warpui::elements::ChildView::new(&view.platform_dropdown).finish();
        content.add_child(render_body_item::<CloudSyncPageAction>(
            crate::t!("settings-cloud-sync-platform-label"),
            None::<AdditionalInfo<CloudSyncPageAction>>,
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            dropdown_element,
            Some(crate::t!("settings-cloud-sync-platform-description")),
        ));

        // Token 编辑器
        let editor_element = warpui::elements::ChildView::new(&view.token_editor).finish();
        let save_button = Container::new(
            appearance
                .ui_builder()
                .button(ButtonVariant::Accent, view.save_state.clone())
                .with_style(UiComponentStyles {
                    font_size: Some(appearance.ui_font_body()),
                    padding: Some(Coords::uniform(BUTTON_PADDING)),
                    ..Default::default()
                })
                .with_text_label(crate::t!("common-save"))
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(CloudSyncPageAction::SaveToken);
                })
                .finish(),
        )
        .with_margin_left(6.)
        .finish();
        let clear_button = Container::new(
            appearance
                .ui_builder()
                .button(ButtonVariant::Text, view.clear_state.clone())
                .with_style(UiComponentStyles {
                    font_size: Some(appearance.ui_font_body()),
                    padding: Some(Coords::uniform(BUTTON_PADDING)),
                    ..Default::default()
                })
                .with_text_label(crate::t!("settings-cloud-sync-clear"))
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(CloudSyncPageAction::ClearToken);
                })
                .finish(),
        )
        .with_margin_left(4.)
        .finish();

        let input_area = ConstrainedBox::new(
            Flex::row()
                .with_main_axis_size(MainAxisSize::Min)
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    ConstrainedBox::new(editor_element)
                        .with_max_width(INPUT_AREA_MAX_WIDTH - 120.0)
                        .finish(),
                )
                .with_child(save_button)
                .with_child(clear_button)
                .finish(),
        )
        .with_max_width(INPUT_AREA_MAX_WIDTH)
        .finish();

        content.add_child(render_body_item::<CloudSyncPageAction>(
            crate::t!("settings-cloud-sync-token-label"),
            None::<AdditionalInfo<CloudSyncPageAction>>,
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            input_area,
            Some(crate::t!("settings-cloud-sync-token-description")),
        ));

        // 同步操作区域
        let is_syncing = matches!(view.sync_state, SyncState::Syncing { .. } | SyncState::Validating);

        let render_sync_button = |label: &str,
                                  mouse: &MouseStateHandle,
                                  action: CloudSyncPageAction,
                                  disabled: bool|
         -> Box<dyn Element> {
            let mut btn = appearance
                .ui_builder()
                .button(ButtonVariant::Secondary, mouse.clone())
                .with_style(UiComponentStyles {
                    font_size: Some(appearance.ui_font_body()),
                    padding: Some(Coords::uniform(BUTTON_PADDING)),
                    ..Default::default()
                })
                .with_text_label(label.to_string())
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(action.clone());
                });
            if disabled {
                btn = btn.disable();
            }
            btn.finish()
        };

        let upload_btn = render_sync_button(
            &crate::t!("settings-cloud-sync-upload-label"),
            &view.upload_mouse,
            CloudSyncPageAction::Upload,
            is_syncing,
        );
        let download_btn = render_sync_button(
            &crate::t!("settings-cloud-sync-download-label"),
            &view.download_mouse,
            CloudSyncPageAction::Download,
            is_syncing,
        );

        let buttons_row = Flex::row()
            .with_child(upload_btn)
            .with_child(Container::new(download_btn).with_margin_left(8.).finish())
            .finish();

        content.add_child(render_body_item::<CloudSyncPageAction>(
            crate::t!("settings-cloud-sync-operations-header"),
            None::<AdditionalInfo<CloudSyncPageAction>>,
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            buttons_row,
            None,
        ));

        // 同步状态区域
        let version = with_conn(|c| Ok(SshRepository::get_sync_version(c)?))
            .map(|v| v.to_string())
            .unwrap_or_else(|_| crate::t!("settings-cloud-sync-na"));

        let last_sync_time = with_conn(|c| Ok(SshRepository::get_last_sync_time(c)?))
            .unwrap_or_else(|e| {
                log::debug!("Failed to get last sync time: {e}");
                crate::t!("settings-cloud-sync-never")
            });

        let last_sync_platform =
            with_conn(|c| Ok(SshRepository::get_last_sync_platform(c)?))
                .unwrap_or_else(|e| {
                    log::debug!("Failed to get last sync platform: {e}");
                    crate::t!("settings-cloud-sync-na")
                });

        let info_color = theme.nonactive_ui_text_color();

        let version_text = Text::new(version, appearance.ui_font_family(), appearance.ui_font_body())
            .with_color(info_color.into())
            .finish();
        content.add_child(render_body_item::<CloudSyncPageAction>(
            crate::t!("settings-cloud-sync-local-version-label"),
            None::<AdditionalInfo<CloudSyncPageAction>>,
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            version_text,
            None,
        ));

        let time_text = Text::new(last_sync_time, appearance.ui_font_family(), appearance.ui_font_body())
            .with_color(info_color.into())
            .finish();
        content.add_child(render_body_item::<CloudSyncPageAction>(
            crate::t!("settings-cloud-sync-last-time-label"),
            None::<AdditionalInfo<CloudSyncPageAction>>,
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            time_text,
            None,
        ));

        let platform_text = Text::new(last_sync_platform, appearance.ui_font_family(), appearance.ui_font_body())
            .with_color(info_color.into())
            .finish();
        content.add_child(render_body_item::<CloudSyncPageAction>(
            crate::t!("settings-cloud-sync-last-platform-label"),
            None::<AdditionalInfo<CloudSyncPageAction>>,
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            platform_text,
            None,
        ));

        // 同步操作状态（带颜色区分）
        let state_color: Option<pathfinder_color::ColorU> = match &view.sync_state {
            SyncState::Idle => None,
            SyncState::Validating => Some(theme.ansi_fg_yellow()),
            SyncState::TokenValid { .. } => Some(theme.ansi_fg_green()),
            SyncState::Success { .. } => Some(theme.ansi_fg_green()),
            SyncState::Failed { .. } => Some(theme.ansi_fg_red()),
            SyncState::Conflict { .. } => Some(theme.ansi_fg_yellow()),
            SyncState::Syncing { .. } => Some(theme.active_ui_text_color().into_solid()),
        };

        let state_text = match &view.sync_state {
            SyncState::Idle => None,
            SyncState::Validating => {
                Some(crate::t!("settings-cloud-sync-validating"))
            }
            SyncState::TokenValid { username } => {
                Some(crate::t!("settings-cloud-sync-token-valid", username = username.clone()))
            }
            SyncState::Syncing { platform, direction } => {
                let dir = match direction {
                    SyncDirection::Upload => crate::t!("settings-cloud-sync-direction-upload"),
                    SyncDirection::Download => crate::t!("settings-cloud-sync-direction-download"),
                };
                Some(crate::t!("settings-cloud-sync-syncing", direction = dir, platform = platform.label().to_string()))
            }
            SyncState::Success {
                platform,
                direction,
                version,
            } => {
                let dir = match direction {
                    SyncDirection::Upload => crate::t!("settings-cloud-sync-direction-upload"),
                    SyncDirection::Download => crate::t!("settings-cloud-sync-direction-download"),
                };
                Some(crate::t!("settings-cloud-sync-success", direction = dir, platform = platform.label().to_string(), version = (*version as i64)))
            }
            SyncState::Failed { message } => {
                Some(crate::t!("settings-cloud-sync-failed", error = message.clone()))
            }
            SyncState::Conflict {
                local_version,
                remote_version,
                ..
            } => {
                Some(crate::t!("settings-cloud-sync-conflict-status", local = (*local_version as i64), remote = (*remote_version as i64)))
            }
        };

        if let Some(text) = state_text {
            let color = state_color.unwrap_or_else(|| theme.nonactive_ui_text_color().into_solid());
            content.add_child(
                Container::new(
                    Text::new(text, appearance.ui_font_family(), appearance.ui_font_body())
                        .with_color(color)
                        .finish(),
                )
                .with_margin_top(8.)
                .finish(),
            );
        }

        // 冲突弹窗
        if view.conflict_visible {
            let description_text = crate::t!(
                "settings-cloud-sync-conflict-description",
                remote = view.conflict_remote_version,
                local = view.conflict_local_version
            );

            let force_button = Container::new(
                appearance
                    .ui_builder()
                    .button(ButtonVariant::Accent, view.conflict_force_mouse.clone())
                    .with_style(UiComponentStyles {
                        font_size: Some(appearance.ui_font_body()),
                        padding: Some(Coords::uniform(BUTTON_PADDING)),
                        ..Default::default()
                    })
                    .with_text_label(crate::t!("settings-cloud-sync-force-upload"))
                    .build()
                    .on_click({
                        let platform = view.conflict_platform;
                        move |ctx, _, _| {
                            ctx.dispatch_typed_action(CloudSyncPageAction::ForceUpload { platform });
                        }
                    })
                    .finish(),
            )
            .with_margin_left(12.)
            .finish();

            let cancel_button = appearance
                .ui_builder()
                .button(ButtonVariant::Secondary, view.conflict_cancel_mouse.clone())
                .with_style(UiComponentStyles {
                    font_size: Some(appearance.ui_font_body()),
                    padding: Some(Coords::uniform(BUTTON_PADDING)),
                    ..Default::default()
                })
                .with_text_label(crate::t!("common-cancel"))
                .build()
                .on_click(|ctx, _, _| {
                    ctx.dispatch_typed_action(CloudSyncPageAction::CancelConflict);
                })
                .finish();

            use crate::ui_components::dialog::{dialog_styles, Dialog};

            let dialog = Dialog::new(
                crate::t!("settings-cloud-sync-conflict-title"),
                Some(description_text),
                dialog_styles(appearance),
            )
            .with_bottom_row_child(cancel_button)
            .with_bottom_row_child(force_button)
            .with_width(DIALOG_WIDTH)
            .build()
            .finish();

            content.add_child(
                Dismiss::new(dialog)
                    .prevent_interaction_with_other_elements()
                    .on_dismiss(|ctx, _app| {
                        ctx.dispatch_typed_action(CloudSyncPageAction::CancelConflict);
                    })
                    .finish(),
            );
        }

        // 下载确认弹窗
        if view.download_confirm_visible {
            use crate::ui_components::dialog::{dialog_styles, Dialog};

            let confirm_button = Container::new(
                appearance
                    .ui_builder()
                    .button(ButtonVariant::Accent, view.download_confirm_mouse.clone())
                    .with_style(UiComponentStyles {
                        font_size: Some(appearance.ui_font_body()),
                        padding: Some(Coords::uniform(BUTTON_PADDING)),
                        ..Default::default()
                    })
                    .with_text_label(crate::t!("settings-cloud-sync-download-confirm-button"))
                    .build()
                    .on_click({
                        let platform = view.download_confirm_platform;
                        move |ctx, _, _| {
                            ctx.dispatch_typed_action(CloudSyncPageAction::ConfirmDownload { platform });
                        }
                    })
                    .finish(),
            )
            .with_margin_left(12.)
            .finish();

            let cancel_button = appearance
                .ui_builder()
                .button(ButtonVariant::Secondary, view.download_confirm_cancel_mouse.clone())
                .with_style(UiComponentStyles {
                    font_size: Some(appearance.ui_font_body()),
                    padding: Some(Coords::uniform(BUTTON_PADDING)),
                    ..Default::default()
                })
                .with_text_label(crate::t!("common-cancel"))
                .build()
                .on_click(|ctx, _, _| {
                    ctx.dispatch_typed_action(CloudSyncPageAction::CancelDownloadConfirm);
                })
                .finish();

            let dialog = Dialog::new(
                crate::t!("settings-cloud-sync-download-confirm-title"),
                Some(crate::t!("settings-cloud-sync-download-confirm-description")),
                dialog_styles(appearance),
            )
            .with_bottom_row_child(cancel_button)
            .with_bottom_row_child(confirm_button)
            .with_width(DIALOG_WIDTH)
            .build()
            .finish();

            content.add_child(
                Dismiss::new(dialog)
                    .prevent_interaction_with_other_elements()
                    .on_dismiss(|ctx, _app| {
                        ctx.dispatch_typed_action(CloudSyncPageAction::CancelDownloadConfirm);
                    })
                    .finish(),
            );
        }

        content.finish()
    }
}
