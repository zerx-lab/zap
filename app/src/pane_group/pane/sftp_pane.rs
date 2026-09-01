//! SFTP 文件浏览器 pane(中央 pane,通过 SSH 管理器树打开)。
//!
//! 仿 `ssh_server_pane.rs` 的极简结构。pane 不持久化(
//! `LeafContents::Sftp { .. }` 在 `is_persisted()` 返回 false),
//! 业务数据走 SFTP 连接操作。
//! author: logic
//! date: 2026-05-26

use warpui::{AppContext, ModelHandle, View, ViewContext, ViewHandle};

use crate::app_state::LeafContents;
use crate::pane_group::{BackingView, PaneConfiguration, PaneContent, PaneGroup, PaneView};
use crate::sftp_manager::browser::SftpBrowserView;

use super::{DetachType, PaneId, ShareableLink, ShareableLinkError};

/// SFTP 文件浏览器 pane 内容
pub struct SftpPane {
    view: ViewHandle<PaneView<SftpBrowserView>>,
    pane_configuration: ModelHandle<PaneConfiguration>,
    /// 业务节点 id(不是 pane view id),用于 snapshot 序列化。
    node_id: String,
}

impl SftpPane {
    /// 创建新的 SFTP 浏览器 pane
    pub fn new<V: View>(node_id: String, ctx: &mut ViewContext<V>) -> Self {
        let id_for_view = node_id.clone();
        let browser_view =
            ctx.add_typed_action_view(move |ctx| SftpBrowserView::new(id_for_view.clone(), ctx));
        let pane_configuration = browser_view.as_ref(ctx).pane_configuration();
        let pane_view = ctx.add_typed_action_view(|ctx| {
            let pane_id = PaneId::from_sftp_pane_ctx(ctx);
            PaneView::new(pane_id, browser_view, (), pane_configuration.clone(), ctx)
        });
        Self {
            view: pane_view,
            pane_configuration,
            node_id,
        }
    }
}

impl PaneContent for SftpPane {
    fn id(&self) -> PaneId {
        PaneId::from_sftp_pane_view(&self.view)
    }

    fn attach(
        &self,
        _group: &PaneGroup,
        focus_handle: crate::pane_group::focus_state::PaneFocusHandle,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        self.view
            .update(ctx, |view, ctx| view.set_focus_handle(focus_handle, ctx));
        let child = self.view.as_ref(ctx).child(ctx);

        let pane_id = self.id();
        ctx.subscribe_to_view(&child, move |pane_group, _, event, ctx| {
            pane_group.handle_pane_event(pane_id, event, ctx);
        });
    }

    fn detach(
        &self,
        _group: &PaneGroup,
        _detach_type: DetachType,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        let child = self.view.as_ref(ctx).child(ctx);
        ctx.unsubscribe_to_view(&child);
    }

    fn snapshot(&self, _ctx: &AppContext) -> LeafContents {
        LeafContents::Sftp {
            node_id: self.node_id.clone(),
        }
    }

    fn has_application_focus(&self, ctx: &mut ViewContext<PaneGroup>) -> bool {
        self.view.is_self_or_child_focused(ctx)
    }

    fn focus(&self, ctx: &mut ViewContext<PaneGroup>) {
        self.view
            .as_ref(ctx)
            .child(ctx)
            .update(ctx, BackingView::focus_contents)
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
