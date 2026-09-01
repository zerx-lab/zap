//! Read-only image viewer pane contents. Renders a local image fit-to-pane via the warpui
//! `Image` element. Mirrors the markdown `FileNotebookView`, but without the rich-text editor,
//! workflow, or link machinery — an image emits nothing and only needs to be displayed.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use warpui::{
    accessibility::{AccessibilityContent, WarpA11yRole},
    assets::asset_cache::{AssetCache, AssetSource},
    elements::{Align, CacheOption, DispatchEventResult, Empty, EventHandler, Image, Text},
    image_cache::ImageType,
    AppContext, Element, Entity, ModelHandle, SingletonEntity, TypedActionView, View, ViewContext,
};

use crate::{
    appearance::Appearance,
    code::buffer_location::RemotePath,
    pane_group::{
        focus_state::PaneFocusHandle, pane::view, BackingView, PaneConfiguration, PaneEvent,
    },
    terminal::model::session::Session,
};

/// View for a read-only image backed by a file.
pub struct ImageViewerView {
    /// The path of the open image, cached for the title and snapshot/restore.
    /// Only set for local images; remote images keep their name in `remote_name`.
    path: Option<PathBuf>,
    /// Title for a remote image (its filename). `None` for local images, which
    /// derive their title from `path`.
    remote_name: Option<String>,
    /// True while a remote image's bytes are being fetched, so `render` shows a
    /// loading indicator before any `source` exists.
    loading: bool,
    /// The asset source to render. `None` until an image has been opened.
    source: Option<AssetSource>,
    pane_configuration: ModelHandle<PaneConfiguration>,
    focus_handle: Option<PaneFocusHandle>,
}

#[derive(Debug, Clone)]
pub enum ImageViewerEvent {
    /// The image was opened; used to persist the pane for session restoration.
    Opened,
    Pane(PaneEvent),
}

impl From<PaneEvent> for ImageViewerEvent {
    fn from(event: PaneEvent) -> Self {
        ImageViewerEvent::Pane(event)
    }
}

#[derive(Debug, Clone)]
pub enum ImageViewerAction {
    Focus,
    Close,
}

impl ImageViewerView {
    /// Create a new image viewer view, with no open image.
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let pane_configuration = ctx.add_model(|_ctx| PaneConfiguration::new(""));
        Self {
            path: None,
            remote_name: None,
            loading: false,
            source: None,
            pane_configuration,
            focus_handle: None,
        }
    }

    /// Open a local image file, rendering it from disk via `AssetSource::LocalFile`.
    pub fn open_local(
        &mut self,
        path: impl Into<PathBuf>,
        _session: Option<Arc<Session>>,
        ctx: &mut ViewContext<Self>,
    ) {
        let local_path = path.into();
        self.pane_configuration.update(ctx, |pane_config, ctx| {
            pane_config.set_title(Self::title_for(&local_path), ctx);
        });
        self.source = Some(AssetSource::LocalFile {
            path: local_path.to_string_lossy().into_owned(),
        });
        self.path = Some(local_path);

        ctx.notify();
        // Persist the open image so the tab restores after restart.
        ctx.emit(ImageViewerEvent::Opened);
    }

    /// Set the title and loading state for a remote image whose bytes are still
    /// being fetched, so the pane shows its filename and a spinner up front.
    #[cfg_attr(not(feature = "local_tty"), allow(dead_code))]
    pub fn set_loading_remote(&mut self, remote_path: &RemotePath, ctx: &mut ViewContext<Self>) {
        let name = remote_path.file_name().to_string();
        self.pane_configuration.update(ctx, |pane_config, ctx| {
            pane_config.set_title(name.clone(), ctx);
        });
        self.remote_name = Some(name);
        self.loading = true;
        ctx.notify();
    }

    /// Open a remote image from already-fetched bytes, rendering via
    /// `AssetSource::Raw` (the same path the terminal uses for inline images).
    #[cfg_attr(not(feature = "local_tty"), allow(dead_code))]
    pub fn open_remote(
        &mut self,
        remote_path: &RemotePath,
        bytes: &[u8],
        ctx: &mut ViewContext<Self>,
    ) {
        let name = remote_path.file_name().to_string();
        self.pane_configuration.update(ctx, |pane_config, ctx| {
            pane_config.set_title(name.clone(), ctx);
        });

        // 用 host_id:path 作为稳定 asset id —— 同一远端图片复用缓存,不同主机/路径不碰撞。
        let asset_id = format!("{}:{}", remote_path.host_id, remote_path.path.as_str());
        AssetCache::handle(ctx).update(ctx, |cache, ctx| {
            cache.insert_raw_asset_bytes::<ImageType>(asset_id.clone(), bytes, ctx);
        });

        self.remote_name = Some(name);
        self.loading = false;
        self.source = Some(AssetSource::Raw { id: asset_id });
        ctx.notify();
        ctx.emit(ImageViewerEvent::Opened);
    }

    /// The path to the currently-open image, if it is local.
    pub fn local_path(&self) -> Option<PathBuf> {
        self.path.clone()
    }

    pub fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }

    pub fn focus(&self, ctx: &mut ViewContext<Self>) {
        if let Some(a11y_content) = self.accessibility_contents(ctx) {
            ctx.emit_a11y_content(a11y_content);
        }
        ctx.focus_self();
    }

    fn title(&self) -> String {
        self.path
            .as_deref()
            .map(Self::title_for)
            .or_else(|| self.remote_name.clone())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    fn title_for(path: &Path) -> String {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string())
    }
}

impl Entity for ImageViewerView {
    type Event = ImageViewerEvent;
}

impl View for ImageViewerView {
    fn ui_name() -> &'static str {
        "ImageViewerView"
    }

    fn accessibility_contents(&self, _ctx: &AppContext) -> Option<AccessibilityContent> {
        Some(AccessibilityContent::new_without_help(
            format!("{} image", self.title()),
            WarpA11yRole::TextRole,
        ))
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);

        // A loading indicator, shown while remote bytes are fetched (no `source` yet)
        // and as the `Image` element's `before_load` placeholder while bytes decode.
        let loading_element = || {
            Align::new(
                Text::new(
                    crate::t!("notebook-file-loading", name = self.title()),
                    appearance.ui_font_family(),
                    appearance.ui_font_size(),
                )
                .with_color(appearance.theme().foreground().into_solid())
                .finish(),
            )
            .finish()
        };

        let body: Box<dyn Element> = match &self.source {
            Some(source) => {
                // Model the Image-element usage on `ui_components/src/lightbox.rs`: contain to
                // fit-to-pane (preserving aspect ratio), with a loading element shown until the
                // bytes decode. The pane wraps us in a `Shrinkable`, so `contain` fills the pane.
                Image::new(source.clone(), CacheOption::Original)
                    .contain()
                    .before_load(loading_element())
                    .finish()
            }
            // 远端图片在抓取字节期间还没有 source —— 先显示 loading 占位。
            None if self.loading => loading_element(),
            None => Empty::new().finish(),
        };

        EventHandler::new(Align::new(body).finish())
            .on_left_mouse_down(|ctx, _, _| {
                ctx.dispatch_typed_action(ImageViewerAction::Focus);
                DispatchEventResult::StopPropagation
            })
            .finish()
    }
}

impl TypedActionView for ImageViewerView {
    type Action = ImageViewerAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            ImageViewerAction::Focus => ctx.focus_self(),
            ImageViewerAction::Close => ctx.emit(ImageViewerEvent::Pane(PaneEvent::Close)),
        }
    }
}

impl BackingView for ImageViewerView {
    type PaneHeaderOverflowMenuAction = ImageViewerAction;
    type CustomAction = ();
    type AssociatedData = ();

    fn handle_pane_header_overflow_menu_action(
        &mut self,
        action: &Self::PaneHeaderOverflowMenuAction,
        ctx: &mut ViewContext<Self>,
    ) {
        self.handle_action(action, ctx);
    }

    fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(ImageViewerEvent::Pane(PaneEvent::Close));
    }

    fn focus_contents(&mut self, ctx: &mut ViewContext<Self>) {
        self.focus(ctx);
    }

    fn render_header_content(
        &self,
        _ctx: &view::HeaderRenderContext<'_>,
        _app: &AppContext,
    ) -> view::HeaderContent {
        view::HeaderContent::simple(self.title())
    }

    fn set_focus_handle(&mut self, focus_handle: PaneFocusHandle, _ctx: &mut ViewContext<Self>) {
        self.focus_handle = Some(focus_handle);
    }
}
