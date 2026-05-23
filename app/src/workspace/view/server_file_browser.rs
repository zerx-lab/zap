use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{Local, TimeZone};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::{vec2f, Vector2F};
use remote_server::client::RemoteServerClient;
use remote_server::proto::{
    create_directory_response, list_directory_response, read_file_chunk_response,
    resolve_path_response, run_command_response, write_file_chunk_response, FileSystemEntryKind,
};
use uuid::Uuid;
use walkdir::WalkDir;
use warp_completer::completer::CommandExitStatus;
use warp_core::ui::theme::color::internal_colors;
use warp_core::{HostId, SessionId};
use warp_util::standardized_path::StandardizedPath;
use warpui::clipboard::ClipboardContent;
use warpui::elements::{
    Border, ChildAnchor, ChildView, Clipped, ConstrainedBox, Container, CornerRadius,
    CrossAxisAlignment,
    DispatchEventResult, Dismiss, Element, Empty, EventHandler, Flex, Hoverable, MainAxisAlignment,
    MainAxisSize,
    MouseStateHandle, OffsetPositioning, ParentAnchor, ParentElement, ParentOffsetBounds, Radius,
    SavePosition, ScrollStateHandle, Scrollable, ScrollableElement, ScrollbarWidth, Shrinkable,
    Stack, Text, UniformList, UniformListState,
};
use warpui::modals::{AlertDialogWithCallbacks, ModalButton};
use warpui::platform::{Cursor, FilePickerConfiguration, SaveFilePickerConfiguration};
use warpui::ui_components::components::{Coords, UiComponent, UiComponentStyles};
use warpui::r#async::{SpawnedFutureHandle, Timer};
use warpui::{
    AppContext, Entity, FocusContext, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle,
};

use crate::code::buffer_location::RemotePath;
use crate::editor::{
    EditorView, Event as EditorEvent, PropagateAndNoOpNavigationKeys,
    PropagateHorizontalNavigationKeys, SingleLineEditorOptions, TextOptions,
};
use crate::appearance::Appearance;
use crate::menu::{
    Event as MenuEvent, Menu, MenuItem, MenuItemFields, SubMenu, DEFAULT_WIDTH as MENU_DEFAULT_WIDTH,
    MENU_ITEM_VERTICAL_PADDING, SUBMENU_OVERLAP,
};
use crate::remote_server::manager::RemoteServerManager;
use crate::terminal::model::session::{ExecuteCommandOptions, Session};
use crate::ui_components::icons::Icon;

const ITEM_FONT_SIZE: f32 = 14.0;
const TOOLBAR_BUTTON_SIZE: f32 = 26.0;
const TOOLBAR_ICON_SIZE: f32 = 14.0;
const ITEM_ICON_SIZE: f32 = 14.0;
const ITEM_PADDING_VERTICAL: f32 = 5.0;
const ITEM_PADDING_HORIZONTAL: f32 = 8.0;
const ITEM_ICON_TEXT_SPACING: f32 = 8.0;
const PANEL_HORIZONTAL_PADDING: f32 = 8.0;
const INPUT_HEIGHT: f32 = 30.0;
const CONTEXT_MENU_POSITION_ID: &str = "server_file_browser_panel_root";
const CONTEXT_MENU_WIDTH: f32 = MENU_DEFAULT_WIDTH;
const UPLOAD_PROGRESS_PANEL_POSITION: &str = "server_file_browser_upload_panel_anchor";
const TRANSFER_CHUNK_BYTES: u64 = 1024 * 1024;
const UPLOAD_PROGRESS_POLL_MS: u64 = 100;
const UPLOAD_PROGRESS_PANEL_TOP_OFFSET: f32 = 46.0;
const UPLOAD_PROGRESS_PANEL_MAX_HEIGHT: f32 = 240.0;
const UPLOAD_STAGING_DIR_NAME: &str = ".zap-upload-staging";

#[derive(Clone, Debug)]
pub enum ServerFileBrowserAction {
    Refresh,
    JumpToPath,
    ClickEntry(usize),
    OpenEntry(usize),
    ToggleDirectory(String),
    SelectPreviousItem,
    SelectNextItem,
    ExpandSelectedItem,
    CollapseSelectedItem,
    ExecuteSelectedItem,
    OpenContextMenu {
        index: usize,
        position: Vector2F,
    },
    DismissContextMenu,
    CopyPath(String),
    CopyName(String),
    CdToTerminal(String),
    Download(String),
    UploadFiles(String),
    UploadFolder(String),
    CreateFile(String),
    CreateFolder(String),
    RenameEntry(usize),
    DeleteEntry(usize),
    CommitRename,
    CancelRename,
    DismissRenameEditor,
    ToggleUploadProgressPanel,
    DismissUploadProgressPanel,
    ClearCompletedUploads,
}

#[derive(Clone, Debug)]
pub enum ServerFileBrowserEvent {
    OpenRemoteFile { remote_path: RemotePath },
    CdToDirectory { path: String },
}

#[derive(Clone, Debug)]
struct ServerFileBrowserEntry {
    name: String,
    path: String,
    kind: FileSystemEntryKind,
    size_bytes: Option<u64>,
    modified_epoch_millis: Option<u64>,
    depth: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UploadConflictPolicy {
    Proceed,
    SkipExisting,
    OverwriteAll,
}

#[derive(Clone, Debug)]
struct UploadConflict {
    path: String,
    display_name: String,
    kind: FileSystemEntryKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UploadBatchPhase {
    Uploading,
    Verifying,
    Promoting,
}

#[derive(Clone, Debug)]
enum UploadTaskStatus {
    Pending,
    Uploading,
    Completed,
    Failed(String),
    Skipped,
}

struct ServerFileUploadTask {
    local_path: PathBuf,
    file_name: String,
    final_remote_path: String,
    staging_remote_path: String,
    total_bytes: u64,
    uploaded_bytes: Arc<AtomicU64>,
    status: UploadTaskStatus,
}

struct ServerFileUploadBatch {
    staging_root: String,
    remote_directory: String,
    conflict_policy: UploadConflictPolicy,
    /// Paths that existed on the remote host when the batch started.
    conflict_paths: HashSet<String>,
    directory_overwrite_roots: HashSet<String>,
    phase: UploadBatchPhase,
    tasks: Vec<ServerFileUploadTask>,
    next_task_index: usize,
    progress_poll_handle: Option<SpawnedFutureHandle>,
}

#[derive(Clone)]
struct PendingUploadFile {
    local_path: PathBuf,
    final_remote_path: String,
    /// Shown in the upload progress panel (includes folder-relative path for directory uploads).
    display_name: String,
    total_bytes: u64,
}

struct PendingUploadStart {
    client: Arc<RemoteServerClient>,
    remote_directory: String,
    pending_files: Vec<PendingUploadFile>,
    directory_roots: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NewRemoteEntryKind {
    File,
    Directory,
}

pub struct ServerFileBrowserView {
    host_id: Option<HostId>,
    session_id: Option<SessionId>,
    /// Fallback session for executing remote commands when the
    /// remote server daemon is not yet installed / connected.
    session: Option<Arc<Session>>,
    current_path: String,
    path_editor: ViewHandle<EditorView>,
    rename_editor: ViewHandle<EditorView>,
    pending_rename_index: Option<usize>,
    pending_rename_path_after_reload: Option<String>,
    entries: Vec<ServerFileBrowserEntry>,
    expanded_directories: HashSet<String>,
    loaded_directories: HashMap<String, Vec<ServerFileBrowserEntry>>,
    selected_index: Option<usize>,
    list_state: UniformListState,
    scroll_state: ScrollStateHandle,
    loading: bool,
    status: Option<String>,
    refresh_button: MouseStateHandle,
    upload_file_button: MouseStateHandle,
    upload_folder_button: MouseStateHandle,
    row_states: HashMap<String, MouseStateHandle>,
    context_menu: ViewHandle<Menu<ServerFileBrowserAction>>,
    context_menu_position: Option<Vector2F>,
    upload_batches: Vec<ServerFileUploadBatch>,
    active_upload_batch_index: Option<usize>,
    /// Only one upload pipeline (staging + upload + promote) runs at a time.
    upload_pipeline_claimed: bool,
    pending_upload_starts: VecDeque<PendingUploadStart>,
    upload_progress_panel_open: bool,
    upload_progress_button: MouseStateHandle,
    clear_completed_uploads_button: MouseStateHandle,
}

impl ServerFileBrowserView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let context_menu = ctx.add_typed_action_view(|_| {
            Menu::new()
                .prevent_interaction_with_other_elements()
                .with_drop_shadow()
                .with_width(CONTEXT_MENU_WIDTH)
                .with_safe_triangle()
                .with_ignore_hover_when_covered()
        });
        ctx.subscribe_to_view(&context_menu, |me, _, event, ctx| {
            me.handle_menu_event(event, ctx);
        });

        let path_editor = ctx.add_typed_action_view(|ctx| {
            let appearance = crate::appearance::Appearance::as_ref(ctx);
            let mut editor = EditorView::single_line(
                SingleLineEditorOptions {
                    text: TextOptions::ui_text(Some(ITEM_FONT_SIZE), appearance),
                    select_all_on_focus: true,
                    clear_selections_on_blur: true,
                    propagate_and_no_op_vertical_navigation_keys:
                        PropagateAndNoOpNavigationKeys::Always,
                    propagate_horizontal_navigation_keys: PropagateHorizontalNavigationKeys::Always,
                    ..Default::default()
                },
                ctx,
            );
            editor.set_placeholder_text(crate::t!("server-file-browser-path-placeholder"), ctx);
            editor
        });

        ctx.subscribe_to_view(&path_editor, |me, _, event, ctx| match event {
            EditorEvent::Enter => me.jump_to_editor_path(ctx),
            EditorEvent::Escape => me.sync_editor_to_current_path(ctx),
            _ => {}
        });

        let rename_editor = ctx.add_typed_action_view(|ctx| {
            let appearance = crate::appearance::Appearance::as_ref(ctx);
            let mut editor = EditorView::single_line(
                SingleLineEditorOptions {
                    text: TextOptions::ui_text(Some(ITEM_FONT_SIZE), appearance),
                    select_all_on_focus: true,
                    clear_selections_on_blur: false,
                    propagate_and_no_op_vertical_navigation_keys:
                        PropagateAndNoOpNavigationKeys::Always,
                    propagate_horizontal_navigation_keys: PropagateHorizontalNavigationKeys::Always,
                    ..Default::default()
                },
                ctx,
            );
            editor.set_placeholder_text(crate::t!("server-file-browser-menu-rename"), ctx);
            editor
        });

        ctx.subscribe_to_view(&rename_editor, |me, _, event, ctx| match event {
            EditorEvent::Enter => me.commit_rename(ctx),
            EditorEvent::Escape => me.cancel_rename(ctx),
            EditorEvent::Blurred => me.commit_rename(ctx),
            _ => {}
        });

        Self {
            host_id: None,
            session_id: None,
            session: None,
            current_path: String::new(),
            path_editor,
            rename_editor,
            pending_rename_index: None,
            pending_rename_path_after_reload: None,
            entries: Vec::new(),
            expanded_directories: HashSet::new(),
            loaded_directories: HashMap::new(),
            selected_index: None,
            list_state: UniformListState::new(),
            scroll_state: ScrollStateHandle::default(),
            loading: false,
            status: Some(crate::t!("server-file-browser-empty")),
            refresh_button: Default::default(),
            upload_file_button: Default::default(),
            upload_folder_button: Default::default(),
            row_states: HashMap::new(),
            context_menu,
            context_menu_position: None,
            upload_batches: Vec::new(),
            active_upload_batch_index: None,
            upload_pipeline_claimed: false,
            pending_upload_starts: VecDeque::new(),
            upload_progress_panel_open: false,
            upload_progress_button: Default::default(),
            clear_completed_uploads_button: Default::default(),
        }
    }

    pub fn set_remote_root(
        &mut self,
        host_id: HostId,
        path: String,
        session_id: Option<SessionId>,
        session: Option<Arc<Session>>,
        ctx: &mut ViewContext<Self>,
    ) {
        let session_changed = match (&self.session, &session) {
            (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
            (None, None) => false,
            _ => true,
        };
        let session_id_changed = self.session_id != session_id;
        let should_load = self.host_id.as_ref() != Some(&host_id)
            || self.current_path != path
            || session_changed
            || session_id_changed;
        self.host_id = Some(host_id);
        self.session_id = session_id;
        self.session = session;
        if should_load {
            self.current_path = path;
            self.sync_editor_to_current_path(ctx);
            self.load_current_directory(ctx);
        }
    }

    pub fn on_left_panel_focused(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus_self();
        if self.selected_index.is_none() && !self.entries.is_empty() {
            self.selected_index = Some(0);
        }
        ctx.notify();
    }

    fn sync_editor_to_current_path(&mut self, ctx: &mut ViewContext<Self>) {
        self.path_editor.update(ctx, |editor, ctx| {
            editor.set_buffer_text(&self.current_path, ctx);
        });
    }

    fn jump_to_editor_path(&mut self, ctx: &mut ViewContext<Self>) {
        let path = self.path_editor.as_ref(ctx).buffer_text(ctx).trim().to_string();
        if path.is_empty() {
            return;
        }
        self.resolve_and_open(path, ctx);
    }

    fn client(&self, ctx: &AppContext) -> Option<Arc<RemoteServerClient>> {
        let host_id = self.host_id.as_ref()?;
        RemoteServerManager::as_ref(ctx).client_for_host(host_id).cloned()
    }

    fn remote_session_id(&self, ctx: &AppContext) -> Option<SessionId> {
        let manager = RemoteServerManager::as_ref(ctx);
        bound_remote_session_id(self.session_id, |session_id| {
            manager.client_for_session(session_id).is_some()
        })
    }

    fn set_error(&mut self, message: impl Into<String>, ctx: &mut ViewContext<Self>) {
        self.loading = false;
        self.status = Some(message.into());
        ctx.notify();
    }

    fn load_current_directory(&mut self, ctx: &mut ViewContext<Self>) {
        self.reload_directory(ctx, true);
    }

    /// Reloads the current directory listing. When `reset_tree` is false, keeps
    /// expanded folders and scroll position (used after upload/rename/delete).
    fn reload_directory(&mut self, ctx: &mut ViewContext<Self>, reset_tree: bool) {
        let path = if self.current_path.is_empty() {
            "~".to_string()
        } else {
            self.current_path.clone()
        };

        let expanded_directories = if reset_tree {
            HashSet::new()
        } else {
            self.expanded_directories.clone()
        };
        let depth_by_path: HashMap<String, usize> = expanded_directories
            .iter()
            .map(|directory_path| {
                (
                    directory_path.clone(),
                    self.depth_for_directory(directory_path),
                )
            })
            .collect();
        let selected_path = self
            .selected_index
            .and_then(|index| self.entries.get(index))
            .map(|entry| entry.path.clone());

        if let Some(client) = self.client(ctx) {
            self.loading = true;
            if reset_tree {
                self.status = None;
            }
            ctx.notify();
            ctx.spawn(
                async move {
                    reload_directory_tree(
                        DirectoryListingSource::Client(client),
                        path,
                        expanded_directories,
                        depth_by_path,
                    )
                    .await
                },
                move |me, result, ctx| {
                    me.finish_directory_reload(result, selected_path, reset_tree, ctx);
                },
            );
        } else if let Some(session) = self.session.clone() {
            self.loading = true;
            if reset_tree {
                self.status = None;
            }
            ctx.notify();
            ctx.spawn(
                async move {
                    reload_directory_tree(
                        DirectoryListingSource::Session(session),
                        path,
                        expanded_directories,
                        depth_by_path,
                    )
                    .await
                },
                move |me, result, ctx| {
                    me.finish_directory_reload(result, selected_path, reset_tree, ctx);
                },
            );
        } else {
            self.set_error(crate::t!("server-file-browser-no-session"), ctx);
        }
    }

    fn refresh_directory_tree(&mut self, ctx: &mut ViewContext<Self>) {
        self.reload_directory(ctx, false);
    }

    fn depth_for_directory(&self, path: &str) -> usize {
        if self.directory_listing_matches_current(path) {
            return 0;
        }
        self.entries
            .iter()
            .find(|entry| entry.path == path)
            .map(|entry| entry.depth + 1)
            .unwrap_or(1)
    }

    fn finish_directory_reload(
        &mut self,
        result: Result<DirectoryTreeReload, String>,
        selected_path: Option<String>,
        reset_tree: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        self.loading = false;
        match result {
            Ok(reloaded) => {
                self.current_path = reloaded.current_path;
                self.sync_editor_to_current_path(ctx);
                if reset_tree {
                    self.reset_tree_state();
                } else {
                    self.expanded_directories = reloaded.expanded_directories;
                }
                self.entries = reloaded.root_entries;
                self.loaded_directories = reloaded.loaded_directories;
                self.rebuild_entries();
                self.selected_index = selected_index_after_rebuild(
                    &self.entries,
                    selected_path.as_deref(),
                    self.selected_index,
                );
                self.sync_row_states();
                if let Some(path) = self.pending_rename_path_after_reload.take() {
                    if let Some(index) = self.entries.iter().position(|entry| entry.path == path) {
                        self.start_rename(index, ctx);
                    }
                }
            }
            Err(error) => {
                self.pending_rename_path_after_reload = None;
                self.status = Some(error);
            }
        }
        ctx.notify();
    }

    fn resolve_and_open(&mut self, path: String, ctx: &mut ViewContext<Self>) {
        let host_id = self.host_id.clone();
        let path_for_spawn = path.clone();

        if let Some(client) = self.client(ctx) {
            self.loading = true;
            self.status = None;
            ctx.notify();
            ctx.spawn(
                async move { resolve_path(client, path_for_spawn).await },
                move |me, result, ctx| {
                    me.finish_resolve_and_open(result, host_id, ctx);
                },
            );
        } else if let Some(session) = self.session.clone() {
            self.loading = true;
            self.status = None;
            ctx.notify();
            ctx.spawn(
                async move { resolve_path_via_session(session, path_for_spawn).await },
                move |me, result, ctx| {
                    me.finish_resolve_and_open(result, host_id, ctx);
                },
            );
        } else {
            self.set_error(crate::t!("server-file-browser-no-session"), ctx);
        }
    }

    fn finish_resolve_and_open(
        &mut self,
        result: Result<ResolvedRemotePath, String>,
        host_id: Option<HostId>,
        ctx: &mut ViewContext<Self>,
    ) {
        self.loading = false;
        match result {
            Ok(resolved) if resolved.kind == FileSystemEntryKind::Directory => {
                self.current_path = resolved.canonical_path;
                self.sync_editor_to_current_path(ctx);
                self.load_current_directory(ctx);
            }
            Ok(resolved) if resolved.kind == FileSystemEntryKind::File => {
                if let (Some(host_id), Ok(path)) = (
                    host_id.clone(),
                    StandardizedPath::try_new(&resolved.canonical_path),
                ) {
                    ctx.emit(ServerFileBrowserEvent::OpenRemoteFile {
                        remote_path: RemotePath::new(host_id, path),
                    });
                }
                if let Some(parent) = remote_parent(&resolved.canonical_path) {
                    self.current_path = parent;
                    self.sync_editor_to_current_path(ctx);
                    self.load_current_directory(ctx);
                }
            }
            Ok(_) => {
                self.status = Some(crate::t!("server-file-browser-unsupported-path"));
            }
            Err(error) => {
                self.status = Some(error);
            }
        }
        ctx.notify();
    }

    fn reset_tree_state(&mut self) {
        self.expanded_directories.clear();
        self.loaded_directories.clear();
        self.selected_index = None;
        self.list_state = UniformListState::new();
        self.scroll_state = ScrollStateHandle::default();
        self.context_menu_position = None;
        self.row_states.clear();
    }

    fn sync_row_states(&mut self) {
        let active_paths: HashSet<String> = self.entries.iter().map(|entry| entry.path.clone()).collect();
        for path in &active_paths {
            self.row_states.entry(path.clone()).or_default();
        }
        self.row_states.retain(|path, _| active_paths.contains(path));
    }

    fn toggle_directory(&mut self, path: String, ctx: &mut ViewContext<Self>) {
        if self.expanded_directories.remove(&path) {
            self.rebuild_entries();
            ctx.notify();
            return;
        }

        let child_depth = self
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .map(|entry| entry.depth + 1)
            .unwrap_or(1);
        self.expanded_directories.insert(path.clone());
        if self.loaded_directories.contains_key(&path) {
            self.rebuild_entries();
            ctx.notify();
            return;
        }

        let path_for_spawn = path.clone();
        if let Some(client) = self.client(ctx) {
            self.loading = true;
            ctx.notify();
            ctx.spawn(
                async move { list_directory(client, path_for_spawn).await },
                move |me, result, ctx| {
                    me.loading = false;
                    match result {
                        Ok((path, entries)) => {
                            let entries = entries_with_depth(entries, child_depth);
                            me.loaded_directories.insert(path, entries);
                            me.rebuild_entries();
                        }
                        Err(error) => me.status = Some(error),
                    }
                    ctx.notify();
                },
            );
        } else if let Some(session) = self.session.clone() {
            self.loading = true;
            ctx.notify();
            ctx.spawn(
                async move { list_directory_via_session(session, path_for_spawn).await },
                move |me, result, ctx| {
                    me.loading = false;
                    match result {
                        Ok((path, entries)) => {
                            let entries = entries_with_depth(entries, child_depth);
                            me.loaded_directories.insert(path, entries);
                            me.rebuild_entries();
                        }
                        Err(error) => me.status = Some(error),
                    }
                    ctx.notify();
                },
            );
        } else {
            self.set_error(crate::t!("server-file-browser-no-session"), ctx);
        }
    }

    fn rebuild_entries(&mut self) {
        let selected_path = self
            .selected_index
            .and_then(|index| self.entries.get(index))
            .map(|entry| entry.path.clone());
        let roots = self.entries.iter().filter(|entry| entry.depth == 0).cloned().collect();
        self.entries =
            rebuild_entries_from(roots, &self.expanded_directories, &self.loaded_directories);
        self.selected_index =
            selected_index_after_rebuild(&self.entries, selected_path.as_deref(), self.selected_index);
        self.sync_row_states();
    }

    fn select_index(&mut self, index: usize, ctx: &mut ViewContext<Self>) {
        if index < self.entries.len() {
            self.selected_index = Some(index);
            ctx.notify();
        }
    }

    fn click_entry(&mut self, index: usize, ctx: &mut ViewContext<Self>) {
        if self.pending_rename_index.is_some() {
            self.commit_rename(ctx);
        }
        let Some(entry) = self.entries.get(index).cloned() else {
            return;
        };
        self.selected_index = Some(index);
        if entry.kind == FileSystemEntryKind::Directory {
            self.toggle_directory(entry.path, ctx);
        } else {
            ctx.notify();
        }
    }

    fn open_index(&mut self, index: usize, ctx: &mut ViewContext<Self>) {
        let Some(entry) = self.entries.get(index).cloned() else {
            return;
        };
        self.selected_index = Some(index);
        match entry.kind {
            FileSystemEntryKind::Directory => self.toggle_directory(entry.path, ctx),
            FileSystemEntryKind::File => {
                if let (Some(host_id), Ok(path)) = (
                    self.host_id.clone(),
                    StandardizedPath::try_new(entry.path.as_str()),
                ) {
                    ctx.emit(ServerFileBrowserEvent::OpenRemoteFile {
                        remote_path: RemotePath::new(host_id, path),
                    });
                }
                ctx.notify();
            }
            FileSystemEntryKind::Symlink
            | FileSystemEntryKind::Other
            | FileSystemEntryKind::Unspecified => {
                self.resolve_and_open(entry.path, ctx);
            }
        }
    }

    fn open_context_menu(&mut self, index: usize, position: Vector2F, ctx: &mut ViewContext<Self>) {
        let Some(entry) = self.entries.get(index).cloned() else {
            return;
        };
        self.selected_index = Some(index);
        self.context_menu_position = Some(position);
        let menu_items = self.context_menu_items(index, &entry, ctx);
        let menu_origin = ctx
            .element_position_by_id(CONTEXT_MENU_POSITION_ID)
            .map(|bounds| bounds.origin() + position);
        self.context_menu.update(ctx, move |menu, menu_ctx| {
            menu.set_origin(menu_origin);
            menu.set_items(menu_items, menu_ctx);
            menu_ctx.notify();
        });
        ctx.focus(&self.context_menu);
        ctx.notify();
    }

    fn dismiss_context_menu(&mut self, ctx: &mut ViewContext<Self>) {
        let mut empty_items: Vec<MenuItem<ServerFileBrowserAction>> = Vec::new();
        clear_context_menu_state(&mut self.context_menu_position, &mut empty_items);
        self.context_menu.update(ctx, |menu, ctx| {
            menu.set_safe_zone_target(None);
            menu.set_submenu_being_shown_for_item_index(None);
            menu.set_items(Vec::new(), ctx);
        });
        ctx.notify();
    }

    fn handle_menu_event(&mut self, event: &MenuEvent, ctx: &mut ViewContext<Self>) {
        match event {
            MenuEvent::ItemHovered | MenuEvent::ItemSelected => {
                self.update_context_menu_safe_triangle(ctx);
            }
            MenuEvent::Close { .. } => {
                let mut empty_items: Vec<MenuItem<ServerFileBrowserAction>> = Vec::new();
                clear_context_menu_state(&mut self.context_menu_position, &mut empty_items);
                self.context_menu.update(ctx, |menu, _| {
                    menu.set_safe_zone_target(None);
                    menu.set_submenu_being_shown_for_item_index(None);
                });
            }
        }
        ctx.notify();
    }

    fn update_context_menu_safe_triangle(&mut self, ctx: &mut ViewContext<Self>) {
        let window_id = ctx.window_id();
        let submenu_parent = self.context_menu.read(ctx, |menu, _| {
            let index = menu.selected_index()?;
            match menu.items().get(index)? {
                MenuItem::Submenu { .. } => Some((index, menu.submenu_row_save_position_id(index))),
                MenuItem::Item(_) => None,
                MenuItem::Separator => None,
                MenuItem::ItemsRow { .. } => None,
                MenuItem::Header { .. } => None,
            }
        });

        let Some((parent_index, anchor_id)) = submenu_parent else {
            self.context_menu.update(ctx, |menu, _| {
                menu.set_safe_zone_target(None);
                menu.set_submenu_being_shown_for_item_index(None);
            });
            return;
        };

        let submenu_height = self.context_menu.read(ctx, |menu, _| {
            let MenuItem::Submenu { menu: submenu, .. } = menu.items().get(parent_index)? else {
                return None;
            };
            let row_count = submenu
                .items()
                .iter()
                .filter(|item| matches!(item, MenuItem::Item(_)))
                .count();
            let row_height = MENU_ITEM_VERTICAL_PADDING * 2.0 + ITEM_FONT_SIZE;
            Some(row_count as f32 * row_height + 18.0)
        });
        let submenu_rect = ctx
            .element_position_by_id_at_last_frame(window_id, &anchor_id)
            .map(|anchor_rect| {
                let height = submenu_height
                    .unwrap_or(anchor_rect.height())
                    .max(anchor_rect.height());
                RectF::new(
                    vec2f(anchor_rect.max_x() - SUBMENU_OVERLAP, anchor_rect.min_y()),
                    vec2f(CONTEXT_MENU_WIDTH + SUBMENU_OVERLAP, height),
                )
            });
        self.context_menu.update(ctx, |menu, _| {
            menu.set_safe_zone_target(submenu_rect);
            menu.set_submenu_being_shown_for_item_index(Some(parent_index));
        });
    }

    fn copy_path(&mut self, path: String, ctx: &mut ViewContext<Self>) {
        ctx.clipboard()
            .write(ClipboardContent::plain_text(path.clone()));
        self.status = Some(crate::t!("server-file-browser-copied-path"));
        self.dismiss_context_menu(ctx);
    }

    fn copy_name(&mut self, name: String, ctx: &mut ViewContext<Self>) {
        ctx.clipboard()
            .write(ClipboardContent::plain_text(name.clone()));
        self.status = Some(crate::t!("server-file-browser-copied-name"));
        self.dismiss_context_menu(ctx);
    }

    fn select_previous_item(&mut self, ctx: &mut ViewContext<Self>) {
        self.selected_index = previous_index(self.selected_index, self.entries.len());
        ctx.notify();
    }

    fn select_next_item(&mut self, ctx: &mut ViewContext<Self>) {
        self.selected_index = next_index(self.selected_index, self.entries.len());
        ctx.notify();
    }

    fn expand_selected_item(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(entry) = self
            .selected_index
            .and_then(|index| self.entries.get(index))
            .cloned()
        else {
            return;
        };
        if entry.kind == FileSystemEntryKind::Directory
            && !self.expanded_directories.contains(&entry.path)
        {
            self.toggle_directory(entry.path, ctx);
        }
    }

    fn collapse_selected_item(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(entry) = self
            .selected_index
            .and_then(|index| self.entries.get(index))
            .cloned()
        else {
            return;
        };
        if entry.kind == FileSystemEntryKind::Directory
            && self.expanded_directories.contains(&entry.path)
        {
            self.toggle_directory(entry.path, ctx);
        }
    }

    fn execute_selected_item(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(index) = self.selected_index {
            self.open_index(index, ctx);
        }
    }

    fn context_menu_items(
        &self,
        index: usize,
        target: &ServerFileBrowserEntry,
        ctx: &AppContext,
    ) -> Vec<MenuItem<ServerFileBrowserAction>> {
        let target_is_directory = target.kind == FileSystemEntryKind::Directory;
        let upload_target = if target_is_directory {
            target.path.clone()
        } else {
            remote_parent(&target.path).unwrap_or_else(|| self.current_path.clone())
        };
        let cd_target = if target_is_directory {
            target.path.clone()
        } else {
            remote_parent(&target.path).unwrap_or_else(|| self.current_path.clone())
        };
        let delete_color = Appearance::as_ref(ctx).theme().ansi_fg_red();

        vec![
            MenuItemFields::new(crate::t!("server-file-browser-menu-refresh"))
                .with_icon(Icon::Refresh)
                .with_on_select_action(ServerFileBrowserAction::Refresh)
                .into_item(),
            MenuItem::Separator,
            context_menu_submenu(
                crate::t!("server-file-browser-menu-upload"),
                Icon::UploadCloud,
                vec![
                    MenuItemFields::new(crate::t!("server-file-browser-menu-upload-file"))
                        .with_icon(Icon::UploadCloud)
                        .with_on_select_action(ServerFileBrowserAction::UploadFiles(
                            upload_target.clone(),
                        ))
                        .into_item(),
                    MenuItemFields::new(crate::t!("server-file-browser-menu-upload-folder"))
                        .with_icon(Icon::Folder)
                        .with_on_select_action(ServerFileBrowserAction::UploadFolder(
                            upload_target.clone(),
                        ))
                        .into_item(),
                ],
            ),
            context_menu_submenu(
                crate::t!("server-file-browser-menu-new"),
                Icon::Plus,
                vec![
                    MenuItemFields::new(crate::t!("server-file-browser-menu-new-file"))
                        .with_icon(Icon::File)
                        .with_on_select_action(ServerFileBrowserAction::CreateFile(
                            upload_target.clone(),
                        ))
                        .into_item(),
                    MenuItemFields::new(crate::t!("server-file-browser-menu-new-folder"))
                        .with_icon(Icon::Folder)
                        .with_on_select_action(ServerFileBrowserAction::CreateFolder(
                            upload_target,
                        ))
                        .into_item(),
                ],
            ),
            MenuItem::Separator,
            MenuItemFields::new(crate::t!("server-file-browser-menu-download"))
                .with_icon(Icon::Download)
                .with_on_select_action(ServerFileBrowserAction::Download(target.path.clone()))
                .into_item(),
            MenuItemFields::new(crate::t!("server-file-browser-menu-copy-path"))
                .with_icon(Icon::Copy)
                .with_on_select_action(ServerFileBrowserAction::CopyPath(target.path.clone()))
                .into_item(),
            MenuItem::Separator,
            context_menu_submenu(
                crate::t!("server-file-browser-menu-terminal"),
                Icon::Terminal,
                vec![MenuItemFields::new(crate::t!(
                    "server-file-browser-menu-cd-to-terminal"
                ))
                .with_icon(Icon::Terminal)
                .with_on_select_action(ServerFileBrowserAction::CdToTerminal(cd_target))
                .into_item()],
            ),
            context_menu_submenu(
                crate::t!("server-file-browser-menu-other"),
                Icon::DotsHorizontal,
                vec![
                    MenuItemFields::new(crate::t!("server-file-browser-menu-rename"))
                        .with_icon(Icon::Rename)
                        .with_on_select_action(ServerFileBrowserAction::RenameEntry(index))
                        .into_item(),
                    MenuItemFields::new(crate::t!("server-file-browser-menu-copy-filename"))
                        .with_icon(Icon::Copy)
                        .with_on_select_action(ServerFileBrowserAction::CopyName(
                            target.name.clone(),
                        ))
                        .into_item(),
                ],
            ),
            MenuItem::Separator,
            MenuItemFields::new(crate::t!("server-file-browser-menu-delete"))
                .with_icon(Icon::Trash)
                .with_override_icon_color(delete_color.into())
                .with_override_text_color(delete_color)
                .with_on_select_action(ServerFileBrowserAction::DeleteEntry(index))
                .into_item(),
        ]
    }

    fn create_new_entry(
        &mut self,
        remote_directory: String,
        kind: NewRemoteEntryKind,
        ctx: &mut ViewContext<Self>,
    ) {
        self.dismiss_context_menu(ctx);
        let Some(client) = self.client(ctx) else {
            self.status = Some(crate::t!("server-file-browser-create-requires-session"));
            ctx.notify();
            return;
        };

        self.loading = true;
        self.status = None;
        ctx.notify();
        ctx.spawn(
            async move { create_remote_entry(client, remote_directory, kind).await },
            move |me, result, ctx| {
                me.loading = false;
                match result {
                    Ok(entry) => {
                        me.status = Some(match kind {
                            NewRemoteEntryKind::File => {
                                crate::t!("server-file-browser-created-file")
                            }
                            NewRemoteEntryKind::Directory => {
                                crate::t!("server-file-browser-created-folder")
                            }
                        });
                        me.pending_rename_path_after_reload = Some(entry.path);
                        me.reload_directory(ctx, false);
                    }
                    Err(error) => {
                        me.status = Some(crate::t!("server-file-browser-operation-failed", error = error));
                        ctx.notify();
                    }
                }
            },
        );
    }

    fn start_rename(&mut self, index: usize, ctx: &mut ViewContext<Self>) {
        let Some(entry) = self.entries.get(index).cloned() else {
            return;
        };
        self.dismiss_context_menu(ctx);
        self.pending_rename_index = Some(index);
        self.selected_index = Some(index);
        self.rename_editor.update(ctx, |editor, ctx| {
            editor.set_buffer_text(&entry.name, ctx);
        });
        ctx.focus(&self.rename_editor);
        ctx.notify();
    }

    fn cancel_rename(&mut self, ctx: &mut ViewContext<Self>) {
        if self.pending_rename_index.is_none() {
            return;
        }
        self.pending_rename_index = None;
        self.rename_editor.update(ctx, |editor, ctx| {
            editor.clear_buffer(ctx);
        });
        ctx.focus_self();
        ctx.notify();
    }

    fn commit_rename(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(index) = self.pending_rename_index.take() else {
            return;
        };
        let Some(entry) = self.entries.get(index).cloned() else {
            return;
        };
        let new_name = self.rename_editor.as_ref(ctx).buffer_text(ctx).trim().to_string();
        self.rename_editor.update(ctx, |editor, ctx| {
            editor.clear_buffer(ctx);
        });
        if new_name.is_empty() {
            self.status = Some(crate::t!("server-file-browser-rename-empty"));
            ctx.focus_self();
            ctx.notify();
            return;
        }
        if new_name == entry.name {
            self.status = Some(crate::t!("server-file-browser-rename-unchanged"));
            ctx.focus_self();
            ctx.notify();
            return;
        }
        if new_name.contains('/') {
            self.status = Some(crate::t!("server-file-browser-rename-invalid-name"));
            ctx.focus_self();
            ctx.notify();
            return;
        }

        let session = self.session.clone();
        let client = self.client(ctx);
        let remote_session_id = self.remote_session_id(ctx);
        let can_rename = session.is_some()
            || (client.is_some() && remote_session_id.is_some());
        if !can_rename {
            self.status = Some(crate::t!("server-file-browser-rename-requires-session"));
            ctx.focus_self();
            ctx.notify();
            return;
        }

        let from_path = entry.path.clone();
        let is_directory = entry.kind == FileSystemEntryKind::Directory;
        let from_path_for_rename = from_path.clone();
        let new_name_for_rename = new_name.clone();
        self.loading = true;
        self.status = None;
        ctx.notify();
        ctx.spawn(
            async move {
                rename_remote_path(
                    session,
                    client,
                    remote_session_id,
                    from_path_for_rename,
                    new_name_for_rename,
                )
                .await
            },
            move |me, result, ctx| {
                me.loading = false;
                match result {
                    Ok(()) => {
                        let new_path = remote_parent(&from_path)
                            .map(|parent| join_remote_path(&parent, &new_name))
                            .unwrap_or_else(|| from_path.clone());
                        me.apply_rename_to_local_tree_state(
                            &from_path,
                            &new_path,
                            &new_name,
                            is_directory,
                        );
                        me.rebuild_entries();
                        me.status = Some(crate::t!("server-file-browser-renamed"));
                        ctx.notify();
                    }
                    Err(error) => {
                        me.status = Some(crate::t!("server-file-browser-operation-failed", error = error));
                        ctx.notify();
                    }
                }
            },
        );
        ctx.focus_self();
    }

    /// Updates cached tree paths after `mv` so we do not re-list every expanded folder
    /// (each `ListDirectory` stats every child on the remote host).
    fn apply_rename_to_local_tree_state(
        &mut self,
        from_path: &str,
        new_path: &str,
        new_name: &str,
        is_directory: bool,
    ) {
        remap_loaded_directories_after_rename(
            &mut self.loaded_directories,
            from_path,
            new_path,
            new_name,
            is_directory,
        );

        if is_directory {
            self.expanded_directories = self
                .expanded_directories
                .iter()
                .map(|path| remap_path_after_rename(path, from_path, new_path))
                .collect();
        }

        for entry in &mut self.entries {
            if entry.path == from_path {
                entry.path = new_path.to_string();
                entry.name = new_name.to_string();
            } else if is_directory {
                entry.path = remap_path_after_rename(&entry.path, from_path, new_path);
            }
        }
    }

    fn confirm_delete(&mut self, index: usize, ctx: &mut ViewContext<Self>) {
        let Some(entry) = self.entries.get(index).cloned() else {
            return;
        };
        self.dismiss_context_menu(ctx);
        self.selected_index = Some(index);

        let is_directory = entry.kind == FileSystemEntryKind::Directory;
        let info = if is_directory {
            crate::t!("server-file-browser-delete-info-directory")
        } else {
            crate::t!("server-file-browser-delete-info-file")
        };
        let path = entry.path.clone();
        let client = self.client(ctx);
        let session = self.session.clone();
        let remote_session_id = self.remote_session_id(ctx);

        let dialog = AlertDialogWithCallbacks::for_view(
            crate::t!("server-file-browser-delete-title", name = entry.name),
            info,
            vec![
                ModalButton::for_view(
                    crate::t!("common-delete"),
                    move |me: &mut ServerFileBrowserView, ctx| {
                        me.delete_entry_confirmed(
                            path.clone(),
                            is_directory,
                            client.clone(),
                            session.clone(),
                            remote_session_id,
                            ctx,
                        );
                    },
                ),
                ModalButton::for_view(crate::t!("common-cancel"), |_: &mut ServerFileBrowserView, _| {}),
            ],
            |_, _| {},
        );
        ctx.show_native_platform_modal(dialog);
    }

    fn delete_entry_confirmed(
        &mut self,
        path: String,
        is_directory: bool,
        client: Option<Arc<RemoteServerClient>>,
        session: Option<Arc<Session>>,
        remote_session_id: Option<SessionId>,
        ctx: &mut ViewContext<Self>,
    ) {
        self.loading = true;
        self.status = None;
        ctx.notify();
        let path_for_cleanup = path.clone();
        ctx.spawn(
            async move {
                delete_remote_path(client, session, remote_session_id, path, is_directory).await
            },
            move |me, result, ctx| {
                me.loading = false;
                match result {
                    Ok(()) => {
                        me.remove_deleted_entry(&path_for_cleanup);
                        me.status = Some(crate::t!("server-file-browser-deleted"));
                        ctx.notify();
                    }
                    Err(error) => {
                        me.status = Some(crate::t!("server-file-browser-operation-failed", error = error));
                        ctx.notify();
                    }
                }
            },
        );
    }

    fn remove_path_from_tree_state(&mut self, path: &str) {
        let child_prefix = child_path_prefix(path);
        self.expanded_directories.retain(|key| {
            key != path && child_prefix.as_ref().is_none_or(|prefix| !key.starts_with(prefix))
        });
        self.loaded_directories.retain(|key, _| {
            key != path && child_prefix.as_ref().is_none_or(|prefix| !key.starts_with(prefix))
        });
        self.row_states.retain(|key, _| {
            key != path && child_prefix.as_ref().is_none_or(|prefix| !key.starts_with(prefix))
        });
    }

    fn remove_deleted_entry(&mut self, path: &str) {
        let child_prefix = child_path_prefix(path);
        self.remove_path_from_tree_state(path);
        if let Some(parent) = remote_parent(path) {
            if let Some(children) = self.loaded_directories.get_mut(&parent) {
                children.retain(|entry| {
                    entry.path != path
                        && child_prefix
                            .as_ref()
                            .is_none_or(|prefix| !entry.path.starts_with(prefix))
                });
            }
        }
        self.entries.retain(|entry| {
            entry.path != path
                && child_prefix
                    .as_ref()
                    .is_none_or(|prefix| !entry.path.starts_with(prefix))
        });
        self.rebuild_entries();
        self.sync_row_states();
    }

    fn depth_for_directory_path(&self, directory_path: &str) -> usize {
        if self.directory_listing_matches_current(directory_path) {
            return 0;
        }
        self.entries
            .iter()
            .find(|entry| entry.path == directory_path)
            .map(|entry| entry.depth + 1)
            .unwrap_or(1)
    }

    fn directories_to_refresh_for_paths(&self, changed_paths: &[String]) -> HashSet<String> {
        let mut directories = HashSet::new();
        for changed_path in changed_paths {
            let mut parent = remote_parent(changed_path);
            while let Some(directory) = parent {
                let is_current = self.directory_listing_matches_current(&directory);
                if is_current || self.expanded_directories.contains(&directory) {
                    directories.insert(directory.clone());
                }
                if is_current {
                    break;
                }
                parent = remote_parent(&directory);
            }
        }
        directories
    }

    fn directory_listing_matches_current(&self, directory_path: &str) -> bool {
        let current = self.current_path.trim_end_matches('/');
        let directory = directory_path.trim_end_matches('/');
        if current.is_empty() {
            directory == "~" || directory.is_empty()
        } else {
            current == directory
        }
    }

    fn collect_directories_to_refresh_for_completed_uploads(&self) -> HashSet<String> {
        let Some(batch) = self.active_upload_batch() else {
            return HashSet::new();
        };
        let final_paths: Vec<String> = batch
            .tasks
            .iter()
            .filter(|task| matches!(task.status, UploadTaskStatus::Completed))
            .map(|task| task.final_remote_path.clone())
            .collect();
        self.directories_to_refresh_for_paths(&final_paths)
    }

    fn apply_directory_listing_update(
        &mut self,
        canonical_path: String,
        children: Vec<ServerFileBrowserEntry>,
        depth: usize,
    ) {
        let depth = if self.directory_listing_matches_current(&canonical_path) {
            0
        } else {
            depth
        };
        let children = entries_with_depth(children, depth);
        if self.directory_listing_matches_current(&canonical_path) {
            self.entries = children;
        } else {
            self.loaded_directories.insert(canonical_path, children);
        }
    }

    fn reload_directories_selective(
        &mut self,
        directories: HashSet<String>,
        ctx: &mut ViewContext<Self>,
    ) {
        if directories.is_empty() {
            ctx.notify();
            return;
        }

        let depth_by_path: HashMap<String, usize> = directories
            .iter()
            .map(|directory| (directory.clone(), self.depth_for_directory_path(directory)))
            .collect();
        let selected_path = self
            .selected_index
            .and_then(|index| self.entries.get(index))
            .map(|entry| entry.path.clone());

        if let Some(client) = self.client(ctx) {
            self.loading = true;
            ctx.notify();
            ctx.spawn(
                async move {
                    fetch_directory_listings_selective(
                        DirectoryListingSource::Client(client),
                        directories,
                        depth_by_path,
                    )
                    .await
                },
                move |me, result, ctx| {
                    me.loading = false;
                    match result {
                        Ok(updates) => {
                            for (canonical_path, children, depth) in updates {
                                me.apply_directory_listing_update(canonical_path, children, depth);
                            }
                            me.rebuild_entries();
                            me.selected_index = selected_index_after_rebuild(
                                &me.entries,
                                selected_path.as_deref(),
                                me.selected_index,
                            );
                            me.sync_row_states();
                        }
                        Err(error) => me.status = Some(error),
                    }
                    ctx.notify();
                },
            );
        } else if let Some(session) = self.session.clone() {
            self.loading = true;
            ctx.notify();
            ctx.spawn(
                async move {
                    fetch_directory_listings_selective(
                        DirectoryListingSource::Session(session),
                        directories,
                        depth_by_path,
                    )
                    .await
                },
                move |me, result, ctx| {
                    me.loading = false;
                    match result {
                        Ok(updates) => {
                            for (canonical_path, children, depth) in updates {
                                me.apply_directory_listing_update(canonical_path, children, depth);
                            }
                            me.rebuild_entries();
                            me.selected_index = selected_index_after_rebuild(
                                &me.entries,
                                selected_path.as_deref(),
                                me.selected_index,
                            );
                            me.sync_row_states();
                        }
                        Err(error) => me.status = Some(error),
                    }
                    ctx.notify();
                },
            );
        } else {
            ctx.notify();
        }
    }

    fn active_upload_batch(&self) -> Option<&ServerFileUploadBatch> {
        self.active_upload_batch_index
            .and_then(|index| self.upload_batches.get(index))
    }

    fn active_upload_batch_mut(&mut self) -> Option<&mut ServerFileUploadBatch> {
        self.active_upload_batch_index
            .and_then(|index| self.upload_batches.get_mut(index))
    }

    fn has_active_upload(&self) -> bool {
        self.upload_pipeline_claimed
            || !self.pending_upload_starts.is_empty()
            || self.active_upload_batch().is_some_and(|batch| {
                matches!(
                    batch.phase,
                    UploadBatchPhase::Verifying | UploadBatchPhase::Promoting
                ) || batch.tasks.iter().any(|task| {
                    matches!(
                        task.status,
                        UploadTaskStatus::Pending | UploadTaskStatus::Uploading
                    )
                })
            })
    }

    fn release_upload_pipeline_and_continue(&mut self, ctx: &mut ViewContext<Self>) {
        self.upload_pipeline_claimed = false;
        self.start_next_pending_upload(ctx);
    }

    fn start_next_pending_upload(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(next) = self.pending_upload_starts.pop_front() else {
            return;
        };
        self.start_upload_after_conflict_scan(
            next.client,
            next.remote_directory,
            next.pending_files,
            next.directory_roots,
            ctx,
        );
    }

    fn reserved_upload_destination_paths(&self) -> HashSet<String> {
        let mut paths = HashSet::new();
        if let Some(batch) = self.active_upload_batch() {
            for task in &batch.tasks {
                if !matches!(task.status, UploadTaskStatus::Failed(_)) {
                    paths.insert(task.final_remote_path.clone());
                }
            }
        }
        for start in &self.pending_upload_starts {
            for file in &start.pending_files {
                paths.insert(file.final_remote_path.clone());
            }
        }
        paths
    }

    fn enqueue_upload_start(
        &mut self,
        client: Arc<RemoteServerClient>,
        remote_directory: String,
        pending_files: Vec<PendingUploadFile>,
        directory_roots: Vec<String>,
        ctx: &mut ViewContext<Self>,
    ) {
        self.pending_upload_starts.push_back(PendingUploadStart {
            client,
            remote_directory,
            pending_files,
            directory_roots,
        });
        self.upload_progress_panel_open = true;
        self.status = Some(crate::t!("server-file-browser-upload-queued"));
        ctx.notify();
    }

    fn active_upload_count(&self) -> usize {
        let Some(batch) = self.active_upload_batch() else {
            return 0;
        };
        let in_flight = batch
            .tasks
            .iter()
            .filter(|task| {
                matches!(
                    task.status,
                    UploadTaskStatus::Pending | UploadTaskStatus::Uploading
                )
            })
            .count();
        if in_flight > 0 {
            return in_flight;
        }
        if matches!(
            batch.phase,
            UploadBatchPhase::Verifying | UploadBatchPhase::Promoting
        ) {
            return 1;
        }
        0
    }

    fn has_completed_upload_tasks(&self) -> bool {
        self.upload_batches.iter().any(|batch| {
            batch
                .tasks
                .iter()
                .any(|task| matches!(task.status, UploadTaskStatus::Completed))
        })
    }

    fn stop_progress_poll(&mut self) {
        if let Some(batch) = self.active_upload_batch_mut() {
            if let Some(handle) = batch.progress_poll_handle.take() {
                handle.abort();
            }
        }
    }

    fn schedule_progress_poll(&mut self, ctx: &mut ViewContext<Self>) {
        if !self.has_active_upload() {
            self.stop_progress_poll();
            return;
        }
        let handle = ctx.spawn_abortable(
            Timer::after(Duration::from_millis(UPLOAD_PROGRESS_POLL_MS)),
            |me, _, ctx| {
                ctx.notify();
                me.schedule_progress_poll(ctx);
            },
            |_, _| {},
        );
        if let Some(batch) = self.active_upload_batch_mut() {
            batch.progress_poll_handle = Some(handle);
        }
    }

    fn begin_upload_batch(
        &mut self,
        client: Arc<RemoteServerClient>,
        remote_directory: String,
        pending_files: Vec<PendingUploadFile>,
        directory_roots: Vec<String>,
        conflict_policy: UploadConflictPolicy,
        conflicts: Vec<UploadConflict>,
        ctx: &mut ViewContext<Self>,
    ) {
        if self.upload_pipeline_claimed {
            self.enqueue_upload_start(
                client,
                remote_directory,
                pending_files,
                directory_roots,
                ctx,
            );
            return;
        }
        self.begin_upload_batch_impl(
            client,
            remote_directory,
            pending_files,
            conflict_policy,
            conflicts,
            ctx,
        );
    }

    fn begin_upload_batch_impl(
        &mut self,
        client: Arc<RemoteServerClient>,
        remote_directory: String,
        pending_files: Vec<PendingUploadFile>,
        conflict_policy: UploadConflictPolicy,
        conflicts: Vec<UploadConflict>,
        ctx: &mut ViewContext<Self>,
    ) {
        self.upload_pipeline_claimed = true;

        let conflict_paths: HashSet<String> = conflicts.iter().map(|c| c.path.clone()).collect();
        let directory_overwrite_roots: HashSet<String> = if conflict_policy
            == UploadConflictPolicy::OverwriteAll
        {
            conflicts
                .iter()
                .filter(|c| c.kind == FileSystemEntryKind::Directory)
                .map(|c| c.path.clone())
                .collect()
        } else {
            HashSet::new()
        };

        let pending_files =
            filter_upload_tasks_by_policy(pending_files, conflict_policy, &conflict_paths);
        if pending_files.is_empty() {
            self.status = Some(crate::t!("server-file-browser-upload-all-skipped"));
            self.release_upload_pipeline_and_continue(ctx);
            ctx.notify();
            return;
        }

        let batch_id = Uuid::new_v4().as_simple().to_string();
        let staging_parent = join_remote_path(&remote_directory, UPLOAD_STAGING_DIR_NAME);
        let staging_root = join_remote_path(&staging_parent, &batch_id);

        let tasks: Vec<ServerFileUploadTask> = pending_files
            .into_iter()
            .map(|file| {
                let relative = relative_remote_path_from_base(&remote_directory, &file.final_remote_path);
                let staging_remote_path = if relative.is_empty() {
                    staging_root.clone()
                } else {
                    join_remote_path(&staging_root, &relative)
                };
                ServerFileUploadTask {
                    local_path: file.local_path,
                    file_name: file.display_name,
                    final_remote_path: file.final_remote_path,
                    staging_remote_path,
                    total_bytes: file.total_bytes,
                    uploaded_bytes: Arc::new(AtomicU64::new(0)),
                    status: UploadTaskStatus::Pending,
                }
            })
            .collect();

        let staging_root_for_spawn = staging_root.clone();
        let client_for_mkdir = client.clone();
        ctx.spawn(
            async move {
                create_remote_directory(client_for_mkdir, staging_root_for_spawn).await
            },
            move |me, result, ctx| match result {
                Ok(()) => {
                    me.start_upload_batch_after_staging_ready(
                        client,
                        remote_directory,
                        staging_root,
                        conflict_policy,
                        conflict_paths,
                        directory_overwrite_roots,
                        tasks,
                        ctx,
                    );
                }
                Err(error) => {
                    me.release_upload_pipeline_and_continue(ctx);
                    me.set_error(error, ctx);
                }
            },
        );
    }

    fn start_upload_batch_after_staging_ready(
        &mut self,
        client: Arc<RemoteServerClient>,
        remote_directory: String,
        staging_root: String,
        conflict_policy: UploadConflictPolicy,
        conflict_paths: HashSet<String>,
        directory_overwrite_roots: HashSet<String>,
        tasks: Vec<ServerFileUploadTask>,
        ctx: &mut ViewContext<Self>,
    ) {
        if tasks.is_empty() {
            self.release_upload_pipeline_and_continue(ctx);
            return;
        }
        if self.active_upload_batch_index.is_some() {
            log::warn!(
                "server file browser: upload batch ready while another batch is still active; \
                 queueing is handled at begin_upload_batch"
            );
        }
        self.upload_batches.push(ServerFileUploadBatch {
            staging_root,
            remote_directory,
            conflict_policy,
            conflict_paths,
            directory_overwrite_roots,
            phase: UploadBatchPhase::Uploading,
            tasks,
            next_task_index: 0,
            progress_poll_handle: None,
        });
        self.active_upload_batch_index = Some(self.upload_batches.len() - 1);
        self.upload_progress_panel_open = true;
        self.upload_next_task(client, ctx);
        ctx.notify();
    }

    fn upload_next_task(&mut self, client: Arc<RemoteServerClient>, ctx: &mut ViewContext<Self>) {
        let Some(batch_index) = self.active_upload_batch_index else {
            return;
        };
        if self.upload_batches.get(batch_index).is_some_and(|batch| {
            batch.next_task_index >= batch.tasks.len()
        }) {
            self.stop_progress_poll();
            self.verify_and_promote_batch(client, ctx);
            return;
        }

        let index = self
            .upload_batches
            .get(batch_index)
            .map(|batch| batch.next_task_index)
            .unwrap_or(0);
        if let Some(batch) = self.upload_batches.get_mut(batch_index) {
            batch.tasks[index].status = UploadTaskStatus::Uploading;
            batch.tasks[index].uploaded_bytes.store(0, Ordering::Relaxed);
            batch.next_task_index += 1;
        }

        let (local_path, staging_remote_path, uploaded_bytes) = {
            let batch = self
                .upload_batches
                .get(batch_index)
                .expect("active upload batch exists");
            let task = &batch.tasks[index];
            (
                task.local_path.clone(),
                task.staging_remote_path.clone(),
                task.uploaded_bytes.clone(),
            )
        };

        self.schedule_progress_poll(ctx);
        let client_for_next = client.clone();
        ctx.spawn(
            async move {
                upload_file_with_progress(client, local_path, staging_remote_path, uploaded_bytes)
                    .await
            },
            move |me, result, ctx| {
                if let Some(batch) = me.upload_batches.get_mut(batch_index) {
                    batch.tasks[index].status = match result {
                        Ok(()) => UploadTaskStatus::Completed,
                        Err(error) => UploadTaskStatus::Failed(error),
                    };
                }
                if !me.has_active_upload() {
                    me.stop_progress_poll();
                }
                me.upload_next_task(client_for_next, ctx);
            },
        );
        ctx.notify();
    }

    fn verify_and_promote_batch(
        &mut self,
        client: Arc<RemoteServerClient>,
        ctx: &mut ViewContext<Self>,
    ) {
        let Some(batch) = self.active_upload_batch() else {
            return;
        };
        if batch.tasks.iter().any(|task| {
            matches!(task.status, UploadTaskStatus::Failed(_))
        }) {
            self.finish_upload_batch_failed(ctx);
            return;
        }

        if let Some(batch) = self.active_upload_batch_mut() {
            batch.phase = UploadBatchPhase::Verifying;
        }
        ctx.notify();

        let verify_tasks: Vec<(String, u64)> = self
            .active_upload_batch()
            .map(|batch| {
                batch
                    .tasks
                    .iter()
                    .filter(|task| matches!(task.status, UploadTaskStatus::Completed))
                    .map(|task| {
                        (
                            task.staging_remote_path.clone(),
                            task.total_bytes,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        let client_for_verify = client.clone();
        ctx.spawn(
            async move { verify_staging_files(client_for_verify, verify_tasks).await },
            move |me, result, ctx| match result {
                Ok(()) => me.promote_staging_batch(client, ctx),
                Err(error) => {
                    me.fail_upload_batch_with_cleanup(client, error, ctx);
                }
            },
        );
    }

    fn promote_staging_batch(&mut self, client: Arc<RemoteServerClient>, ctx: &mut ViewContext<Self>) {
        let Some(batch_snapshot) = self.active_upload_batch().map(|batch| {
            (
                batch.staging_root.clone(),
                batch.conflict_policy,
                batch.directory_overwrite_roots.clone(),
                batch
                    .tasks
                    .iter()
                    .filter(|task| matches!(task.status, UploadTaskStatus::Completed))
                    .map(|task| {
                        (
                            task.staging_remote_path.clone(),
                            task.final_remote_path.clone(),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        }) else {
            return;
        };

        if let Some(batch) = self.active_upload_batch_mut() {
            batch.phase = UploadBatchPhase::Promoting;
        }
        ctx.notify();

        let (staging_root, conflict_policy, directory_overwrite_roots, promote_pairs) =
            batch_snapshot;
        let session = self.session.clone();
        let remote_session_id = self.remote_session_id(ctx);
        let client_for_promote = client.clone();
        let client_for_cleanup = client.clone();

        ctx.spawn(
            async move {
                promote_staging_files(
                    client_for_promote,
                    session,
                    remote_session_id,
                    staging_root.clone(),
                    conflict_policy,
                    directory_overwrite_roots,
                    promote_pairs,
                )
                .await
            },
            move |me, result, ctx| {
                let cleanup_client = client_for_cleanup.clone();
                let staging_root = me
                    .active_upload_batch()
                    .map(|batch| batch.staging_root.clone());
                match result {
                    Ok(()) => {
                        if let Some(root) = staging_root {
                            me.spawn_cleanup_staging(cleanup_client, root, ctx);
                        }
                        me.finish_upload_batch_success(ctx);
                    }
                    Err(error) => {
                        me.fail_upload_batch_with_cleanup(
                            cleanup_client,
                            format_upload_promote_error(&error),
                            ctx,
                        );
                    }
                }
            },
        );
    }

    fn spawn_cleanup_staging(
        &mut self,
        client: Arc<RemoteServerClient>,
        staging_root: String,
        ctx: &mut ViewContext<Self>,
    ) {
        let session = self.session.clone();
        let remote_session_id = self.remote_session_id(ctx);
        ctx.spawn(
            async move {
                cleanup_staging_root(client, session, remote_session_id, staging_root).await
            },
            |_, _result, _ctx| {},
        );
    }

    fn reset_upload_batch_phase(&mut self) {
        if let Some(batch) = self.active_upload_batch_mut() {
            batch.phase = UploadBatchPhase::Uploading;
        }
    }

    fn finish_active_upload_batch(&mut self) {
        self.active_upload_batch_index = None;
    }

    fn finish_upload_batch_success(&mut self, ctx: &mut ViewContext<Self>) {
        self.reset_upload_batch_phase();
        let directories_to_reload = self.collect_directories_to_refresh_for_completed_uploads();
        self.finish_active_upload_batch();
        self.release_upload_pipeline_and_continue(ctx);
        self.status = Some(crate::t!("server-file-browser-transfer-complete"));
        self.reload_directories_selective(directories_to_reload, ctx);
    }

    fn finish_upload_batch_failed(&mut self, ctx: &mut ViewContext<Self>) {
        let (error, staging_root, client) = {
            let batch = self.active_upload_batch();
            let error = batch.and_then(|b| {
                b.tasks.iter().find_map(|task| {
                    if let UploadTaskStatus::Failed(error) = &task.status {
                        Some(error.clone())
                    } else {
                        None
                    }
                })
            });
            let staging_root = batch.map(|b| b.staging_root.clone());
            (error, staging_root, self.client(ctx))
        };
        if let (Some(client), Some(staging_root)) = (client, staging_root) {
            self.spawn_cleanup_staging(client, staging_root, ctx);
        }
        self.reset_upload_batch_phase();
        self.finish_active_upload_batch();
        self.release_upload_pipeline_and_continue(ctx);
        if let Some(error) = error {
            self.status = Some(crate::t!("server-file-browser-operation-failed", error = error));
        }
        ctx.notify();
    }

    fn fail_upload_batch_with_cleanup(
        &mut self,
        client: Arc<RemoteServerClient>,
        error: String,
        ctx: &mut ViewContext<Self>,
    ) {
        if let Some(staging_root) = self
            .active_upload_batch()
            .map(|batch| batch.staging_root.clone())
        {
            self.spawn_cleanup_staging(client, staging_root, ctx);
        }
        self.reset_upload_batch_phase();
        self.finish_active_upload_batch();
        self.release_upload_pipeline_and_continue(ctx);
        self.status = Some(crate::t!("server-file-browser-operation-failed", error = error));
        ctx.notify();
    }

    fn confirm_upload_conflicts(
        &mut self,
        client: Arc<RemoteServerClient>,
        remote_directory: String,
        pending_files: Vec<PendingUploadFile>,
        directory_roots: Vec<String>,
        conflicts: Vec<UploadConflict>,
        ctx: &mut ViewContext<Self>,
    ) {
        let conflict_summary = format_upload_conflict_summary(&conflicts);
        let pending_files_for_skip = pending_files.clone();
        let pending_files_for_overwrite = pending_files;
        let directory_roots_for_skip = directory_roots.clone();
        let directory_roots_for_overwrite = directory_roots;
        let conflicts_for_skip = conflicts.clone();
        let conflicts_for_overwrite = conflicts;
        let client_for_overwrite = client.clone();
        let client_for_skip = client;
        let remote_directory_for_overwrite = remote_directory.clone();
        let remote_directory_for_skip = remote_directory;

        let dialog = AlertDialogWithCallbacks::for_view(
            crate::t!("server-file-browser-upload-conflict-title"),
            conflict_summary,
            vec![
                ModalButton::for_view(
                    crate::t!("server-file-browser-upload-conflict-overwrite"),
                    move |me: &mut ServerFileBrowserView, ctx| {
                        me.begin_upload_batch(
                            client_for_overwrite,
                            remote_directory_for_overwrite,
                            pending_files_for_overwrite,
                            directory_roots_for_overwrite,
                            UploadConflictPolicy::OverwriteAll,
                            conflicts_for_overwrite,
                            ctx,
                        );
                    },
                ),
                ModalButton::for_view(
                    crate::t!("server-file-browser-upload-conflict-skip"),
                    move |me: &mut ServerFileBrowserView, ctx| {
                        me.begin_upload_batch(
                            client_for_skip,
                            remote_directory_for_skip,
                            pending_files_for_skip,
                            directory_roots_for_skip,
                            UploadConflictPolicy::SkipExisting,
                            conflicts_for_skip,
                            ctx,
                        );
                    },
                ),
                ModalButton::for_view(crate::t!("common-cancel"), |_: &mut ServerFileBrowserView, _| {}),
            ],
            |_, _| {},
        );
        ctx.show_native_platform_modal(dialog);
    }

    fn start_upload_after_conflict_scan(
        &mut self,
        client: Arc<RemoteServerClient>,
        remote_directory: String,
        pending_files: Vec<PendingUploadFile>,
        directory_roots: Vec<String>,
        ctx: &mut ViewContext<Self>,
    ) {
        if pending_files.is_empty() {
            return;
        }
        if self.upload_pipeline_claimed {
            self.enqueue_upload_start(
                client,
                remote_directory,
                pending_files,
                directory_roots,
                ctx,
            );
            return;
        }
        let reserved_paths = self.reserved_upload_destination_paths();
        let client_for_scan = client.clone();
        let pending_files_for_scan = pending_files.clone();
        let directory_roots_for_scan = directory_roots.clone();
        let directory_roots_for_begin = directory_roots.clone();
        ctx.spawn(
            async move {
                let mut conflicts = scan_upload_conflicts(
                    &client_for_scan,
                    &pending_files_for_scan,
                    &directory_roots_for_scan,
                )
                .await?;
                append_reserved_path_conflicts(
                    &mut conflicts,
                    &pending_files_for_scan,
                    &reserved_paths,
                );
                Ok::<_, String>((pending_files, conflicts))
            },
            move |me, scan_result, ctx| match scan_result {
                Ok((files, conflicts)) if conflicts.is_empty() => {
                    me.begin_upload_batch(
                        client.clone(),
                        remote_directory.clone(),
                        files,
                        directory_roots_for_begin,
                        UploadConflictPolicy::Proceed,
                        Vec::new(),
                        ctx,
                    );
                }
                Ok((files, conflicts)) => {
                    me.confirm_upload_conflicts(
                        client,
                        remote_directory,
                        files,
                        directory_roots_for_begin,
                        conflicts,
                        ctx,
                    );
                }
                Err(error) => {
                    me.release_upload_pipeline_and_continue(ctx);
                    me.set_error(error, ctx);
                }
            },
        );
    }

    fn handle_collected_upload_tasks(
        &mut self,
        client: Arc<RemoteServerClient>,
        remote_directory: String,
        result: Result<(Vec<PendingUploadFile>, Vec<String>), String>,
        ctx: &mut ViewContext<Self>,
    ) {
        match result {
            Ok((pending_files, _)) if pending_files.is_empty() => {}
            Ok((pending_files, directory_roots)) => {
                self.start_upload_after_conflict_scan(
                    client,
                    remote_directory,
                    pending_files,
                    directory_roots,
                    ctx,
                );
            }
            Err(error) => self.set_error(error, ctx),
        }
    }

    fn toggle_upload_progress_panel(&mut self, ctx: &mut ViewContext<Self>) {
        self.upload_progress_panel_open = !self.upload_progress_panel_open;
        ctx.notify();
    }

    fn dismiss_upload_progress_panel(&mut self, ctx: &mut ViewContext<Self>) {
        self.upload_progress_panel_open = false;
        ctx.notify();
    }

    fn clear_completed_uploads(&mut self, ctx: &mut ViewContext<Self>) {
        for batch in &mut self.upload_batches {
            batch.tasks.retain(|task| !matches!(task.status, UploadTaskStatus::Completed));
        }
        self.upload_batches.retain(|batch| !batch.tasks.is_empty());
        if self.upload_batches.is_empty() {
            self.active_upload_batch_index = None;
            self.upload_progress_panel_open = false;
            self.stop_progress_poll();
        } else if let Some(active_index) = self.active_upload_batch_index {
            if active_index >= self.upload_batches.len() {
                self.active_upload_batch_index = None;
            }
        }
        ctx.notify();
    }

    fn choose_and_upload_files(&mut self, remote_directory: String, ctx: &mut ViewContext<Self>) {
        let Some(client) = self.client(ctx) else {
            self.set_error(crate::t!("server-file-browser-no-session"), ctx);
            return;
        };
        let remote_directory = remote_directory;
        ctx.spawn(
            async {},
            move |me, _, ctx| {
                me.open_upload_files_picker(client, remote_directory, ctx);
            },
        );
    }

    fn open_upload_files_picker(
        &mut self,
        client: Arc<RemoteServerClient>,
        remote_directory: String,
        ctx: &mut ViewContext<Self>,
    ) {
        ctx.open_file_picker(
            move |result, ctx| match result {
                Ok(paths) if !paths.is_empty() => {
                    let local_paths = paths.into_iter().map(PathBuf::from).collect();
                    let client_for_batch = client.clone();
                    let remote_directory_for_collect = remote_directory.clone();
                    let remote_directory_for_handler = remote_directory.clone();
                    ctx.spawn(
                        async move {
                            collect_upload_tasks(
                                local_paths,
                                remote_directory_for_collect,
                                false,
                            )
                        },
                        move |me, result, ctx| {
                            me.handle_collected_upload_tasks(
                                client_for_batch,
                                remote_directory_for_handler,
                                result,
                                ctx,
                            );
                        },
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    log::warn!("server file browser file picker failed: {error}");
                }
            },
            FilePickerConfiguration::new().allow_multi_select(),
        );
    }

    fn choose_and_upload_folder(&mut self, remote_directory: String, ctx: &mut ViewContext<Self>) {
        let Some(client) = self.client(ctx) else {
            self.set_error(crate::t!("server-file-browser-no-session"), ctx);
            return;
        };
        let remote_directory = remote_directory;
        ctx.spawn(
            async {},
            move |me, _, ctx| {
                me.open_upload_folder_picker(client, remote_directory, ctx);
            },
        );
    }

    fn open_upload_folder_picker(
        &mut self,
        client: Arc<RemoteServerClient>,
        remote_directory: String,
        ctx: &mut ViewContext<Self>,
    ) {
        ctx.open_file_picker(
            move |result, ctx| match result {
                Ok(paths) if !paths.is_empty() => {
                    let local_paths = paths.into_iter().map(PathBuf::from).collect();
                    let client_for_batch = client.clone();
                    let remote_directory_for_collect = remote_directory.clone();
                    let remote_directory_for_handler = remote_directory.clone();
                    ctx.spawn(
                        async move {
                            collect_upload_tasks(
                                local_paths,
                                remote_directory_for_collect,
                                true,
                            )
                        },
                        move |me, result, ctx| {
                            me.handle_collected_upload_tasks(
                                client_for_batch,
                                remote_directory_for_handler,
                                result,
                                ctx,
                            );
                        },
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    log::warn!("server file browser folder picker failed: {error}");
                }
            },
            FilePickerConfiguration::new().folders_only(),
        );
    }

    fn choose_download_destination(&mut self, remote_path: String, ctx: &mut ViewContext<Self>) {
        let Some(entry) = self.entries.iter().find(|entry| entry.path == remote_path).cloned() else {
            return;
        };
        let Some(client) = self.client(ctx) else {
            self.set_error(crate::t!("server-file-browser-no-session"), ctx);
            return;
        };

        match entry.kind {
            FileSystemEntryKind::Directory => {
                ctx.open_file_picker(
                    move |result, ctx| match result {
                        Ok(paths) if !paths.is_empty() => {
                            let destination = PathBuf::from(&paths[0]);
                            ctx.spawn(
                                async move { download_directory(client, entry.path, destination).await },
                                |me: &mut Self, result, ctx| {
                                    me.finish_transfer(result, ctx);
                                },
                            );
                        }
                        Ok(_) => {}
                        Err(error) => {
                            log::warn!("server file browser download picker failed: {error}");
                        }
                    },
                    FilePickerConfiguration::new().folders_only(),
                );
            }
            _ => {
                let default_filename = remote_basename(&entry.path).unwrap_or(entry.name);
                ctx.open_save_file_picker(
                    move |path, _me, ctx| {
                        if let Some(path) = path {
                            ctx.spawn(
                                async move {
                                    download_file(client, entry.path, PathBuf::from(path)).await
                                },
                                |me: &mut Self, result, ctx| {
                                    me.finish_transfer(result, ctx);
                                },
                            );
                        }
                    },
                    SaveFilePickerConfiguration::new().with_default_filename(default_filename),
                );
            }
        }
    }

    fn upload_overall_summary(&self) -> Option<(usize, usize)> {
        if self.upload_batches.is_empty() {
            return None;
        }
        let total: usize = self.upload_batches.iter().map(|batch| batch.tasks.len()).sum();
        let done = self
            .upload_batches
            .iter()
            .flat_map(|batch| &batch.tasks)
            .filter(|task| {
                matches!(
                    task.status,
                    UploadTaskStatus::Completed | UploadTaskStatus::Failed(_)
                )
            })
            .count();
        Some((done, total))
    }

    fn render_upload_progress_button(
        &self,
        appearance: &crate::appearance::Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let icon_color = theme.sub_text_color(theme.background());
        let icon_el = ConstrainedBox::new(Icon::ListOpen.to_warpui_icon(icon_color).finish())
            .with_width(TOOLBAR_ICON_SIZE)
            .with_height(TOOLBAR_ICON_SIZE)
            .finish();
        let mut button_stack = Stack::new().with_child(
            Hoverable::new(self.upload_progress_button.clone(), move |_| {
                Container::new(
                    ConstrainedBox::new(icon_el)
                        .with_width(TOOLBAR_BUTTON_SIZE)
                        .with_height(TOOLBAR_BUTTON_SIZE)
                        .finish(),
                )
                .with_uniform_padding(2.0)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)))
                .finish()
            })
            .with_cursor(Cursor::PointingHand)
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(ServerFileBrowserAction::ToggleUploadProgressPanel);
            })
            .finish(),
        );
        if self.has_active_upload() {
            let active_count = self.active_upload_count();
            let badge_label = if active_count > 9 {
                "9+".to_string()
            } else {
                active_count.to_string()
            };
            let theme = appearance.theme();
            let badge = Container::new(
                ConstrainedBox::new(
                    Text::new_inline(
                        badge_label,
                        appearance.ui_font_family(),
                        9.0,
                    )
                    .with_color(theme.main_text_color(theme.accent()).into())
                    .finish(),
                )
                .with_width(14.0)
                .with_height(14.0)
                .finish(),
            )
            .with_background(warpui::elements::Fill::Solid(theme.accent().into_solid()))
            .with_corner_radius(CornerRadius::with_all(Radius::Percentage(50.0)))
            .finish();
            button_stack.add_positioned_overlay_child(
                badge,
                OffsetPositioning::offset_from_parent(
                    Vector2F::new(TOOLBAR_BUTTON_SIZE - 4.0, 0.0),
                    ParentOffsetBounds::ParentByPosition,
                    ParentAnchor::TopLeft,
                    ChildAnchor::TopLeft,
                ),
            );
        }
        button_stack.finish()
    }

    fn render_upload_progress_panel(
        &self,
        appearance: &crate::appearance::Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let sub_text = theme.sub_text_color(theme.background());

        let title = Text::new(
            crate::t!("server-file-browser-upload-progress-title"),
            appearance.ui_font_family(),
            13.0,
        )
        .with_color(theme.main_text_color(theme.background()).into())
        .finish();

        let mut header_row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Shrinkable::new(
                    1.0,
                    Clipped::new(title).finish(),
                )
                .finish(),
            );

        if self.has_completed_upload_tasks() {
            let clear_label = crate::t!("server-file-browser-upload-clear-completed");
            header_row.add_child(
                Hoverable::new(self.clear_completed_uploads_button.clone(), move |_| {
                    Text::new_inline(clear_label.clone(), appearance.ui_font_family(), 11.0)
                        .with_color(theme.accent().into())
                        .finish()
                })
                .with_cursor(Cursor::PointingHand)
                .on_click(|ctx, _, _| {
                    ctx.dispatch_typed_action(ServerFileBrowserAction::ClearCompletedUploads);
                })
                .finish(),
            );
        }

        let mut column = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(header_row.finish());

        if let Some((done, total)) = self.upload_overall_summary() {
            column.add_child(
                Container::new(
                    Text::new_inline(
                        crate::t!("server-file-browser-upload-overall", done = done, total = total),
                        appearance.ui_font_family(),
                        11.0,
                    )
                    .with_color(sub_text.into())
                    .finish(),
                )
                .with_padding_top(4.0)
                .finish(),
            );
        }

        if let Some(batch) = self.active_upload_batch() {
            if batch.phase != UploadBatchPhase::Uploading {
                column.add_child(
                    Container::new(
                        Text::new_inline(
                            upload_batch_phase_label(batch.phase),
                            appearance.ui_font_family(),
                            11.0,
                        )
                        .with_color(sub_text.into())
                        .finish(),
                    )
                    .with_padding_top(4.0)
                    .finish(),
                );
            }
        }

        let total_tasks: usize = self.upload_batches.iter().map(|batch| batch.tasks.len()).sum();
        if total_tasks == 0 {
            column.add_child(
                Container::new(
                    Text::new_inline(
                        crate::t!("server-file-browser-upload-progress-empty"),
                        appearance.ui_font_family(),
                        ITEM_FONT_SIZE,
                    )
                    .with_color(sub_text.into())
                    .finish(),
                )
                .with_padding_top(8.0)
                .finish(),
            );
        } else {
            let mut list = Flex::column().with_spacing(8.0);
            for batch in self.upload_batches.iter().rev() {
                for task in &batch.tasks {
                    let progress = upload_task_progress(task);
                    let progress_bar = render_flex_progress_bar(
                        progress,
                        theme.accent().into(),
                        theme.background().into(),
                    );
                    list.add_child(
                        Flex::column()
                            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .with_child(
                                Clipped::new(
                                    Text::new_inline(
                                        task.file_name.clone(),
                                        appearance.ui_font_family(),
                                        ITEM_FONT_SIZE,
                                    )
                                    .with_color(theme.main_text_color(theme.background()).into())
                                    .finish(),
                                )
                                .finish(),
                            )
                            .with_child(
                                Container::new(
                                    Text::new_inline(
                                        upload_task_status_label(task, batch.phase),
                                        appearance.ui_font_family(),
                                        11.0,
                                    )
                                    .with_color(sub_text.into())
                                    .finish(),
                                )
                                .with_padding_top(2.0)
                                .finish(),
                            )
                            .with_child(
                                Container::new(progress_bar)
                                    .with_padding_top(4.0)
                                    .finish(),
                            )
                            .finish(),
                    );
                }
            }
            column.add_child(
                Container::new(
                    ConstrainedBox::new(list.finish())
                        .with_max_height(UPLOAD_PROGRESS_PANEL_MAX_HEIGHT)
                        .finish(),
                )
                .with_padding_top(8.0)
                .finish(),
            );
        }

        let panel_body = Container::new(column.finish())
            .with_uniform_padding(12.0)
            .with_background(warpui::elements::Fill::Solid(theme.surface_2().into_solid()))
            .with_border(Border::all(1.0).with_border_color(theme.outline().into()))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.0)))
            .finish();

        Dismiss::new(panel_body)
            .prevent_interaction_with_other_elements()
            .on_dismiss(|ctx, _app| {
                ctx.dispatch_typed_action(ServerFileBrowserAction::DismissUploadProgressPanel);
            })
            .finish()
    }

    fn finish_transfer(&mut self, result: Result<(), String>, ctx: &mut ViewContext<Self>) {
        match result {
            Ok(()) => {
                self.status = Some(crate::t!("server-file-browser-transfer-complete"));
                self.refresh_directory_tree(ctx);
            }
            Err(error) => {
                self.status = Some(error);
                ctx.notify();
            }
        }
    }

    fn render_toolbar(&self, appearance: &crate::appearance::Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let icon_color = theme.sub_text_color(theme.background());
        let make_btn =
            |icon: Icon, state: MouseStateHandle, action: ServerFileBrowserAction| -> Box<dyn Element> {
                let icon_el = ConstrainedBox::new(icon.to_warpui_icon(icon_color).finish())
                    .with_width(TOOLBAR_ICON_SIZE)
                    .with_height(TOOLBAR_ICON_SIZE)
                    .finish();
                Hoverable::new(state, move |_| {
                    Container::new(
                        ConstrainedBox::new(icon_el)
                            .with_width(TOOLBAR_BUTTON_SIZE)
                            .with_height(TOOLBAR_BUTTON_SIZE)
                            .finish(),
                    )
                    .with_uniform_padding(2.0)
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)))
                    .finish()
                })
                .with_cursor(Cursor::PointingHand)
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(action.clone());
                })
                .finish()
            };

        Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(6.0)
            .with_child(Shrinkable::new(
                1.0,
                appearance
                    .ui_builder()
                    .text_input(self.path_editor.clone())
                    .with_style(UiComponentStyles {
                        height: Some(INPUT_HEIGHT),
                        padding: Some(Coords::uniform(6.0)),
                        background: Some(theme.surface_2().into()),
                        border_color: Some(theme.nonactive_ui_detail().into()),
                        border_width: Some(1.0),
                        border_radius: Some(CornerRadius::with_all(Radius::Pixels(4.0))),
                        font_size: Some(ITEM_FONT_SIZE),
                        ..Default::default()
                    })
                    .build()
                    .finish(),
            )
            .finish())
            .with_child(make_btn(
                Icon::Refresh,
                self.refresh_button.clone(),
                ServerFileBrowserAction::Refresh,
            ))
            .with_child(make_btn(
                Icon::UploadCloud,
                self.upload_file_button.clone(),
                ServerFileBrowserAction::UploadFiles(self.current_path.clone()),
            ))
            .with_child(make_btn(
                Icon::Folder,
                self.upload_folder_button.clone(),
                ServerFileBrowserAction::UploadFolder(self.current_path.clone()),
            ))
            .with_child(self.render_upload_progress_button(appearance))
            .finish()
    }

    fn render_entries(&self, appearance: &crate::appearance::Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        if self.host_id.is_none() {
            return self.render_status_text(crate::t!("server-file-browser-no-session"), appearance);
        } else if self.loading && self.entries.is_empty() {
            return self.render_status_text(crate::t!("server-file-browser-loading"), appearance);
        } else if self.entries.is_empty() {
            return self.render_status_text(crate::t!("server-file-browser-empty-directory"), appearance);
        }

        let entries = self.entries.clone();
        let selected_index = self.selected_index;
        let expanded_directories = self.expanded_directories.clone();
        let row_states = self.row_states.clone();
        let pending_rename_index = self.pending_rename_index;
        let rename_editor = self.rename_editor.clone();
        let uniform_list = UniformList::new(
            self.list_state.clone(),
            entries.len(),
            move |range, app| {
                let appearance = crate::appearance::Appearance::as_ref(app);
                range
                    .filter_map(|index| {
                        let entry = entries.get(index)?;
                        let state = row_states
                            .get(&entry.path)
                            .cloned()
                            .unwrap_or_default();
                        Some(render_entry_row(
                            index,
                            entry,
                            selected_index == Some(index),
                            expanded_directories.contains(&entry.path),
                            pending_rename_index == Some(index),
                            &rename_editor,
                            state,
                            appearance,
                        ))
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
            },
        )
        .finish_scrollable();

        let scrollable = Shrinkable::new(
            1.0,
            Scrollable::vertical(
                self.scroll_state.clone(),
                uniform_list,
                ScrollbarWidth::Auto,
                theme.nonactive_ui_detail().into(),
                theme.active_ui_detail().into(),
                warpui::elements::Fill::None,
            )
            .with_overlayed_scrollbar()
            .finish(),
        )
        .finish();

        let mut col = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_child(scrollable);
        if let Some(status) = &self.status {
            col.add_child(
                Container::new(
                    Text::new_inline(status.clone(), appearance.ui_font_family(), 12.0)
                        .with_color(theme.sub_text_color(theme.background()).into())
                        .finish(),
                )
                .with_padding_top(10.0)
                .with_padding_left(ITEM_PADDING_HORIZONTAL)
                .with_padding_right(ITEM_PADDING_HORIZONTAL)
                .finish(),
            );
        }

        let content = Container::new(
            col.with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .finish(),
        )
        .with_horizontal_padding(PANEL_HORIZONTAL_PADDING - ITEM_PADDING_HORIZONTAL);

        content.finish()
    }

    fn render_status_text(
        &self,
        text: String,
        appearance: &crate::appearance::Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        Container::new(
            Text::new_inline(text, appearance.ui_font_family(), ITEM_FONT_SIZE)
                .with_color(theme.sub_text_color(theme.background()).into())
                .finish(),
        )
        .with_padding_top(20.0)
        .with_padding_bottom(20.0)
        .with_padding_left(ITEM_PADDING_HORIZONTAL)
        .with_padding_right(ITEM_PADDING_HORIZONTAL)
        .finish()
    }

}

fn render_entry_row(
    index: usize,
    entry: &ServerFileBrowserEntry,
    is_selected: bool,
    is_expanded: bool,
    is_renaming: bool,
    rename_editor: &ViewHandle<EditorView>,
    state: MouseStateHandle,
    appearance: &crate::appearance::Appearance,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let icon_color = theme.sub_text_color(theme.background());
    let is_directory = entry.kind == FileSystemEntryKind::Directory;

    let chevron: Box<dyn Element> = if is_directory {
        let icon = if is_expanded {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        };
        ConstrainedBox::new(icon.to_warpui_icon(icon_color).finish())
            .with_width(ITEM_ICON_SIZE)
            .with_height(ITEM_ICON_SIZE)
            .finish()
    } else {
        ConstrainedBox::new(Empty::new().finish())
            .with_width(ITEM_ICON_SIZE)
            .finish()
    };
    let icon = if is_directory { Icon::Folder } else { Icon::File };
    let icon_el = ConstrainedBox::new(icon.to_warpui_icon(icon_color).finish())
        .with_width(ITEM_ICON_SIZE)
        .with_height(ITEM_ICON_SIZE)
        .finish();
    let text_column = if is_renaming {
        Shrinkable::new(
            1.0,
            Dismiss::new(Clipped::new(ChildView::new(rename_editor).finish()).finish())
                .on_dismiss(|ctx, _app| {
                    ctx.dispatch_typed_action(ServerFileBrowserAction::DismissRenameEditor);
                })
                .finish(),
        )
        .finish()
    } else {
        let label = Text::new_inline(
            entry.name.clone(),
            appearance.ui_font_family(),
            ITEM_FONT_SIZE,
        )
        .with_color(theme.main_text_color(theme.background()).into())
        .finish();

        let mut metadata_parts = Vec::new();
        if let Some(size) = entry.size_bytes {
            metadata_parts.push(format_file_size(size));
        }
        if let Some(epoch_millis) = entry.modified_epoch_millis {
            if let Some(formatted) = format_modified_epoch_millis(epoch_millis) {
                metadata_parts.push(formatted);
            }
        }
        let metadata = (!metadata_parts.is_empty()).then(|| {
            Text::new_inline(metadata_parts.join(" · "), appearance.ui_font_family(), 11.0)
                .with_color(theme.sub_text_color(theme.background()).into())
                .finish()
        });

        let mut col = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_child(label);
        if let Some(metadata) = metadata {
            col.add_child(metadata);
        }
        col.finish()
    };

    let row = Flex::row()
        .with_main_axis_size(MainAxisSize::Max)
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_spacing(ITEM_ICON_TEXT_SPACING)
        .with_child(
            ConstrainedBox::new(Empty::new().finish())
                .with_width(entry.depth as f32 * 16.0)
                .finish(),
        )
        .with_child(chevron)
        .with_child(icon_el)
        .with_child(Shrinkable::new(1.0, text_column).finish())
        .finish();

    let mut hoverable = Hoverable::new(state, move |_| {
        let mut container = Container::new(row)
            .with_padding_top(ITEM_PADDING_VERTICAL)
            .with_padding_bottom(ITEM_PADDING_VERTICAL)
            .with_padding_left(ITEM_PADDING_HORIZONTAL)
            .with_padding_right(ITEM_PADDING_HORIZONTAL)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)));
        if is_selected {
            container = container.with_background(internal_colors::fg_overlay_3(theme));
        }
        container.finish()
    });
    if !is_renaming {
        hoverable = hoverable
            .with_cursor(Cursor::PointingHand)
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(ServerFileBrowserAction::ClickEntry(index));
            })
            .on_double_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(ServerFileBrowserAction::OpenEntry(index));
            })
            .on_right_click(move |ctx, _, position| {
                let offset = match ctx.element_position_by_id(CONTEXT_MENU_POSITION_ID) {
                    Some(bounds) => position - bounds.origin(),
                    None => position,
                };
                ctx.dispatch_typed_action(ServerFileBrowserAction::OpenContextMenu {
                    index,
                    position: offset,
                });
            });
    }
    let hoverable = hoverable.finish();

    Container::new(hoverable).finish()
}

impl Entity for ServerFileBrowserView {
    type Event = ServerFileBrowserEvent;
}

impl TypedActionView for ServerFileBrowserView {
    type Action = ServerFileBrowserAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            ServerFileBrowserAction::Refresh => self.refresh_directory_tree(ctx),
            ServerFileBrowserAction::JumpToPath => self.jump_to_editor_path(ctx),
            ServerFileBrowserAction::ClickEntry(index) => {
                ctx.focus_self();
                self.click_entry(*index, ctx);
            }
            ServerFileBrowserAction::OpenEntry(index) => {
                ctx.focus_self();
                self.open_index(*index, ctx);
            }
            ServerFileBrowserAction::ToggleDirectory(path) => self.toggle_directory(path.clone(), ctx),
            ServerFileBrowserAction::SelectPreviousItem => self.select_previous_item(ctx),
            ServerFileBrowserAction::SelectNextItem => self.select_next_item(ctx),
            ServerFileBrowserAction::ExpandSelectedItem => self.expand_selected_item(ctx),
            ServerFileBrowserAction::CollapseSelectedItem => self.collapse_selected_item(ctx),
            ServerFileBrowserAction::ExecuteSelectedItem => self.execute_selected_item(ctx),
            ServerFileBrowserAction::OpenContextMenu { index, position } => {
                self.open_context_menu(*index, *position, ctx);
            }
            ServerFileBrowserAction::DismissContextMenu => self.dismiss_context_menu(ctx),
            ServerFileBrowserAction::CopyPath(path) => self.copy_path(path.clone(), ctx),
            ServerFileBrowserAction::CopyName(name) => self.copy_name(name.clone(), ctx),
            ServerFileBrowserAction::CdToTerminal(path) => {
                self.dismiss_context_menu(ctx);
                ctx.emit(ServerFileBrowserEvent::CdToDirectory {
                    path: path.clone(),
                });
            }
            ServerFileBrowserAction::Download(path) => {
                self.dismiss_context_menu(ctx);
                self.choose_download_destination(path.clone(), ctx);
            }
            ServerFileBrowserAction::UploadFiles(path) => {
                self.dismiss_context_menu(ctx);
                self.choose_and_upload_files(path.clone(), ctx);
            }
            ServerFileBrowserAction::UploadFolder(path) => {
                self.dismiss_context_menu(ctx);
                self.choose_and_upload_folder(path.clone(), ctx);
            }
            ServerFileBrowserAction::CreateFile(path) => {
                self.create_new_entry(path.clone(), NewRemoteEntryKind::File, ctx);
            }
            ServerFileBrowserAction::CreateFolder(path) => {
                self.create_new_entry(path.clone(), NewRemoteEntryKind::Directory, ctx);
            }
            ServerFileBrowserAction::RenameEntry(index) => self.start_rename(*index, ctx),
            ServerFileBrowserAction::DeleteEntry(index) => self.confirm_delete(*index, ctx),
            ServerFileBrowserAction::CommitRename => self.commit_rename(ctx),
            ServerFileBrowserAction::CancelRename => self.cancel_rename(ctx),
            ServerFileBrowserAction::DismissRenameEditor => self.commit_rename(ctx),
            ServerFileBrowserAction::ToggleUploadProgressPanel => {
                self.toggle_upload_progress_panel(ctx)
            }
            ServerFileBrowserAction::DismissUploadProgressPanel => {
                self.dismiss_upload_progress_panel(ctx)
            }
            ServerFileBrowserAction::ClearCompletedUploads => self.clear_completed_uploads(ctx),
        }
    }
}

impl View for ServerFileBrowserView {
    fn ui_name() -> &'static str {
        "ServerFileBrowserView"
    }

    fn on_focus(&mut self, focus_ctx: &FocusContext, ctx: &mut ViewContext<Self>) {
        if focus_ctx.is_self_focused() {
            if self.selected_index.is_none() && !self.entries.is_empty() {
                self.selected_index = Some(0);
                ctx.notify();
            }
        }
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = crate::appearance::Appearance::as_ref(app);
        let toolbar = Container::new(self.render_toolbar(appearance))
            .with_uniform_padding(8.0)
            .finish();
        let entries = Shrinkable::new(1.0, self.render_entries(appearance)).finish();
        let panel = SavePosition::new(
            Container::new(
                Flex::column()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .with_child(toolbar)
                    .with_child(entries)
                    .finish(),
            )
            .finish(),
            CONTEXT_MENU_POSITION_ID,
        )
        .finish();

        let mut stack = Stack::new();
        stack.add_child(panel);
        if let Some(position) = self.context_menu_position {
            stack.add_positioned_overlay_child(
                ChildView::new(&self.context_menu).finish(),
                OffsetPositioning::offset_from_parent(
                    position,
                    ParentOffsetBounds::WindowByPosition,
                    ParentAnchor::TopLeft,
                    ChildAnchor::TopLeft,
                ),
            );
        }
        if self.upload_progress_panel_open {
            stack.add_positioned_overlay_child(
                SavePosition::new(
                    self.render_upload_progress_panel(appearance),
                    UPLOAD_PROGRESS_PANEL_POSITION,
                )
                .finish(),
                OffsetPositioning::offset_from_parent(
                    Vector2F::new(0.0, UPLOAD_PROGRESS_PANEL_TOP_OFFSET),
                    ParentOffsetBounds::ParentBySize,
                    ParentAnchor::TopLeft,
                    ChildAnchor::TopLeft,
                ),
            );
        }
        let context_menu_open = self.context_menu_position.is_some();
        EventHandler::new(stack.finish())
            .on_keydown(move |ctx, _app, keystroke| {
                if context_menu_open {
                    return DispatchEventResult::PropagateToParent;
                }
                match keystroke.normalized().as_str() {
                    "up" => {
                        ctx.dispatch_typed_action(ServerFileBrowserAction::SelectPreviousItem);
                        DispatchEventResult::StopPropagation
                    }
                    "down" => {
                        ctx.dispatch_typed_action(ServerFileBrowserAction::SelectNextItem);
                        DispatchEventResult::StopPropagation
                    }
                    "right" => {
                        ctx.dispatch_typed_action(ServerFileBrowserAction::ExpandSelectedItem);
                        DispatchEventResult::StopPropagation
                    }
                    "left" => {
                        ctx.dispatch_typed_action(ServerFileBrowserAction::CollapseSelectedItem);
                        DispatchEventResult::StopPropagation
                    }
                    "enter" => {
                        ctx.dispatch_typed_action(ServerFileBrowserAction::ExecuteSelectedItem);
                        DispatchEventResult::StopPropagation
                    }
                    "escape" => {
                        ctx.dispatch_typed_action(ServerFileBrowserAction::DismissContextMenu);
                        DispatchEventResult::StopPropagation
                    }
                    _ => DispatchEventResult::PropagateToParent,
                }
            })
            .finish()
    }
}

fn entries_with_depth(
    mut entries: Vec<ServerFileBrowserEntry>,
    depth: usize,
) -> Vec<ServerFileBrowserEntry> {
    for entry in &mut entries {
        entry.depth = depth;
    }
    entries
}

fn rebuild_entries_from(
    entries: Vec<ServerFileBrowserEntry>,
    expanded_directories: &HashSet<String>,
    loaded_directories: &HashMap<String, Vec<ServerFileBrowserEntry>>,
) -> Vec<ServerFileBrowserEntry> {
    let roots = entries
        .into_iter()
        .filter(|entry| entry.depth == 0)
        .collect();
    let mut rebuilt = Vec::new();
    append_entries_from(roots, expanded_directories, loaded_directories, &mut rebuilt);
    rebuilt
}

fn append_entries_from(
    entries: Vec<ServerFileBrowserEntry>,
    expanded_directories: &HashSet<String>,
    loaded_directories: &HashMap<String, Vec<ServerFileBrowserEntry>>,
    out: &mut Vec<ServerFileBrowserEntry>,
) {
    for entry in entries {
        let path = entry.path.clone();
        out.push(entry);
        if expanded_directories.contains(&path) {
            if let Some(children) = loaded_directories.get(&path) {
                append_entries_from(
                    children.clone(),
                    expanded_directories,
                    loaded_directories,
                    out,
                );
            }
        }
    }
}

fn previous_index(selected_index: Option<usize>, len: usize) -> Option<usize> {
    if len == 0 {
        None
    } else {
        Some(selected_index.unwrap_or(0).saturating_sub(1))
    }
}

fn next_index(selected_index: Option<usize>, len: usize) -> Option<usize> {
    if len == 0 {
        None
    } else {
        Some((selected_index.unwrap_or(0) + 1).min(len - 1))
    }
}

fn selected_index_after_rebuild(
    entries: &[ServerFileBrowserEntry],
    selected_path: Option<&str>,
    fallback_index: Option<usize>,
) -> Option<usize> {
    selected_path
        .and_then(|path| entries.iter().position(|entry| entry.path == path))
        .or_else(|| {
            (!entries.is_empty()).then_some(
                fallback_index
                    .unwrap_or(0)
                    .min(entries.len().saturating_sub(1)),
            )
        })
}

enum DirectoryListingSource {
    Client(Arc<RemoteServerClient>),
    Session(Arc<Session>),
}

struct DirectoryTreeReload {
    current_path: String,
    root_entries: Vec<ServerFileBrowserEntry>,
    loaded_directories: HashMap<String, Vec<ServerFileBrowserEntry>>,
    expanded_directories: HashSet<String>,
}

async fn list_directory_with_source(
    source: &DirectoryListingSource,
    path: String,
) -> Result<(String, Vec<ServerFileBrowserEntry>), String> {
    match source {
        DirectoryListingSource::Client(client) => list_directory(client.clone(), path).await,
        DirectoryListingSource::Session(session) => {
            list_directory_via_session(session.clone(), path).await
        }
    }
}

async fn fetch_directory_listings_selective(
    source: DirectoryListingSource,
    directories: HashSet<String>,
    depth_by_path: HashMap<String, usize>,
) -> Result<Vec<(String, Vec<ServerFileBrowserEntry>, usize)>, String> {
    let mut updates = Vec::new();
    for directory in directories {
        let depth = depth_by_path.get(&directory).copied().unwrap_or(1);
        let (canonical_path, entries) =
            list_directory_with_source(&source, directory.clone()).await?;
        updates.push((canonical_path, entries, depth));
    }
    Ok(updates)
}

async fn reload_directory_tree(
    source: DirectoryListingSource,
    current_path: String,
    expanded_directories: HashSet<String>,
    depth_by_path: HashMap<String, usize>,
) -> Result<DirectoryTreeReload, String> {
    let (current_path, root_entries) =
        list_directory_with_source(&source, current_path).await?;

    let mut loaded_directories = HashMap::new();
    let mut still_expanded = HashSet::new();
    for directory_path in expanded_directories {
        let depth = depth_by_path.get(&directory_path).copied().unwrap_or(1);
        match list_directory_with_source(&source, directory_path.clone()).await {
            Ok((canonical_path, entries)) => {
                still_expanded.insert(canonical_path.clone());
                loaded_directories.insert(
                    canonical_path,
                    entries_with_depth(entries, depth),
                );
            }
            Err(_) => {
                // The folder may have been removed or is no longer accessible.
            }
        }
    }

    Ok(DirectoryTreeReload {
        current_path,
        root_entries,
        loaded_directories,
        expanded_directories: still_expanded,
    })
}

async fn resolve_path(
    client: Arc<RemoteServerClient>,
    path: String,
) -> Result<ResolvedRemotePath, String> {
    let response = client.resolve_path(path).await.map_err(|error| error.to_string())?;
    match response.result {
        Some(resolve_path_response::Result::Success(success)) => {
            let kind = FileSystemEntryKind::try_from(success.kind)
                .unwrap_or(FileSystemEntryKind::Other);
            Ok(ResolvedRemotePath {
                canonical_path: success.canonical_path,
                kind,
            })
        }
        Some(resolve_path_response::Result::Error(error)) => Err(error.message),
        None => Err(crate::t!("server-file-browser-empty-response")),
    }
}

async fn list_directory(
    client: Arc<RemoteServerClient>,
    path: String,
) -> Result<(String, Vec<ServerFileBrowserEntry>), String> {
    let response = client.list_directory(path).await.map_err(|error| error.to_string())?;
    match response.result {
        Some(list_directory_response::Result::Success(success)) => {
            let canonical_path = success.canonical_path;
            let mut entries = Vec::with_capacity(success.entries.len());
            for entry in success.entries {
                let kind =
                    FileSystemEntryKind::try_from(entry.kind).unwrap_or(FileSystemEntryKind::Other);
                entries.push(ServerFileBrowserEntry {
                    path: join_remote_path(&canonical_path, &entry.name),
                    name: entry.name,
                    kind,
                    size_bytes: entry.size_bytes,
                    modified_epoch_millis: entry.modified_epoch_millis,
                    depth: 0,
                });
            }
            Ok((canonical_path, entries))
        }
        Some(list_directory_response::Result::Error(error)) => Err(error.message),
        None => Err(crate::t!("server-file-browser-empty-response")),
    }
}

/// Fallback directory listing via `Session::execute_command` when the
/// remote server daemon is not installed.
async fn list_directory_via_session(
    session: Arc<Session>,
    path: String,
) -> Result<(String, Vec<ServerFileBrowserEntry>), String> {
    let escaped = warp_util::path::ShellFamily::Posix.shell_escape(&path);
    let script = format!(
        "cd {escaped} && find . -maxdepth 1 -type d -print0 && printf '\\000' && find . -maxdepth 1 -not -type d -print0"
    );
    let output = session
        .execute_command(&script, None, None, ExecuteCommandOptions::default())
        .await
        .map_err(|e| format!("{e:#}"))?;

    if output.status != CommandExitStatus::Success {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ls failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut parts = stdout.split('\0');
    // Directories come first, separated from files by an empty entry
    // (double null). Find the separator.
    let mut dirs: Vec<&str> = Vec::new();
    let mut files: Vec<&str> = Vec::new();
    let mut found_separator = false;
    for part in parts.by_ref() {
        if part.is_empty() {
            found_separator = true;
            break;
        }
        if part != "." {
            dirs.push(part);
        }
    }
    if found_separator {
        for part in parts {
            if !part.is_empty() {
                files.push(part);
            }
        }
    }

    let mut entries = Vec::with_capacity(dirs.len() + files.len());
    // Canonical path: if the path is relative, use it as-is (the remote
    // host may not support canonicalize).
    let canonical_path = path.clone();
    for name in dirs {
        let name = Path::new(name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(name)
            .to_string();
        entries.push(ServerFileBrowserEntry {
            path: join_remote_path(&canonical_path, &name),
            name,
            kind: FileSystemEntryKind::Directory,
            size_bytes: None,
            modified_epoch_millis: None,
            depth: 0,
        });
    }
    for name in files {
        let name = Path::new(name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(name)
            .to_string();
        entries.push(ServerFileBrowserEntry {
            path: join_remote_path(&canonical_path, &name),
            name,
            kind: FileSystemEntryKind::File,
            size_bytes: None,
            modified_epoch_millis: None,
            depth: 0,
        });
    }
    // Sort alphabetically, directories first.
    entries.sort_by(|a, b| {
        let a_is_dir = a.kind == FileSystemEntryKind::Directory;
        let b_is_dir = b.kind == FileSystemEntryKind::Directory;
        b_is_dir
            .cmp(&a_is_dir)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok((canonical_path, entries))
}

/// Fallback path resolution via `Session::execute_command`.
async fn resolve_path_via_session(
    session: Arc<Session>,
    path: String,
) -> Result<ResolvedRemotePath, String> {
    let escaped = warp_util::path::ShellFamily::Posix.shell_escape(&path);
    // Use a single stat command to determine file type.
    let script = format!(
        "if [ -d {escaped} ]; then echo d; elif [ -f {escaped} ]; then echo f; elif [ -L {escaped} ]; then echo l; else echo o; fi"
    );
    let output = session
        .execute_command(&script, None, None, ExecuteCommandOptions::default())
        .await
        .map_err(|e| format!("{e:#}"))?;

    if output.status != CommandExitStatus::Success {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("stat failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let kind = match stdout.as_str() {
        "d" => FileSystemEntryKind::Directory,
        "f" => FileSystemEntryKind::File,
        "l" => FileSystemEntryKind::Symlink,
        _ => FileSystemEntryKind::Other,
    };

    Ok(ResolvedRemotePath {
        canonical_path: path,
        kind,
    })
}

#[derive(Clone)]
struct ResolvedRemotePath {
    canonical_path: String,
    kind: FileSystemEntryKind,
}

fn collect_upload_tasks(
    local_paths: Vec<PathBuf>,
    remote_directory: String,
    preserve_directory_root: bool,
) -> Result<(Vec<PendingUploadFile>, Vec<String>), String> {
    let mut files = Vec::new();
    let mut directory_roots = Vec::new();
    for local_path in local_paths {
        if local_path.is_dir() {
            let root_name = local_path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "upload".to_string());
            let root_remote = if preserve_directory_root {
                let root = join_remote_path(&remote_directory, &root_name);
                directory_roots.push(root.clone());
                root
            } else {
                remote_directory.clone()
            };
            for entry in WalkDir::new(&local_path).into_iter().filter_map(Result::ok) {
                let path = entry.path();
                let Ok(relative) = path.strip_prefix(&local_path) else {
                    continue;
                };
                if relative.as_os_str().is_empty() {
                    continue;
                }
                if entry.file_type().is_file() {
                    let relative_str = relative.to_string_lossy().to_string();
                    let final_remote_path = join_remote_path(&root_remote, &relative_str);
                    let display_name = if preserve_directory_root {
                        format!("{root_name}/{relative_str}")
                    } else {
                        relative_str
                    };
                    let total_bytes = std::fs::metadata(path)
                        .map(|meta| meta.len())
                        .unwrap_or(0);
                    files.push(PendingUploadFile {
                        local_path: path.to_path_buf(),
                        final_remote_path,
                        display_name,
                        total_bytes,
                    });
                }
            }
        } else if local_path.is_file() {
            let Some(name) = local_path.file_name().map(|name| name.to_string_lossy().to_string())
            else {
                continue;
            };
            let final_remote_path = join_remote_path(&remote_directory, &name);
            let total_bytes = std::fs::metadata(&local_path)
                .map(|meta| meta.len())
                .unwrap_or(0);
            files.push(PendingUploadFile {
                local_path,
                final_remote_path,
                display_name: name,
                total_bytes,
            });
        }
    }
    dedupe_pending_upload_files(&mut files);
    Ok((files, directory_roots))
}

fn dedupe_pending_upload_files(files: &mut Vec<PendingUploadFile>) {
    let mut seen = HashSet::new();
    files.retain(|file| seen.insert(file.final_remote_path.clone()));
}

async fn create_remote_entry(
    client: Arc<RemoteServerClient>,
    remote_directory: String,
    kind: NewRemoteEntryKind,
) -> Result<ServerFileBrowserEntry, String> {
    let (canonical_directory, entries) = list_directory(client.clone(), remote_directory).await?;
    let base_name = match kind {
        NewRemoteEntryKind::File => crate::t!("server-file-browser-default-file-name"),
        NewRemoteEntryKind::Directory => crate::t!("server-file-browser-default-folder-name"),
    };
    let name = next_available_entry_name(&base_name, &entries);
    let path = join_remote_path(&canonical_directory, &name);
    match kind {
        NewRemoteEntryKind::File => create_remote_file(client, path.clone()).await?,
        NewRemoteEntryKind::Directory => create_remote_directory(client, path.clone()).await?,
    }
    Ok(ServerFileBrowserEntry {
        name,
        path,
        kind: match kind {
            NewRemoteEntryKind::File => FileSystemEntryKind::File,
            NewRemoteEntryKind::Directory => FileSystemEntryKind::Directory,
        },
        size_bytes: Some(0),
        modified_epoch_millis: None,
        depth: 0,
    })
}

fn next_available_entry_name(base_name: &str, entries: &[ServerFileBrowserEntry]) -> String {
    let existing_names: HashSet<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();
    if !existing_names.contains(base_name) {
        return base_name.to_string();
    }
    for suffix in 2.. {
        let candidate = format!("{base_name} {suffix}");
        if !existing_names.contains(candidate.as_str()) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search must find an available name")
}

async fn create_remote_file(
    client: Arc<RemoteServerClient>,
    remote_path: String,
) -> Result<(), String> {
    let response = client
        .write_file_chunk(remote_path, 0, Vec::new(), true, None)
        .await
        .map_err(|error| error.to_string())?;
    match response.result {
        Some(write_file_chunk_response::Result::Success(_)) => Ok(()),
        Some(write_file_chunk_response::Result::Error(error)) => Err(error.message),
        None => Ok(()),
    }
}

async fn create_remote_directory(
    client: Arc<RemoteServerClient>,
    remote_path: String,
) -> Result<(), String> {
    let response = client
        .create_directory(remote_path)
        .await
        .map_err(|error| error.to_string())?;
    match response.result {
        Some(create_directory_response::Result::Success(_)) => Ok(()),
        Some(create_directory_response::Result::Error(error)) => Err(error.message),
        None => Ok(()),
    }
}

async fn upload_file_with_progress(
    client: Arc<RemoteServerClient>,
    local_path: PathBuf,
    remote_path: String,
    uploaded_bytes: Arc<AtomicU64>,
) -> Result<(), String> {
    let bytes = tokio::fs::read(local_path)
        .await
        .map_err(|error| error.to_string())?;
    let mut offset = 0;
    let mut truncate = true;
    for chunk in bytes.chunks(TRANSFER_CHUNK_BYTES as usize) {
        let response = client
            .write_file_chunk(remote_path.clone(), offset, chunk.to_vec(), truncate, None)
            .await
            .map_err(|error| error.to_string())?;
        match response.result {
            Some(write_file_chunk_response::Result::Success(success)) => {
                offset = success.next_offset;
                uploaded_bytes.store(offset, Ordering::Relaxed);
                truncate = false;
            }
            Some(write_file_chunk_response::Result::Error(error)) => return Err(error.message),
            None => return Err(crate::t!("server-file-browser-empty-response")),
        }
    }
    if bytes.is_empty() {
        let response = client
            .write_file_chunk(remote_path, 0, Vec::new(), true, None)
            .await
            .map_err(|error| error.to_string())?;
        if let Some(write_file_chunk_response::Result::Error(error)) = response.result {
            return Err(error.message);
        }
        uploaded_bytes.store(0, Ordering::Relaxed);
    }
    Ok(())
}

fn upload_task_progress(task: &ServerFileUploadTask) -> f32 {
    match &task.status {
        UploadTaskStatus::Pending => 0.0,
        UploadTaskStatus::Completed => 1.0,
        UploadTaskStatus::Failed(_) => 0.0,
        UploadTaskStatus::Skipped => 0.0,
        UploadTaskStatus::Uploading => {
            if task.total_bytes == 0 {
                0.0
            } else {
                let uploaded = task.uploaded_bytes.load(Ordering::Relaxed);
                (uploaded as f32 / task.total_bytes as f32).clamp(0.0, 1.0)
            }
        }
    }
}

fn render_flex_progress_bar(progress: f32, accent: pathfinder_color::ColorU, track: pathfinder_color::ColorU) -> Box<dyn Element> {
    let progress = progress.clamp(0.0, 1.0);
    let filled_weight = progress.max(0.001);
    let empty_weight = (1.0 - progress).max(0.001);
    let bar_height = 2.0;
    Flex::row()
        .with_child(
            Shrinkable::new(
                filled_weight,
                ConstrainedBox::new(
                    Container::new(Empty::new().finish())
                        .with_background(accent)
                        .finish(),
                )
                .with_height(bar_height)
                .finish(),
            )
            .finish(),
        )
        .with_child(
            Shrinkable::new(
                empty_weight,
                ConstrainedBox::new(
                    Container::new(Empty::new().finish())
                        .with_background(track)
                        .finish(),
                )
                .with_height(bar_height)
                .finish(),
            )
            .finish(),
        )
        .finish()
}

fn upload_batch_phase_label(phase: UploadBatchPhase) -> String {
    match phase {
        UploadBatchPhase::Uploading => crate::t!("server-file-browser-upload-phase-uploading"),
        UploadBatchPhase::Verifying => crate::t!("server-file-browser-upload-phase-verifying"),
        UploadBatchPhase::Promoting => crate::t!("server-file-browser-upload-phase-promoting"),
    }
}

fn upload_task_status_label(task: &ServerFileUploadTask, batch_phase: UploadBatchPhase) -> String {
    if matches!(task.status, UploadTaskStatus::Completed | UploadTaskStatus::Failed(_)) {
        match &task.status {
            UploadTaskStatus::Completed => {
                return crate::t!("server-file-browser-upload-status-completed");
            }
            UploadTaskStatus::Failed(error) => {
                return crate::t!("server-file-browser-upload-status-failed", error = error.clone());
            }
            UploadTaskStatus::Pending | UploadTaskStatus::Uploading | UploadTaskStatus::Skipped => {}
        }
    }
    match batch_phase {
        UploadBatchPhase::Verifying => {
            return crate::t!("server-file-browser-upload-status-verifying");
        }
        UploadBatchPhase::Promoting => {
            return crate::t!("server-file-browser-upload-status-promoting");
        }
        UploadBatchPhase::Uploading => {}
    }
    match &task.status {
        UploadTaskStatus::Pending => crate::t!("server-file-browser-upload-status-pending"),
        UploadTaskStatus::Uploading => {
            let percent = (upload_task_progress(task) * 100.0).round() as u32;
            crate::t!("server-file-browser-upload-status-uploading", percent = percent)
        }
        UploadTaskStatus::Completed => crate::t!("server-file-browser-upload-status-completed"),
        UploadTaskStatus::Failed(error) => {
            crate::t!("server-file-browser-upload-status-failed", error = error.clone())
        }
        UploadTaskStatus::Skipped => crate::t!("server-file-browser-upload-status-skipped"),
    }
}

fn relative_remote_path_from_base(base: &str, path: &str) -> String {
    let base_trimmed = base.trim_end_matches('/');
    if path == base_trimmed {
        return String::new();
    }
    if let Some(prefix) = child_path_prefix(base_trimmed) {
        if path.starts_with(&prefix) {
            return path[prefix.len()..].to_string();
        }
    }
    path.to_string()
}

fn path_is_under_conflict(path: &str, conflict_path: &str) -> bool {
    if path == conflict_path {
        return true;
    }
    child_path_prefix(conflict_path)
        .is_some_and(|prefix| path.starts_with(&prefix))
}

fn filter_upload_tasks_by_policy(
    files: Vec<PendingUploadFile>,
    policy: UploadConflictPolicy,
    conflict_paths: &HashSet<String>,
) -> Vec<PendingUploadFile> {
    if policy == UploadConflictPolicy::Proceed || policy == UploadConflictPolicy::OverwriteAll {
        return files;
    }
    files
        .into_iter()
        .filter(|file| {
            !conflict_paths.iter().any(|conflict| {
                path_is_under_conflict(&file.final_remote_path, conflict)
            })
        })
        .collect()
}

fn format_upload_conflict_summary(conflicts: &[UploadConflict]) -> String {
    let mut lines: Vec<String> = conflicts
        .iter()
        .take(8)
        .map(|conflict| {
            let kind_label = match conflict.kind {
                FileSystemEntryKind::Directory => {
                    crate::t!("server-file-browser-upload-conflict-kind-directory")
                }
                FileSystemEntryKind::File => {
                    crate::t!("server-file-browser-upload-conflict-kind-file")
                }
                FileSystemEntryKind::Symlink => {
                    crate::t!("server-file-browser-upload-conflict-kind-symlink")
                }
                FileSystemEntryKind::Other | FileSystemEntryKind::Unspecified => {
                    crate::t!("server-file-browser-upload-conflict-kind-other")
                }
            };
            format!("• {} ({kind_label})", conflict.display_name)
        })
        .collect();
    if conflicts.len() > 8 {
        lines.push(crate::t!(
            "server-file-browser-upload-conflict-more",
            count = ((conflicts.len() - 8) as i32)
        ));
    }
    format!(
        "{}\n\n{}",
        crate::t!("server-file-browser-upload-conflict-info"),
        lines.join("\n")
    )
}

fn append_reserved_path_conflicts(
    conflicts: &mut Vec<UploadConflict>,
    files: &[PendingUploadFile],
    reserved_paths: &HashSet<String>,
) {
    let existing: HashSet<String> = conflicts.iter().map(|c| c.path.clone()).collect();
    for file in files {
        if !reserved_paths.contains(&file.final_remote_path) {
            continue;
        }
        if existing.contains(&file.final_remote_path) {
            continue;
        }
        conflicts.push(UploadConflict {
            path: file.final_remote_path.clone(),
            display_name: file.display_name.clone(),
            kind: FileSystemEntryKind::File,
        });
    }
}

fn format_upload_promote_error(error: &str) -> String {
    if error.contains("not replacing") {
        if let Some(path) = error
            .split('\'')
            .nth(1)
            .filter(|segment| segment.starts_with('/'))
        {
            return crate::t!("server-file-browser-upload-promote-not-replacing", path = path);
        }
        return crate::t!("server-file-browser-upload-promote-not-replacing-generic");
    }
    error.to_string()
}

async fn scan_upload_conflicts(
    client: &RemoteServerClient,
    files: &[PendingUploadFile],
    directory_roots: &[String],
) -> Result<Vec<UploadConflict>, String> {
    let mut seen = HashSet::new();
    let mut conflicts = Vec::new();
    let mut paths_to_check: Vec<String> = files
        .iter()
        .map(|file| file.final_remote_path.clone())
        .collect();
    paths_to_check.extend(directory_roots.iter().cloned());
    for path in paths_to_check {
        if !seen.insert(path.clone()) {
            continue;
        }
        if let Some(conflict) = remote_path_conflict(client, &path).await? {
            conflicts.push(conflict);
        }
    }
    Ok(conflicts)
}

async fn remote_path_conflict(
    client: &RemoteServerClient,
    path: &str,
) -> Result<Option<UploadConflict>, String> {
    let response = client
        .resolve_path(path.to_string())
        .await
        .map_err(|error| error.to_string())?;
    match response.result {
        Some(resolve_path_response::Result::Success(success)) => {
            let display_name = remote_basename(path).unwrap_or_else(|| path.to_string());
            let kind = FileSystemEntryKind::try_from(success.kind)
                .unwrap_or(FileSystemEntryKind::Unspecified);
            Ok(Some(UploadConflict {
                path: path.to_string(),
                display_name,
                kind,
            }))
        }
        Some(resolve_path_response::Result::Error(_)) | None => Ok(None),
    }
}

async fn verify_staging_files(
    client: Arc<RemoteServerClient>,
    tasks: Vec<(String, u64)>,
) -> Result<(), String> {
    for (staging_path, expected_bytes) in tasks {
        let response = client
            .resolve_path(staging_path.clone())
            .await
            .map_err(|error| error.to_string())?;
        let Some(resolve_path_response::Result::Success(success)) = response.result else {
            return Err(crate::t!(
                "server-file-browser-upload-verify-missing",
                path = staging_path
            ));
        };
        let remote_size = success.size_bytes.unwrap_or(0);
        if remote_size != expected_bytes {
            return Err(crate::t!(
                "server-file-browser-upload-verify-size",
                path = staging_path
            ));
        }
    }
    Ok(())
}

fn staging_cleanup_shell_commands(staging_root: &str) -> String {
    let escaped_staging_root = warp_util::path::ShellFamily::Posix.shell_escape(staging_root);
    let mut commands = vec![format!("rm -rf -- {escaped_staging_root}")];
    if let Some(staging_parent) = remote_parent(staging_root) {
        let escaped_staging_parent =
            warp_util::path::ShellFamily::Posix.shell_escape(&staging_parent);
        commands.push(format!("rmdir -- {escaped_staging_parent} 2>/dev/null || true"));
    }
    commands.join("; ")
}

fn escape_for_single_quoted_trap_body(command: &str) -> String {
    command.replace('\'', "'\\''")
}

fn append_staging_cleanup_script(script_lines: &mut Vec<String>, staging_root: &str) {
    script_lines.push(staging_cleanup_shell_commands(staging_root));
}

async fn promote_staging_files(
    client: Arc<RemoteServerClient>,
    session: Option<Arc<Session>>,
    remote_session_id: Option<SessionId>,
    staging_root: String,
    conflict_policy: UploadConflictPolicy,
    directory_overwrite_roots: HashSet<String>,
    promote_pairs: Vec<(String, String)>,
) -> Result<(), String> {
    let mut script_lines = Vec::new();
    let cleanup_trap_body = escape_for_single_quoted_trap_body(&staging_cleanup_shell_commands(
        &staging_root,
    ));
    script_lines.push(format!("trap '{cleanup_trap_body}' EXIT"));
    script_lines.push("set -e".to_string());
    if conflict_policy == UploadConflictPolicy::OverwriteAll {
        let mut roots: Vec<String> = directory_overwrite_roots.into_iter().collect();
        roots.sort_by(|a, b| b.len().cmp(&a.len()));
        for root in roots {
            let escaped = warp_util::path::ShellFamily::Posix.shell_escape(&root);
            script_lines.push(format!("rm -rf -- {escaped}"));
        }
    }
    let mv_flag = if conflict_policy == UploadConflictPolicy::OverwriteAll {
        "-f"
    } else {
        "-n"
    };
    for (staging_path, final_path) in promote_pairs {
        let Some(parent) = remote_parent(&final_path) else {
            continue;
        };
        let escaped_parent = warp_util::path::ShellFamily::Posix.shell_escape(&parent);
        let escaped_staging = warp_util::path::ShellFamily::Posix.shell_escape(&staging_path);
        let escaped_final = warp_util::path::ShellFamily::Posix.shell_escape(&final_path);
        script_lines.push(format!("mkdir -p -- {escaped_parent}"));
        script_lines.push(format!(
            "mv {mv_flag} -- {escaped_staging} {escaped_final}"
        ));
    }
    let script = script_lines.join("\n");
    execute_remote_shell_script(session, Some(client), remote_session_id, script).await
}

async fn cleanup_staging_root(
    client: Arc<RemoteServerClient>,
    session: Option<Arc<Session>>,
    remote_session_id: Option<SessionId>,
    staging_root: String,
) -> Result<(), String> {
    let mut script_lines = Vec::new();
    append_staging_cleanup_script(&mut script_lines, &staging_root);
    let script = script_lines.join("\n");
    execute_remote_shell_script(session, Some(client), remote_session_id, script).await
}

async fn download_file(
    client: Arc<RemoteServerClient>,
    remote_path: String,
    local_path: PathBuf,
) -> Result<(), String> {
    if let Some(parent) = local_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| error.to_string())?;
    }
    let mut output = tokio::fs::File::create(local_path)
        .await
        .map_err(|error| error.to_string())?;
    let mut offset = 0;
    loop {
        let response = client
            .read_file_chunk(remote_path.clone(), offset, TRANSFER_CHUNK_BYTES)
            .await
            .map_err(|error| error.to_string())?;
        match response.result {
            Some(read_file_chunk_response::Result::Success(success)) => {
                use tokio::io::AsyncWriteExt;
                output
                    .write_all(&success.bytes)
                    .await
                    .map_err(|error| error.to_string())?;
                offset = success.next_offset;
                if success.eof {
                    break;
                }
            }
            Some(read_file_chunk_response::Result::Error(error)) => return Err(error.message),
            None => return Err(crate::t!("server-file-browser-empty-response")),
        }
    }
    Ok(())
}

async fn download_directory(
    client: Arc<RemoteServerClient>,
    remote_path: String,
    local_directory: PathBuf,
) -> Result<(), String> {
    let root_name = remote_basename(&remote_path).unwrap_or_else(|| "download".to_string());
    let root_destination = local_directory.join(root_name);
    tokio::fs::create_dir_all(&root_destination)
        .await
        .map_err(|error| error.to_string())?;
    download_directory_into(client, remote_path, root_destination).await
}

async fn download_directory_into(
    client: Arc<RemoteServerClient>,
    remote_path: String,
    local_directory: PathBuf,
) -> Result<(), String> {
    let (_, entries) = list_directory(client.clone(), remote_path).await?;
    for entry in entries {
        let local_path = local_directory.join(&entry.name);
        match entry.kind {
            FileSystemEntryKind::Directory => {
                tokio::fs::create_dir_all(&local_path)
                    .await
                    .map_err(|error| error.to_string())?;
                Box::pin(download_directory_into(client.clone(), entry.path, local_path)).await?;
            }
            FileSystemEntryKind::File
            | FileSystemEntryKind::Symlink
            | FileSystemEntryKind::Other
            | FileSystemEntryKind::Unspecified => {
                download_file(client.clone(), entry.path, local_path).await?;
            }
        }
    }
    Ok(())
}

async fn delete_remote_path(
    client: Option<Arc<RemoteServerClient>>,
    session: Option<Arc<Session>>,
    remote_session_id: Option<SessionId>,
    path: String,
    is_directory: bool,
) -> Result<(), String> {
    if is_directory {
        if session.is_none() && client.is_none() {
            return Err(crate::t!("server-file-browser-delete-requires-session"));
        }
        let escaped = warp_util::path::ShellFamily::Posix.shell_escape(&path);
        let script = format!("rm -rf -- {escaped}");
        return execute_remote_shell_script(session, client, remote_session_id, script).await;
    }

    if let Some(client) = &client {
        if client.delete_file(path.clone()).await.is_ok() {
            return Ok(());
        }
    }

    if session.is_none() && client.is_none() {
        return Err(crate::t!("server-file-browser-no-session"));
    }
    let escaped = warp_util::path::ShellFamily::Posix.shell_escape(&path);
    let script = format!("rm -f -- {escaped}");
    execute_remote_shell_script(session, client, remote_session_id, script).await
}

async fn rename_remote_path(
    session: Option<Arc<Session>>,
    client: Option<Arc<RemoteServerClient>>,
    remote_session_id: Option<SessionId>,
    from_path: String,
    new_name: String,
) -> Result<(), String> {
    let parent = remote_parent(&from_path).ok_or_else(|| {
        crate::t!("server-file-browser-operation-failed", error = "missing parent path")
    })?;
    let new_path = join_remote_path(&parent, &new_name);
    let escaped_from = warp_util::path::ShellFamily::Posix.shell_escape(&from_path);
    let escaped_to = warp_util::path::ShellFamily::Posix.shell_escape(&new_path);
    let script = format!("mv -- {escaped_from} {escaped_to}");
    execute_remote_shell_script(session, client, remote_session_id, script).await
}

async fn execute_remote_shell_script(
    session: Option<Arc<Session>>,
    client: Option<Arc<RemoteServerClient>>,
    remote_session_id: Option<SessionId>,
    script: String,
) -> Result<(), String> {
    if let Some(session) = session {
        let output = session
            .execute_command(&script, None, None, ExecuteCommandOptions::default())
            .await
            .map_err(|error| format!("{error:#}"))?;
        if output.status == CommandExitStatus::Success {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(stderr.trim().to_string())
        }
    } else if let (Some(client), Some(session_id)) = (client, remote_session_id) {
        let response = client
            .run_command(session_id, script, None, HashMap::new())
            .await
            .map_err(|error| format!("{error:#}"))?;
        match response.result {
            Some(run_command_response::Result::Success(success))
                if success.exit_code.unwrap_or(1) == 0 =>
            {
                Ok(())
            }
            Some(run_command_response::Result::Success(success)) => {
                let stderr = String::from_utf8_lossy(&success.stderr);
                Err(stderr.trim().to_string())
            }
            Some(run_command_response::Result::Error(err)) => Err(err.message),
            None => Err(crate::t!(
                "server-file-browser-operation-failed",
                error = "empty response"
            )),
        }
    } else {
        Err(crate::t!("server-file-browser-no-session"))
    }
}

fn child_path_prefix(path: &str) -> Option<String> {
    if path == "/" {
        None
    } else {
        Some(format!("{path}/"))
    }
}

fn remap_path_after_rename(path: &str, from_path: &str, new_path: &str) -> String {
    if path == from_path {
        return new_path.to_string();
    }
    let Some(from_prefix) = child_path_prefix(from_path) else {
        return path.to_string();
    };
    if path.starts_with(&from_prefix) {
        let suffix = &path[from_prefix.len()..];
        return join_remote_path(new_path, suffix);
    }
    path.to_string()
}

fn remap_loaded_directories_after_rename(
    loaded_directories: &mut HashMap<String, Vec<ServerFileBrowserEntry>>,
    from_path: &str,
    new_path: &str,
    new_name: &str,
    is_directory: bool,
) {
    if is_directory {
        let mut new_loaded = HashMap::new();
        for (dir_path, mut children) in loaded_directories.drain() {
            let remapped_dir = remap_path_after_rename(&dir_path, from_path, new_path);
            for child in &mut children {
                child.path = remap_path_after_rename(&child.path, from_path, new_path);
                if child.path == new_path {
                    child.name = new_name.to_string();
                }
            }
            new_loaded.insert(remapped_dir, children);
        }
        *loaded_directories = new_loaded;
        return;
    }

    for children in loaded_directories.values_mut() {
        for child in children {
            if child.path == from_path {
                child.path = new_path.to_string();
                child.name = new_name.to_string();
            }
        }
    }
}

fn join_remote_path(base: &str, name: &str) -> String {
    let normalized_name = name.replace('\\', "/");
    if base == "/" {
        format!("/{normalized_name}")
    } else if base.ends_with('/') {
        format!("{base}{normalized_name}")
    } else {
        format!("{base}/{normalized_name}")
    }
}

fn context_menu_submenu(
    label: String,
    icon: Icon,
    items: Vec<MenuItem<ServerFileBrowserAction>>,
) -> MenuItem<ServerFileBrowserAction> {
    MenuItem::Submenu {
        fields: MenuItemFields::new_submenu(label).with_icon(icon),
        menu: SubMenu::new(items),
    }
}

fn clear_context_menu_state<A: warpui::Action + Clone>(
    position: &mut Option<Vector2F>,
    items: &mut Vec<MenuItem<A>>,
) {
    *position = None;
    items.clear();
}

fn remote_parent(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches('/');
    let idx = trimmed.rfind('/')?;
    if idx == 0 {
        Some("/".to_string())
    } else {
        Some(trimmed[..idx].to_string())
    }
}

fn remote_basename(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .or_else(|| path.trim_end_matches('/').rsplit('/').next().map(str::to_string))
}

fn format_modified_epoch_millis(epoch_millis: u64) -> Option<String> {
    if epoch_millis == 0 {
        return None;
    }
    Local
        .timestamp_millis_opt(epoch_millis as i64)
        .single()
        .map(|timestamp| timestamp.format("%Y-%m-%d %H:%M").to_string())
}

fn format_file_size(size: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let size = size as f64;
    if size >= GB {
        format!("{:.1} GB", size / GB)
    } else if size >= MB {
        format!("{:.1} MB", size / MB)
    } else if size >= KB {
        format!("{:.1} KB", size / KB)
    } else {
        format!("{} B", size as u64)
    }
}

fn bound_remote_session_id(
    session_id: Option<SessionId>,
    is_session_connected: impl FnOnce(SessionId) -> bool,
) -> Option<SessionId> {
    let session_id = session_id?;
    is_session_connected(session_id).then_some(session_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        path: &str,
        name: &str,
        kind: FileSystemEntryKind,
        depth: usize,
    ) -> ServerFileBrowserEntry {
        ServerFileBrowserEntry {
            name: name.to_string(),
            path: path.to_string(),
            kind,
            size_bytes: None,
            modified_epoch_millis: None,
            depth,
        }
    }

    #[test]
    fn rebases_loaded_directory_entries_to_parent_depth() {
        let entries = vec![
            entry(
                "/root/.openwarp/remote-server/warp-oss",
                "warp-oss",
                FileSystemEntryKind::File,
                0,
            ),
            entry(
                "/root/.openwarp/remote-server/logs",
                "logs",
                FileSystemEntryKind::Directory,
                0,
            ),
        ];

        let entries = entries_with_depth(entries, 1);

        assert_eq!(
            entries.iter().map(|entry| entry.depth).collect::<Vec<_>>(),
            vec![1, 1]
        );
    }

    #[test]
    fn remap_path_after_rename_updates_directory_subtree() {
        assert_eq!(
            remap_path_after_rename(
                "/root/test/old",
                "/root/test/old",
                "/root/test/new"
            ),
            "/root/test/new"
        );
        assert_eq!(
            remap_path_after_rename(
                "/root/test/old/bin/warp-oss",
                "/root/test/old",
                "/root/test/new"
            ),
            "/root/test/new/bin/warp-oss"
        );
        assert_eq!(
            remap_path_after_rename("/root/other", "/root/test/old", "/root/test/new"),
            "/root/other"
        );
    }

    #[test]
    fn rebuild_entries_hides_listing_when_current_directory_uses_wrong_depth() {
        let mislabeled = entry(
            "/root/Lemon5.3.1.dmg",
            "Lemon5.3.1.dmg",
            FileSystemEntryKind::File,
            1,
        );
        let roots = [mislabeled]
            .into_iter()
            .filter(|entry| entry.depth == 0)
            .collect::<Vec<_>>();
        assert!(roots.is_empty());

        let fixed = entries_with_depth(
            vec![entry(
                "/root/Lemon5.3.1.dmg",
                "Lemon5.3.1.dmg",
                FileSystemEntryKind::File,
                0,
            )],
            0,
        );
        let rebuilt = rebuild_entries_from(fixed, &HashSet::new(), &HashMap::new());
        assert_eq!(rebuilt.len(), 1);
        assert_eq!(rebuilt[0].depth, 0);
    }

    #[test]
    fn rebuild_entries_does_not_promote_loaded_children_to_roots() {
        let root = entry(
            "/root/.openwarp/remote-server",
            "remote-server",
            FileSystemEntryKind::Directory,
            0,
        );
        let child = entry(
            "/root/.openwarp/remote-server/warp-oss",
            "warp-oss",
            FileSystemEntryKind::File,
            0,
        );
        let expanded_directories = HashSet::from([root.path.clone()]);
        let loaded_directories =
            HashMap::from([(root.path.clone(), entries_with_depth(vec![child], 1))]);

        let rebuilt = rebuild_entries_from(
            vec![root.clone()],
            &expanded_directories,
            &loaded_directories,
        );
        let rebuilt_again =
            rebuild_entries_from(rebuilt, &expanded_directories, &loaded_directories);

        assert_eq!(
            rebuilt_again
                .iter()
                .map(|entry| (entry.path.as_str(), entry.depth))
                .collect::<Vec<_>>(),
            vec![
                ("/root/.openwarp/remote-server", 0),
                ("/root/.openwarp/remote-server/warp-oss", 1),
            ]
        );
    }

    #[test]
    fn selected_index_navigation_stays_in_bounds() {
        assert_eq!(previous_index(None, 0), None);
        assert_eq!(next_index(None, 0), None);
        assert_eq!(previous_index(None, 3), Some(0));
        assert_eq!(previous_index(Some(0), 3), Some(0));
        assert_eq!(previous_index(Some(2), 3), Some(1));
        assert_eq!(next_index(None, 3), Some(1));
        assert_eq!(next_index(Some(1), 3), Some(2));
        assert_eq!(next_index(Some(2), 3), Some(2));
    }

    #[test]
    fn selected_index_preserves_matching_path_after_rebuild() {
        let entries = vec![
            entry("/root/.openwarp", ".openwarp", FileSystemEntryKind::Directory, 0),
            entry(
                "/root/.openwarp/remote-server",
                "remote-server",
                FileSystemEntryKind::Directory,
                1,
            ),
        ];

        assert_eq!(
            selected_index_after_rebuild(&entries, Some("/root/.openwarp/remote-server"), Some(0)),
            Some(1)
        );
    }

    #[test]
    fn format_modified_epoch_millis_rejects_zero_and_formats_timestamp() {
        assert_eq!(format_modified_epoch_millis(0), None);
        let formatted = format_modified_epoch_millis(1_700_000_000_000).expect("valid timestamp");
        assert!(formatted.contains('-') && formatted.contains(':'));
    }

    #[test]
    fn bound_remote_session_id_uses_bound_session_instead_of_host_fallback() {
        let first_session = SessionId::from(1);
        let second_session = SessionId::from(2);

        assert_eq!(
            bound_remote_session_id(Some(second_session), |session_id| {
                session_id == first_session || session_id == second_session
            }),
            Some(second_session)
        );
    }

    #[test]
    fn next_available_entry_name_appends_suffix_to_avoid_sibling_conflicts() {
        let entries = vec![
            entry("/root/untitled", "untitled", FileSystemEntryKind::File, 0),
            entry("/root/untitled 2", "untitled 2", FileSystemEntryKind::File, 0),
            entry(
                "/root/untitled folder",
                "untitled folder",
                FileSystemEntryKind::Directory,
                0,
            ),
        ];

        assert_eq!(next_available_entry_name("untitled", &entries), "untitled 3");
        assert_eq!(
            next_available_entry_name("untitled folder", &entries),
            "untitled folder 2"
        );
    }

    #[test]
    fn apply_rename_updates_loaded_directory_file_entries() {
        let mut loaded_directories = HashMap::from([(
            "/root/project".to_string(),
            vec![entry(
                "/root/project/untitled",
                "untitled",
                FileSystemEntryKind::File,
                1,
            )],
        )]);

        remap_loaded_directories_after_rename(
            &mut loaded_directories,
            "/root/project/untitled",
            "/root/project/renamed.txt",
            "renamed.txt",
            false,
        );

        let child = &loaded_directories["/root/project"][0];
        assert_eq!(child.path, "/root/project/renamed.txt");
        assert_eq!(child.name, "renamed.txt");
    }

    #[test]
    fn clear_context_menu_state_removes_items_and_selection() {
        let mut position = Some(vec2f(10.0, 20.0));
        let mut menu_items = vec![
            MenuItemFields::new("Refresh")
                .with_on_select_action(ServerFileBrowserAction::Refresh)
                .into_item(),
        ];

        clear_context_menu_state(&mut position, &mut menu_items);

        assert_eq!(position, None);
        assert!(menu_items.is_empty());
    }

    #[test]
    fn selected_index_falls_back_when_collapsed_child_disappears() {
        let entries = vec![entry(
            "/root/.openwarp",
            ".openwarp",
            FileSystemEntryKind::Directory,
            0,
        )];

        assert_eq!(
            selected_index_after_rebuild(
                &entries,
                Some("/root/.openwarp/remote-server"),
                Some(4),
            ),
            Some(0)
        );
    }
}
