use warpui::{
    elements::{
        ChildView, ConstrainedBox, Container, CrossAxisAlignment, Element, Flex, MainAxisAlignment,
        MainAxisSize, ParentElement, Text,
    },
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use crate::{
    appearance::Appearance,
    project_organization::domain::RepositoryId,
    view_components::action_button::{ActionButton, ButtonSize, DangerPrimaryTheme, NakedTheme},
};

#[derive(Clone, Debug)]
pub enum RemoveRepositoryDialogEvent {
    Close,
    Confirm { repository_id: RepositoryId },
}

#[derive(Clone, Debug)]
pub enum RemoveRepositoryDialogAction {
    Close,
    Confirm,
}

/// 删除 repository 的确认界面。
///
/// 只移除 Zap 中的组织记录。若仍有 workspace，确认后一并移除这些记录，
/// 不删除磁盘上的仓库或 worktree。
pub struct RemoveRepositoryDialog {
    target: Option<RepositoryId>,
    display_name: String,
    workspace_count: usize,
    cancel_button: ViewHandle<ActionButton>,
    confirm_button: ViewHandle<ActionButton>,
}

pub fn removal_details(display_name: &str, workspace_count: usize) -> String {
    if workspace_count == 0 {
        format!(
            "Remove `{display_name}` from Zap. The local repository on disk will not be deleted."
        )
    } else {
        format!(
            "Remove `{display_name}` from Zap. Associated workspaces will also be removed from Zap. The local repository and worktrees on disk will not be deleted."
        )
    }
}

impl RemoveRepositoryDialog {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let cancel_button = ctx.add_view(|_| {
            ActionButton::new("Cancel", NakedTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| ctx.dispatch_typed_action(RemoveRepositoryDialogAction::Close))
        });
        let confirm_button = ctx.add_view(|_| {
            ActionButton::new("Remove repository", DangerPrimaryTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| ctx.dispatch_typed_action(RemoveRepositoryDialogAction::Confirm))
        });
        Self {
            target: None,
            display_name: String::new(),
            workspace_count: 0,
            cancel_button,
            confirm_button,
        }
    }

    pub fn configure(
        &mut self,
        repository_id: RepositoryId,
        display_name: String,
        workspace_count: usize,
        ctx: &mut ViewContext<Self>,
    ) {
        self.target = Some(repository_id);
        self.display_name = display_name;
        self.workspace_count = workspace_count;
        ctx.notify();
    }

    pub fn reset(&mut self, ctx: &mut ViewContext<Self>) {
        self.target = None;
        self.workspace_count = 0;
        ctx.notify();
    }
}

impl Entity for RemoveRepositoryDialog {
    type Event = RemoveRepositoryDialogEvent;
}

impl TypedActionView for RemoveRepositoryDialog {
    type Action = RemoveRepositoryDialogAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            RemoveRepositoryDialogAction::Close => ctx.emit(RemoveRepositoryDialogEvent::Close),
            RemoveRepositoryDialogAction::Confirm => {
                let Some(repository_id) = self.target else {
                    return;
                };
                ctx.emit(RemoveRepositoryDialogEvent::Confirm { repository_id });
            }
        }
    }
}

impl View for RemoveRepositoryDialog {
    fn ui_name() -> &'static str {
        "RemoveRepositoryDialog"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let details = removal_details(&self.display_name, self.workspace_count);

        let content = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(12.)
            .with_child(
                Text::new_inline(
                    "Remove repository",
                    appearance.ui_font_family(),
                    appearance.ui_font_heading_3(),
                )
                .with_color(theme.main_text_color(theme.background()).into())
                .finish(),
            )
            .with_child(
                ConstrainedBox::new(
                    Text::new(
                        details,
                        appearance.ui_font_family(),
                        appearance.ui_font_body(),
                    )
                    .with_color(theme.sub_text_color(theme.background()).into())
                    .finish(),
                )
                .with_max_width(440.)
                .finish(),
            )
            .finish();

        let footer = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::End)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.)
            .with_child(ChildView::new(&self.cancel_button).finish())
            .with_child(ChildView::new(&self.confirm_button).finish())
            .finish();
        Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(Container::new(content).with_uniform_padding(20.).finish())
            .with_child(Container::new(footer).with_uniform_padding(12.).finish())
            .finish()
    }
}

#[cfg(test)]
#[path = "remove_repository_dialog_tests.rs"]
mod tests;
