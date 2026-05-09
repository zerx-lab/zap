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
    OpenReleasePage(String),
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

        // 始终用纯图标 logo,品牌名以独立文本 "OpenWarp" 呈现,不再依赖带 "warp" 字样的 svg
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
                    .span("OpenWarp")
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
    /// 渲染"更新状态"行:状态文字 + 操作链接(检查更新 / 打开 GitHub Release)。
    fn render_update_status(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        let ui_builder = appearance.ui_builder();

        // 当前 stage 决定文案与操作:
        // - NoUpdateAvailable / 未知错误:已是最新 + "检查更新"
        // - CheckingForUpdate / DownloadingUpdate:正在检查 / 下载...(无操作)
        // - UpdateReady{version} / UpdatedPendingRestart{version} / UnableTo*{version}:
        //     发现新版本 + "前往 GitHub 下载"超链接
        let stage = autoupdate::get_update_state(app);

        let (status_text, action) = match &stage {
            AutoupdateStage::CheckingForUpdate => (
                crate::t!("settings-about-update-checking"),
                UpdateAction::None,
            ),
            AutoupdateStage::DownloadingUpdate => (
                // 在 OSS 模式下不会进入此状态(我们不下载),但官方 channel 仍可能。
                crate::t!("settings-about-update-checking"),
                UpdateAction::None,
            ),
            AutoupdateStage::NoUpdateAvailable => (
                crate::t!("settings-about-update-up-to-date"),
                UpdateAction::Check,
            ),
            stage if stage.available_new_version().is_some() => {
                let new_version = stage.available_new_version().unwrap();
                let text = crate::t!(
                    "settings-about-update-available",
                    version = new_version.version.as_str()
                );
                // 优先用缓存中的 release.html_url(由 fetch_latest_release 填充);
                // 兜底:若缓存为空(理论上不会,因为已经走到 UpdateReady),仓库主页 releases。
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
        }

        row.finish()
    }
}

/// 更新状态区域要展示的操作:无 / 检查更新 / 打开 GitHub Release。
enum UpdateAction {
    None,
    Check,
    OpenReleasePage(String),
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
