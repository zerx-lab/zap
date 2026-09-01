use warpui::{
    elements::{
        ChildView, Container, CrossAxisAlignment, Element, Flex, MainAxisAlignment, MainAxisSize,
        MouseStateHandle, ParentElement, Text,
    },
    platform::Cursor,
    ui_components::components::UiComponent,
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use crate::{
    appearance::Appearance,
    project_organization::domain::RepositoryWorkspaceId,
    view_components::action_button::{ActionButton, ButtonSize, DangerPrimaryTheme, NakedTheme},
};

#[derive(Clone, Debug)]
pub enum DeleteWorkspaceDialogEvent {
    Close,
    Confirm {
        workspace_id: RepositoryWorkspaceId,
        delete_branch: bool,
        force_branch: bool,
    },
}

#[derive(Clone, Debug)]
pub enum DeleteWorkspaceDialogAction {
    Close,
    ToggleDeleteBranch,
    Confirm,
}

#[derive(Clone, Copy)]
struct DeleteWorkspaceTarget {
    workspace_id: RepositoryWorkspaceId,
}

/// 删除 repository workspace 的确认界面。
///
/// 预检和 Git 操作均由窗口根执行。该视图仅保存用户的分支删除选择，并在未合并
/// 分支场景要求再次明确确认。
pub struct DeleteWorkspaceDialog {
    target: Option<DeleteWorkspaceTarget>,
    display_name: String,
    branch: String,
    delete_branch: bool,
    force_branch: bool,
    checkbox_mouse_state: MouseStateHandle,
    cancel_button: ViewHandle<ActionButton>,
    confirm_button: ViewHandle<ActionButton>,
}

impl DeleteWorkspaceDialog {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let cancel_button = ctx.add_view(|_| {
            ActionButton::new("Cancel", NakedTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| ctx.dispatch_typed_action(DeleteWorkspaceDialogAction::Close))
        });
        let confirm_button = ctx.add_view(|_| {
            ActionButton::new("Remove workspace", DangerPrimaryTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| ctx.dispatch_typed_action(DeleteWorkspaceDialogAction::Confirm))
        });
        Self {
            target: None,
            display_name: String::new(),
            branch: String::new(),
            delete_branch: true,
            force_branch: false,
            checkbox_mouse_state: Default::default(),
            cancel_button,
            confirm_button,
        }
    }

    pub fn configure(
        &mut self,
        workspace_id: RepositoryWorkspaceId,
        display_name: String,
        branch: String,
        ctx: &mut ViewContext<Self>,
    ) {
        self.target = Some(DeleteWorkspaceTarget { workspace_id });
        self.display_name = display_name;
        self.branch = branch;
        self.delete_branch = true;
        self.force_branch = false;
        ctx.notify();
    }

    pub fn require_force_confirmation(&mut self, ctx: &mut ViewContext<Self>) {
        self.force_branch = true;
        ctx.notify();
    }

    pub fn reset(&mut self, ctx: &mut ViewContext<Self>) {
        self.target = None;
        self.force_branch = false;
        ctx.notify();
    }
}

impl Entity for DeleteWorkspaceDialog {
    type Event = DeleteWorkspaceDialogEvent;
}

impl TypedActionView for DeleteWorkspaceDialog {
    type Action = DeleteWorkspaceDialogAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            DeleteWorkspaceDialogAction::Close => ctx.emit(DeleteWorkspaceDialogEvent::Close),
            DeleteWorkspaceDialogAction::ToggleDeleteBranch => {
                self.delete_branch = !self.delete_branch;
                ctx.notify();
            }
            DeleteWorkspaceDialogAction::Confirm => {
                let Some(target) = self.target else {
                    return;
                };
                ctx.emit(DeleteWorkspaceDialogEvent::Confirm {
                    workspace_id: target.workspace_id,
                    delete_branch: self.delete_branch,
                    force_branch: self.force_branch,
                });
            }
        }
    }
}

impl View for DeleteWorkspaceDialog {
    fn ui_name() -> &'static str {
        "DeleteWorkspaceDialog"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let title = if self.force_branch {
            "Force delete unmerged branch"
        } else {
            "Remove workspace"
        };
        let details = if self.force_branch {
            format!(
                "`{}` is not merged. Removing it will permanently delete local branch `{}`.",
                self.display_name, self.branch
            )
        } else {
            format!(
                "Remove worktree for `{}`. Git will reject the operation when the worktree has changes.",
                self.display_name
            )
        };

        let mut content = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(12.)
            .with_child(
                Text::new_inline(
                    title,
                    appearance.ui_font_family(),
                    appearance.ui_font_heading_3(),
                )
                .with_color(theme.main_text_color(theme.background()).into())
                .finish(),
            )
            .with_child(
                Text::new_inline(
                    details,
                    appearance.ui_font_family(),
                    appearance.ui_font_body(),
                )
                .with_color(theme.sub_text_color(theme.background()).into())
                .finish(),
            );

        if !self.force_branch {
            let checkbox = appearance
                .ui_builder()
                .checkbox(self.checkbox_mouse_state.clone(), Some(12.))
                .check(self.delete_branch)
                .build()
                .with_cursor(Cursor::PointingHand)
                .on_click(|ctx, _, _| {
                    ctx.dispatch_typed_action(DeleteWorkspaceDialogAction::ToggleDeleteBranch);
                })
                .finish();
            content.add_child(
                Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(checkbox)
                    .with_child(
                        Container::new(
                            Text::new_inline(
                                "Also delete local branch",
                                appearance.ui_font_family(),
                                appearance.ui_font_body(),
                            )
                            .with_color(theme.main_text_color(theme.background()).into())
                            .finish(),
                        )
                        .with_margin_left(8.)
                        .finish(),
                    )
                    .finish(),
            );
        }

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
            .with_child(
                Container::new(content.finish())
                    .with_uniform_padding(20.)
                    .finish(),
            )
            .with_child(Container::new(footer).with_uniform_padding(12.).finish())
            .finish()
    }
}
