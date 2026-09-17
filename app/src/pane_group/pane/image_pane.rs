use std::{path::PathBuf, sync::Arc};

use warpui::{AppContext, ModelHandle, SingletonEntity, View, ViewContext, ViewHandle};

use crate::{
    app_state::LeafContents,
    notebooks::image::{ImageViewerEvent, ImageViewerView},
    terminal::model::session::Session,
    workspace::ActiveSession,
};

use super::{
    view::PaneView, DetachType, PaneConfiguration, PaneContent, PaneGroup, PaneId, ShareableLink,
    ShareableLinkError,
};

/// A read-only pane that displays a local image. Mirrors [`super::file_pane::FilePane`], but
/// backs onto [`ImageViewerView`] and drops the workflow/link plumbing — images emit nothing.
pub struct ImagePane {
    view: ViewHandle<PaneView<ImageViewerView>>,
    pane_configuration: ModelHandle<PaneConfiguration>,
}

impl ImagePane {
    fn from_view(image_view: ViewHandle<ImageViewerView>, ctx: &mut AppContext) -> Self {
        let pane_configuration = image_view.as_ref(ctx).pane_configuration();

        let view = ctx.add_typed_action_view(image_view.window_id(ctx), |ctx| {
            let pane_id = PaneId::from_image_pane_ctx(ctx);
            PaneView::new(pane_id, image_view, (), pane_configuration.clone(), ctx)
        });

        Self {
            view,
            pane_configuration,
        }
    }

    /// Create a new image pane for the given path and optional target session. Follows the same
    /// session-fallback behavior as [`super::file_pane::FilePane::new`]: a remote target session
    /// leaves the pane empty, while a missing session waits for the next local one.
    pub fn new<V: View>(
        path: Option<PathBuf>,
        target_session: Option<Arc<Session>>,
        ctx: &mut ViewContext<V>,
    ) -> Self {
        let view = ctx.add_typed_action_view(move |ctx| {
            let mut view = ImageViewerView::new(ctx);

            if let Some(path) = path {
                if let Some(target_session) = target_session {
                    if target_session.is_local() {
                        view.open_local(path, Some(target_session), ctx);
                    }
                } else {
                    let session = ActiveSession::as_ref(ctx)
                        .session(ctx.window_id())
                        .filter(|session| session.is_local());
                    view.open_local(path, session, ctx);
                }
            }

            view
        });
        Self::from_view(view, ctx)
    }

    /// Create an image pane for a remote image. The pane starts in a loading state
    /// (showing a spinner with the remote filename); the caller fetches the bytes
    /// asynchronously and fills them in via [`ImageViewerView::open_remote`].
    #[cfg_attr(not(feature = "local_tty"), allow(dead_code))]
    pub fn new_remote<V: View>(
        remote_path: crate::code::buffer_location::RemotePath,
        ctx: &mut ViewContext<V>,
    ) -> Self {
        let view = ctx.add_typed_action_view(move |ctx| {
            let mut view = ImageViewerView::new(ctx);
            view.set_loading_remote(&remote_path, ctx);
            view
        });
        Self::from_view(view, ctx)
    }

    pub fn image_view(&self, ctx: &AppContext) -> ViewHandle<ImageViewerView> {
        self.view.as_ref(ctx).child(ctx)
    }
}

impl PaneContent for ImagePane {
    fn id(&self) -> PaneId {
        PaneId::from_image_pane_view(&self.view)
    }

    fn attach(
        &self,
        _group: &PaneGroup,
        focus_handle: crate::pane_group::focus_state::PaneFocusHandle,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        self.view
            .update(ctx, |view, ctx| view.set_focus_handle(focus_handle, ctx));

        let pane_id = self.id();

        ctx.subscribe_to_view(
            &self.image_view(ctx),
            move |pane_group, _, event, ctx| match event {
                ImageViewerEvent::Opened => ctx.emit(crate::pane_group::Event::AppStateChanged),
                ImageViewerEvent::Pane(pane_event) => {
                    pane_group.handle_pane_event(pane_id, pane_event, ctx)
                }
            },
        );

        ctx.subscribe_to_view(&self.view, move |group, _, event, ctx| {
            group.handle_pane_view_event(pane_id, event, ctx);
        });
    }

    fn detach(
        &self,
        _group: &PaneGroup,
        _detach_type: DetachType,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        let image_view = self.image_view(ctx);
        ctx.unsubscribe_to_view(&image_view);
        ctx.unsubscribe_to_view(&self.view);
    }

    fn snapshot(&self, app: &AppContext) -> LeafContents {
        let path = self.image_view(app).as_ref(app).local_path();
        LeafContents::Image { path }
    }

    fn has_application_focus(&self, ctx: &mut ViewContext<PaneGroup>) -> bool {
        self.view.is_self_or_child_focused(ctx)
    }

    fn focus(&self, ctx: &mut ViewContext<PaneGroup>) {
        self.image_view(ctx)
            .update(ctx, |view, ctx| view.focus(ctx));
    }

    fn shareable_link(
        &self,
        _ctx: &mut ViewContext<PaneGroup>,
    ) -> Result<ShareableLink, ShareableLinkError> {
        Ok(ShareableLink::Base)
    }

    fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }

    fn is_pane_being_dragged(&self, ctx: &AppContext) -> bool {
        self.view.as_ref(ctx).is_being_dragged()
    }
}
