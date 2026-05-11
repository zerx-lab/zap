/// This module contains a model that can be used for loading and saving text files
/// and displaying them in a code editor.
/// It also handles applying an optional diff to the file content that will be applied
/// when the file is loaded.
//
// LSP 全栈下线后,本文件不再承载任何 LSP / hover / goto-definition /
// find-references / 诊断装饰相关逻辑;只保留文件 load/save、diff 接受/拒绝、
// 选区上下文 tooltip、版本冲突横幅、TabConfig footer 等本地能力。
use std::{
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
};

use pathfinder_geometry::vector::Vector2F;
use warp_core::{features::FeatureFlag, ui::appearance::Appearance};
use warp_editor::{content::buffer::InitialBufferState, render::model::LineCount};
use warp_util::{
    content_version::ContentVersion,
    file::{FileId, FileLoadError, FileSaveError},
    path::to_relative_path,
};
use warpui::platform::SaveFilePickerConfiguration;
use warpui::{
    elements::{
        Border, ChildAnchor, ChildView, ConstrainedBox, Container, CornerRadius,
        CrossAxisAlignment, DropShadow, Flex, Hoverable, MainAxisAlignment, MainAxisSize,
        MouseStateHandle, OffsetPositioning, ParentAnchor, ParentElement, ParentOffsetBounds,
        Radius, Rect, Shrinkable, Stack, Text,
    },
    keymap::{macros::*, FixedBinding},
    text::point::Point,
    ui_components::{
        button::ButtonVariant,
        components::{Coords, UiComponent, UiComponentStyles},
    },
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
    WindowId,
};

use crate::{
    code::{editor::EditorReviewComment, global_buffer_model::GlobalBufferModelEvent},
    code_review::comments::CommentId,
};
use crate::{
    code::{
        footer::{CodeFooterView, CodeFooterViewEvent},
        global_buffer_model::{BufferState, GlobalBufferModel},
        SaveOutcome,
    },
    settings::AISettings,
    terminal::TerminalView,
    util::sync::Condition,
};
use ai::diff_validation::DiffType;
use pathfinder_color::ColorU;
use vim::vim::{MotionType, VimMode};
use warp_core::ui::icons::Icon;

use crate::workspace::WorkspaceAction;

const DROP_SHADOW_COLOR: ColorU = ColorU {
    r: 0,
    g: 0,
    b: 0,
    a: 48,
};

use super::diff_viewer::DiffViewer;
use super::editor::{
    scroll::{ScrollPosition, ScrollTrigger},
    view::{CodeEditorEvent, CodeEditorView},
};
use super::ImmediateSaveError;

type SaveCallback =
    Box<dyn FnOnce(SaveOutcome, &mut ViewContext<LocalCodeEditorView>) + Send + Sync + 'static>;

pub fn init(app: &mut AppContext) {
    app.register_fixed_bindings([FixedBinding::new(
        "cmdorctrl-l",
        LocalCodeEditorAction::InsertSelectedTextToInput,
        id!("LocalCodeEditorView") & !id!("IMEOpen"),
    )]);
}

pub enum LocalCodeEditorEvent {
    FileLoaded,
    FailedToLoad {
        error: Rc<FileLoadError>,
    },
    FileSaved,
    FailedToSave {
        error: Rc<FileSaveError>,
    },
    DiffAccepted,
    DiffRejected,
    /// Emitted when a user presses Escape in Vim Normal mode inside the embedded editor.
    VimMinimizeRequested,
    /// Emitted when a user edits the file.
    UserEdited,
    /// Emitted when the diff status changes (e.g., line counts update).
    DiffStatusUpdated,
    SelectionAddedAsContext {
        relative_file_path: String,
        /// 1-indexed line range of the selection: `[start, end]` both inclusive.
        line_range: Range<LineCount>,
        /// Literal text content of the selection.
        selected_text: String,
    },
    DiscardUnsavedChanges {
        path: PathBuf,
    },
    /// Emitted when a comment is saved. This propagates the comment content
    /// changes to the CodeReviewView, which will update the comment model.
    CommentSaved {
        comment: EditorReviewComment,
    },
    RequestOpenComment(CommentId),
    DeleteComment {
        id: CommentId,
    },
    /// Emitted when the viewport is updated after layout
    ViewportUpdated,
    /// Emitted when the render state layout has been updated.
    LayoutInvalidated,
    /// TabConfig footer 上点击「/update-tab-config」后递到上层处理。
    RunTabConfigSkill {
        path: PathBuf,
    },
    DelayedRenderingFlushed,
}

/// Metadata about a file that is opened in the code view.
#[derive(Debug, Clone)]
enum LoadedFileMetadata {
    /// Normal file with both FileId and path (for files that are actually opened)
    LocalFile { id: FileId, path: PathBuf },
}

pub use super::diff_viewer::DisplayMode;

type TerminalTargetFn = dyn Fn(WindowId, &AppContext) -> Option<ViewHandle<TerminalView>>;

struct SelectionAsContextTooltip {
    mouse_state: MouseStateHandle,
    terminal_target_fn: Box<TerminalTargetFn>,
}

#[derive(Debug, Clone)]
pub enum LocalCodeEditorAction {
    InsertSelectedTextToInput,
    SaveFile,
    DiscardUnsavedChanges,
}

#[derive(Default)]
struct ConflictResolutionBannerMouseStates {
    discard_mouse_state: MouseStateHandle,
    overwrite_mouse_state: MouseStateHandle,
}

pub struct LocalCodeEditorView {
    pub(super) editor: ViewHandle<CodeEditorView>,
    metadata: Option<LoadedFileMetadata>,
    enable_diff_nav_by_default: bool,
    is_new_file: bool,
    diff_type: Option<DiffType>,
    selection_as_context_tooltip: Option<SelectionAsContextTooltip>,
    /// A marker for when the backing file has first been loaded. This is used to prevent applying
    /// a diff before it can be properly calculated.
    file_loaded: Condition,
    /// Whether content was changed from its base.
    was_edited: bool,
    /// Content version of the base file state.
    base_content_version: Option<ContentVersion>,
    conflict_banner_mouse_states: ConflictResolutionBannerMouseStates,
    /// Default directory to use for save dialogs when creating new files
    default_directory: Option<PathBuf>,
    /// Footer for displaying TabConfig actions. Only created for tab config TOML files.
    footer: Option<ViewHandle<CodeFooterView>>,
    /// Pending scroll position to apply after the file is loaded. This is used when
    /// `set_pending_scroll` is called before the file content has finished loading
    /// (e.g., in the GlobalBuffer path where content loads asynchronously).
    pending_scroll_on_load: Option<ScrollPosition>,
}

impl LocalCodeEditorView {
    pub fn new(
        editor: ViewHandle<CodeEditorView>,
        diff_type: Option<DiffType>,
        enable_diff_nav_by_default: bool,
        display_mode: Option<DisplayMode>,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        ctx.subscribe_to_view(&editor, |me, _, event, ctx| match event {
            CodeEditorEvent::UnifiedDiffComputed(_) => {
                ctx.emit(LocalCodeEditorEvent::DiffAccepted);
            }
            CodeEditorEvent::ContentChanged { origin, .. } => {
                me.update_diff_hunk_gutter_buttons(ctx);

                if origin.from_user() {
                    me.was_edited = true;
                    ctx.emit(LocalCodeEditorEvent::UserEdited);
                }
            }
            CodeEditorEvent::VimEscapeInNormalMode => {
                ctx.emit(LocalCodeEditorEvent::VimMinimizeRequested);
            }
            CodeEditorEvent::EscapePressed => {}
            CodeEditorEvent::DiffUpdated => {
                ctx.emit(LocalCodeEditorEvent::DiffStatusUpdated);
            }
            CodeEditorEvent::SelectionEnd => {
                ctx.notify();
            }
            CodeEditorEvent::MouseHovered { .. } => {
                // LSP 下线后,鼠标 hover 不再触发 hover/goto-definition;保留 event 订阅但不做处理。
            }
            CodeEditorEvent::CommentSaved { comment } => {
                ctx.emit(LocalCodeEditorEvent::CommentSaved {
                    comment: comment.clone(),
                });
            }
            CodeEditorEvent::DeleteComment { id } => {
                ctx.emit(LocalCodeEditorEvent::DeleteComment { id: *id });
            }
            CodeEditorEvent::RequestOpenComment(uuid) => {
                ctx.emit(LocalCodeEditorEvent::RequestOpenComment(*uuid));
            }
            CodeEditorEvent::ViewportUpdated => {
                ctx.emit(LocalCodeEditorEvent::ViewportUpdated);
            }
            CodeEditorEvent::LayoutInvalidated => {
                ctx.emit(LocalCodeEditorEvent::LayoutInvalidated);
            }
            CodeEditorEvent::DelayedRenderingFlushed => {
                ctx.emit(LocalCodeEditorEvent::DelayedRenderingFlushed);
            }
            CodeEditorEvent::Focused
            | CodeEditorEvent::SelectionChanged
            | CodeEditorEvent::SelectionStart
            | CodeEditorEvent::CopiedEmptyText
            | CodeEditorEvent::DiffHunkContextAdded { .. }
            | CodeEditorEvent::DiffReverted
            | CodeEditorEvent::HiddenSectionExpanded => {}
            #[cfg(windows)]
            CodeEditorEvent::WindowsCtrlC { .. } => {}
        });

        let is_new_file = matches!(diff_type, Some(DiffType::Create { .. }));

        let model = Self {
            editor,
            diff_type,
            is_new_file,
            metadata: None,
            enable_diff_nav_by_default,
            file_loaded: Condition::new(),
            selection_as_context_tooltip: None,
            was_edited: false,
            base_content_version: None,
            conflict_banner_mouse_states: Default::default(),
            default_directory: None,
            footer: None,
            pending_scroll_on_load: None,
        };

        if let Some(display_mode) = display_mode {
            model.set_display_mode(display_mode, ctx);
        }
        model
    }

    fn perform_save(&mut self, file_id: FileId, ctx: &mut ViewContext<Self>) {
        self.base_content_version = Some(self.editor.as_ref(ctx).version(ctx));

        let result = match self.diff() {
            Some(DiffType::Update {
                rename: Some(new_path),
                ..
            }) => self.editor.update(ctx, |editor, ctx| {
                let content = editor.text(ctx);
                let buffer_version = editor.version(ctx);

                GlobalBufferModel::handle(ctx).update(ctx, move |model, ctx| {
                    model.rename_and_save(
                        file_id,
                        new_path.clone(),
                        content.into_string(),
                        buffer_version,
                        ctx,
                    )
                })
            }),
            Some(DiffType::Delete { .. }) => self.editor.update(ctx, |editor, ctx| {
                let buffer_version = editor.version(ctx);
                GlobalBufferModel::handle(ctx).update(ctx, move |model, ctx| {
                    model.delete(file_id, buffer_version, ctx)
                })
            }),
            _ => self.editor.update(ctx, |editor, ctx| {
                let content = editor.text(ctx);
                let buffer_version = editor.version(ctx);

                GlobalBufferModel::handle(ctx).update(ctx, move |model, ctx| {
                    model.save(file_id, content.into_string(), buffer_version, ctx)
                })
            }),
        };

        if let Err(err) = result {
            log::error!("Failed to save file: {err:?}");
            ctx.emit(LocalCodeEditorEvent::FailedToSave {
                error: Rc::new(err),
            });
        }
    }

    pub fn is_new_file(&self) -> bool {
        self.is_new_file
    }

    // This is a footgun - we should remove this codepath.
    pub fn set_new_file(&mut self, is_new: bool) {
        self.is_new_file = is_new;
    }

    pub fn set_default_directory(&mut self, directory: Option<PathBuf>) {
        self.default_directory = directory;
    }

    pub fn reset_with_state(&mut self, state: InitialBufferState, ctx: &mut ViewContext<Self>) {
        self.base_content_version = Some(state.version);
        self.editor
            .update(ctx, |editor, ctx| editor.reset(state, ctx));
    }

    /// Whether the content of the source file this editor is based on has been loaded into the buffer.
    pub fn file_loaded(&self, ctx: &mut ViewContext<Self>) -> bool {
        // For global buffer, we could have utilized a shared buffer from another open editor. To avoid
        // any race condition, we directly check with the GlobalBufferModel rather than relying on self.base_content_version
        // which is updated via an async event.
        let Some(file_id) = self.file_id() else {
            return false;
        };

        GlobalBufferModel::as_ref(ctx).buffer_loaded(file_id)
    }

    /// Construct a new local editor view with a shared buffer.
    pub fn new_with_global_buffer<T>(
        path: &Path,
        editor_constructor: T,
        enable_diff_nav_by_default: bool,
        display_mode: Option<DisplayMode>,
        ctx: &mut ViewContext<Self>,
    ) -> Self
    where
        T: FnOnce(BufferState, &mut ViewContext<Self>) -> ViewHandle<CodeEditorView>,
    {
        let buffer_state = GlobalBufferModel::handle(ctx)
            .update(ctx, |model, ctx| model.open(path.to_path_buf(), ctx));
        let file_id = buffer_state.file_id;
        let editor = editor_constructor(buffer_state, ctx);

        editor.update(ctx, |editor, ctx| {
            editor.set_language_with_path(path, ctx);
            // Rebuild layout and bootstrap syntax highlighting for the editor with existing buffer content.
            editor.model.update(ctx, |model, ctx| {
                model.rebuild_layout_with_syntax_highlighting(ctx)
            });
        });

        let mut local_editor =
            Self::new(editor, None, enable_diff_nav_by_default, display_mode, ctx);

        local_editor.metadata = Some(LoadedFileMetadata::LocalFile {
            id: file_id,
            path: path.to_path_buf(),
        });

        Self::subscribe_to_global_buffer_events(file_id, ctx);

        local_editor
    }

    pub fn set_pending_scroll(&mut self, position: ScrollPosition, ctx: &mut ViewContext<Self>) {
        // If the file is already loaded, we can set the scroll trigger immediately with the
        // current buffer version. Otherwise, store it and apply when the file finishes loading.
        // This handles the race condition where set_pending_scroll is called before the file
        // content has been loaded (e.g., in the GlobalBuffer path).
        if self.file_loaded(ctx) {
            self.editor.update(ctx, |editor, ctx| {
                let version = editor.buffer_version(ctx);
                editor.set_pending_scroll(ScrollTrigger::new(position, version));
            });
        } else {
            self.pending_scroll_on_load = Some(position);
        }
    }

    fn on_file_loaded(&mut self, ctx: &mut ViewContext<Self>) {
        self.apply_diffs_if_any(ctx);
        self.file_loaded.set();

        // Apply any pending scroll position that was set before the file finished loading.
        if let Some(position) = self.pending_scroll_on_load.take() {
            self.editor.update(ctx, |editor, ctx| {
                let version = editor.buffer_version(ctx);
                editor.set_pending_scroll(ScrollTrigger::new(position, version));
            });
        }
    }

    /// Updates the enablement state of the visible "add as context" gutter button based on the file state.
    /// If the button is hidden to begin with, this is a no-op.
    pub fn update_diff_hunk_gutter_buttons(&self, ctx: &mut ViewContext<Self>) {
        let has_unsaved_changes = self.has_unsaved_changes(ctx);
        let enabled = !has_unsaved_changes;
        self.editor.update(ctx, |code_editor, ctx| {
            code_editor.set_add_diff_hunk_as_context_button(enabled, ctx);
        });
    }

    pub fn has_unsaved_changes(&self, ctx: &AppContext) -> bool {
        if self.is_new_file {
            let text = self.editor.as_ref(ctx).text(ctx);
            if text.as_str().is_empty() {
                return false;
            }
        }

        self.base_content_version
            .map(|base_version| !self.editor.as_ref(ctx).version_match(&base_version, ctx))
            .unwrap_or(false)
    }

    /// Enables the selection-as-context tooltip. For now, we only want this to be rendered within editors in code panes.
    pub(crate) fn with_selection_as_context(
        mut self,
        terminal_target_fn: Box<TerminalTargetFn>,
    ) -> Self {
        self.selection_as_context_tooltip = Some(SelectionAsContextTooltip {
            mouse_state: Default::default(),
            terminal_target_fn,
        });
        self
    }

    /// Adds the TabConfig footer to the editor view if the file is a tab config TOML.
    /// LSP 下线后,普通源码文件不再渲染 footer。
    pub(crate) fn add_footer(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(path) = self.file_path() else {
            return;
        };
        if !CodeFooterView::is_tab_config_path(path) {
            return;
        }
        let path_buf = path.to_path_buf();
        let footer = ctx.add_typed_action_view(|ctx| CodeFooterView::new(path_buf, ctx));
        ctx.subscribe_to_view(&footer, |_, _, event, ctx| match event {
            CodeFooterViewEvent::RunTabConfigSkill { path } => {
                ctx.emit(LocalCodeEditorEvent::RunTabConfigSkill { path: path.clone() });
            }
        });
        self.footer = Some(footer);
    }

    /// Unsubscribes from any existing GlobalBufferModel subscription and sets up a
    /// new one for the given `file_id`.  Handles BufferLoaded, FailedToLoad,
    /// BufferUpdatedFromFileEvent, FileSaved, and FailedToSave events.
    fn subscribe_to_global_buffer_events(file_id: FileId, ctx: &mut ViewContext<Self>) {
        ctx.unsubscribe_to_model(&GlobalBufferModel::handle(ctx));
        ctx.subscribe_to_model(&GlobalBufferModel::handle(ctx), move |me, _, event, ctx| {
            if event.file_id() != file_id {
                return;
            }
            me.update_diff_hunk_gutter_buttons(ctx);
            match event {
                GlobalBufferModelEvent::BufferLoaded {
                    content_version, ..
                } => {
                    if me.base_content_version.is_some() {
                        return;
                    }
                    me.base_content_version = Some(*content_version);
                    me.on_file_loaded(ctx);
                    ctx.emit(LocalCodeEditorEvent::FileLoaded);
                }
                GlobalBufferModelEvent::FailedToLoad { error, .. } => {
                    me.is_new_file = true;
                    me.on_file_loaded(ctx);
                    ctx.emit(LocalCodeEditorEvent::FailedToLoad {
                        error: error.clone(),
                    });
                }
                GlobalBufferModelEvent::BufferUpdatedFromFileEvent {
                    success,
                    content_version,
                    ..
                } => {
                    if !*success {
                        ctx.notify();
                    } else {
                        me.base_content_version = Some(*content_version);
                    }
                }
                GlobalBufferModelEvent::FileSaved { .. } => {
                    ctx.emit(LocalCodeEditorEvent::FileSaved);
                }
                GlobalBufferModelEvent::FailedToSave { error, .. } => {
                    me.base_content_version = GlobalBufferModel::as_ref(ctx).base_version(file_id);
                    ctx.emit(LocalCodeEditorEvent::FailedToSave {
                        error: error.clone(),
                    });
                }
            }
        });
    }

    pub fn has_version_conflicts(&self, app: &AppContext) -> bool {
        let Some(file_id) = self.file_id() else {
            return false;
        };
        self.has_unsaved_changes(app)
            && self.base_content_version != GlobalBufferModel::as_ref(app).base_version(file_id)
    }
    /// Save the file to the local file system.
    /// This will only return an error immediately if there is a failure in the sync part of the call.
    /// Other errors could be returned asynchronously via the FileModelEvent::FailedToSave event.
    pub fn save_local(&mut self, ctx: &mut ViewContext<Self>) -> Result<(), ImmediateSaveError> {
        let Some(file_id) = self.file_id() else {
            return Err(ImmediateSaveError::NoFileId);
        };

        // LSP 下线后不再在保存前调用 LSP format。
        self.perform_save(file_id, ctx);
        Ok(())
    }

    /// Open a save dialog to save the file with a new name, optionally with a completion callback.
    pub fn save_as(&mut self, callback: Option<SaveCallback>, ctx: &mut ViewContext<Self>) {
        ctx.open_save_file_picker(
            move |path_opt, me, ctx| Self::handle_save_as(callback, path_opt, me, ctx),
            if let Some(default_dir) = &self.default_directory {
                SaveFilePickerConfiguration::new().with_default_directory(default_dir.clone())
            } else {
                SaveFilePickerConfiguration::new()
            },
        );
    }

    fn handle_save_as(
        callback: Option<SaveCallback>,
        path_opt: Option<String>,
        me: &mut Self,
        ctx: &mut ViewContext<Self>,
    ) {
        let callback = callback.unwrap_or(Box::new(|_, _| {}));
        let Some(path_str) = path_opt else {
            callback(SaveOutcome::Canceled, ctx);
            return;
        };
        let path = PathBuf::from(path_str);

        // Ensure parent directories exist before registering file watcher / LSP.
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        let buffer = me.editor.as_ref(ctx).model.as_ref(ctx).buffer().clone();
        let buffer_state = GlobalBufferModel::handle(ctx)
            .update(ctx, |model, ctx| model.register(path.clone(), buffer, ctx));

        let file_id = buffer_state.file_id;
        me.metadata = Some(LoadedFileMetadata::LocalFile {
            id: file_id,
            path: path.clone(),
        });

        me.set_new_file(false);

        me.editor.update(ctx, |editor, ctx| {
            editor.set_language_with_path(&path, ctx);
        });

        let content = me.editor.as_ref(ctx).text(ctx).into_string();
        let buffer_version = me.editor.as_ref(ctx).version(ctx);

        me.base_content_version = Some(buffer_version);
        let save_outcome = if let Err(err) = GlobalBufferModel::handle(ctx)
            .update(ctx, move |model, ctx| {
                model.save(file_id, content, buffer_version, ctx)
            }) {
            log::error!("Failed to save file to new path: {err:?}");
            ctx.emit(LocalCodeEditorEvent::FailedToSave {
                error: Rc::new(err),
            });
            SaveOutcome::Failed
        } else {
            Self::subscribe_to_global_buffer_events(file_id, ctx);
            SaveOutcome::Succeeded
        };
        callback(save_outcome, ctx);
    }

    pub fn cursor_at(&self, point: Point, ctx: &mut ViewContext<Self>) {
        self.editor.update(ctx, |editor, ctx| {
            editor.cursor_at(point, ctx);
        });
    }

    /// If there is a pending diff available, apply it on the buffer. This should only be called _after_ the buffer
    /// has been loaded.
    fn apply_diffs_if_any(&mut self, ctx: &mut ViewContext<Self>) -> Option<usize> {
        let diff = self.diff_type.clone()?;
        let deltas = match diff {
            DiffType::Create { delta } => vec![delta],
            DiffType::Update { mut deltas, .. } => {
                deltas.sort_by_key(|delta| delta.replacement_line_range.start);
                deltas
            }
            DiffType::Delete { delta } => vec![delta],
        };

        // Early return if the pending diff itself is empty.
        let first_line_start = deltas
            .first()
            .map(|diff| diff.replacement_line_range.start)?;

        self.editor.update(ctx, |editor, ctx| {
            editor.apply_diffs(deltas, ctx);

            if self.enable_diff_nav_by_default {
                editor.toggle_diff_nav(None, ctx);
            }
        });

        Some(first_line_start)
    }

    pub fn file_id(&self) -> Option<FileId> {
        self.metadata.as_ref().map(|metadata| match metadata {
            LoadedFileMetadata::LocalFile { id, .. } => *id,
        })
    }

    pub fn file_path(&self) -> Option<&Path> {
        self.metadata.as_ref().map(|metadata| match metadata {
            LoadedFileMetadata::LocalFile { path, .. } => path.as_path(),
        })
    }

    /// Update this editor's file identity after a `GlobalBufferModel::rename`.
    /// Sets the new file_id and path, re-subscribes to `GlobalBufferModelEvent`,
    /// and updates the language from the new path.
    #[cfg(feature = "local_fs")]
    pub fn apply_rename(
        &mut self,
        buffer_state: BufferState,
        new_path: &Path,
        ctx: &mut ViewContext<Self>,
    ) {
        let file_id = buffer_state.file_id;
        self.metadata = Some(LoadedFileMetadata::LocalFile {
            id: file_id,
            path: new_path.to_path_buf(),
        });

        self.editor.update(ctx, |editor, ctx| {
            editor.set_language_with_path(new_path, ctx);
        });

        // Re-subscribe to GlobalBufferModel events for the new file_id.
        Self::subscribe_to_global_buffer_events(file_id, ctx);
    }

    pub fn editor(&self) -> &ViewHandle<CodeEditorView> {
        &self.editor
    }

    /// Accept the diff that is currently in the editor. For local files, this can only be called after the file contents
    /// have been loaded into the editor.
    /// If it is a local file, the diff content will be retrieved and the pending diff will be marked as completed.
    /// If it is not a local file, the pending diff will be marked as completed with an empty diff.
    pub fn accept_diff(&mut self, ctx: &mut ViewContext<Self>) {
        match self.file_path() {
            Some(file) => {
                // Begin calculating the diff that will be saved.  When the result comes back, the diff will be marked completed.
                self.editor.update(ctx, |view, ctx| {
                    view.retrieve_unified_diff(file.display().to_string(), ctx)
                });
            }
            None => {
                ctx.emit(LocalCodeEditorEvent::DiffAccepted);
            }
        };
    }

    pub fn close_find_bar(&mut self, should_focus_editor: bool, ctx: &mut ViewContext<Self>) {
        self.editor.update(ctx, |editor, ctx| {
            editor.close_find_bar(should_focus_editor, ctx);
        });
    }

    /// If a single terminal view exists in the active window, returns the active file path's relative to to the terminal's session.
    fn file_path_relative_to_terminal_view(&self, app: &AppContext) -> Option<String> {
        if let Some(terminal_target_fn) = self
            .selection_as_context_tooltip
            .as_ref()
            .map(|tooltip| &tooltip.terminal_target_fn)
        {
            app.windows().active_window().and_then(|window_id| {
                terminal_target_fn(window_id, app).and_then(|terminal_view| {
                    terminal_view
                        .as_ref(app)
                        .active_session_path_if_local(app)
                        .and_then(|cwd| {
                            let is_wsl = terminal_view
                                .as_ref(app)
                                .active_session_wsl_distro(app)
                                .is_some();
                            self.file_path()
                                .and_then(|file_path| to_relative_path(is_wsl, file_path, &cwd))
                        })
                })
            })
        } else {
            None
        }
    }

    fn render_selection_tooltip(&self, app: &AppContext) -> Option<Box<dyn Element>> {
        // If there's a single selection and an active terminal view, we want to give the user an option to add the selection as context.
        self.selection_as_context_tooltip
            .as_ref()
            .and_then(|selection_as_context_tooltip| {
                if self.editor.as_ref(app).selected_lines(app).is_some()
                    && self.file_path_relative_to_terminal_view(app).is_some()
                {
                    let appearance = Appearance::as_ref(app);
                    let theme = appearance.theme();
                    let modifier_keys = if cfg!(target_os = "macos") {
                        "⌘L"
                    } else {
                        "Ctrl-L"
                    };

                    let mut row = Flex::row()
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_main_axis_alignment(MainAxisAlignment::Center)
                        .with_main_axis_size(MainAxisSize::Min);
                    row.add_child(
                        Shrinkable::new(
                            1.,
                            Text::new_inline(
                                "Add as context",
                                appearance.ui_font_family(),
                                appearance.ui_font_size(),
                            )
                            .with_color(theme.active_ui_text_color().into())
                            .finish(),
                        )
                        .finish(),
                    );
                    row.add_child(
                        Container::new(
                            Text::new_inline(
                                modifier_keys,
                                appearance.ui_font_family(),
                                appearance.ui_font_size() * 0.75,
                            )
                            .with_color(theme.disabled_ui_text_color().into())
                            .finish(),
                        )
                        .with_margin_left(8.)
                        .finish(),
                    );

                    Some(
                        Hoverable::new(selection_as_context_tooltip.mouse_state.clone(), |state| {
                            let background_color = if state.is_hovered() {
                                theme.surface_2()
                            } else {
                                theme.surface_1()
                            };
                            let internal_container = Container::new(row.finish())
                                .with_padding_left(12.)
                                .with_padding_right(12.)
                                .with_padding_top(4.)
                                .with_padding_bottom(4.)
                                .finish();
                            Container::new(internal_container)
                                .with_background(background_color)
                                .with_padding_top(4.)
                                .with_padding_bottom(4.)
                                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
                                .with_border(Border::all(1.5).with_border_fill(theme.surface_2()))
                                .with_drop_shadow(DropShadow::new_with_standard_offset_and_spread(
                                    DROP_SHADOW_COLOR,
                                ))
                                .finish()
                        })
                        .on_click(move |ctx, _app, _pos| {
                            ctx.dispatch_typed_action(
                                LocalCodeEditorAction::InsertSelectedTextToInput,
                            );
                        })
                        .finish(),
                    )
                } else {
                    None
                }
            })
    }

    fn insert_selected_text_to_input(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(relative_file_path) = self.file_path_relative_to_terminal_view(ctx) else {
            return;
        };

        let mut line_range: Option<Range<LineCount>> = None;
        let mut selected_text: Option<String> = None;
        self.editor.update(ctx, |editor, ctx| {
            // If we have a vim visual selection, update the editor model to use that as a selection range
            let has_vim_visual = matches!(editor.vim_mode(ctx), Some(VimMode::Visual(_)));
            if has_vim_visual {
                editor.model.update(ctx, |model, ctx| {
                    model.vim_visual_selection_range(MotionType::Linewise, false, ctx);
                });
            }

            if let Some((start, end)) = editor.selected_lines(ctx) {
                // selected_lines() returns 1-indexed row numbers.
                line_range = Some(LineCount::from(start as usize)..LineCount::from(end as usize));
                selected_text = Some(editor.selected_text(ctx).unwrap_or_default());
            }

            // Enter normal mode
            if has_vim_visual {
                editor.enter_vim_normal_mode(ctx);
            }
        });

        let (Some(line_range), Some(selected_text)) = (line_range, selected_text) else {
            return;
        };

        ctx.emit(LocalCodeEditorEvent::SelectionAddedAsContext {
            relative_file_path,
            line_range,
            selected_text,
        });
        self.editor.update(ctx, |editor, ctx| {
            editor.clear_selection(ctx);
        });
    }

    pub fn diff(&self) -> Option<&DiffType> {
        self.diff_type.as_ref()
    }
}

impl DiffViewer for LocalCodeEditorView {
    fn editor(&self) -> &ViewHandle<CodeEditorView> {
        &self.editor
    }

    fn diff(&self) -> Option<&DiffType> {
        self.diff_type.as_ref()
    }

    fn was_edited(&self) -> bool {
        self.was_edited
    }

    /// Automatically accept and save this diff. Unlike [`Self::accept_diff`] and [`Self::save_local`], this
    /// waits for the initial file contents to be loaded.
    fn accept_and_save_diff(&self, ctx: &mut ViewContext<Self>) {
        ctx.spawn(self.file_loaded.wait(), move |me, _, ctx| {
            me.accept_diff(ctx);
            if let Err(err) = me.save_local(ctx) {
                log::error!("{err:?}");
                if let ImmediateSaveError::FailedToSave(err) = err {
                    ctx.emit(LocalCodeEditorEvent::FailedToSave {
                        error: Rc::new(err),
                    });
                }
            }
        });
    }

    fn reject_diff(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(LocalCodeEditorEvent::DiffRejected);
    }

    fn restore_diff_base(&mut self, ctx: &mut ViewContext<Self>) -> Result<(), String> {
        if self.is_new_file {
            if let Some(file_id) = self.file_id() {
                GlobalBufferModel::handle(ctx).update(ctx, |model, ctx| {
                    model.remove(file_id, ctx);
                });
            }
            if let Some(path) = self.file_path().map(|p| p.to_path_buf()) {
                if let Err(e) = std::fs::remove_file(&path) {
                    log::error!("Failed to delete file after save: {e}");
                } else {
                    // This will close tabs with the file open
                    ctx.dispatch_typed_action(&WorkspaceAction::FileDeleted { path });
                }
            }

            return Ok(());
        }

        let base_content = self
            .editor
            .as_ref(ctx)
            .model
            .as_ref(ctx)
            .diff()
            .as_ref(ctx)
            .base()
            .ok_or_else(|| "Missing base content".to_string())?
            .to_string();

        let file_id = self
            .file_id()
            .ok_or_else(|| "Missing file_id".to_string())?;

        let buffer_version = self.editor.as_ref(ctx).version(ctx);

        GlobalBufferModel::handle(ctx)
            .update(ctx, |model, ctx| {
                model.save(file_id, base_content, buffer_version, ctx)
            })
            .map_err(|e| format!("Failed to save file: {e:?}"))
    }
}

impl Entity for LocalCodeEditorView {
    type Event = LocalCodeEditorEvent;
}

impl View for LocalCodeEditorView {
    fn ui_name() -> &'static str {
        "LocalCodeEditorView"
    }

    fn on_focus(&mut self, focus_ctx: &warpui::FocusContext, ctx: &mut ViewContext<Self>) {
        if focus_ctx.is_self_focused() {
            self.editor.update(ctx, |editor, ctx| editor.focus(ctx));
        }
    }

    fn render(&self, app: &AppContext) -> Box<dyn warpui::Element> {
        // Rendering the version conflict banner.
        let base: Box<dyn Element> = if self.has_version_conflicts(app) {
            let appearance = Appearance::as_ref(app);
            let banner = render_unsaved_changes_banner(
                appearance,
                self.conflict_banner_mouse_states
                    .discard_mouse_state
                    .clone(),
                self.conflict_banner_mouse_states
                    .overwrite_mouse_state
                    .clone(),
            );
            let mut col = Flex::column().with_child(banner);

            let editor_view = ChildView::new(&self.editor).finish();
            if self.editor.as_ref(app).needs_vertical_constraint() {
                col.add_child(Shrinkable::new(1., editor_view).finish());
            } else {
                col.add_child(editor_view);
            }
            col.finish()
        } else {
            ChildView::new(&self.editor).finish()
        };

        let base_with_handler = base;

        let mut stack = Stack::new()
            .with_constrain_absolute_children()
            .with_child(base_with_handler);

        let editor = self.editor().as_ref(app);
        if self.selection_as_context_tooltip.is_some() {
            // When a single terminal exists in the window and the user has made a selection (but isn't currently selecting),
            // we render a tooltip that allows them to add the selected text to the terminal context.
            let is_ai_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);
            if is_ai_enabled
                && FeatureFlag::SelectionAsContext.is_enabled()
                && !editor.is_selecting()
            {
                let tooltip = self.render_selection_tooltip(app);
                if let Some(tooltip) = tooltip {
                    stack.add_positioned_child(tooltip, editor.selection_position_anchor(app))
                }
            }
        }

        if let Some(footer) = &self.footer {
            let mut col = Flex::column();

            if self.editor.as_ref(app).needs_vertical_constraint() {
                col.add_child(Shrinkable::new(1., stack.finish()).finish());
            } else {
                col.add_child(stack.finish());
            }
            col.with_child(ChildView::new(footer).finish()).finish()
        } else {
            stack.finish()
        }
    }
}

impl TypedActionView for LocalCodeEditorView {
    type Action = LocalCodeEditorAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            LocalCodeEditorAction::InsertSelectedTextToInput => {
                self.insert_selected_text_to_input(ctx);
            }
            LocalCodeEditorAction::SaveFile => {
                if let Err(ImmediateSaveError::FailedToSave(err)) = self.save_local(ctx) {
                    log::error!("Failed to save file {err:?}");
                    ctx.emit(LocalCodeEditorEvent::FailedToSave {
                        error: Rc::new(err),
                    });
                };
            }
            LocalCodeEditorAction::DiscardUnsavedChanges => {
                if let Some(path) = self.file_path().map(Path::to_path_buf) {
                    self.base_content_version = Some(self.editor().as_ref(ctx).version(ctx));
                    ctx.emit(LocalCodeEditorEvent::DiscardUnsavedChanges { path });
                }
            }
        }
    }
}

/// Renders a banner warning that the file has saved changes not reflected in the diff
pub fn render_unsaved_changes_banner(
    appearance: &Appearance,
    discard_mouse_state: MouseStateHandle,
    overwrite_mouse_state: MouseStateHandle,
) -> Box<dyn Element> {
    let left = Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(
            Container::new(
                ConstrainedBox::new(
                    Icon::Warning
                        .to_warpui_icon(appearance.theme().active_ui_text_color())
                        .finish(),
                )
                .with_height(16.)
                .with_width(16.)
                .finish(),
            )
            .with_margin_right(8.)
            .finish(),
        )
        .with_child(
            Shrinkable::new(
                1.,
                Text::new(
                    "This file has saved changes that are not reflected here.",
                    appearance.ui_font_family(),
                    appearance.ui_font_size(),
                )
                .with_color(appearance.theme().active_ui_text_color().into())
                .soft_wrap(true)
                .finish(),
            )
            .finish(),
        )
        .finish();

    let right = Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(
            appearance
                .ui_builder()
                .button(ButtonVariant::Text, discard_mouse_state)
                .with_text_label(crate::t!("code-discard-this-version"))
                .with_style(UiComponentStyles {
                    height: Some(24.),
                    padding: Some(Coords {
                        left: 8.,
                        right: 8.,
                        ..Default::default()
                    }),
                    font_color: Some(appearance.theme().active_ui_text_color().into()),
                    ..Default::default()
                })
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(LocalCodeEditorAction::DiscardUnsavedChanges)
                })
                .finish(),
        )
        .with_child(
            Container::new(
                appearance
                    .ui_builder()
                    .button(ButtonVariant::Outlined, overwrite_mouse_state)
                    .with_text_label(crate::t!("code-overwrite"))
                    .with_style(UiComponentStyles {
                        font_color: Some(appearance.theme().active_ui_text_color().into()),
                        ..Default::default()
                    })
                    .build()
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(LocalCodeEditorAction::SaveFile)
                    })
                    .finish(),
            )
            .with_margin_left(4.)
            .finish(),
        )
        .finish();

    Container::new(
        Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_main_axis_size(MainAxisSize::Max)
            .with_child(Shrinkable::new(1., left).finish())
            .with_child(right)
            .finish(),
    )
    .with_background(appearance.theme().text_selection_as_context_color())
    .with_padding_top(4.)
    .with_padding_bottom(4.)
    .with_padding_left(12.)
    .with_padding_right(12.)
    .finish()
}

/// Renders a small yellow circle with tooltip indicating unsaved changes
pub fn render_unsaved_circle_with_tooltip(
    mouse_state: MouseStateHandle,
    tooltip_text: String,
    size: f32,
    right_margin: f32,
    appearance: &Appearance,
) -> Box<dyn Element> {
    Hoverable::new(mouse_state, |state| {
        let rect = Container::new(
            ConstrainedBox::new(
                Rect::new()
                    .with_background_color(appearance.theme().active_ui_text_color().into())
                    .with_corner_radius(CornerRadius::with_all(Radius::Percentage(50.)))
                    .finish(),
            )
            .with_width(size)
            .with_height(size)
            .finish(),
        )
        .with_margin_right(right_margin)
        .finish();

        if state.is_hovered() {
            let mut stack = Stack::new().with_child(rect);

            let tooltip = appearance
                .ui_builder()
                .tool_tip(tooltip_text)
                .build()
                .finish();

            stack.add_positioned_overlay_child(
                tooltip,
                OffsetPositioning::offset_from_parent(
                    Vector2F::new(0., 4.),
                    ParentOffsetBounds::Unbounded,
                    ParentAnchor::BottomMiddle,
                    ChildAnchor::TopMiddle,
                ),
            );
            stack.finish()
        } else {
            rect
        }
    })
    .finish()
}
