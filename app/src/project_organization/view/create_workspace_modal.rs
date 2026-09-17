use std::{collections::HashMap, path::PathBuf};

use warpui::{
    elements::{
        ChildView, Clipped, ConstrainedBox, Container, CrossAxisAlignment, Element, Flex,
        MainAxisAlignment, MainAxisSize, ParentElement, Text,
    },
    AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use crate::project_organization::{
    domain::{RepositoryId, RepositoryWorkspaceId},
    git::{
        existing_worktree_options, is_primary_worktree_path, workspace_dir_name, BranchRef,
        ExistingWorktreeOption, WorktreeInfo,
    },
};
use crate::{
    appearance::Appearance,
    editor::{EditorView, Event as EditorEvent, SingleLineEditorOptions},
    view_components::action_button::{
        ActionButton, ButtonSize, NakedTheme, PrimaryTheme, SecondaryTheme,
    },
    view_components::{DropdownItem, FilterableDropdown},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CreateWorkspaceMode {
    #[default]
    RemoteBranch,
    ExistingLocalBranch,
    ExistingWorktree,
}

fn submit_is_disabled(
    mode: CreateWorkspaceMode,
    has_remote_fetch_error: bool,
    has_existing_worktree_fetch_error: bool,
    has_existing_worktree_selection: bool,
) -> bool {
    match mode {
        CreateWorkspaceMode::RemoteBranch => has_remote_fetch_error,
        CreateWorkspaceMode::ExistingLocalBranch => false,
        CreateWorkspaceMode::ExistingWorktree => {
            has_existing_worktree_fetch_error || !has_existing_worktree_selection
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CreateWorkspaceForm {
    mode: CreateWorkspaceMode,
    remote_ref: Option<String>,
    local_branch: Option<String>,
    existing_worktree_branch: Option<String>,
    new_branch: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateWorkspaceRequest {
    pub repository_id: RepositoryId,
    pub workspace_id: RepositoryWorkspaceId,
    pub display_name: String,
    pub worktree_path: PathBuf,
    pub source: CreateWorkspaceSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CreateWorkspaceSource {
    RemoteBranch {
        remote_ref: String,
        new_branch: String,
    },
    ExistingLocalBranch {
        local_branch: String,
    },
    ExistingWorktree {
        local_branch: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteBranchOption {
    full_ref: String,
    remote: String,
    branch_name: String,
    display_label: String,
}

impl RemoteBranchOption {
    fn new(
        full_ref: impl Into<String>,
        remote: impl Into<String>,
        branch_name: impl Into<String>,
        display_label: impl Into<String>,
    ) -> Self {
        Self {
            full_ref: full_ref.into(),
            remote: remote.into(),
            branch_name: branch_name.into(),
            display_label: display_label.into(),
        }
    }
}

pub fn branch_ref_options(
    refs: impl IntoIterator<Item = BranchRef>,
) -> (Vec<RemoteBranchOption>, Vec<String>) {
    let refs = refs.into_iter().collect::<Vec<_>>();
    let mut remote_branch_counts = HashMap::<String, usize>::new();
    for branch_ref in &refs {
        if let BranchRef::Remote { name, .. } = branch_ref {
            *remote_branch_counts.entry(name.clone()).or_default() += 1;
        }
    }

    let mut remote_options = Vec::new();
    let mut local_branches = Vec::new();
    for branch_ref in refs {
        match branch_ref {
            BranchRef::Remote {
                remote,
                name,
                full_ref,
            } => {
                let display_label = if remote_branch_counts[&name] > 1 {
                    format!("{remote}/{name}")
                } else {
                    name.clone()
                };
                remote_options.push(RemoteBranchOption::new(
                    full_ref,
                    remote,
                    name,
                    display_label,
                ));
            }
            BranchRef::Local { name, .. } => local_branches.push(name),
        }
    }
    remote_options.sort_unstable_by(|left, right| {
        left.display_label
            .cmp(&right.display_label)
            .then_with(|| left.full_ref.cmp(&right.full_ref))
    });
    local_branches.sort_unstable();
    (remote_options, local_branches)
}

pub fn default_worktree_path(home: PathBuf, repository_name: &str, branch_name: &str) -> PathBuf {
    home.join(".warp")
        .join("worktrees")
        .join(workspace_dir_name(repository_name, ""))
        .join(workspace_dir_name(branch_name, ""))
}

fn existing_worktree_display_label(worktree: &ExistingWorktreeOption) -> String {
    if worktree.is_primary {
        format!("{} (local)", worktree.branch_name)
    } else {
        worktree.branch_name.clone()
    }
}

fn existing_worktree_default_name(worktree: &ExistingWorktreeOption) -> &str {
    if worktree.is_primary {
        "local"
    } else {
        &worktree.branch_name
    }
}

fn primary_worktree_error(
    repository_root: &std::path::Path,
    worktrees: &[WorktreeInfo],
) -> Option<String> {
    worktrees
        .iter()
        .find(|worktree| is_primary_worktree_path(repository_root, &worktree.path))
        .filter(|worktree| {
            worktree.is_bare
                || worktree.is_detached
                || worktree
                    .branch
                    .as_deref()
                    .and_then(|branch| branch.strip_prefix("refs/heads/"))
                    .is_none_or(str::is_empty)
        })
        .map(|_| {
            "The repository root worktree is detached and cannot be used as the local workspace."
                .to_string()
        })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateWorkspaceDefaults {
    home: PathBuf,
    repository_name: String,
    new_branch: String,
    workspace_name: String,
    worktree_path: PathBuf,
}

impl CreateWorkspaceDefaults {
    pub fn new(home: PathBuf, repository_name: String) -> Self {
        Self {
            home,
            repository_name,
            new_branch: String::new(),
            workspace_name: String::new(),
            worktree_path: PathBuf::new(),
        }
    }

    pub fn apply_branch(&mut self, branch_name: &str) {
        self.new_branch = branch_name.to_string();
        self.workspace_name = branch_name.to_string();
        self.worktree_path =
            default_worktree_path(self.home.clone(), &self.repository_name, branch_name);
    }
}

impl CreateWorkspaceForm {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mode(&self) -> CreateWorkspaceMode {
        self.mode
    }

    pub fn remote_ref(&self) -> Option<&str> {
        self.remote_ref.as_deref()
    }

    pub fn new_branch(&self) -> &str {
        &self.new_branch
    }

    pub fn set_mode(&mut self, mode: CreateWorkspaceMode) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        match mode {
            CreateWorkspaceMode::RemoteBranch => {
                self.local_branch = None;
                self.existing_worktree_branch = None;
            }
            CreateWorkspaceMode::ExistingLocalBranch => {
                self.remote_ref = None;
                self.existing_worktree_branch = None;
                self.new_branch.clear();
            }
            CreateWorkspaceMode::ExistingWorktree => {
                self.remote_ref = None;
                self.local_branch = None;
                self.new_branch.clear();
            }
        }
    }

    pub fn set_remote_ref(&mut self, remote_ref: String) {
        self.remote_ref = Some(remote_ref);
    }

    pub fn set_local_branch(&mut self, local_branch: String) {
        self.local_branch = Some(local_branch);
    }

    pub fn set_existing_worktree_branch(&mut self, branch: String) {
        self.existing_worktree_branch = Some(branch);
    }

    pub fn set_new_branch(&mut self, new_branch: String) {
        self.new_branch = new_branch;
    }

    pub fn can_submit(&self) -> bool {
        match self.mode {
            CreateWorkspaceMode::RemoteBranch => {
                self.remote_ref
                    .as_deref()
                    .is_some_and(|remote_ref| remote_ref.starts_with("refs/remotes/"))
                    && !self.new_branch.trim().is_empty()
            }
            CreateWorkspaceMode::ExistingLocalBranch => self
                .local_branch
                .as_deref()
                .is_some_and(|branch| !branch.trim().is_empty() && !branch.starts_with("refs/")),
            CreateWorkspaceMode::ExistingWorktree => self
                .existing_worktree_branch
                .as_deref()
                .is_some_and(|branch| !branch.trim().is_empty() && !branch.starts_with("refs/")),
        }
    }

    pub fn build_request(
        &self,
        repository_id: RepositoryId,
        workspace_id: RepositoryWorkspaceId,
        display_name: String,
        worktree_path: PathBuf,
    ) -> Option<CreateWorkspaceRequest> {
        if !self.can_submit()
            || display_name.trim().is_empty()
            || worktree_path.as_os_str().is_empty()
        {
            return None;
        }

        let source = match self.mode {
            CreateWorkspaceMode::RemoteBranch => CreateWorkspaceSource::RemoteBranch {
                remote_ref: self.remote_ref.clone()?,
                new_branch: self.new_branch.trim().to_string(),
            },
            CreateWorkspaceMode::ExistingLocalBranch => {
                CreateWorkspaceSource::ExistingLocalBranch {
                    local_branch: self.local_branch.clone()?,
                }
            }
            CreateWorkspaceMode::ExistingWorktree => CreateWorkspaceSource::ExistingWorktree {
                local_branch: self.existing_worktree_branch.clone()?,
            },
        };
        Some(CreateWorkspaceRequest {
            repository_id,
            workspace_id,
            display_name,
            worktree_path,
            source,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CreateWorkspaceModalEvent {
    Close,
    RetryBranchRefs {
        repository_id: RepositoryId,
        workspace_id: RepositoryWorkspaceId,
    },
    RetryExistingWorktrees {
        repository_id: RepositoryId,
        workspace_id: RepositoryWorkspaceId,
    },
    Submit(CreateWorkspaceRequest),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CreateWorkspaceModalAction {
    Close,
    NoOp,
    SetMode(CreateWorkspaceMode),
    SelectRemoteBranch(RemoteBranchOption),
    SelectLocalBranch(String),
    SelectExistingWorktree(ExistingWorktreeOption),
    RetryBranchRefs,
    RetryExistingWorktrees,
    Submit,
}

#[derive(Clone, Copy)]
struct CreateWorkspaceTarget {
    repository_id: RepositoryId,
    workspace_id: RepositoryWorkspaceId,
}

impl CreateWorkspaceTarget {
    fn retry_branch_refs_event(self) -> CreateWorkspaceModalEvent {
        CreateWorkspaceModalEvent::RetryBranchRefs {
            repository_id: self.repository_id,
            workspace_id: self.workspace_id,
        }
    }

    fn retry_existing_worktrees_event(self) -> CreateWorkspaceModalEvent {
        CreateWorkspaceModalEvent::RetryExistingWorktrees {
            repository_id: self.repository_id,
            workspace_id: self.workspace_id,
        }
    }
}

/// 创建 repository workspace 的表单视图。
///
/// Git 操作和持久化不在此视图执行。视图仅生成经过基础校验的
/// [`CreateWorkspaceRequest`]，由窗口根协调 Git、SQLite 和首个终端页签。
pub struct CreateWorkspaceModal {
    target: Option<CreateWorkspaceTarget>,
    repository_root: Option<PathBuf>,
    form: CreateWorkspaceForm,
    defaults: Option<CreateWorkspaceDefaults>,
    remote_branch_picker: ViewHandle<FilterableDropdown<CreateWorkspaceModalAction>>,
    local_branch_picker: ViewHandle<FilterableDropdown<CreateWorkspaceModalAction>>,
    existing_worktree_picker: ViewHandle<FilterableDropdown<CreateWorkspaceModalAction>>,
    new_branch_editor: ViewHandle<EditorView>,
    display_name_editor: ViewHandle<EditorView>,
    worktree_path_editor: ViewHandle<EditorView>,
    remote_mode_button: ViewHandle<ActionButton>,
    local_mode_button: ViewHandle<ActionButton>,
    existing_worktree_mode_button: ViewHandle<ActionButton>,
    cancel_button: ViewHandle<ActionButton>,
    retry_remote_button: ViewHandle<ActionButton>,
    retry_existing_worktree_button: ViewHandle<ActionButton>,
    submit_button: ViewHandle<ActionButton>,
    validation_error: Option<String>,
    remote_fetch_error: Option<String>,
    existing_worktree_fetch_error: Option<String>,
    primary_worktree_error: Option<String>,
    remote_branch_options: Vec<RemoteBranchOption>,
    local_branches: Vec<String>,
    existing_worktree_options: Vec<ExistingWorktreeOption>,
    local_branch_fallback_loaded: bool,
    selected_remote_branch: Option<RemoteBranchOption>,
    selected_local_branch: Option<String>,
    selected_existing_worktree: Option<ExistingWorktreeOption>,
}

impl CreateWorkspaceModal {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let remote_branch_picker = ctx.add_typed_action_view(|ctx| {
            let mut picker = FilterableDropdown::new(ctx);
            picker.set_top_bar_max_width(480.);
            picker.set_menu_width(480., ctx);
            picker.set_disabled(ctx);
            picker
        });
        let local_branch_picker = ctx.add_typed_action_view(|ctx| {
            let mut picker = FilterableDropdown::new(ctx);
            picker.set_top_bar_max_width(480.);
            picker.set_menu_width(480., ctx);
            picker.set_disabled(ctx);
            picker
        });
        let existing_worktree_picker = ctx.add_typed_action_view(|ctx| {
            let mut picker = FilterableDropdown::new(ctx);
            picker.set_top_bar_max_width(480.);
            picker.set_menu_width(480., ctx);
            picker.set_disabled(ctx);
            picker
        });
        let new_branch_editor = Self::build_editor("New local branch", ctx);
        let display_name_editor = Self::build_editor("Workspace name", ctx);
        let worktree_path_editor = Self::build_editor("Worktree path", ctx);
        let remote_mode_button = ctx.add_view(|_| {
            ActionButton::new("From remote branch", SecondaryTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(CreateWorkspaceModalAction::SetMode(
                        CreateWorkspaceMode::RemoteBranch,
                    ));
                })
        });
        let local_mode_button = ctx.add_view(|_| {
            ActionButton::new("Use local branch", SecondaryTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(CreateWorkspaceModalAction::SetMode(
                        CreateWorkspaceMode::ExistingLocalBranch,
                    ));
                })
        });
        let existing_worktree_mode_button = ctx.add_view(|_| {
            ActionButton::new("Use existing worktree", SecondaryTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(CreateWorkspaceModalAction::SetMode(
                        CreateWorkspaceMode::ExistingWorktree,
                    ));
                })
        });
        let cancel_button = ctx.add_view(|_| {
            ActionButton::new("Cancel", NakedTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| ctx.dispatch_typed_action(CreateWorkspaceModalAction::Close))
        });
        let retry_remote_button = ctx.add_view(|_| {
            ActionButton::new("Retry", SecondaryTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(CreateWorkspaceModalAction::RetryBranchRefs)
                })
        });
        let retry_existing_worktree_button = ctx.add_view(|_| {
            ActionButton::new("Retry", SecondaryTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(CreateWorkspaceModalAction::RetryExistingWorktrees)
                })
        });
        let submit_button = ctx.add_view(|_| {
            ActionButton::new("Create workspace", PrimaryTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| ctx.dispatch_typed_action(CreateWorkspaceModalAction::Submit))
        });

        let mut modal = Self {
            target: None,
            repository_root: None,
            form: CreateWorkspaceForm::new(),
            defaults: None,
            remote_branch_picker,
            local_branch_picker,
            existing_worktree_picker,
            new_branch_editor,
            display_name_editor,
            worktree_path_editor,
            remote_mode_button,
            local_mode_button,
            existing_worktree_mode_button,
            cancel_button,
            retry_remote_button,
            retry_existing_worktree_button,
            submit_button,
            validation_error: None,
            remote_fetch_error: None,
            existing_worktree_fetch_error: None,
            primary_worktree_error: None,
            remote_branch_options: Vec::new(),
            local_branches: Vec::new(),
            existing_worktree_options: Vec::new(),
            local_branch_fallback_loaded: false,
            selected_remote_branch: None,
            selected_local_branch: None,
            selected_existing_worktree: None,
        };
        modal.subscribe_to_editors(ctx);
        modal
    }

    pub fn configure(
        &mut self,
        repository_id: RepositoryId,
        workspace_id: RepositoryWorkspaceId,
        repository_root: PathBuf,
        home: PathBuf,
        repository_name: String,
        ctx: &mut ViewContext<Self>,
    ) {
        self.target = Some(CreateWorkspaceTarget {
            repository_id,
            workspace_id,
        });
        self.repository_root = Some(repository_root);
        self.form = CreateWorkspaceForm::new();
        self.defaults = Some(CreateWorkspaceDefaults::new(home, repository_name));
        self.validation_error = None;
        self.primary_worktree_error = None;
        self.local_branches.clear();
        self.local_branch_fallback_loaded = false;
        self.selected_remote_branch = None;
        self.selected_local_branch = None;
        self.selected_existing_worktree = None;
        self.reset_editor(&self.new_branch_editor, "", ctx);
        self.reset_editor(&self.display_name_editor, "", ctx);
        self.reset_editor(&self.worktree_path_editor, "", ctx);
        self.local_branch_picker.update(ctx, |picker, ctx| {
            picker.set_items(Vec::new(), ctx);
            picker.set_disabled(ctx);
        });
        self.begin_branch_fetch(ctx);
        self.begin_existing_worktree_fetch(ctx);
    }

    pub fn on_close(&mut self, ctx: &mut ViewContext<Self>) {
        self.target = None;
        self.repository_root = None;
        self.validation_error = None;
        self.remote_fetch_error = None;
        self.existing_worktree_fetch_error = None;
        self.primary_worktree_error = None;
        self.sync_submit_button_disabled_state(ctx);
        ctx.notify();
    }

    pub fn matches_target(
        &self,
        repository_id: RepositoryId,
        workspace_id: RepositoryWorkspaceId,
    ) -> bool {
        self.target.is_some_and(|target| {
            target.repository_id == repository_id && target.workspace_id == workspace_id
        })
    }

    pub fn begin_branch_fetch(&mut self, ctx: &mut ViewContext<Self>) {
        self.remote_fetch_error = None;
        self.sync_submit_button_disabled_state(ctx);
        self.remote_branch_options.clear();
        self.selected_remote_branch = None;
        self.remote_branch_picker.update(ctx, |picker, ctx| {
            picker.set_items(
                vec![DropdownItem::new(
                    "Fetching remote branches...",
                    CreateWorkspaceModalAction::NoOp,
                )],
                ctx,
            );
            picker.set_selected_by_action(CreateWorkspaceModalAction::NoOp, ctx);
            picker.set_disabled(ctx);
        });
        if self.local_branch_fallback_loaded {
            self.local_branch_picker.update(ctx, |picker, ctx| {
                picker.set_enabled(ctx);
            });
        } else {
            self.local_branch_picker.update(ctx, |picker, ctx| {
                picker.set_disabled(ctx);
            });
        }
        ctx.notify();
    }

    pub fn begin_existing_worktree_fetch(&mut self, ctx: &mut ViewContext<Self>) {
        self.existing_worktree_fetch_error = None;
        self.primary_worktree_error = None;
        self.existing_worktree_options.clear();
        self.selected_existing_worktree = None;
        self.existing_worktree_picker.update(ctx, |picker, ctx| {
            picker.set_items(
                vec![DropdownItem::new(
                    "Fetching existing worktrees...",
                    CreateWorkspaceModalAction::NoOp,
                )],
                ctx,
            );
            picker.set_selected_by_action(CreateWorkspaceModalAction::NoOp, ctx);
            picker.set_disabled(ctx);
        });
        self.sync_submit_button_disabled_state(ctx);
        ctx.notify();
    }

    pub fn set_existing_worktrees(
        &mut self,
        worktrees: Vec<WorktreeInfo>,
        ctx: &mut ViewContext<Self>,
    ) {
        let Some(repository_root) = self.repository_root.as_deref() else {
            return;
        };
        self.existing_worktree_fetch_error = None;
        self.primary_worktree_error = primary_worktree_error(repository_root, &worktrees);
        self.existing_worktree_options = existing_worktree_options(repository_root, worktrees);
        self.selected_existing_worktree = None;
        let existing_worktree_items = self
            .existing_worktree_options
            .iter()
            .cloned()
            .map(|worktree| {
                DropdownItem::new(
                    existing_worktree_display_label(&worktree),
                    CreateWorkspaceModalAction::SelectExistingWorktree(worktree),
                )
            })
            .collect();
        self.existing_worktree_picker.update(ctx, |picker, ctx| {
            picker.set_items(existing_worktree_items, ctx);
            picker.set_enabled(ctx);
        });

        if let Some(worktree) = self.existing_worktree_options.first().cloned() {
            self.existing_worktree_picker.update(ctx, |picker, ctx| {
                picker.set_selected_by_action(
                    CreateWorkspaceModalAction::SelectExistingWorktree(worktree.clone()),
                    ctx,
                );
            });
            self.selected_existing_worktree = Some(worktree.clone());
            if self.form.mode() == CreateWorkspaceMode::ExistingWorktree {
                self.select_existing_worktree(worktree, ctx);
            }
        }
        self.sync_submit_button_disabled_state(ctx);
        ctx.notify();
    }

    pub fn set_existing_worktree_fetch_error(
        &mut self,
        message: String,
        ctx: &mut ViewContext<Self>,
    ) {
        self.existing_worktree_fetch_error = Some(message);
        self.existing_worktree_picker.update(ctx, |picker, ctx| {
            picker.set_disabled(ctx);
        });
        self.sync_submit_button_disabled_state(ctx);
        ctx.notify();
    }

    pub fn set_branch_refs(&mut self, refs: Vec<BranchRef>, ctx: &mut ViewContext<Self>) {
        let (remote_branch_options, local_branches) = branch_ref_options(refs);
        self.remote_fetch_error = None;
        self.sync_submit_button_disabled_state(ctx);
        self.remote_branch_options = remote_branch_options;
        self.local_branches = local_branches;
        self.local_branch_fallback_loaded = false;
        self.selected_remote_branch = None;
        self.selected_local_branch = None;
        let remote_items = self
            .remote_branch_options
            .iter()
            .cloned()
            .map(|branch| {
                DropdownItem::new(
                    branch.display_label.clone(),
                    CreateWorkspaceModalAction::SelectRemoteBranch(branch),
                )
            })
            .collect();
        let local_items = self
            .local_branches
            .iter()
            .cloned()
            .map(|branch| {
                DropdownItem::new(
                    branch.clone(),
                    CreateWorkspaceModalAction::SelectLocalBranch(branch),
                )
            })
            .collect();

        self.remote_branch_picker.update(ctx, |picker, ctx| {
            picker.set_items(remote_items, ctx);
            picker.set_enabled(ctx);
        });
        self.local_branch_picker.update(ctx, |picker, ctx| {
            picker.set_items(local_items, ctx);
            picker.set_enabled(ctx);
        });

        if let Some(branch) = self.remote_branch_options.first().cloned() {
            self.remote_branch_picker.update(ctx, |picker, ctx| {
                picker.set_selected_by_action(
                    CreateWorkspaceModalAction::SelectRemoteBranch(branch.clone()),
                    ctx,
                );
            });
            self.selected_remote_branch = Some(branch.clone());
            if self.form.mode() == CreateWorkspaceMode::RemoteBranch {
                self.select_remote_branch(branch, ctx);
            }
        }
        ctx.notify();
    }

    pub fn set_local_branch_refs(&mut self, refs: Vec<BranchRef>, ctx: &mut ViewContext<Self>) {
        let (_, local_branches) = branch_ref_options(refs);
        self.remote_branch_options.clear();
        self.selected_remote_branch = None;
        self.local_branches = local_branches;
        self.selected_local_branch = None;
        self.local_branch_fallback_loaded = true;
        let local_items = self
            .local_branches
            .iter()
            .cloned()
            .map(|branch| {
                DropdownItem::new(
                    branch.clone(),
                    CreateWorkspaceModalAction::SelectLocalBranch(branch),
                )
            })
            .collect();
        self.local_branch_picker.update(ctx, |picker, ctx| {
            picker.set_items(local_items, ctx);
            picker.set_enabled(ctx);
        });
        self.remote_branch_picker.update(ctx, |picker, ctx| {
            picker.set_items(Vec::new(), ctx);
            picker.set_disabled(ctx);
        });
        ctx.notify();
    }

    pub fn set_branch_fetch_error(&mut self, message: String, ctx: &mut ViewContext<Self>) {
        self.remote_fetch_error = Some(message);
        self.sync_submit_button_disabled_state(ctx);
        self.remote_branch_picker.update(ctx, |picker, ctx| {
            picker.set_disabled(ctx);
        });
        ctx.notify();
    }

    pub fn set_validation_error(&mut self, message: String, ctx: &mut ViewContext<Self>) {
        self.validation_error = Some(message);
        ctx.notify();
    }

    fn build_editor(placeholder: &str, ctx: &mut ViewContext<Self>) -> ViewHandle<EditorView> {
        let placeholder = placeholder.to_string();
        ctx.add_typed_action_view(move |ctx| {
            let mut editor = EditorView::single_line(SingleLineEditorOptions::default(), ctx);
            editor.set_placeholder_text(&placeholder, ctx);
            editor
        })
    }

    fn subscribe_to_editors(&mut self, ctx: &mut ViewContext<Self>) {
        for editor in [
            self.new_branch_editor.clone(),
            self.display_name_editor.clone(),
            self.worktree_path_editor.clone(),
        ] {
            ctx.subscribe_to_view(&editor, |modal, _, event, ctx| match event {
                EditorEvent::Enter => modal.try_submit(ctx),
                EditorEvent::Escape => ctx.emit(CreateWorkspaceModalEvent::Close),
                EditorEvent::Edited(_) => {
                    modal.validation_error = None;
                    ctx.notify();
                }
                _ => {}
            });
        }
    }

    fn reset_editor(
        &self,
        editor: &ViewHandle<EditorView>,
        text: &str,
        ctx: &mut ViewContext<Self>,
    ) {
        editor.update(ctx, |editor, ctx| {
            editor.system_reset_buffer_text(text, ctx);
        });
    }

    fn editor_text(editor: &ViewHandle<EditorView>, app: &AppContext) -> String {
        editor.as_ref(app).buffer_text(app).trim().to_string()
    }

    fn set_mode(&mut self, mode: CreateWorkspaceMode, ctx: &mut ViewContext<Self>) {
        if self.form.mode() == mode {
            return;
        }
        self.form.set_mode(mode);
        self.sync_submit_button_disabled_state(ctx);
        self.validation_error = None;
        match mode {
            CreateWorkspaceMode::RemoteBranch => {
                if let Some(branch) = self.selected_remote_branch.clone() {
                    self.select_remote_branch(branch, ctx);
                }
            }
            CreateWorkspaceMode::ExistingLocalBranch => {
                if let Some(branch) = self.selected_local_branch.clone() {
                    self.select_local_branch(branch, ctx);
                }
            }
            CreateWorkspaceMode::ExistingWorktree => {
                if let Some(worktree) = self.selected_existing_worktree.clone() {
                    self.select_existing_worktree(worktree, ctx);
                }
            }
        }
        ctx.notify();
    }

    fn sync_submit_button_disabled_state(&mut self, ctx: &mut ViewContext<Self>) {
        let disabled = submit_is_disabled(
            self.form.mode(),
            self.remote_fetch_error.is_some(),
            self.existing_worktree_fetch_error.is_some(),
            self.selected_existing_worktree.is_some(),
        );
        self.submit_button.update(ctx, |button, ctx| {
            button.set_disabled(disabled, ctx);
        });
    }

    fn select_remote_branch(&mut self, branch: RemoteBranchOption, ctx: &mut ViewContext<Self>) {
        self.form.set_remote_ref(branch.full_ref.clone());
        self.selected_remote_branch = Some(branch.clone());
        self.apply_defaults(&branch.branch_name, true, ctx);
        self.validation_error = None;
        self.sync_submit_button_disabled_state(ctx);
        ctx.notify();
    }

    fn select_local_branch(&mut self, branch: String, ctx: &mut ViewContext<Self>) {
        self.form.set_local_branch(branch.clone());
        self.selected_local_branch = Some(branch.clone());
        self.apply_defaults(&branch, false, ctx);
        self.validation_error = None;
        self.sync_submit_button_disabled_state(ctx);
        ctx.notify();
    }

    fn select_existing_worktree(
        &mut self,
        worktree: ExistingWorktreeOption,
        ctx: &mut ViewContext<Self>,
    ) {
        self.form
            .set_existing_worktree_branch(worktree.branch_name.clone());
        self.selected_existing_worktree = Some(worktree.clone());
        self.reset_editor(
            &self.display_name_editor,
            existing_worktree_default_name(&worktree),
            ctx,
        );
        self.validation_error = None;
        self.sync_submit_button_disabled_state(ctx);
        ctx.notify();
    }

    fn apply_defaults(
        &mut self,
        branch_name: &str,
        reset_new_branch: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        let (new_branch, workspace_name, worktree_path) = {
            let Some(defaults) = self.defaults.as_mut() else {
                return;
            };
            defaults.apply_branch(branch_name);
            (
                defaults.new_branch.clone(),
                defaults.workspace_name.clone(),
                defaults.worktree_path.clone(),
            )
        };
        if reset_new_branch {
            self.form.set_new_branch(new_branch.clone());
            self.reset_editor(&self.new_branch_editor, &new_branch, ctx);
        }
        self.reset_editor(&self.display_name_editor, &workspace_name, ctx);
        self.reset_editor(
            &self.worktree_path_editor,
            worktree_path.to_string_lossy().as_ref(),
            ctx,
        );
    }

    fn try_submit(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(target) = self.target else {
            self.validation_error =
                Some("Select a repository before creating a workspace.".to_string());
            ctx.notify();
            return;
        };

        match self.form.mode() {
            CreateWorkspaceMode::RemoteBranch => {
                if self.remote_fetch_error.is_some() {
                    self.validation_error =
                        Some("Retry remote branch loading or choose a local branch.".to_string());
                    ctx.notify();
                    return;
                }
                let Some(branch) = self.selected_remote_branch.as_ref() else {
                    self.validation_error =
                        Some("Select a remote branch before creating a workspace.".to_string());
                    ctx.notify();
                    return;
                };
                self.form.set_remote_ref(branch.full_ref.clone());
                self.form
                    .set_new_branch(Self::editor_text(&self.new_branch_editor, ctx));
            }
            CreateWorkspaceMode::ExistingLocalBranch => {
                if let Some(branch) = self.selected_local_branch.clone() {
                    self.form.set_local_branch(branch);
                }
            }
            CreateWorkspaceMode::ExistingWorktree => {
                if self.existing_worktree_fetch_error.is_some() {
                    self.validation_error = Some(
                        "Retry existing worktree loading before creating a workspace.".to_string(),
                    );
                    ctx.notify();
                    return;
                }
                let Some(worktree) = self.selected_existing_worktree.clone() else {
                    self.validation_error = Some(
                        "Select an existing worktree before creating a workspace.".to_string(),
                    );
                    ctx.notify();
                    return;
                };
                self.form.set_existing_worktree_branch(worktree.branch_name);
            }
        }

        let source_branch = match self.form.mode() {
            CreateWorkspaceMode::RemoteBranch => self.form.new_branch().to_string(),
            CreateWorkspaceMode::ExistingLocalBranch => {
                self.selected_local_branch.clone().unwrap_or_default()
            }
            CreateWorkspaceMode::ExistingWorktree => self
                .selected_existing_worktree
                .as_ref()
                .map_or_else(String::new, |worktree| worktree.branch_name.clone()),
        };
        let display_name = {
            let name = Self::editor_text(&self.display_name_editor, ctx);
            if name.is_empty() {
                source_branch.clone()
            } else {
                name
            }
        };
        let worktree_path = if self.form.mode() == CreateWorkspaceMode::ExistingWorktree {
            self.selected_existing_worktree
                .as_ref()
                .expect("existing worktree selection was checked before request construction")
                .path
                .clone()
        } else {
            PathBuf::from(Self::editor_text(&self.worktree_path_editor, ctx))
        };

        let Some(request) = self.form.build_request(
            target.repository_id,
            target.workspace_id,
            display_name,
            worktree_path,
        ) else {
            self.validation_error = Some(
                "Enter a valid branch reference, workspace name, and worktree path.".to_string(),
            );
            ctx.notify();
            return;
        };
        ctx.emit(CreateWorkspaceModalEvent::Submit(request));
    }

    fn section(label: &str, child: Box<dyn Element>, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(
                Text::new_inline(
                    label.to_string(),
                    appearance.ui_font_family(),
                    appearance.ui_font_size(),
                )
                .with_color(theme.sub_text_color(theme.background()).into())
                .finish(),
            )
            .with_child(Container::new(child).with_margin_top(4.).finish())
            .finish()
    }

    fn constrain_editor(child: Box<dyn Element>) -> Box<dyn Element> {
        ConstrainedBox::new(Clipped::new(child).finish())
            .with_max_width(480.)
            .finish()
    }
}

impl Entity for CreateWorkspaceModal {
    type Event = CreateWorkspaceModalEvent;
}

impl TypedActionView for CreateWorkspaceModal {
    type Action = CreateWorkspaceModalAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            CreateWorkspaceModalAction::Close => ctx.emit(CreateWorkspaceModalEvent::Close),
            CreateWorkspaceModalAction::NoOp => {}
            CreateWorkspaceModalAction::SetMode(mode) => self.set_mode(*mode, ctx),
            CreateWorkspaceModalAction::SelectRemoteBranch(branch) => {
                self.select_remote_branch(branch.clone(), ctx)
            }
            CreateWorkspaceModalAction::SelectLocalBranch(branch) => {
                self.select_local_branch(branch.clone(), ctx)
            }
            CreateWorkspaceModalAction::SelectExistingWorktree(worktree) => {
                self.select_existing_worktree(worktree.clone(), ctx)
            }
            CreateWorkspaceModalAction::RetryBranchRefs => {
                if let Some(target) = self.target {
                    ctx.emit(target.retry_branch_refs_event());
                }
            }
            CreateWorkspaceModalAction::RetryExistingWorktrees => {
                if let Some(target) = self.target {
                    ctx.emit(target.retry_existing_worktrees_event());
                }
            }
            CreateWorkspaceModalAction::Submit => self.try_submit(ctx),
        }
    }
}

impl View for CreateWorkspaceModal {
    fn ui_name() -> &'static str {
        "CreateWorkspaceModal"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let is_remote = self.form.mode() == CreateWorkspaceMode::RemoteBranch;
        let is_existing_worktree = self.form.mode() == CreateWorkspaceMode::ExistingWorktree;
        let (branch_label, branch_picker) = match self.form.mode() {
            CreateWorkspaceMode::RemoteBranch => (
                "Remote branch",
                ChildView::new(&self.remote_branch_picker).finish(),
            ),
            CreateWorkspaceMode::ExistingLocalBranch => (
                "Local branch",
                ChildView::new(&self.local_branch_picker).finish(),
            ),
            CreateWorkspaceMode::ExistingWorktree => (
                "Existing worktree",
                ChildView::new(&self.existing_worktree_picker).finish(),
            ),
        };
        let mut form = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(12.)
            .with_child(
                Text::new_inline(
                    "Create workspace",
                    appearance.ui_font_family(),
                    appearance.ui_font_heading_3(),
                )
                .with_color(theme.main_text_color(theme.background()).into())
                .finish(),
            )
            .with_child(
                Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_spacing(8.)
                    .with_child(ChildView::new(&self.remote_mode_button).finish())
                    .with_child(ChildView::new(&self.local_mode_button).finish())
                    .with_child(ChildView::new(&self.existing_worktree_mode_button).finish())
                    .finish(),
            )
            .with_child(Self::section(branch_label, branch_picker, appearance));
        if is_remote {
            if let Some(error) = &self.remote_fetch_error {
                form.add_child(Self::constrain_editor(
                    Text::new_inline(
                        error.clone(),
                        appearance.ui_font_family(),
                        appearance.ui_font_body(),
                    )
                    .with_color(theme.ui_error_color())
                    .finish(),
                ));
                form.add_child(ChildView::new(&self.retry_remote_button).finish());
            }
        }
        if is_existing_worktree {
            if let Some(error) = &self.primary_worktree_error {
                form.add_child(Self::constrain_editor(
                    Text::new_inline(
                        error.clone(),
                        appearance.ui_font_family(),
                        appearance.ui_font_body(),
                    )
                    .with_color(theme.ui_error_color())
                    .finish(),
                ));
            }
            if let Some(error) = &self.existing_worktree_fetch_error {
                form.add_child(Self::constrain_editor(
                    Text::new_inline(
                        error.clone(),
                        appearance.ui_font_family(),
                        appearance.ui_font_body(),
                    )
                    .with_color(theme.ui_error_color())
                    .finish(),
                ));
                form.add_child(ChildView::new(&self.retry_existing_worktree_button).finish());
            }
        }
        if is_remote {
            form.add_child(Self::section(
                "New local branch",
                Self::constrain_editor(ChildView::new(&self.new_branch_editor).finish()),
                appearance,
            ));
        }
        form.add_child(Self::section(
            "Workspace name",
            Self::constrain_editor(ChildView::new(&self.display_name_editor).finish()),
            appearance,
        ));
        let worktree_path = if is_existing_worktree {
            let path = self
                .selected_existing_worktree
                .as_ref()
                .map_or_else(String::new, |worktree| {
                    worktree.path.to_string_lossy().into_owned()
                });
            Self::constrain_editor(
                Text::new_inline(path, appearance.ui_font_family(), appearance.ui_font_body())
                    .with_color(theme.main_text_color(theme.background()).into())
                    .finish(),
            )
        } else {
            Self::constrain_editor(ChildView::new(&self.worktree_path_editor).finish())
        };
        form.add_child(Self::section("Worktree path", worktree_path, appearance));
        if let Some(error) = &self.validation_error {
            form.add_child(Self::constrain_editor(
                Text::new_inline(
                    error.clone(),
                    appearance.ui_font_family(),
                    appearance.ui_font_body(),
                )
                .with_color(theme.ui_error_color())
                .finish(),
            ));
        }

        let footer = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::End)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.)
            .with_child(ChildView::new(&self.cancel_button).finish())
            .with_child(ChildView::new(&self.submit_button).finish())
            .finish();
        Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(
                Container::new(form.finish())
                    .with_uniform_padding(20.)
                    .finish(),
            )
            .with_child(Container::new(footer).with_uniform_padding(12.).finish())
            .finish()
    }
}

#[cfg(test)]
#[path = "create_workspace_modal_tests.rs"]
mod tests;
