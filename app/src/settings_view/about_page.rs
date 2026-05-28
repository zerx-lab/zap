use super::{
    settings_page::{
        render_body_item, MatchData, PageType, SettingsPageEvent, SettingsPageMeta,
        SettingsPageViewHandle, SettingsWidget,
    },
    LocalOnlyIconState, SettingsSection, ToggleState,
};
use crate::{
    appearance::Appearance,
    autoupdate::{self, github, AutoupdateStage, AutoupdateState},
    channel::ChannelState,
    report_if_error,
    settings::AutoupdateSettings,
    workspace::WorkspaceAction,
};
use settings::Setting as _;
use warp_core::{execution_mode::AppExecutionMode, settings::ToggleableSetting as _};
use warpui::ui_components::switch::SwitchStateHandle;
use warpui::{
    assets::asset_cache::AssetSource,
    elements::{
        Align, CacheOption, ConstrainedBox, Container, CrossAxisAlignment, Element, Flex, Image,
        MainAxisAlignment, MouseStateHandle, ParentElement, Wrap,
    },
    ui_components::components::UiComponent,
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

#[derive(Debug, Clone)]
pub enum AboutPageAction {
    ToggleAutomaticUpdates,
    /// 用户点击"检查更新"按钮:主动触发一次检查(等价 RequestType::ManualCheck)。
    CheckForUpdate,
    /// 用户点击"前往 GitHub 下载"链接:用系统默认浏览器打开 release 页面。
    /// 仅在异常 fallback 路径里使用(例如下载失败 / 没有可用资产)。
    OpenReleasePage(String),
    /// 用户点击"立即安装"链接:dispatch 给 workspace,触发与菜单 `ApplyUpdate`
    /// 完全等价的安装+重启流程。具体平台行为见 `autoupdate::apply_update`。
    InstallUpdate,
    /// 用户点击"导出日志"链接:弹出原生 save-file 对话框,用户选择保存
    /// 位置后将主日志、MCP 日志、自动更新器日志以及诊断摘要打包为 zip
    /// 直接写入用户指定的路径,完成后通过 workspace toast 反馈成功 / 失败。
    /// 由 `WorkspaceAction::ExportLogsToPath` 负责实现。
    #[cfg(not(target_family = "wasm"))]
    ExportLogs,
}

pub struct AboutPageView {
    page: PageType<Self>,
}

impl AboutPageView {
    pub fn new(ctx: &mut ViewContext<AboutPageView>) -> Self {
        // 订阅 AutoupdateState,stage 变化(检查中 / 发现新版本 / 失败 等)时刷新 UI。
        let autoupdate_handle = AutoupdateState::handle(ctx);
        ctx.observe(&autoupdate_handle, |_, _, ctx| {
            ctx.notify();
        });

        AboutPageView {
            page: PageType::new_monolith(AboutPageWidget::default(), None, false),
        }
    }
}

impl Entity for AboutPageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for AboutPageView {
    type Action = AboutPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            AboutPageAction::ToggleAutomaticUpdates => {
                AutoupdateSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings
                        .automatic_updates_enabled
                        .toggle_and_save_value(ctx));
                });
                ctx.notify();
            }
            AboutPageAction::CheckForUpdate => {
                AutoupdateState::handle(ctx).update(ctx, |state, ctx| {
                    state.manually_check_for_update(ctx);
                });
                ctx.notify();
            }
            AboutPageAction::OpenReleasePage(url) => {
                ctx.open_url(url);
            }
            AboutPageAction::InstallUpdate => {
                // 复用 WorkspaceAction::ApplyUpdate:它会调 autoupdate::apply_update +
                // initiate_relaunch_for_update,平台层在 relaunch() 里决定具体安装动作
                // (mac OSS: open dmg / Win OSS: 非 silent 安装向导 / Linux: 重启新二进制)。
                ctx.dispatch_typed_action(&WorkspaceAction::ApplyUpdate);
            }
            #[cfg(not(target_family = "wasm"))]
            AboutPageAction::ExportLogs => {
                // 触发 workspace 层弹出 save-file 对话框、由用户选择保存路径
                // 后完成打包与 toast 反馈。
                ctx.dispatch_typed_action(&WorkspaceAction::ExportLogsToPath);
            }
        }
    }
}

impl View for AboutPageView {
    fn ui_name() -> &'static str {
        "AboutPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

#[derive(Default)]
struct AboutPageWidget {
    copy_version_button_mouse_state: MouseStateHandle,
    automatic_updates_switch_state: SwitchStateHandle,
    update_action_link_mouse_state: MouseStateHandle,
    /// "导出日志"链接的悬停 / 按下状态。
    #[cfg(not(target_family = "wasm"))]
    export_logs_link_mouse_state: MouseStateHandle,
}

impl SettingsWidget for AboutPageWidget {
    type View = AboutPageView;

    fn search_terms(&self) -> &str {
        "about warp version automatic updates auto update 自动更新 检查更新 新版本"
    }

    fn render(
        &self,
        _view: &AboutPageView,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ui_builder = appearance.ui_builder();

        // 始终用纯图标 logo,品牌名以独立文本 "Zap" 呈现,不再依赖带 "warp" 字样的 svg
        let image_path = "bundled/svg/warp-logo-light.svg";

        // GIT_RELEASE_TAG 注入 → 显示 tag;否则进入 Dev 开发模式
        let version = ChannelState::app_version().unwrap_or("Dev");

        let version_text = ui_builder
            .span(version.to_string())
            .with_soft_wrap()
            .build()
            .with_margin_top(16.)
            .finish();

        let copy_version_icon = appearance
            .ui_builder()
            .copy_button(16., self.copy_version_button_mouse_state.clone())
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(WorkspaceAction::CopyVersion(version));
            })
            .finish();

        let version_row = Wrap::row()
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_children([
                version_text,
                Container::new(copy_version_icon)
                    .with_margin_top(16.)
                    .with_padding_left(6.)
                    .finish(),
            ]);

        let mut content = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                ConstrainedBox::new(
                    Image::new(
                        AssetSource::Bundled { path: image_path },
                        CacheOption::BySize,
                    )
                    .finish(),
                )
                .with_max_height(100.)
                .with_max_width(350.)
                .finish(),
            )
            .with_child(
                ui_builder
                    .span("Zap")
                    .build()
                    .with_margin_top(12.)
                    .finish(),
            )
            .with_child(version_row.finish());

        // 更新状态区域:显示当前是否有新版本,并提供"检查更新"或"前往 GitHub 下载"链接。
        // 仅在能进入 autoupdate 流程的执行模式下渲染(与下方"自动更新"开关共用条件)。
        if AppExecutionMode::as_ref(app).can_autoupdate() {
            content.add_child(
                Container::new(self.render_update_status(appearance, app))
                    .with_margin_top(16.)
                    .finish(),
            );
        }

        content.add_child(
            ui_builder
                .span(crate::t!("settings-about-copyright"))
                .build()
                .with_margin_top(16.)
                .finish(),
        );

        // "导出日志"链接:平台原生导出 zip 给排查人员分享。WASM 平台没有
        // 文件系统日志,跳过。
        #[cfg(not(target_family = "wasm"))]
        {
            let export_link = ui_builder
                .link(
                    crate::t!("settings-about-export-logs"),
                    None,
                    Some(Box::new(|ctx| {
                        ctx.dispatch_typed_action(AboutPageAction::ExportLogs);
                    })),
                    self.export_logs_link_mouse_state.clone(),
                )
                .soft_wrap(false)
                .build()
                .finish();

            // 用一个垂直 Flex 列同时呈现链接和说明文字(说明为什么导出、包含什么)。
            let export_section = Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(export_link)
                .with_child(
                    ui_builder
                        .span(crate::t!("settings-about-export-logs-description"))
                        .with_soft_wrap()
                        .build()
                        .with_margin_top(4.)
                        .finish(),
                )
                .finish();

            content.add_child(Container::new(export_section).with_margin_top(16.).finish());
        }

        if AppExecutionMode::as_ref(app).can_autoupdate() {
            content.add_child(
                Container::new(
                    ConstrainedBox::new(render_body_item::<AboutPageAction>(
                        crate::t!("settings-about-automatic-updates-label"),
                        None,
                        LocalOnlyIconState::Hidden,
                        ToggleState::Enabled,
                        appearance,
                        appearance
                            .ui_builder()
                            .switch(self.automatic_updates_switch_state.clone())
                            .check(
                                *AutoupdateSettings::as_ref(app)
                                    .automatic_updates_enabled
                                    .value(),
                            )
                            .build()
                            .on_click(move |ctx, _, _| {
                                ctx.dispatch_typed_action(AboutPageAction::ToggleAutomaticUpdates);
                            })
                            .finish(),
                        Some(crate::t!("settings-about-automatic-updates-description")),
                    ))
                    .with_max_width(520.)
                    .finish(),
                )
                .with_margin_top(24.)
                .finish(),
            );
        }

        Align::new(content.finish()).finish()
    }
}

impl AboutPageWidget {
    /// 渲染"更新状态"行:状态文字 + 操作链接(检查更新 / 进度展示 / 立即安装 / GitHub 兜底)。
    fn render_update_status(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        let ui_builder = appearance.ui_builder();

        // 当前 stage 决定文案与操作:
        // - NoUpdateAvailable / 未知错误:已是最新 + "检查更新"
        // - CheckingForUpdate:正在检查...(无操作)
        // - DownloadingUpdate:正在下载 X% (X MB / Y MB) (无操作)
        // - UpdateReady / UpdatedPendingRestart:可以安装 + "立即安装"按钮
        // - UnableTo*: 自动安装失败 + "前往 GitHub 下载"兜底链接
        let stage = autoupdate::get_update_state(app);
        let progress = autoupdate::AutoupdateState::as_ref(app).download_progress().cloned();

        let (status_text, action) = match &stage {
            AutoupdateStage::CheckingForUpdate => (
                crate::t!("settings-about-update-checking"),
                UpdateAction::None,
            ),
            AutoupdateStage::DownloadingUpdate => {
                // 三平台共用:从 AutoupdateState.download_progress 拿到下载字节,
                // 拼成"X.X MB / Y.Y MB (P%)";总大小未知时只显示已下载字节。
                let new_version = stage
                    .available_new_version()
                    .map(|v| v.version.as_str())
                    .unwrap_or("");
                let text = match &progress {
                    Some(p) => {
                        // i18n_embed_fl::fl! 要求参数是引用且有 lifetime,所以
                        // 先把进度字符串绑到 let,不要塞临时表达式。
                        let progress_str = format_download_progress(p);
                        crate::t!(
                            "settings-about-update-downloading",
                            version = new_version,
                            progress = progress_str.as_str()
                        )
                    }
                    None => crate::t!(
                        "settings-about-update-downloading-init",
                        version = new_version
                    ),
                };
                (text, UpdateAction::None)
            }
            AutoupdateStage::NoUpdateAvailable => (
                crate::t!("settings-about-update-up-to-date"),
                UpdateAction::Check,
            ),
            AutoupdateStage::UpdateReady { new_version, .. }
            | AutoupdateStage::UpdatedPendingRestart { new_version } => {
                let text = crate::t!(
                    "settings-about-update-ready",
                    version = new_version.version.as_str()
                );
                (text, UpdateAction::Install)
            }
            stage if stage.available_new_version().is_some() => {
                // UnableToUpdateToNewVersion / UnableToLaunchNewVersion / Updating(残留):
                // 自动安装出错或被打断 → 给用户一个手动下载兜底。
                let new_version = stage.available_new_version().unwrap();
                let text = crate::t!(
                    "settings-about-update-available",
                    version = new_version.version.as_str()
                );
                let url = github::cached_release()
                    .map(|r| r.html_url)
                    .unwrap_or_else(|| {
                        "https://github.com/zerx-lab/warp/releases/latest".to_owned()
                    });
                (text, UpdateAction::OpenReleasePage(url))
            }
            // 兜底(理论上不可达):任何剩余 stage 都视为"已是最新"。
            _ => (
                crate::t!("settings-about-update-up-to-date"),
                UpdateAction::Check,
            ),
        };

        let mut row = Flex::row()
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(ui_builder.span(status_text).build().finish());

        match action {
            UpdateAction::None => {}
            UpdateAction::Check => {
                row.add_child(
                    Container::new(
                        ui_builder
                            .link(
                                crate::t!("settings-about-update-check-now"),
                                None,
                                Some(Box::new(|ctx| {
                                    ctx.dispatch_typed_action(AboutPageAction::CheckForUpdate);
                                })),
                                self.update_action_link_mouse_state.clone(),
                            )
                            .soft_wrap(false)
                            .build()
                            .finish(),
                    )
                    .with_padding_left(8.)
                    .finish(),
                );
            }
            UpdateAction::OpenReleasePage(url) => {
                let url_clone = url.clone();
                row.add_child(
                    Container::new(
                        ui_builder
                            .link(
                                crate::t!("settings-about-update-open-release"),
                                None,
                                Some(Box::new(move |ctx| {
                                    ctx.dispatch_typed_action(AboutPageAction::OpenReleasePage(
                                        url_clone.clone(),
                                    ));
                                })),
                                self.update_action_link_mouse_state.clone(),
                            )
                            .soft_wrap(false)
                            .build()
                            .finish(),
                    )
                    .with_padding_left(8.)
                    .finish(),
                );
            }
            UpdateAction::Install => {
                row.add_child(
                    Container::new(
                        ui_builder
                            .link(
                                crate::t!("settings-about-update-install-now"),
                                None,
                                Some(Box::new(|ctx| {
                                    ctx.dispatch_typed_action(AboutPageAction::InstallUpdate);
                                })),
                                self.update_action_link_mouse_state.clone(),
                            )
                            .soft_wrap(false)
                            .build()
                            .finish(),
                    )
                    .with_padding_left(8.)
                    .finish(),
                );
            }
        }

        // 安装提示:仅在 UpdateReady/UpdatedPendingRestart 状态(Install 操作)显示,
        // 让用户在点击前预知接下来会看到什么(打开 dmg / 启动安装向导 / 重启 AppImage)。
        if matches!(
            autoupdate::get_update_state(app),
            AutoupdateStage::UpdateReady { .. } | AutoupdateStage::UpdatedPendingRestart { .. }
        ) {
            // t! 是宏,必须传 literal,不能用变量。按 cfg 分支挑选具体 key。
            #[cfg(target_os = "macos")]
            let hint = crate::t!("settings-about-update-install-hint-macos");
            #[cfg(windows)]
            let hint = crate::t!("settings-about-update-install-hint-windows");
            #[cfg(all(not(target_os = "macos"), not(windows)))]
            let hint = crate::t!("settings-about-update-install-hint-linux");

            return Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(row.finish())
                .with_child(
                    ui_builder
                        .span(hint)
                        .with_soft_wrap()
                        .build()
                        .with_margin_top(4.)
                        .finish(),
                )
                .finish();
        }

        row.finish()
    }
}

/// 把字节数格式化为 "X.X MB" / "X KB",用于下载进度文案。
fn format_bytes(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/// 把 DownloadProgress 渲染成 "1.2 MB / 3.4 MB (35%)";total 未知时只显示已下载。
fn format_download_progress(p: &autoupdate::DownloadProgress) -> String {
    let downloaded = format_bytes(p.downloaded);
    match p.total {
        Some(total) if total > 0 => {
            let pct = ((p.downloaded as f64 / total as f64) * 100.0).clamp(0.0, 100.0);
            format!("{} / {} ({:.0}%)", downloaded, format_bytes(total), pct)
        }
        _ => downloaded,
    }
}

/// 更新状态区域要展示的操作:无 / 检查更新 / 打开 GitHub Release / 立即安装。
enum UpdateAction {
    None,
    Check,
    OpenReleasePage(String),
    Install,
}

impl SettingsPageMeta for AboutPageView {
    fn section() -> SettingsSection {
        SettingsSection::About
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

impl From<ViewHandle<AboutPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<AboutPageView>) -> Self {
        SettingsPageViewHandle::About(view_handle)
    }
}
