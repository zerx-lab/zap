//! 云同步设置页面 — 平台选择、Token 配置、同步操作、状态显示
//!
// author: logic
// date: 2026-05-25

use pathfinder_geometry::vector::vec2f;
use settings::Setting;
use warpui::{
    elements::{
        ChildAnchor, Container, CrossAxisAlignment, Dismiss, Element, Flex, MainAxisSize,
        MouseStateHandle, OffsetPositioning, ParentAnchor, ParentElement, ParentOffsetBounds,
        Stack, Text,
    },
    ui_components::{
        button::ButtonVariant,
        components::{Coords, UiComponent, UiComponentStyles},
        switch::SwitchStateHandle,
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
use crate::settings::{CloudSyncTokenStore, GITHUB_TOKEN_KEY, GITEE_TOKEN_KEY};
use crate::ssh_manager::{SshTreeChangedEvent, SshTreeChangedNotifier};
use crate::view_components::dropdown::{Dropdown, DropdownItem};

use warp_ssh_manager::{with_conn, DbVersionStore, SyncMetaRepository, SshSyncProvider};
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
    AlreadyUpToDate {
        version: i64,
    },
    Failed {
        message: String,
    },
    /// 自动同步上传成功
    AutoSyncSuccess,
    Conflict {
        local_version: i64,
        remote_version: i64,
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
    /// Token 验证完成。platform/token 由 SaveToken 时捕获,避免与 SetPlatform 竞态
    TokenValidated {
        platform_setting: SyncPlatformSetting,
        token: String,
        result: Result<String, String>,
    },
    /// 请求上传同步（弹出确认弹窗,避免误覆盖云端历史）
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
    /// 确认上传 — token 在 View 字段中捕获,无需通过 action 传递(避免 String clone 开销)
    ConfirmUpload { platform: SyncPlatform },
    /// 取消上传确认
    CancelUploadConfirm,
    /// 切换自动同步开关
    ToggleAutoSync,
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
    /// 进入 Conflict 状态时捕获的 token,Force Upload 时使用,避免确认期间用户切平台
    conflict_token: String,
    download_confirm_visible: bool,
    download_confirm_platform: SyncPlatform,
    /// 打开下载确认弹窗时捕获的 token 快照,Confirm 时直接使用,
    /// 避免确认过程中用户切平台或 ClearToken 导致 spawn 用错凭据
    download_confirm_token: String,
    download_confirm_mouse: MouseStateHandle,
    download_confirm_cancel_mouse: MouseStateHandle,
    upload_confirm_visible: bool,
    upload_confirm_platform: SyncPlatform,
    upload_confirm_token: String,
    upload_confirm_mouse: MouseStateHandle,
    upload_confirm_cancel_mouse: MouseStateHandle,
    cached_version: String,
    cached_last_sync_time: String,
    cached_last_sync_platform: String,
    has_valid_token: bool,
    /// 自动同步开关状态
    auto_sync_mouse: MouseStateHandle,
    auto_sync_switch: SwitchStateHandle,
    /// 自动同步防抖标记 — 为 true 时跳过 SshTreeChanged 触发的自动上传
    suppress_auto_upload: bool,
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

/// 从 CloudSyncSettings 同步 Dropdown 选中状态
fn sync_from_settings(me: &mut CloudSyncPageView, ctx: &mut ViewContext<CloudSyncPageView>) {
    let platform = *CloudSyncSettings::as_ref(ctx).sync_platform.value();

    let label = platform.label().to_string();
    me.platform_dropdown.update(ctx, |dropdown, ctx| {
        dropdown.set_selected_by_name(&label, ctx);
    });
}

/// 从 CloudSyncTokenStore 加载当前平台的 Token 到编辑器
fn load_token_from_store(me: &mut CloudSyncPageView, ctx: &mut ViewContext<CloudSyncPageView>) {
    let platform = *CloudSyncSettings::as_ref(ctx).sync_platform.value();
    let key = match platform {
        SyncPlatformSetting::GitHub => GITHUB_TOKEN_KEY,
        SyncPlatformSetting::Gitee => GITEE_TOKEN_KEY,
    };
    let token = CloudSyncTokenStore::as_ref(ctx)
        .get(key)
        .unwrap_or("")
        .to_string();
    me.has_valid_token = !token.is_empty();
    me.token_editor.update(ctx, |editor, ctx| {
        if editor.buffer_text(ctx) != token {
            editor.set_buffer_text(&token, ctx);
        }
    });
}

/// 获取当前选中平台对应的 Token（从 OS 密钥库读取）
fn current_token(ctx: &AppContext) -> String {
    let platform = *CloudSyncSettings::as_ref(ctx).sync_platform.value();
    token_for_platform(ctx, platform.to_sync_platform())
}

/// 获取指定 SyncPlatform 对应的 Token,不依赖当前 dropdown 选中状态。
/// 用于 force_upload 重新捕获场景:必须读取冲突所属 platform 的 token,
/// 而非用户在冲突期间可能切换到的新 platform。
fn token_for_platform(ctx: &AppContext, platform: SyncPlatform) -> String {
    let key = match platform {
        SyncPlatform::GitHub => GITHUB_TOKEN_KEY,
        SyncPlatform::Gitee => GITEE_TOKEN_KEY,
    };
    CloudSyncTokenStore::as_ref(ctx)
        .get(key)
        .unwrap_or("")
        .to_string()
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
            conflict_token: String::new(),
            download_confirm_visible: false,
            download_confirm_platform: SyncPlatform::GitHub,
            download_confirm_token: String::new(),
            download_confirm_mouse: MouseStateHandle::default(),
            download_confirm_cancel_mouse: MouseStateHandle::default(),
            upload_confirm_visible: false,
            upload_confirm_platform: SyncPlatform::GitHub,
            upload_confirm_token: String::new(),
            upload_confirm_mouse: MouseStateHandle::default(),
            upload_confirm_cancel_mouse: MouseStateHandle::default(),
            cached_version: String::new(),
            cached_last_sync_time: String::new(),
            cached_last_sync_platform: String::new(),
            has_valid_token: false,
            auto_sync_mouse: MouseStateHandle::default(),
            auto_sync_switch: SwitchStateHandle::default(),
            suppress_auto_upload: false,
        };

        // 订阅 SSH 树变更事件，用于自动同步上传
        ctx.subscribe_to_model(
            &SshTreeChangedNotifier::handle(ctx),
            move |me: &mut Self, _, event, ctx| {
                me.handle_ssh_tree_changed(event, ctx);
            },
        );

        me.refresh_sync_cache();
        // 启动时自动下载：如果 auto_sync 启用且有有效 token，异步下载
        {
            let auto_sync_enabled = *CloudSyncSettings::as_ref(ctx).auto_sync.value();
            if auto_sync_enabled {
                let platform = *CloudSyncSettings::as_ref(ctx).sync_platform.value();
                let key = match platform {
                    SyncPlatformSetting::GitHub => GITHUB_TOKEN_KEY,
                    SyncPlatformSetting::Gitee => GITEE_TOKEN_KEY,
                };
                let token = CloudSyncTokenStore::as_ref(ctx)
                    .get(key)
                    .unwrap_or("")
                    .to_string();
                if !token.is_empty() {
                    let sync_platform = platform.to_sync_platform();
                    let spawn_token = token.clone();
                    me.conflict_token = token;
                    ctx.spawn(
                        async move {
                            let engine = SyncEngine::new();
                            let provider = SshSyncProvider::new();
                            let version_store = DbVersionStore;
                            engine
                                .download(sync_platform, &spawn_token, &[&provider], &version_store)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        move |view, result, ctx| {
                            match &result {
                                Ok(SyncResult::Success { .. }) => {
                                    view.suppress_auto_upload = true;
                                    SshTreeChangedNotifier::handle(ctx).update(ctx, |_, ctx| {
                                        ctx.emit(SshTreeChangedEvent::TreeChanged);
                                    });
                                    view.refresh_sync_cache();
                                    ctx.notify();
                                }
                                Ok(SyncResult::Conflict {
                                    local_version,
                                    remote_version,
                                }) => {
                                    view.sync_state = SyncState::Conflict {
                                        local_version: *local_version,
                                        remote_version: *remote_version,
                                        platform: sync_platform,
                                    };
                                    view.conflict_visible = true;
                                    view.conflict_local_version = *local_version;
                                    view.conflict_remote_version = *remote_version;
                                    view.conflict_platform = sync_platform;
                                    if view.conflict_token.is_empty() {
                                        view.conflict_token = token_for_platform(ctx, sync_platform);
                                    }
                                    ctx.notify();
                                }
                                Ok(SyncResult::AlreadyUpToDate { .. }) => {}
                                Err(e) => {
                                    log::warn!("Auto sync download failed: {e}");
                                }
                            }
                        },
                    );
                }
            }
        }
        sync_from_settings(&mut me, ctx);
        load_token_from_store(&mut me, ctx);
        me
    }

    /// 构造当前应作为 overlay 渲染的模态(冲突 / 下载确认 / 上传确认)。
    /// 由 CloudSyncPageWidget::render 内的 Stack 用 ParentOffsetBounds::WindowByPosition 居中,
    /// 必须从本 View 的 render 路径调用,以保证点击事件可路由回 handle_action
    /// (overlay 由 SettingsView 渲染会丢失 view chain)。
    fn build_modal_element(&self, appearance: &Appearance) -> Option<Box<dyn Element>> {
        use crate::ui_components::dialog::{dialog_styles, Dialog};
        if self.conflict_visible {
            let description_text = if self.conflict_local_version == self.conflict_remote_version {
                crate::t!("settings-cloud-sync-conflict-description-equal")
            } else {
                crate::t!(
                    "settings-cloud-sync-conflict-description",
                    remote = self.conflict_remote_version.to_string(),
                    local = self.conflict_local_version.to_string()
                )
            };

            let force_button = Container::new(
                appearance
                    .ui_builder()
                    .button(ButtonVariant::Warn, self.conflict_force_mouse.clone())
                    .with_style(UiComponentStyles {
                        font_size: Some(appearance.ui_font_body()),
                        padding: Some(Coords::uniform(BUTTON_PADDING)),
                        ..Default::default()
                    })
                    .with_text_label(crate::t!("settings-cloud-sync-force-upload"))
                    .build()
                    .on_click({
                        let platform = self.conflict_platform;
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
                .button(ButtonVariant::Secondary, self.conflict_cancel_mouse.clone())
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

            return Some(
                Dismiss::new(dialog)
                    .prevent_interaction_with_other_elements()
                    .on_dismiss(|ctx, _app| {
                        ctx.dispatch_typed_action(CloudSyncPageAction::CancelConflict);
                    })
                    .finish(),
            );
        }

        if self.download_confirm_visible {
            let confirm_button = Container::new(
                appearance
                    .ui_builder()
                    .button(ButtonVariant::Accent, self.download_confirm_mouse.clone())
                    .with_style(UiComponentStyles {
                        font_size: Some(appearance.ui_font_body()),
                        padding: Some(Coords::uniform(BUTTON_PADDING)),
                        ..Default::default()
                    })
                    .with_text_label(crate::t!("settings-cloud-sync-download-confirm-button"))
                    .build()
                    .on_click({
                        let platform = self.download_confirm_platform;
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
                .button(ButtonVariant::Secondary, self.download_confirm_cancel_mouse.clone())
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

            return Some(
                Dismiss::new(dialog)
                    .prevent_interaction_with_other_elements()
                    .on_dismiss(|ctx, _app| {
                        ctx.dispatch_typed_action(CloudSyncPageAction::CancelDownloadConfirm);
                    })
                    .finish(),
            );
        }

        if self.upload_confirm_visible {
            // 用 Accent(主题主色)而非 Warn(黄色警告色);Force Upload 才用 Warn
            let confirm_button = Container::new(
                appearance
                    .ui_builder()
                    .button(ButtonVariant::Accent, self.upload_confirm_mouse.clone())
                    .with_style(UiComponentStyles {
                        font_size: Some(appearance.ui_font_body()),
                        padding: Some(Coords::uniform(BUTTON_PADDING)),
                        ..Default::default()
                    })
                    .with_text_label(crate::t!("settings-cloud-sync-upload-confirm-button"))
                    .build()
                    .on_click({
                        let platform = self.upload_confirm_platform;
                        move |ctx, _, _| {
                            ctx.dispatch_typed_action(CloudSyncPageAction::ConfirmUpload { platform });
                        }
                    })
                    .finish(),
            )
            .with_margin_left(12.)
            .finish();

            let cancel_button = appearance
                .ui_builder()
                .button(ButtonVariant::Secondary, self.upload_confirm_cancel_mouse.clone())
                .with_style(UiComponentStyles {
                    font_size: Some(appearance.ui_font_body()),
                    padding: Some(Coords::uniform(BUTTON_PADDING)),
                    ..Default::default()
                })
                .with_text_label(crate::t!("common-cancel"))
                .build()
                .on_click(|ctx, _, _| {
                    ctx.dispatch_typed_action(CloudSyncPageAction::CancelUploadConfirm);
                })
                .finish();

            let dialog = Dialog::new(
                crate::t!("settings-cloud-sync-upload-confirm-title"),
                Some(crate::t!("settings-cloud-sync-upload-confirm-description")),
                dialog_styles(appearance),
            )
            .with_bottom_row_child(cancel_button)
            .with_bottom_row_child(confirm_button)
            .with_width(DIALOG_WIDTH)
            .build()
            .finish();

            return Some(
                Dismiss::new(dialog)
                    .prevent_interaction_with_other_elements()
                    .on_dismiss(|ctx, _app| {
                        ctx.dispatch_typed_action(CloudSyncPageAction::CancelUploadConfirm);
                    })
                    .finish(),
            );
        }

        None
    }

    /// 刷新同步状态缓存
    fn refresh_sync_cache(&mut self) {
        self.cached_version = with_conn(|c| Ok(SyncMetaRepository::get_sync_version(c)?))
            .map(|v| v.to_string())
            .unwrap_or_else(|_| crate::t!("settings-cloud-sync-na"));
        self.cached_last_sync_time = with_conn(|c| Ok(SyncMetaRepository::get_last_sync_time(c)?))
            .unwrap_or_else(|e| {
                log::debug!("Failed to get last sync time: {e}");
                crate::t!("settings-cloud-sync-never")
            });
        self.cached_last_sync_platform = with_conn(|c| Ok(SyncMetaRepository::get_last_sync_platform(c)?))
            .unwrap_or_else(|e| {
                log::debug!("Failed to get last sync platform: {e}");
                crate::t!("settings-cloud-sync-na")
            });
    }

    /// 启动上传同步。token 由调用方在弹窗打开时捕获,保证与 platform 配对。
    fn spawn_upload(&mut self, platform: SyncPlatform, token: String, ctx: &mut ViewContext<Self>) {
        if token.is_empty() {
            let label = platform.label();
            self.sync_state = SyncState::Failed {
                message: crate::t!("settings-cloud-sync-token-not-configured", platform = label.to_string()),
            };
            ctx.notify();
            return;
        }

        // 把当前 token 保存为 conflict_token,若上传返回 Conflict → Force Upload 重试时复用
        self.conflict_token = token.clone();

        self.sync_state = SyncState::Syncing {
            platform,
            direction: SyncDirection::Upload,
        };
        ctx.notify();

        let spawn_token = token;
        ctx.spawn(
            async move {
                let engine = SyncEngine::new();
                let provider = SshSyncProvider::new();
                let version_store = DbVersionStore;
                engine
                    .upload(platform, &spawn_token, &[&provider], &version_store)
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

    /// 启动下载同步。token 由调用方在弹窗打开时捕获。
    fn spawn_download(&mut self, platform: SyncPlatform, spawn_token: String, ctx: &mut ViewContext<Self>) {
        let token = spawn_token;
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

    /// 启动强制上传同步（覆盖远程）。token 来自冲突弹出时的快照。
    fn spawn_force_upload(&mut self, platform: SyncPlatform, token: String, ctx: &mut ViewContext<Self>) {
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

    /// 处理 SSH 树变更事件 — 自动同步上传的入口
    fn handle_ssh_tree_changed(
        &mut self,
        event: &SshTreeChangedEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            SshTreeChangedEvent::TreeChanged => {
                if self.suppress_auto_upload {
                    self.suppress_auto_upload = false;
                    return;
                }
                self.spawn_auto_upload(ctx);
            }
        }
    }

    /// 防抖自动上传：2 秒延迟后执行
    fn spawn_auto_upload(&mut self, ctx: &mut ViewContext<Self>) {
        let auto_sync_enabled = *CloudSyncSettings::as_ref(ctx).auto_sync.value();
        if !auto_sync_enabled {
            return;
        }
        let is_syncing = matches!(
            self.sync_state,
            SyncState::Syncing { .. } | SyncState::Validating
        );
        if is_syncing {
            return;
        }

        let platform = CloudSyncSettings::as_ref(ctx)
            .sync_platform
            .value()
            .to_sync_platform();
        let token = current_token(ctx);
        if token.is_empty() {
            return;
        }

        self.sync_state = SyncState::Syncing {
            platform,
            direction: SyncDirection::Upload,
        };
        ctx.notify();

        let spawn_token = token;
        ctx.spawn(
            async move {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                let engine = SyncEngine::new();
                let provider = SshSyncProvider::new();
                let version_store = DbVersionStore;
                engine
                    .upload(platform, &spawn_token, &[&provider], &version_store)
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
                self.sync_state = SyncState::Idle;
                CloudSyncSettings::handle(ctx).update(ctx, |settings, ctx| {
                    let _ = settings.sync_platform.set_value(platform, ctx);
                });
                load_token_from_store(self, ctx);
                ctx.notify();
            }
            CloudSyncPageAction::SaveToken => {
                let value = self.token_editor.as_ref(ctx).buffer_text(ctx);
                let platform_setting = *CloudSyncSettings::as_ref(ctx).sync_platform.value();
                let platform = platform_setting.to_sync_platform();
                if value.is_empty() {
                    ctx.notify();
                    return;
                }
                self.sync_state = SyncState::Validating;
                ctx.notify();

                // 派发时捕获 platform + token,避免异步期间用户切平台导致写错 keychain key
                let token = value.clone();
                let captured_token = token.clone();
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
                            &CloudSyncPageAction::TokenValidated {
                                platform_setting,
                                token: captured_token.clone(),
                                result,
                            },
                            ctx,
                        );
                    },
                );
            }
            CloudSyncPageAction::TokenValidated {
                platform_setting,
                token,
                result,
            } => {
                let current_platform = *CloudSyncSettings::as_ref(ctx).sync_platform.value();
                match result {
                    Ok(username) => {
                        let username = username.clone();
                        // 用派发时捕获的 platform / token 写 keychain,而非当前 context
                        let key = match platform_setting {
                            SyncPlatformSetting::GitHub => GITHUB_TOKEN_KEY,
                            SyncPlatformSetting::Gitee => GITEE_TOKEN_KEY,
                        };
                        CloudSyncTokenStore::handle(ctx).update(
                            ctx,
                            |store: &mut CloudSyncTokenStore, ctx| {
                                store.set(key, token.clone(), ctx);
                            },
                        );
                        // 只有当前显示的平台与被验证的平台一致时,才更新 UI 状态;
                        // 否则用户已切到别的平台,验证结果不应覆盖当前 UI
                        if *platform_setting == current_platform {
                            self.has_valid_token = true;
                            self.sync_state = SyncState::TokenValid { username };
                        }
                    }
                    Err(e) => {
                        if *platform_setting == current_platform {
                            self.has_valid_token = false;
                            self.sync_state = SyncState::Failed {
                                message: e.clone(),
                            };
                        }
                    }
                }
                ctx.notify();
            }
            CloudSyncPageAction::ClearToken => {
                let platform = *CloudSyncSettings::as_ref(ctx).sync_platform.value();
                let key = match platform {
                    SyncPlatformSetting::GitHub => GITHUB_TOKEN_KEY,
                    SyncPlatformSetting::Gitee => GITEE_TOKEN_KEY,
                };
                CloudSyncTokenStore::handle(ctx).update(ctx, |store: &mut CloudSyncTokenStore, ctx| {
                    store.set(key, String::new(), ctx);
                });
                self.token_editor.update(ctx, |editor, ctx| {
                    editor.set_buffer_text("", ctx);
                });
                self.has_valid_token = false;
                ctx.notify();
            }
            CloudSyncPageAction::Upload => {
                let platform = CloudSyncSettings::as_ref(ctx)
                    .sync_platform
                    .value()
                    .to_sync_platform();
                let token = current_token(ctx);
                // 早返回:token 为空时直接置 Failed,不弹只能失败的确认框 (PR #161 三轮 review)
                if token.is_empty() {
                    let label = platform.label();
                    self.sync_state = SyncState::Failed {
                        message: crate::t!("settings-cloud-sync-token-not-configured", platform = label.to_string()),
                    };
                    ctx.notify();
                    return;
                }
                // 上传具备覆盖云端历史的破坏性,与下载对称弹出二次确认。
                // 在弹窗打开时立刻捕获 token 快照,避免确认期间用户切平台 / ClearToken 导致
                // spawn_upload 用错凭据 (PR #161 二轮 review)
                self.upload_confirm_visible = true;
                self.upload_confirm_platform = platform;
                self.upload_confirm_token = token;
                ctx.notify();
            }
            CloudSyncPageAction::Download => {
                let platform = CloudSyncSettings::as_ref(ctx)
                    .sync_platform
                    .value()
                    .to_sync_platform();
                let token = current_token(ctx);
                if token.is_empty() {
                    let label = platform.label();
                    self.sync_state = SyncState::Failed {
                        message: crate::t!("settings-cloud-sync-token-not-configured", platform = label.to_string()),
                    };
                    ctx.notify();
                    return;
                }
                self.download_confirm_visible = true;
                self.download_confirm_platform = platform;
                self.download_confirm_token = token;
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
                        // 非冲突结局:清掉 conflict_token,避免 PAT 长期驻留在 view 内存
                        self.conflict_token.clear();
                        if direction == SyncDirection::Download {
                            self.suppress_auto_upload = true;
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
                            platform,
                        };
                        self.conflict_visible = true;
                        self.conflict_local_version = *local_version;
                        self.conflict_remote_version = *remote_version;
                        self.conflict_platform = platform;
                        // 进入 Conflict 时刷新 conflict_token,避免后续 Force Upload 用空 token:
                        // - 首次冲突:spawn_upload 已把 token 写入 conflict_token,这里再次覆盖也无害
                        // - force_upload 又返回 Conflict:之前 mem::take 已清空 conflict_token,
                        //   必须根据冲突所属 platform 重新捕获(而不是当前 dropdown 平台,
                        //   用户可能在冲突期间切换;PR #161 四轮 review)
                        if self.conflict_token.is_empty() {
                            self.conflict_token = token_for_platform(ctx, platform);
                        }
                    }
                    Ok(SyncResult::AlreadyUpToDate { version }) => {
                        self.sync_state = SyncState::AlreadyUpToDate {
                            version: *version,
                        };
                        self.conflict_token.clear();
                    }
                    Err(e) => {
                        self.sync_state = SyncState::Failed {
                            message: e.clone(),
                        };
                        self.conflict_token.clear();
                    }
                }
                self.refresh_sync_cache();
                ctx.notify();
            }
            CloudSyncPageAction::ForceUpload { platform } => {
                let platform = *platform;
                let token = std::mem::take(&mut self.conflict_token);
                self.conflict_visible = false;
                self.spawn_force_upload(platform, token, ctx);
            }
            CloudSyncPageAction::CancelConflict => {
                self.conflict_visible = false;
                self.sync_state = SyncState::Idle;
                // 与 CancelUploadConfirm / CancelDownloadConfirm 保持对称,清掉残留 PAT
                self.conflict_token.clear();
                ctx.notify();
            }
            CloudSyncPageAction::ConfirmDownload { platform } => {
                let platform = *platform;
                let token = std::mem::take(&mut self.download_confirm_token);
                self.download_confirm_visible = false;
                ctx.notify();
                self.spawn_download(platform, token, ctx);
            }
            CloudSyncPageAction::CancelDownloadConfirm => {
                self.download_confirm_visible = false;
                self.download_confirm_token.clear();
                ctx.notify();
            }
            CloudSyncPageAction::ConfirmUpload { platform } => {
                let platform = *platform;
                let token = std::mem::take(&mut self.upload_confirm_token);
                self.upload_confirm_visible = false;
                ctx.notify();
                self.spawn_upload(platform, token, ctx);
            }
            CloudSyncPageAction::CancelUploadConfirm => {
                self.upload_confirm_visible = false;
                self.upload_confirm_token.clear();
                ctx.notify();
            }
            CloudSyncPageAction::ToggleAutoSync => {
                let current = *CloudSyncSettings::as_ref(ctx).auto_sync.value();
                CloudSyncSettings::handle(ctx).update(ctx, |settings, ctx| {
                    let _ = settings.auto_sync.set_value(!current, ctx);
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

        // 同步范围说明 — 放在页面顶部,作为首要提示
        let mut content = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(super::settings_page::render_settings_info_banner(
                &crate::t!("settings-cloud-sync-scope-note"),
                None,
                appearance,
            ));

        content.add_child(
            Container::new(
                Text::new(
                    crate::t!("settings-cloud-sync-description"),
                    appearance.ui_font_family(),
                    appearance.ui_font_body(),
                )
                .with_color(theme.nonactive_ui_text_color().into())
                .finish(),
            )
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

        // Token 编辑器 — 使用 text_input 组件获得一致的边框和布局约束
        let editor_element = appearance
            .ui_builder()
            .text_input(view.token_editor.clone())
            .with_style(
                UiComponentStyles::default()
                    .set_width(INPUT_AREA_MAX_WIDTH - 120.0),
            )
            .build()
            .finish();
        let is_validating = matches!(view.sync_state, SyncState::Validating);
        let save_label = if is_validating {
            crate::t!("settings-cloud-sync-validating")
        } else {
            crate::t!("common-save")
        };
        let save_button = Container::new(
            {
                let mut btn = appearance
                    .ui_builder()
                    .button(ButtonVariant::Accent, view.save_state.clone())
                    .with_style(UiComponentStyles {
                        font_size: Some(appearance.ui_font_body()),
                        padding: Some(Coords::uniform(BUTTON_PADDING)),
                        ..Default::default()
                    })
                    .with_text_label(save_label)
                    .build()
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(CloudSyncPageAction::SaveToken);
                    });
                if is_validating {
                    btn = btn.disable();
                }
                btn.finish()
            },
        )
        .with_margin_left(8.)
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
        .with_margin_left(8.)
        .finish();

        let input_area = Flex::row()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(editor_element)
            .with_child(save_button)
            .with_child(clear_button)
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

        // 自动同步开关
        let auto_sync_enabled = *CloudSyncSettings::as_ref(_app).auto_sync.value();
        let auto_sync_switch = appearance
            .ui_builder()
            .switch(view.auto_sync_switch.clone())
            .check(auto_sync_enabled)
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(CloudSyncPageAction::ToggleAutoSync);
            })
            .finish();
        content.add_child(render_body_item::<CloudSyncPageAction>(
            crate::t!("settings-cloud-sync-auto-sync-label"),
            None::<AdditionalInfo<CloudSyncPageAction>>,
            LocalOnlyIconState::Hidden,
            if view.has_valid_token {
                ToggleState::Enabled
            } else {
                ToggleState::Disabled
            },
            appearance,
            auto_sync_switch,
            Some(crate::t!("settings-cloud-sync-auto-sync-description")),
        ));

        // 同步操作
        content.add_child(
            Container::new(
                super::settings_page::render_sub_header(
                    appearance,
                    crate::t!("settings-cloud-sync-operations-header"),
                    None,
                ),
            )
            .with_margin_top(12.)
            .finish(),
        );
        let is_syncing = matches!(view.sync_state, SyncState::Syncing { .. } | SyncState::Validating);
        let can_sync = view.has_valid_token && !is_syncing;

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
            !can_sync,
        );
        let download_btn = render_sync_button(
            &crate::t!("settings-cloud-sync-download-label"),
            &view.download_mouse,
            CloudSyncPageAction::Download,
            !can_sync,
        );

        let buttons_row = Flex::row()
            .with_child(upload_btn)
            .with_child(Container::new(download_btn).with_margin_left(8.).finish())
            .finish();

        // 与下方版本信息列表保持 12px 间距,避免按钮贴着 本地版本 标签
        content.add_child(
            Container::new(buttons_row)
                .with_margin_bottom(12.)
                .finish(),
        );

        // 同步状态区域（使用缓存）
        let version = &view.cached_version;
        let last_sync_time = &view.cached_last_sync_time;
        let last_sync_platform = &view.cached_last_sync_platform;

        let info_color = theme.nonactive_ui_text_color();

        let version_text = Text::new(version.clone(), appearance.ui_font_family(), appearance.ui_font_body())
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

        let time_text = Text::new(last_sync_time.clone(), appearance.ui_font_family(), appearance.ui_font_body())
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

        let platform_text = Text::new(last_sync_platform.clone(), appearance.ui_font_family(), appearance.ui_font_body())
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
            SyncState::Validating => Some(theme.active_ui_text_color().into_solid()),
            SyncState::TokenValid { .. } => Some(theme.accent().into_solid()),
            SyncState::Success { .. } => Some(theme.accent().into_solid()),
            SyncState::AlreadyUpToDate { .. } => Some(theme.active_ui_text_color().into_solid()),
            SyncState::Failed { .. } => Some(theme.ui_error_color().into()),
            SyncState::AutoSyncSuccess => None,
            SyncState::Conflict { .. } => Some(theme.active_ui_text_color().into_solid()),
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
                match direction {
                    SyncDirection::Upload => Some(crate::t!("settings-cloud-sync-syncing-upload", platform = platform.label().to_string())),
                    SyncDirection::Download => Some(crate::t!("settings-cloud-sync-syncing-download", platform = platform.label().to_string())),
                }
            }
            SyncState::Success {
                platform,
                direction,
                version,
            } => {
                match direction {
                    SyncDirection::Upload => Some(crate::t!("settings-cloud-sync-success-upload", platform = platform.label().to_string(), version = (*version).to_string())),
                    SyncDirection::Download => Some(crate::t!("settings-cloud-sync-success-download", platform = platform.label().to_string(), version = (*version).to_string())),
                }
            }
            SyncState::AlreadyUpToDate { version } => {
                Some(crate::t!("settings-cloud-sync-already-up-to-date", version = (*version).to_string()))
            }
            SyncState::Failed { message } => {
                Some(crate::t!("settings-cloud-sync-failed", error = message.clone()))
            }
            SyncState::AutoSyncSuccess => None,
            SyncState::Conflict {
                local_version,
                remote_version,
                ..
            } => {
                let local = *local_version;
                let remote = *remote_version;
                if local == remote {
                    Some(crate::t!("settings-cloud-sync-conflict-status-equal", local = local.to_string(), remote = remote.to_string()))
                } else {
                    Some(crate::t!("settings-cloud-sync-conflict-status", local = local.to_string(), remote = remote.to_string()))
                }
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

        // 冲突 / 下载确认 / 上传确认弹窗 — 在本 View 的 render 路径构造,
        // 用 Stack overlay child(WindowByPosition + Center)实现窗口居中,
        // 同时保证点击事件能路由回 CloudSyncPageView::handle_action。
        if let Some(modal) = view.build_modal_element(appearance) {
            let mut stack = Stack::new();
            stack.add_child(content.finish());
            stack.add_positioned_overlay_child(
                modal,
                OffsetPositioning::offset_from_parent(
                    vec2f(0., 0.),
                    ParentOffsetBounds::WindowByPosition,
                    ParentAnchor::Center,
                    ChildAnchor::Center,
                ),
            );
            stack.finish()
        } else {
            content.finish()
        }
    }
}
