use std::{cell::RefCell, rc::Rc};

use warp_core::ui::appearance::Appearance;
use warpui::{
    App, Element, Entity, TypedActionView, View, ViewContext, ViewHandle,
    elements::{ChildView, ConstrainedBox},
    platform::WindowStyle,
};

use crate::project_organization::domain::RepositoryId;

use super::{
    RemoveRepositoryDialog, RemoveRepositoryDialogAction, RemoveRepositoryDialogEvent,
    removal_details,
};

struct RemoveRepositoryDialogTestHost {
    dialog: ViewHandle<RemoveRepositoryDialog>,
    events: Rc<RefCell<Vec<RemoveRepositoryDialogEvent>>>,
}

impl RemoveRepositoryDialogTestHost {
    fn new(ctx: &mut ViewContext<Self>) -> Self {
        let events = Rc::new(RefCell::new(Vec::new()));
        let captured_events = events.clone();
        let dialog = ctx.add_typed_action_view(RemoveRepositoryDialog::new);
        ctx.subscribe_to_view(&dialog, move |_, _, event, _| {
            captured_events.borrow_mut().push(event.clone());
        });
        Self { dialog, events }
    }
}

impl Entity for RemoveRepositoryDialogTestHost {
    type Event = ();
}

impl View for RemoveRepositoryDialogTestHost {
    fn ui_name() -> &'static str {
        "RemoveRepositoryDialogTestHost"
    }

    fn render(&self, _app: &warpui::AppContext) -> Box<dyn Element> {
        ConstrainedBox::new(ChildView::new(&self.dialog).finish())
            .with_width(480.)
            .finish()
    }
}

impl TypedActionView for RemoveRepositoryDialogTestHost {
    type Action = ();
}

#[test]
fn empty_repository_details_do_not_mention_workspaces() {
    assert_eq!(
        removal_details("zap", 0),
        "Remove `zap` from Zap. The local repository on disk will not be deleted."
    );
}

#[test]
fn repository_with_workspaces_details_keep_disk_paths() {
    assert_eq!(
        removal_details("zap", 2),
        "Remove `zap` from Zap. Associated workspaces will also be removed from Zap. The local repository and worktrees on disk will not be deleted."
    );
}

#[test]
fn confirm_emits_the_configured_repository_id() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| Appearance::mock());
        let (_window_id, host) = app.add_window(
            WindowStyle::NotStealFocus,
            RemoveRepositoryDialogTestHost::new,
        );
        let repository_id = RepositoryId(uuid::Uuid::from_u128(1));
        let events = host.read(&app, |host, _| host.events.clone());

        host.update(&mut app, |host, ctx| {
            host.dialog.update(ctx, |dialog, ctx| {
                dialog.configure(repository_id, "zap".to_string(), 1, ctx);
                dialog.handle_action(&RemoveRepositoryDialogAction::Confirm, ctx);
            });
        });

        assert!(matches!(
            events.borrow().as_slice(),
            [RemoveRepositoryDialogEvent::Confirm {
                repository_id: event_repository_id
            }] if *event_repository_id == repository_id
        ));
    });
}

#[test]
fn close_emits_close_without_a_repository_id() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| Appearance::mock());
        let (_window_id, host) = app.add_window(
            WindowStyle::NotStealFocus,
            RemoveRepositoryDialogTestHost::new,
        );
        let events = host.read(&app, |host, _| host.events.clone());

        host.update(&mut app, |host, ctx| {
            host.dialog.update(ctx, |dialog, ctx| {
                dialog.handle_action(&RemoveRepositoryDialogAction::Close, ctx);
            });
        });

        assert!(matches!(
            events.borrow().as_slice(),
            [RemoveRepositoryDialogEvent::Close]
        ));
    });
}
