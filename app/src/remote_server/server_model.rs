use crate::terminal::shell::ShellType;
use repo_metadata::repositories::{DetectedRepositories, RepoDetectionSource};
use repo_metadata::{RepoMetadataEvent, RepoMetadataModel, RepositoryIdentifier};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use warp_core::channel::ChannelState;
use warp_core::SessionId;
use warp_util::standardized_path::StandardizedPath;
use warpui::platform::TerminationMode;
use warpui::r#async::{Spawnable, SpawnableOutput, SpawnedFutureHandle};
use warpui::{Entity, ModelContext, SingletonEntity};

use warp_files::{FileModel, FileModelEvent};
use warp_util::content_version::ContentVersion;
use warp_util::file::FileId;

use super::proto::{
    client_message, delete_file_response, run_command_response, server_message,
    write_file_response, Abort, Authenticate, ClientMessage, DeleteFile, DeleteFileResponse,
    DeleteFileSuccess, ErrorCode, ErrorResponse, FailedFileRead, FileContextProto,
    FileOperationError, Initialize, InitializeResponse, NavigatedToDirectory,
    NavigatedToDirectoryResponse, ReadFileContextResponse, RunCommandError, RunCommandErrorCode,
    RunCommandRequest, RunCommandResponse, RunCommandSuccess, ServerMessage, SessionBootstrapped,
    WriteFile, WriteFileResponse, WriteFileSuccess,
};

// Buffer-sync 相关:依赖 GlobalBufferModel,后者的 server-local 操作只在
// `local_fs` 下可用,因此整套服务端 buffer 处理都按 `local_fs` 门控。
#[cfg(feature = "local_fs")]
use super::proto::{
    create_directory_response, list_directory_response, read_file_chunk_response,
    resolve_conflict_response, resolve_path_response, save_buffer_response,
    write_file_chunk_response, BufferEdit, BufferUpdatedPush, CloseBuffer, CreateDirectory,
    CreateDirectoryResponse, CreateDirectorySuccess, DirEntry, FileSystemEntryKind, ListDirectory,
    ListDirectoryResponse, ListDirectorySuccess, OpenBuffer, OpenBufferResponse, ReadFileChunk,
    ReadFileChunkResponse, ReadFileChunkSuccess, ResolveConflict, ResolveConflictResponse,
    ResolveConflictSuccess, ResolvePath, ResolvePathResponse, ResolvePathSuccess, SaveBuffer,
    SaveBufferResponse, SaveBufferSuccess, TextEdit, WriteFileChunk, WriteFileChunkResponse,
    WriteFileChunkSuccess,
};
#[cfg(feature = "local_fs")]
use super::server_buffer_tracker::{PendingBufferRequestKind, ServerBufferTracker};
#[cfg(feature = "local_fs")]
use crate::code::global_buffer_model::{GlobalBufferModel, GlobalBufferModelEvent};

/// How long the daemon waits with no connections before exiting.
pub const GRACE_PERIOD: std::time::Duration = std::time::Duration::from_secs(10 * 60);

/// Unique identifier for a connected proxy session in daemon mode.
pub type ConnectionId = uuid::Uuid;
use super::protocol::RequestId;
use crate::ai::agent::FileLocations;
use crate::ai::blocklist::{read_local_file_context, ReadFileContextResult};
use crate::terminal::model::session::command_executor::{
    ExecuteCommandOptions, LocalCommandExecutor,
};

/// Outcome of dispatching a request-style `ClientMessage`.
///
/// Notifications (fire-and-forget messages like `SessionBootstrapped` and
/// `Abort`) do not produce a `HandlerOutcome`; they are dispatched inline in
/// `handle_message` and return early.
enum HandlerOutcome {
    /// The response is ready synchronously — the caller sends it immediately.
    Sync(server_message::Message),
    /// The handler initiated async work whose response will be sent later.
    ///
    /// When the handle is `Some`, the caller inserts it into `in_progress`
    /// so the request can be cancelled via `Abort`. Removal on
    /// completion/abort is arranged by [`ServerModel::spawn_request_handler`].
    ///
    /// `None` is used for async work whose completion is delivered through
    /// a separate event subscription and is not currently cancellable via
    /// `Abort` (e.g. `FileModel` events for file writes and deletes, which
    /// are tracked by `FileId` in `pending_file_ops` rather than by
    /// `RequestId` in `in_progress`).
    Async(Option<SpawnedFutureHandle>),
}

#[cfg(test)]
impl HandlerOutcome {
    fn into_message(self) -> server_message::Message {
        match self {
            HandlerOutcome::Sync(message) => message,
            HandlerOutcome::Async(_) => panic!("expected synchronous handler outcome"),
        }
    }
}

/// Tracks an in-flight file write or delete so the async completion
/// event can be correlated back to the originating client request.
enum FileOpKind {
    Write,
    Delete,
}

struct PendingFileOp {
    request_id: RequestId,
    conn_id: ConnectionId,
    kind: FileOpKind,
}

/// Manages pending file operations and ensures that the corresponding
/// `FileModel` entry is always cleaned up when an operation completes
/// or fails, preventing `FileState` leaks.
struct PendingFileOps {
    ops: HashMap<FileId, PendingFileOp>,
}

impl PendingFileOps {
    fn new() -> Self {
        Self {
            ops: HashMap::new(),
        }
    }

    /// Registers a file path with `FileModel`, sets the initial version,
    /// and tracks the pending operation. Returns the `FileId` and
    /// `ContentVersion` for the caller to initiate the actual I/O.
    fn insert(
        &mut self,
        path: &Path,
        request_id: RequestId,
        conn_id: ConnectionId,
        kind: FileOpKind,
        ctx: &mut ModelContext<ServerModel>,
    ) -> (FileId, ContentVersion) {
        let file_model = FileModel::handle(ctx);
        let file_id = file_model.update(ctx, |m, ctx| m.register_file_path(path, false, ctx));
        let version = ContentVersion::new();
        file_model.update(ctx, |m, _| m.set_version(file_id, version));
        self.ops.insert(
            file_id,
            PendingFileOp {
                request_id,
                conn_id,
                kind,
            },
        );
        (file_id, version)
    }

    fn get(&self, file_id: &FileId) -> Option<&PendingFileOp> {
        self.ops.get(file_id)
    }

    /// Removes a pending operation and unsubscribes the file from `FileModel`,
    /// preventing the `FileState` entry from leaking.
    fn remove(
        &mut self,
        file_id: FileId,
        ctx: &mut ModelContext<ServerModel>,
    ) -> Option<PendingFileOp> {
        let op = self.ops.remove(&file_id)?;
        FileModel::handle(ctx).update(ctx, |m, ctx| m.unsubscribe(file_id, ctx));
        Some(op)
    }
}

/// The top-level server-side orchestrator model.
///
/// Receives `ClientMessage`s from connected proxy sessions and routes
/// `ServerMessage` responses and push notifications back through each
/// connection's dedicated sender channel.
pub struct ServerModel {
    /// Per-connection outbound channels, keyed by `ConnectionId`.
    ///
    /// The daemon can serve multiple proxy connections simultaneously — one
    /// per SSH session / Zap tab connecting to this host.  Each entry maps
    /// a connection's `Uuid` to the channel the connection task drains to
    /// write `ServerMessage`s back to its proxy.
    connection_senders: HashMap<ConnectionId, async_channel::Sender<ServerMessage>>,
    /// Per-connection set of repo roots for which we've already sent a
    /// snapshot in this connection's lifetime.
    ///
    /// Used to avoid sending duplicate snapshots on repeated
    /// `NavigatedToDirectory` calls while the user `cd`s within the same repo.
    snapshot_sent_roots_by_connection: HashMap<ConnectionId, HashSet<StandardizedPath>>,
    /// Abort handle for the active grace timer, if any.
    /// Calling `.abort()` cancels the timer before it fires.
    grace_timer_cancel: Option<SpawnedFutureHandle>,
    /// Tracks in-progress requests that can be cancelled via `Abort`.
    /// Calling `.abort()` on the handle cancels the background future and
    /// triggers its `on_abort` callback.
    in_progress: HashMap<RequestId, SpawnedFutureHandle>,
    /// Stable host identifier generated once at process startup.
    /// Returned in every `InitializeResponse` so clients can deduplicate
    /// host-scoped models.
    host_id: String,
    /// Per-session command executors created from `SessionBootstrapped` notifications.
    executors: HashMap<SessionId, Arc<LocalCommandExecutor>>,
    /// Tracks in-flight file write/delete operations and handles cleanup.
    pending_file_ops: PendingFileOps,
    /// Tracks open server-local buffers, their connections, and pending
    /// buffer requests (OpenBuffer, SaveBuffer, ResolveConflict).
    #[cfg(feature = "local_fs")]
    buffers: ServerBufferTracker,
    /// Daemon-wide bearer credential for the identity-scoped daemon.
    ///
    /// The token is written by Initialize when the client supplies a
    /// non-empty credential, or by Authenticate during token rotation. It is
    /// intentionally retained across proxy connection teardown and cleared
    /// only by daemon process exit.
    auth_token: Option<String>,
}

impl Entity for ServerModel {
    type Event = ();
}

impl SingletonEntity for ServerModel {}

impl ServerModel {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let host_id = uuid::Uuid::new_v4().to_string();
        log::info!(
            "Daemon started: PID={}, host_id={}",
            std::process::id(),
            host_id
        );
        let mut model = Self {
            connection_senders: HashMap::new(),
            snapshot_sent_roots_by_connection: HashMap::new(),
            grace_timer_cancel: None,
            in_progress: HashMap::new(),
            host_id,
            executors: HashMap::new(),
            pending_file_ops: PendingFileOps::new(),
            #[cfg(feature = "local_fs")]
            buffers: ServerBufferTracker::new(),
            auth_token: None,
        };
        // Subscribe to FileModel and RepoMetadataModel events
        // file operation results and repo metadata pushes are forwarded to all
        // connected proxy sessions.
        {
            let file_model = FileModel::handle(ctx);
            ctx.subscribe_to_model(&file_model, |me, event, ctx| {
                let file_id = event.file_id();
                let Some(pending_kind) = me.pending_file_ops.get(&file_id).map(|op| &op.kind)
                else {
                    return; // Not a file op we're tracking.
                };
                let response_message = match (event, pending_kind) {
                    (FileModelEvent::FileSaved { .. }, FileOpKind::Write) => {
                        server_message::Message::WriteFileResponse(WriteFileResponse {
                            result: Some(write_file_response::Result::Success(WriteFileSuccess {})),
                        })
                    }
                    (FileModelEvent::FileSaved { .. }, FileOpKind::Delete) => {
                        server_message::Message::DeleteFileResponse(DeleteFileResponse {
                            result: Some(delete_file_response::Result::Success(
                                DeleteFileSuccess {},
                            )),
                        })
                    }
                    (FileModelEvent::FailedToSave { error, .. }, FileOpKind::Write) => {
                        server_message::Message::WriteFileResponse(WriteFileResponse {
                            result: Some(write_file_response::Result::Error(FileOperationError {
                                message: format!("{error}"),
                            })),
                        })
                    }
                    (FileModelEvent::FailedToSave { error, .. }, FileOpKind::Delete) => {
                        server_message::Message::DeleteFileResponse(DeleteFileResponse {
                            result: Some(delete_file_response::Result::Error(FileOperationError {
                                message: format!("{error}"),
                            })),
                        })
                    }
                    (FileModelEvent::FileLoaded { .. }, _)
                    | (FileModelEvent::FailedToLoad { .. }, _)
                    | (FileModelEvent::FileUpdated { .. }, _) => return,
                };
                // Remove the pending op and unsubscribe from FileModel.
                let pending = me
                    .pending_file_ops
                    .remove(file_id, ctx)
                    .expect("pending op was confirmed present");
                me.send_server_message(
                    Some(pending.conn_id),
                    Some(&pending.request_id),
                    response_message,
                );
            });
        }
        {
            let repo_model = RepoMetadataModel::handle(ctx);
            ctx.subscribe_to_model(&repo_model, |me, event, ctx| match event {
                RepoMetadataEvent::IncrementalUpdateReady { update } => {
                    me.send_server_message(
                        None,
                        None,
                        server_message::Message::RepoMetadataUpdate(update.into()),
                    );
                }
                RepoMetadataEvent::RepositoryUpdated {
                    id: RepositoryIdentifier::Local(path),
                } => {
                    // A repo finished indexing — push the full tree as a snapshot.
                    let id = RepositoryIdentifier::local(path.clone());
                    let repo_model = RepoMetadataModel::handle(ctx);
                    if let Some(state) = repo_model.as_ref(ctx).get_repository(&id, ctx) {
                        let entries = super::repo_metadata_proto::file_tree_entry_to_snapshot_proto(
                            &state.entry,
                        );
                        me.send_server_message(
                            None,
                            None,
                            server_message::Message::RepoMetadataSnapshot(
                                super::proto::RepoMetadataSnapshot {
                                    repo_path: path.to_string(),
                                    entries,
                                    sync_complete: true,
                                },
                            ),
                        );
                        // Mark this root as snapshot-sent for all active connections
                        // so subsequent NavigatedToDirectory calls skip re-sending.
                        for sent_roots in me.snapshot_sent_roots_by_connection.values_mut() {
                            sent_roots.insert(path.clone());
                        }
                    }
                }
                RepoMetadataEvent::RepositoryRemoved { .. }
                | RepoMetadataEvent::FileTreeUpdated { .. }
                | RepoMetadataEvent::FileTreeEntryUpdated { .. }
                | RepoMetadataEvent::UpdatingRepositoryFailed { .. }
                | RepoMetadataEvent::RepositoryUpdated {
                    id: RepositoryIdentifier::Remote(_),
                } => {}
            });
        }
        // Subscribe to GlobalBufferModel events for server-local buffers.
        #[cfg(feature = "local_fs")]
        {
            let gbm = GlobalBufferModel::handle(ctx);
            ctx.subscribe_to_model(&gbm, |me, event, ctx| match event {
                GlobalBufferModelEvent::BufferLoaded { file_id, .. } => {
                    // Complete all pending OpenBuffer requests for this file.
                    let pending = me
                        .buffers
                        .take_pending_by_kind(file_id, PendingBufferRequestKind::OpenBuffer);
                    if !pending.is_empty() {
                        let gbm = GlobalBufferModel::handle(ctx);
                        let content = gbm.as_ref(ctx).content_for_file(*file_id, ctx);
                        let server_version = gbm
                            .as_ref(ctx)
                            .sync_clock_for_server_local(*file_id)
                            .map(|c| c.server_version.as_u64());

                        for (request_id, conn_id) in pending {
                            let message = match (&content, server_version) {
                                (Some(content), Some(sv)) => {
                                    server_message::Message::OpenBufferResponse(
                                        OpenBufferResponse {
                                            content: content.clone(),
                                            server_version: sv,
                                        },
                                    )
                                }
                                _ => server_message::Message::Error(ErrorResponse {
                                    code: ErrorCode::Internal.into(),
                                    message: format!(
                                        "Buffer loaded but content or sync clock unavailable for file {file_id:?}"
                                    ),
                                }),
                            };
                            me.send_server_message(Some(conn_id), Some(&request_id), message);
                        }
                    }
                }
                GlobalBufferModelEvent::ServerLocalBufferUpdated {
                    file_id,
                    edits,
                    new_server_version,
                    expected_client_version,
                } => {
                    // Push incremental edits to all connections that have this buffer open.
                    let Some(conns) = me.buffers.connections_for_buffer(file_id) else {
                        return;
                    };
                    // Find the path for this file_id; abort the push if tracker
                    // state is inconsistent (空 path 会破坏 path↔buffer 契约)。
                    let Some(path) = me.buffers.path_for_file_id(*file_id) else {
                        log::error!(
                            "Missing path mapping for server-local buffer file_id={file_id:?}"
                        );
                        return;
                    };

                    let proto_edits: Vec<TextEdit> = edits
                        .iter()
                        .map(|edit| TextEdit {
                            start_offset: edit.start.as_usize() as u64,
                            end_offset: edit.end.as_usize() as u64,
                            text: edit.text.clone(),
                        })
                        .collect();

                    let conns: Vec<_> = conns.iter().copied().collect();
                    for conn_id in conns {
                        me.send_server_message(
                            Some(conn_id),
                            None,
                            server_message::Message::BufferUpdated(BufferUpdatedPush {
                                path: path.clone(),
                                new_server_version: new_server_version.as_u64(),
                                expected_client_version: expected_client_version.as_u64(),
                                edits: proto_edits.clone(),
                            }),
                        );
                    }
                }
                GlobalBufferModelEvent::FileSaved { file_id } => {
                    for (request_id, conn_id) in me
                        .buffers
                        .take_pending_by_kind(file_id, PendingBufferRequestKind::SaveBuffer)
                    {
                        me.send_server_message(
                            Some(conn_id),
                            Some(&request_id),
                            server_message::Message::SaveBufferResponse(SaveBufferResponse {
                                result: Some(save_buffer_response::Result::Success(
                                    SaveBufferSuccess {},
                                )),
                            }),
                        );
                    }
                    for (request_id, conn_id) in me
                        .buffers
                        .take_pending_by_kind(file_id, PendingBufferRequestKind::ResolveConflict)
                    {
                        me.send_server_message(
                            Some(conn_id),
                            Some(&request_id),
                            server_message::Message::ResolveConflictResponse(
                                ResolveConflictResponse {
                                    result: Some(resolve_conflict_response::Result::Success(
                                        ResolveConflictSuccess {},
                                    )),
                                },
                            ),
                        );
                    }
                }
                GlobalBufferModelEvent::FailedToSave { file_id, error } => {
                    for (request_id, conn_id) in me
                        .buffers
                        .take_pending_by_kind(file_id, PendingBufferRequestKind::SaveBuffer)
                    {
                        me.send_server_message(
                            Some(conn_id),
                            Some(&request_id),
                            server_message::Message::SaveBufferResponse(SaveBufferResponse {
                                result: Some(save_buffer_response::Result::Error(
                                    FileOperationError {
                                        message: format!("{error}"),
                                    },
                                )),
                            }),
                        );
                    }
                    for (request_id, conn_id) in me
                        .buffers
                        .take_pending_by_kind(file_id, PendingBufferRequestKind::ResolveConflict)
                    {
                        me.send_server_message(
                            Some(conn_id),
                            Some(&request_id),
                            server_message::Message::ResolveConflictResponse(
                                ResolveConflictResponse {
                                    result: Some(resolve_conflict_response::Result::Error(
                                        FileOperationError {
                                            message: format!("{error}"),
                                        },
                                    )),
                                },
                            ),
                        );
                    }
                }
                GlobalBufferModelEvent::FailedToLoad { file_id, error } => {
                    for (request_id, conn_id) in me
                        .buffers
                        .take_pending_by_kind(file_id, PendingBufferRequestKind::OpenBuffer)
                    {
                        me.send_server_message(
                            Some(conn_id),
                            Some(&request_id),
                            server_message::Message::Error(ErrorResponse {
                                code: ErrorCode::Internal.into(),
                                message: format!("Failed to load buffer: {error}"),
                            }),
                        );
                    }
                }
                GlobalBufferModelEvent::BufferUpdatedFromFileEvent { .. }
                | GlobalBufferModelEvent::RemoteBufferConflict { .. } => {
                    // Not relevant for server-local buffers.
                }
            });
        }
        // Start the grace timer immediately so the daemon exits if no proxy
        // connects within GRACE_PERIOD. In practice the spawning proxy connects
        // within milliseconds, so the risk of premature shutdown is negligible;
        // register_connection will cancel the timer the moment the first proxy
        // arrives.
        model.start_grace_timer(ctx);
        model
    }

    /// Called when a proxy connects.  Inserts `conn_tx` into the connection
    /// map so `send_server_message` can route responses to this proxy, and
    /// cancels the grace timer if it was running.
    pub fn register_connection(
        &mut self,
        conn_id: ConnectionId,
        conn_tx: async_channel::Sender<ServerMessage>,
        ctx: &mut ModelContext<Self>,
    ) {
        log::info!(
            "Daemon: connection {conn_id} registered — {} active, host_id={}",
            self.connection_senders.len() + 1,
            self.host_id
        );
        if let Some(handle) = self.grace_timer_cancel.take() {
            handle.abort();
        }
        self.connection_senders.insert(conn_id, conn_tx);
        self.snapshot_sent_roots_by_connection
            .insert(conn_id, HashSet::new());
        ctx.notify();
    }

    /// Called when a proxy disconnects.  Removes it from the connection map
    /// and starts the grace timer if no connections remain.
    pub fn deregister_connection(&mut self, conn_id: ConnectionId, ctx: &mut ModelContext<Self>) {
        self.snapshot_sent_roots_by_connection.remove(&conn_id);
        // Guard against double-deregister (reader and writer tasks both call
        // this on connection close; the second call must be a safe no-op).
        if self.connection_senders.remove(&conn_id).is_none() {
            return;
        }
        // Drop this connection from all open server-local buffers; orphaned
        // buffers (no remaining connections) are deallocated by the tracker.
        #[cfg(feature = "local_fs")]
        self.buffers.remove_connection(conn_id, ctx);
        let remaining = self.connection_senders.len();
        log::info!("Daemon: connection {conn_id} deregistered — {remaining} active remaining");
        if remaining == 0 {
            log::info!("Daemon: grace timer started ({GRACE_PERIOD:?})");
            self.start_grace_timer(ctx);
        }
        ctx.notify();
    }

    /// Starts (or restarts) a timer that shuts the daemon down after
    /// [`GRACE_PERIOD`] with no connected proxies.  If a timer is already
    /// running its abort handle is cancelled before the new one is stored.
    /// When a proxy connects, `register_connection` aborts the handle,
    /// preventing the shutdown.
    fn start_grace_timer(&mut self, ctx: &mut ModelContext<Self>) {
        if let Some(handle) = self.grace_timer_cancel.take() {
            handle.abort();
        }
        let handle = ctx.spawn_abortable(
            async_io::Timer::after(GRACE_PERIOD),
            |_, _, ctx| {
                log::info!("Daemon: grace period expired, shutting down");
                ctx.terminate_app(TerminationMode::ForceTerminate, None);
            },
            |_, _| {
                log::debug!("Daemon: grace timer cancelled");
            },
        );
        self.grace_timer_cancel = Some(handle);
    }

    /// Called by the background stdin reader task via `ModelSpawner`.
    ///
    /// Dispatches on the `oneof message` variant. Notifications are handled
    /// inline; request-style messages return a `HandlerOutcome` that is
    /// centrally acted on here: `Sync` responses are sent immediately and
    /// `Async` handles are tracked in `in_progress` so they can be aborted.
    pub fn handle_message(
        &mut self,
        conn_id: ConnectionId,
        msg: ClientMessage,
        ctx: &mut ModelContext<Self>,
    ) {
        let request_id = RequestId::from(msg.request_id);

        let outcome = match msg.message {
            Some(client_message::Message::Initialize(msg)) => {
                self.handle_initialize(msg, &request_id)
            }
            Some(client_message::Message::Authenticate(msg)) => {
                self.handle_authenticate(msg);
                return;
            }
            Some(client_message::Message::SessionBootstrapped(msg)) => {
                self.handle_session_bootstrapped(msg);
                return;
            }
            Some(client_message::Message::Abort(abort)) => {
                self.handle_abort(abort, &request_id);
                return;
            }
            Some(client_message::Message::RunCommand(req)) => {
                self.handle_run_command(req, &request_id, conn_id, ctx)
            }
            Some(client_message::Message::NavigatedToDirectory(msg)) => {
                self.handle_navigated_to_directory(msg, &request_id, conn_id, ctx)
            }
            Some(client_message::Message::LoadRepoMetadataDirectory(msg)) => {
                self.handle_load_repo_metadata_directory(msg, &request_id, ctx)
            }
            Some(client_message::Message::WriteFile(msg)) => {
                self.handle_write_file(msg, &request_id, conn_id, ctx)
            }
            Some(client_message::Message::DeleteFile(msg)) => {
                self.handle_delete_file(msg, &request_id, conn_id, ctx)
            }
            Some(client_message::Message::ReadFileContext(msg)) => {
                self.handle_read_file_context(msg, &request_id, conn_id, ctx)
            }
            #[cfg(feature = "local_fs")]
            Some(client_message::Message::OpenBuffer(msg)) => {
                self.handle_open_buffer(msg, &request_id, conn_id, ctx)
            }
            #[cfg(feature = "local_fs")]
            Some(client_message::Message::BufferEdit(msg)) => {
                self.handle_buffer_edit(msg, ctx);
                return; // fire-and-forget notification
            }
            #[cfg(feature = "local_fs")]
            Some(client_message::Message::CloseBuffer(msg)) => {
                self.handle_close_buffer(msg, conn_id, ctx);
                return; // fire-and-forget notification
            }
            #[cfg(feature = "local_fs")]
            Some(client_message::Message::SaveBuffer(msg)) => {
                self.handle_save_buffer(msg, &request_id, conn_id, ctx)
            }
            #[cfg(feature = "local_fs")]
            Some(client_message::Message::ResolveConflict(msg)) => {
                self.handle_resolve_conflict(msg, &request_id, conn_id, ctx)
            }
            // Zap:远端终端文件链接的目录列举(校验路径形态用)。
            #[cfg(feature = "local_fs")]
            Some(client_message::Message::ListDirectory(msg)) => self.handle_list_directory(msg),
            #[cfg(feature = "local_fs")]
            Some(client_message::Message::ResolvePath(msg)) => self.handle_resolve_path(msg),
            #[cfg(feature = "local_fs")]
            Some(client_message::Message::CreateDirectory(msg)) => self.handle_create_directory(msg),
            #[cfg(feature = "local_fs")]
            Some(client_message::Message::ReadFileChunk(msg)) => self.handle_read_file_chunk(msg),
            #[cfg(feature = "local_fs")]
            Some(client_message::Message::WriteFileChunk(msg)) => self.handle_write_file_chunk(msg),
            #[cfg(not(feature = "local_fs"))]
            Some(
                client_message::Message::OpenBuffer(_)
                | client_message::Message::BufferEdit(_)
                | client_message::Message::CloseBuffer(_)
                | client_message::Message::SaveBuffer(_)
                | client_message::Message::ResolveConflict(_)
                | client_message::Message::ListDirectory(_)
                | client_message::Message::ResolvePath(_)
                | client_message::Message::CreateDirectory(_)
                | client_message::Message::ReadFileChunk(_)
                | client_message::Message::WriteFileChunk(_),
            ) => HandlerOutcome::Sync(server_message::Message::Error(ErrorResponse {
                code: ErrorCode::InvalidRequest.into(),
                message: "Buffer syncing requires the local_fs feature".to_string(),
            })),
            None => {
                log::warn!(
                    "Received ClientMessage with no message variant (request_id={request_id})"
                );
                HandlerOutcome::Sync(server_message::Message::Error(ErrorResponse {
                    code: ErrorCode::InvalidRequest.into(),
                    message: "ClientMessage had no message variant set".to_string(),
                }))
            }
        };

        match outcome {
            HandlerOutcome::Sync(message) => {
                self.send_server_message(Some(conn_id), Some(&request_id), message);
            }
            HandlerOutcome::Async(Some(handle)) => {
                self.in_progress.insert(request_id, handle);
            }
            HandlerOutcome::Async(None) => {
                // Async work tracked elsewhere (e.g. `pending_file_ops`);
                // the response will be sent via an event subscription.
            }
        }
    }

    /// Routes a server message to its destination.
    ///
    /// - `conn_id = Some(id)` — sends only to the connection that originated
    ///   the request (used for all request/response pairs).
    /// - `conn_id = None` — broadcasts to every connected proxy (used for
    ///   server-initiated push notifications such as repo metadata updates).
    fn send_server_message(
        &self,
        conn_id: Option<ConnectionId>,
        request_id: Option<&RequestId>,
        message: server_message::Message,
    ) {
        let msg = ServerMessage {
            request_id: request_id.map(|id| id.clone().into()).unwrap_or_default(),
            message: Some(message),
        };
        if let Some(target) = conn_id {
            if let Some(conn_tx) = self.connection_senders.get(&target) {
                if let Err(e) = conn_tx.try_send(msg) {
                    log::warn!("Daemon: failed to send to conn {target}: {e}");
                }
            } else {
                log::debug!("Daemon: no sender for conn {target} (already disconnected)");
            }
        } else {
            // Push notification — broadcast to all connections.
            for (id, conn_tx) in &self.connection_senders {
                if let Err(e) = conn_tx.try_send(msg.clone()) {
                    log::warn!("Daemon: failed to send to conn {id}: {e}");
                }
            }
        }
    }

    /// Spawns an abortable future tied to `request_id` and wires up automatic
    /// removal from `in_progress` on completion or abort.
    ///
    /// The returned handle is intended to be returned from a handler as
    /// `HandlerOutcome::Async(Some(handle))`; the caller (`handle_message`)
    /// inserts it into `in_progress`.
    fn spawn_request_handler<S, F>(
        &mut self,
        request_id: RequestId,
        future: S,
        on_resolve: F,
        ctx: &mut ModelContext<Self>,
    ) -> SpawnedFutureHandle
    where
        S: Spawnable,
        <S as Future>::Output: SpawnableOutput,
        F: 'static + FnOnce(&mut Self, <S as Future>::Output, &mut ModelContext<Self>),
    {
        let resolve_id = request_id.clone();
        let abort_id = request_id;
        ctx.spawn_abortable(
            future,
            move |me, output, ctx| {
                me.in_progress.remove(&resolve_id);
                on_resolve(me, output, ctx);
            },
            move |me, _ctx| {
                log::info!("Request cancelled (request_id={abort_id})");
                me.in_progress.remove(&abort_id);
            },
        )
    }

    /// Handles `Initialize` by returning the server version and host id.
    ///
    /// `server_version` is the release tag the daemon was built from
    /// (`GIT_RELEASE_TAG`) or the empty string for `cargo run` / locally
    /// deployed builds. The client treats an empty version as "unknown" and
    /// skips strict version enforcement, which keeps the
    /// `script/deploy_remote_server` developer workflow functional.
    fn handle_initialize(&mut self, msg: Initialize, request_id: &RequestId) -> HandlerOutcome {
        log::info!("Handling Initialize (request_id={request_id})");
        if !msg.auth_token.is_empty() {
            self.auth_token = Some(msg.auth_token);
        }
        let server_version = ChannelState::app_version().unwrap_or("").to_string();
        HandlerOutcome::Sync(server_message::Message::InitializeResponse(
            InitializeResponse {
                server_version,
                host_id: self.host_id.clone(),
            },
        ))
    }

    /// Handles `Authenticate` by replacing the daemon-wide credential.
    /// This is a notification — no response is sent.
    fn handle_authenticate(&mut self, msg: Authenticate) {
        if msg.auth_token.is_empty() {
            log::warn!("Received Authenticate notification with empty auth token; ignoring");
            return;
        }
        self.auth_token = Some(msg.auth_token);
    }

    pub fn auth_token(&self) -> Option<&str> {
        self.auth_token.as_deref()
    }

    /// Handles `Abort` by cancelling the in-progress request it targets.
    /// This is a notification — no response is sent.
    fn handle_abort(&mut self, abort: Abort, request_id: &RequestId) {
        let target_id = RequestId::from(abort.request_id_to_abort);
        if let Some(handle) = self.in_progress.remove(&target_id) {
            log::info!(
                "Aborting in-progress request (request_id={target_id}, \
                 abort_request_id={request_id})"
            );
            handle.abort();
        } else {
            log::info!(
                "Abort for unknown/completed request (request_id={target_id}, \
                 abort_request_id={request_id})"
            );
        }
    }

    /// Handles `SessionBootstrapped` by creating a `LocalCommandExecutor` for
    /// the session. This is a notification — no response is sent.
    fn handle_session_bootstrapped(&mut self, msg: SessionBootstrapped) {
        let session_id = SessionId::from(msg.session_id);
        log::info!(
            "Handling SessionBootstrapped: session_id={session_id:?}, \
             shell_type={:?}, shell_path={:?}",
            msg.shell_type,
            msg.shell_path,
        );

        let Some(shell_type) = ShellType::from_name(&msg.shell_type) else {
            log::error!(
                "Unknown shell_type {:?} in SessionBootstrapped for session {session_id:?}",
                msg.shell_type,
            );
            return;
        };

        let shell_path = msg.shell_path.map(PathBuf::from);
        if shell_path.is_none() {
            log::warn!(
                "SessionBootstrapped for session {session_id:?} had no shell_path; \
                 LocalCommandExecutor will fall back to bare shell name",
            );
        }
        let executor = Arc::new(LocalCommandExecutor::new(shell_path, shell_type));
        if self.executors.insert(session_id, executor).is_some() {
            log::warn!(
                "Overwriting existing executor for session {session_id:?} \
                 (re-SessionBootstrapped with shell_type={:?})",
                msg.shell_type,
            );
        }
    }

    /// Handles `RunCommand` by delegating to the session's `LocalCommandExecutor`.
    ///
    /// On success, returns a `HandlerOutcome::Async` whose task resolves the
    /// request with a `RunCommandResponse`. On validation failure (missing
    /// executor), returns a `HandlerOutcome::Sync` error response.
    fn handle_run_command(
        &mut self,
        req: RunCommandRequest,
        request_id: &RequestId,
        conn_id: ConnectionId,
        ctx: &mut ModelContext<Self>,
    ) -> HandlerOutcome {
        let session_id = SessionId::from(req.session_id);
        log::info!(
            "Handling RunCommand (request_id={request_id}, session_id={session_id:?}): \
             command={:?}, cwd={:?}",
            req.command,
            req.working_directory,
        );

        let command = req.command;
        let cwd = req.working_directory;
        let env_vars = if req.environment_variables.is_empty() {
            None
        } else {
            Some(req.environment_variables)
        };

        let Some(executor) = self.executors.get(&session_id).cloned() else {
            log::error!("No executor for session {session_id:?}, session was never initialized");
            return HandlerOutcome::Sync(server_message::Message::RunCommandResponse(
                RunCommandResponse {
                    result: Some(run_command_response::Result::Error(RunCommandError {
                        code: RunCommandErrorCode::SessionNotFound.into(),
                        message: format!("No executor for session {session_id:?}"),
                    })),
                },
            ));
        };

        // Call `execute_local_command` directly because the
        // `CommandExecutor::execute_command` trait method requires
        // a `&Shell` (version, options, plugins from bootstrap).
        let request_id_for_response = request_id.clone();
        let conn_id_for_response = conn_id;
        let handle = self.spawn_request_handler(
            request_id.clone(),
            async move {
                executor
                    .execute_local_command(
                        &command,
                        cwd.as_deref(),
                        env_vars,
                        ExecuteCommandOptions::default(),
                    )
                    .await
            },
            move |me, result, _ctx| {
                let result_oneof = match result {
                    Ok(output) => {
                        log::info!(
                            "RunCommand completed (request_id={request_id_for_response}): \
                             exit_code={:?}, stdout_len={}, stderr_len={}",
                            output.exit_code,
                            output.stdout.len(),
                            output.stderr.len(),
                        );
                        run_command_response::Result::Success(RunCommandSuccess {
                            stdout: output.stdout.clone(),
                            stderr: output.stderr.clone(),
                            exit_code: output.exit_code.map(|c| c.value()),
                        })
                    }
                    Err(e) => {
                        log::warn!("RunCommand failed (request_id={request_id_for_response}): {e}");
                        run_command_response::Result::Error(RunCommandError {
                            code: RunCommandErrorCode::ExecutionFailed.into(),
                            message: format!("Failed to execute command: {e}"),
                        })
                    }
                };
                me.send_server_message(
                    Some(conn_id_for_response),
                    Some(&request_id_for_response),
                    server_message::Message::RunCommandResponse(RunCommandResponse {
                        result: Some(result_oneof),
                    }),
                );
            },
            ctx,
        );
        HandlerOutcome::Async(Some(handle))
    }

    /// Handles `NavigatedToDirectory` by running git detection first, then
    /// responding. On validation failure returns a `HandlerOutcome::Sync` error;
    /// otherwise spawns a task and returns a `HandlerOutcome::Async(Some(_))`
    /// handle.
    fn handle_navigated_to_directory(
        &mut self,
        msg: NavigatedToDirectory,
        request_id: &RequestId,
        conn_id: ConnectionId,
        ctx: &mut ModelContext<Self>,
    ) -> HandlerOutcome {
        log::info!(
            "Handling NavigatedToDirectory path={} (request_id={request_id})",
            msg.path
        );

        let std_path = match StandardizedPath::from_local_canonicalized(Path::new(&msg.path)) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("Invalid path for NavigatedToDirectory: {e}");
                return HandlerOutcome::Sync(server_message::Message::Error(ErrorResponse {
                    code: ErrorCode::InvalidRequest.into(),
                    message: format!("Invalid path: {e}"),
                }));
            }
        };

        // Kick off git detection. The returned future resolves with the git
        // root path (Some) or None if no git repo was found.
        let path_str = msg.path.clone();
        let git_future = DetectedRepositories::handle(ctx).update(ctx, |repos, ctx| {
            repos.detect_possible_git_repo(&path_str, RepoDetectionSource::TerminalNavigation, ctx)
        });

        let request_id_for_response = request_id.clone();
        let conn_id_for_response = conn_id;
        let handle = self.spawn_request_handler(
            request_id.clone(),
            git_future,
            move |me, git_root, ctx| {
                let (indexed_path, is_git) = if let Some(root) = git_root {
                    // Git repo found. Full indexing was already triggered by
                    // DetectedGitRepo → LocalRepoMetadataModel. The client
                    // waits for RepositoryIndexedPush before FetchFileTree.
                    let root_str = root.to_string_lossy().to_string();
                    log::info!("Git repo detected at {root_str} for path {}", std_path);
                    (root_str, true)
                } else {
                    // No git repo. Lazy-load the directory for first-level data,
                    // then push the snapshot immediately.
                    RepoMetadataModel::handle(ctx).update(ctx, |repo_model, ctx| {
                        if let Err(e) = repo_model.index_lazy_loaded_path(&std_path, ctx) {
                            log::warn!("Failed to lazy-load directory {std_path}: {e}");
                        }
                    });
                    (std_path.to_string(), false)
                };

                me.send_server_message(
                    Some(conn_id_for_response),
                    Some(&request_id_for_response),
                    server_message::Message::NavigatedToDirectoryResponse(
                        NavigatedToDirectoryResponse {
                            indexed_path: indexed_path.clone(),
                            is_git,
                        },
                    ),
                );

                // After responding, push a snapshot if metadata is available.
                //
                // For git repos this is an opportunistic push for the case
                // where the repo was already indexed and RepositoryUpdated
                // won't fire again (which would otherwise leave the client
                // with only a placeholder root). We skip if a snapshot was
                // already sent for this connection+root.
                //
                // For non-git directories the lazy-loaded tree is always
                // broadcast to all connections.
                if let Ok(root_path) =
                    StandardizedPath::from_local_canonicalized(Path::new(&indexed_path))
                {
                    if is_git {
                        let already_sent = me
                            .snapshot_sent_roots_by_connection
                            .get(&conn_id_for_response)
                            .is_some_and(|roots| roots.contains(&root_path));
                        if already_sent {
                            log::debug!(
                                "Snapshot already sent for repo {indexed_path} \
                                 to conn {conn_id_for_response}, skipping"
                            );
                            return;
                        }
                    }

                    let id = RepositoryIdentifier::local(root_path.clone());
                    let repo_model = RepoMetadataModel::handle(ctx);
                    if let Some(state) = repo_model.as_ref(ctx).get_repository(&id, ctx) {
                        let entries = super::repo_metadata_proto::file_tree_entry_to_snapshot_proto(
                            &state.entry,
                        );
                        // Git snapshots target the requesting connection;
                        // non-git snapshots broadcast to all.
                        let target = if is_git {
                            Some(conn_id_for_response)
                        } else {
                            None
                        };
                        me.send_server_message(
                            target,
                            None,
                            server_message::Message::RepoMetadataSnapshot(
                                super::proto::RepoMetadataSnapshot {
                                    repo_path: indexed_path,
                                    entries,
                                    sync_complete: true,
                                },
                            ),
                        );
                        if is_git {
                            if let Some(sent_roots) = me
                                .snapshot_sent_roots_by_connection
                                .get_mut(&conn_id_for_response)
                            {
                                sent_roots.insert(root_path);
                            }
                        }
                    }
                }
            },
            ctx,
        );
        HandlerOutcome::Async(Some(handle))
    }

    /// Handles `LoadRepoMetadataDirectory` by loading a subdirectory on the
    /// server's local model and returning the children synchronously.
    fn handle_load_repo_metadata_directory(
        &mut self,
        msg: super::proto::LoadRepoMetadataDirectory,
        request_id: &RequestId,
        ctx: &mut ModelContext<Self>,
    ) -> HandlerOutcome {
        log::info!(
            "Handling LoadRepoMetadataDirectory repo_path={} dir_path={} (request_id={request_id})",
            msg.repo_path,
            msg.dir_path
        );

        let repo_path = match StandardizedPath::from_local_canonicalized(Path::new(&msg.repo_path))
        {
            Ok(p) => p,
            Err(e) => {
                return HandlerOutcome::Sync(server_message::Message::Error(ErrorResponse {
                    code: ErrorCode::InvalidRequest.into(),
                    message: format!("Invalid repo_path: {e}"),
                }));
            }
        };

        let dir_path = match StandardizedPath::from_local_canonicalized(Path::new(&msg.dir_path)) {
            Ok(p) => p,
            Err(e) => {
                return HandlerOutcome::Sync(server_message::Message::Error(ErrorResponse {
                    code: ErrorCode::InvalidRequest.into(),
                    message: format!("Invalid dir_path: {e}"),
                }));
            }
        };

        // Validate that the directory is a descendant of the repo.
        if !dir_path.starts_with(&repo_path) {
            return HandlerOutcome::Sync(server_message::Message::Error(ErrorResponse {
                code: ErrorCode::InvalidRequest.into(),
                message: format!(
                    "dir_path {dir_path} is not a descendant of repo_path {repo_path}"
                ),
            }));
        }

        // Load the directory on the server's local model.
        let load_result = RepoMetadataModel::handle(ctx).update(ctx, |model, ctx| {
            model.load_directory(&repo_path, &dir_path, ctx)
        });

        if let Err(e) = load_result {
            log::warn!("LoadRepoMetadataDirectory failed: {e}");
            return HandlerOutcome::Sync(server_message::Message::Error(ErrorResponse {
                code: ErrorCode::Internal.into(),
                message: format!("Failed to load directory: {e}"),
            }));
        }

        // Read back the loaded children and serialize them.
        let id = RepositoryIdentifier::local(repo_path.clone());
        let entries = RepoMetadataModel::handle(ctx)
            .as_ref(ctx)
            .get_repository(&id, ctx)
            .map(|state| {
                super::repo_metadata_proto::file_tree_children_to_proto_entries(
                    &state.entry,
                    &dir_path,
                )
            })
            .unwrap_or_default();

        HandlerOutcome::Sync(server_message::Message::LoadRepoMetadataDirectoryResponse(
            super::proto::LoadRepoMetadataDirectoryResponse {
                repo_path: msg.repo_path,
                dir_path: msg.dir_path,
                entries,
            },
        ))
    }

    /// Handles `WriteFile` by registering the path and triggering an async
    /// write via `FileModel`. On a successful dispatch, returns
    /// `HandlerOutcome::Async(None)` — the response is sent later by the
    /// `FileModel` event subscription, and the op is not cancellable via
    /// `Abort`. On failure to dispatch, returns a `HandlerOutcome::Sync`
    /// error response.
    fn handle_write_file(
        &mut self,
        msg: WriteFile,
        request_id: &RequestId,
        conn_id: ConnectionId,
        ctx: &mut ModelContext<Self>,
    ) -> HandlerOutcome {
        log::info!(
            "Handling WriteFile path={} (request_id={request_id})",
            msg.path
        );
        let path = Path::new(&msg.path);

        let (file_id, version) =
            self.pending_file_ops
                .insert(path, request_id.clone(), conn_id, FileOpKind::Write, ctx);

        let file_model = FileModel::handle(ctx);
        if let Err(err) =
            file_model.update(ctx, |m, ctx| m.save(file_id, msg.content, version, ctx))
        {
            self.pending_file_ops.remove(file_id, ctx);
            return HandlerOutcome::Sync(server_message::Message::WriteFileResponse(
                WriteFileResponse {
                    result: Some(write_file_response::Result::Error(FileOperationError {
                        message: format!("Failed to initiate write: {err}"),
                    })),
                },
            ));
        }

        // Response sent asynchronously via the event subscription.
        HandlerOutcome::Async(None)
    }

    /// Handles `DeleteFile` by registering the path and triggering an async
    /// delete via `FileModel`. On a successful dispatch, returns
    /// `HandlerOutcome::Async(None)` — the response is sent later by the
    /// `FileModel` event subscription, and the op is not cancellable via
    /// `Abort`. On failure to dispatch, returns a `HandlerOutcome::Sync`
    /// error response.
    fn handle_delete_file(
        &mut self,
        msg: DeleteFile,
        request_id: &RequestId,
        conn_id: ConnectionId,
        ctx: &mut ModelContext<Self>,
    ) -> HandlerOutcome {
        log::info!(
            "Handling DeleteFile path={} (request_id={request_id})",
            msg.path
        );
        let path = Path::new(&msg.path);

        let (file_id, version) = self.pending_file_ops.insert(
            path,
            request_id.clone(),
            conn_id,
            FileOpKind::Delete,
            ctx,
        );

        let file_model = FileModel::handle(ctx);
        if let Err(err) = file_model.update(ctx, |m, ctx| m.delete(file_id, version, ctx)) {
            self.pending_file_ops.remove(file_id, ctx);
            return HandlerOutcome::Sync(server_message::Message::DeleteFileResponse(
                DeleteFileResponse {
                    result: Some(delete_file_response::Result::Error(FileOperationError {
                        message: format!("Failed to initiate delete: {err}"),
                    })),
                },
            ));
        }

        // Response sent asynchronously via the event subscription.
        HandlerOutcome::Async(None)
    }

    /// Handles `ReadFileContext` by spawning an async batch file read on the
    /// background executor. Returns `HandlerOutcome::Async` with the spawned
    /// handle so the request can be cancelled via `Abort`.
    fn handle_read_file_context(
        &mut self,
        msg: super::proto::ReadFileContextRequest,
        request_id: &RequestId,
        conn_id: ConnectionId,
        ctx: &mut ModelContext<Self>,
    ) -> HandlerOutcome {
        log::info!(
            "Handling ReadFileContext ({} files, request_id={request_id})",
            msg.files.len()
        );

        let max_file_bytes = msg.max_file_bytes.map(|b| b as usize);
        let max_batch_bytes = msg.max_batch_bytes.map(|b| b as usize);
        let file_locations: Vec<FileLocations> = msg
            .files
            .into_iter()
            .map(|f| FileLocations {
                name: f.path,
                lines: f
                    .line_ranges
                    .into_iter()
                    .map(|r| r.start as usize..r.end as usize)
                    .collect(),
            })
            .collect();
        let request_id_for_response = request_id.clone();

        let handle = self.spawn_request_handler(
            request_id.clone(),
            async move {
                read_local_file_context(
                    &file_locations,
                    None,
                    None,
                    max_file_bytes,
                    max_batch_bytes,
                )
                .await
            },
            move |me, result: anyhow::Result<ReadFileContextResult>, _ctx| {
                let response = match result {
                    Ok(result) => file_context_result_to_proto(result),
                    Err(err) => ReadFileContextResponse {
                        file_contexts: vec![],
                        failed_files: vec![FailedFileRead {
                            path: String::new(),
                            error: Some(FileOperationError {
                                message: format!("{err:#}"),
                            }),
                        }],
                    },
                };
                me.send_server_message(
                    Some(conn_id),
                    Some(&request_id_for_response),
                    server_message::Message::ReadFileContextResponse(response),
                );
            },
            ctx,
        );

        HandlerOutcome::Async(Some(handle))
    }

    /// Handles `OpenBuffer` by opening the file via `GlobalBufferModel`.
    /// The response is sent asynchronously when `BufferLoaded` fires.
    #[cfg(feature = "local_fs")]
    fn handle_open_buffer(
        &mut self,
        msg: OpenBuffer,
        request_id: &RequestId,
        conn_id: ConnectionId,
        ctx: &mut ModelContext<Self>,
    ) -> HandlerOutcome {
        log::info!(
            "Handling OpenBuffer path={path} (request_id={request_id})",
            path = msg.path
        );

        let path = PathBuf::from(&msg.path);
        let gbm = GlobalBufferModel::handle(ctx);
        let buffer_state = gbm.update(ctx, |gbm, ctx| gbm.open_server_local(path, ctx));
        let file_id = buffer_state.file_id;

        // Track path → FileId mapping and connection。track_open_buffer 同时持有
        // buffer 的强引用 —— daemon 没有编辑器 view,不持有的话 buffer 会在
        // FileModel 异步加载完成前被回收(见 ServerBufferTracker::buffer_handles)。
        self.buffers
            .track_open_buffer(msg.path.clone(), file_id, buffer_state.buffer);
        self.buffers.add_connection(file_id, conn_id);

        // If already loaded, respond immediately.
        if gbm.as_ref(ctx).buffer_loaded(file_id) {
            let content = gbm
                .as_ref(ctx)
                .content_for_file(file_id, ctx)
                .unwrap_or_default();
            let server_version = gbm
                .as_ref(ctx)
                .sync_clock_for_server_local(file_id)
                .map(|c| c.server_version.as_u64())
                .unwrap_or(1);
            return HandlerOutcome::Sync(server_message::Message::OpenBufferResponse(
                OpenBufferResponse {
                    content,
                    server_version,
                },
            ));
        }

        // Not yet loaded — stash request info so the GlobalBufferModelEvent
        // subscription can send the response when content arrives.
        self.buffers.insert_pending(
            file_id,
            request_id.clone(),
            conn_id,
            PendingBufferRequestKind::OpenBuffer,
        );
        HandlerOutcome::Async(None)
    }

    /// Handles `BufferEdit` notification (fire-and-forget).
    /// Delegates to `GlobalBufferModel::apply_client_edit`. On rejection
    /// (stale server version), the edit is silently dropped.
    #[cfg(feature = "local_fs")]
    fn handle_buffer_edit(&mut self, msg: BufferEdit, ctx: &mut ModelContext<Self>) {
        let Some(file_id) = self.buffers.file_id_for_path(&msg.path) else {
            log::warn!("BufferEdit for unknown buffer: {path}", path = msg.path);
            return;
        };

        let expected_sv = ContentVersion::from_wire_u64(msg.expected_server_version);
        let new_cv = ContentVersion::from_wire_u64(msg.new_client_version);

        // Per spec: if the edit is rejected (stale server version),
        // the server silently drops it.
        GlobalBufferModel::handle(ctx).update(ctx, |gbm, ctx| {
            gbm.apply_client_edit(file_id, &msg.edits, expected_sv, new_cv, ctx);
        });
    }

    /// Handles `SaveBuffer` by persisting the buffer to disk.
    #[cfg(feature = "local_fs")]
    fn handle_save_buffer(
        &mut self,
        msg: SaveBuffer,
        request_id: &RequestId,
        conn_id: ConnectionId,
        ctx: &mut ModelContext<Self>,
    ) -> HandlerOutcome {
        log::info!(
            "Handling SaveBuffer path={path} (request_id={request_id})",
            path = msg.path
        );

        let Some(file_id) = self.buffers.file_id_for_path(&msg.path) else {
            return HandlerOutcome::Sync(server_message::Message::SaveBufferResponse(
                SaveBufferResponse {
                    result: Some(save_buffer_response::Result::Error(FileOperationError {
                        message: format!("Buffer not open: {path}", path = msg.path),
                    })),
                },
            ));
        };

        let result = GlobalBufferModel::handle(ctx)
            .update(ctx, |gbm, ctx| gbm.save_server_local(file_id, ctx));

        match result {
            Ok(()) => {
                // Response will come via the FileSaved event subscription.
                // Track the file_id → (request_id, conn_id) so the event
                // handler can correlate.
                self.buffers.insert_pending(
                    file_id,
                    request_id.clone(),
                    conn_id,
                    PendingBufferRequestKind::SaveBuffer,
                );
                HandlerOutcome::Async(None)
            }
            Err(err) => HandlerOutcome::Sync(server_message::Message::SaveBufferResponse(
                SaveBufferResponse {
                    result: Some(save_buffer_response::Result::Error(FileOperationError {
                        message: format!("Failed to save: {err}"),
                    })),
                },
            )),
        }
    }

    /// Handles `ResolveConflict` by replacing the server buffer with the
    /// client's content and persisting to disk. Returns an async
    /// `HandlerOutcome` — the response is sent when `FileSaved` or
    /// `FailedToSave` fires.
    #[cfg(feature = "local_fs")]
    fn handle_resolve_conflict(
        &mut self,
        msg: ResolveConflict,
        request_id: &RequestId,
        conn_id: ConnectionId,
        ctx: &mut ModelContext<Self>,
    ) -> HandlerOutcome {
        log::info!(
            "Handling ResolveConflict path={path} (request_id={request_id})",
            path = msg.path
        );

        let Some(file_id) = self.buffers.file_id_for_path(&msg.path) else {
            return HandlerOutcome::Sync(server_message::Message::ResolveConflictResponse(
                ResolveConflictResponse {
                    result: Some(resolve_conflict_response::Result::Error(
                        FileOperationError {
                            message: format!("Buffer not open: {path}", path = msg.path),
                        },
                    )),
                },
            ));
        };

        let ack_sv = ContentVersion::from_wire_u64(msg.acknowledged_server_version);
        let current_cv = ContentVersion::from_wire_u64(msg.current_client_version);
        let result = GlobalBufferModel::handle(ctx).update(ctx, |gbm, ctx| {
            gbm.resolve_conflict(file_id, ack_sv, current_cv, &msg.client_content, ctx)
        });

        match result {
            Ok(()) => {
                self.buffers.insert_pending(
                    file_id,
                    request_id.clone(),
                    conn_id,
                    PendingBufferRequestKind::ResolveConflict,
                );
                HandlerOutcome::Async(None)
            }
            Err(err) => HandlerOutcome::Sync(server_message::Message::ResolveConflictResponse(
                ResolveConflictResponse {
                    result: Some(resolve_conflict_response::Result::Error(
                        FileOperationError {
                            message: format!("Failed to resolve conflict: {err}"),
                        },
                    )),
                },
            )),
        }
    }

    /// Zap:处理 `ListDirectory` —— 同步列举一个目录下的直接子项。
    ///
    /// 给远端终端文件链接检测做精确校验用:客户端缓存某个 cwd 下的
    /// 真实目录项,链接检测器据此从 `ls -l` 整行里切出正确的文件名。
    /// `std::fs::read_dir` 在 daemon 端是廉价的同步调用,故直接返回
    /// `HandlerOutcome::Sync`,不走异步 spawn。
    #[cfg(feature = "local_fs")]
    fn handle_list_directory(&self, msg: ListDirectory) -> HandlerOutcome {
        log::info!("Handling ListDirectory path={}", msg.path);

        let path = expand_user_path(&msg.path);
        let result = match std::fs::read_dir(&path) {
            Ok(read_dir) => {
                let mut entries = Vec::new();
                for entry in read_dir.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    // 优先用 `file_type()`(不跟随符号链接、无需额外 stat);
                    // 失败时回退到 `metadata()`(会跟随符号链接)。
                    let file_type = entry.file_type().ok();
                    let metadata = entry.metadata().ok();
                    let kind = entry_kind(file_type.as_ref(), metadata.as_ref());
                    let is_dir = kind == FileSystemEntryKind::Directory as i32;
                    let size_bytes =
                        metadata.as_ref().filter(|m| m.is_file()).map(|m| m.len());
                    let modified_epoch_millis = metadata
                        .as_ref()
                        .and_then(|m| m.modified().ok())
                        .and_then(system_time_to_epoch_millis);
                    entries.push(DirEntry {
                        name,
                        is_dir,
                        kind,
                        size_bytes,
                        modified_epoch_millis,
                    });
                }
                entries.sort_by(|a, b| a.name.cmp(&b.name));
                let canonical_path = path
                    .canonicalize()
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();
                list_directory_response::Result::Success(ListDirectorySuccess {
                    entries,
                    canonical_path,
                })
            }
            Err(err) => list_directory_response::Result::Error(FileOperationError {
                message: format!("Failed to list directory {}: {err}", msg.path),
            }),
        };

        HandlerOutcome::Sync(server_message::Message::ListDirectoryResponse(
            ListDirectoryResponse {
                result: Some(result),
            },
        ))
    }

    #[cfg(feature = "local_fs")]
    fn handle_resolve_path(&self, msg: ResolvePath) -> HandlerOutcome {
        let path = expand_user_path(&msg.path);
        let result = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => {
                let file_type = metadata.file_type();
                let kind = entry_kind(Some(&file_type), Some(&metadata));
                let canonical_path = path
                    .canonicalize()
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();
                resolve_path_response::Result::Success(ResolvePathSuccess {
                    canonical_path,
                    kind,
                    size_bytes: metadata.is_file().then_some(metadata.len()),
                })
            }
            Err(err) => resolve_path_response::Result::Error(FileOperationError {
                message: format!("Failed to resolve path {}: {err}", msg.path),
            }),
        };

        HandlerOutcome::Sync(server_message::Message::ResolvePathResponse(
            ResolvePathResponse {
                result: Some(result),
            },
        ))
    }

    #[cfg(feature = "local_fs")]
    fn handle_create_directory(&self, msg: CreateDirectory) -> HandlerOutcome {
        let path = expand_user_path(&msg.path);
        let result = match std::fs::create_dir_all(&path) {
            Ok(()) => {
                let canonical_path = path
                    .canonicalize()
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();
                create_directory_response::Result::Success(CreateDirectorySuccess {
                    canonical_path,
                })
            }
            Err(err) => create_directory_response::Result::Error(FileOperationError {
                message: format!("Failed to create directory {}: {err}", msg.path),
            }),
        };

        HandlerOutcome::Sync(server_message::Message::CreateDirectoryResponse(
            CreateDirectoryResponse {
                result: Some(result),
            },
        ))
    }

    #[cfg(feature = "local_fs")]
    fn handle_read_file_chunk(&self, msg: ReadFileChunk) -> HandlerOutcome {
        use std::io::{Read, Seek, SeekFrom};

        let path = expand_user_path(&msg.path);
        let result = (|| -> std::io::Result<ReadFileChunkSuccess> {
            let mut file = std::fs::File::open(&path)?;
            let total_size = file.metadata().ok().map(|m| m.len());
            file.seek(SeekFrom::Start(msg.offset))?;
            let max_bytes = msg.max_bytes.min(8 * 1024 * 1024) as usize;
            let mut bytes = vec![0; max_bytes];
            let read = file.read(&mut bytes)?;
            bytes.truncate(read);
            let next_offset = msg.offset + read as u64;
            let eof = total_size.is_some_and(|size| next_offset >= size) || read == 0;
            Ok(ReadFileChunkSuccess {
                bytes,
                next_offset,
                total_size,
                eof,
            })
        })();

        let result = match result {
            Ok(success) => read_file_chunk_response::Result::Success(success),
            Err(err) => read_file_chunk_response::Result::Error(FileOperationError {
                message: format!("Failed to read file chunk {}: {err}", msg.path),
            }),
        };

        HandlerOutcome::Sync(server_message::Message::ReadFileChunkResponse(
            ReadFileChunkResponse {
                result: Some(result),
            },
        ))
    }

    #[cfg(feature = "local_fs")]
    fn handle_write_file_chunk(&self, msg: WriteFileChunk) -> HandlerOutcome {
        use std::io::{Seek, SeekFrom, Write};

        let path = expand_user_path(&msg.path);
        let result = (|| -> std::io::Result<WriteFileChunkSuccess> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut options = std::fs::OpenOptions::new();
            options.create(true).write(true);
            if msg.truncate {
                options.truncate(true);
            }
            let mut file = options.open(&path)?;
            file.seek(SeekFrom::Start(msg.offset))?;
            file.write_all(&msg.bytes)?;
            #[cfg(unix)]
            if let Some(executable) = msg.executable {
                use std::os::unix::fs::PermissionsExt;

                let mode = if executable { 0o755 } else { 0o644 };
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))?;
            }
            Ok(WriteFileChunkSuccess {
                next_offset: msg.offset + msg.bytes.len() as u64,
            })
        })();

        let result = match result {
            Ok(success) => write_file_chunk_response::Result::Success(success),
            Err(err) => write_file_chunk_response::Result::Error(FileOperationError {
                message: format!("Failed to write file chunk {}: {err}", msg.path),
            }),
        };

        HandlerOutcome::Sync(server_message::Message::WriteFileChunkResponse(
            WriteFileChunkResponse {
                result: Some(result),
            },
        ))
    }

    /// Handles `CloseBuffer` notification (fire-and-forget).
    /// Removes the connection from the buffer's connection set.
    /// Deallocates the buffer if no connections remain.
    #[cfg(feature = "local_fs")]
    fn handle_close_buffer(
        &mut self,
        msg: CloseBuffer,
        conn_id: ConnectionId,
        ctx: &mut ModelContext<Self>,
    ) {
        log::info!(
            "Handling CloseBuffer path={path} conn={conn_id}",
            path = msg.path
        );
        self.buffers.close_buffer(&msg.path, conn_id, ctx);
    }
}

#[cfg(feature = "local_fs")]
fn expand_user_path(path: &str) -> PathBuf {
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(path)
}

#[cfg(feature = "local_fs")]
fn entry_kind(
    file_type: Option<&std::fs::FileType>,
    metadata: Option<&std::fs::Metadata>,
) -> i32 {
    if file_type.is_some_and(|ft| ft.is_symlink()) {
        return FileSystemEntryKind::Symlink as i32;
    }
    if metadata.is_some_and(|metadata| metadata.is_dir()) {
        return FileSystemEntryKind::Directory as i32;
    }
    if metadata.is_some_and(|metadata| metadata.is_file()) {
        return FileSystemEntryKind::File as i32;
    }
    FileSystemEntryKind::Other as i32
}

#[cfg(feature = "local_fs")]
fn system_time_to_epoch_millis(time: std::time::SystemTime) -> Option<u64> {
    time.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
}

/// Converts a [`ReadFileContextResult`] into its protobuf equivalent.
fn file_context_result_to_proto(result: ReadFileContextResult) -> ReadFileContextResponse {
    use crate::ai::agent::AnyFileContent;

    let file_contexts = result
        .file_contexts
        .into_iter()
        .map(|fc| {
            let content = match fc.content {
                AnyFileContent::StringContent(text) => {
                    super::proto::file_context_proto::Content::TextContent(text)
                }
                AnyFileContent::BinaryContent(bytes) => {
                    super::proto::file_context_proto::Content::BinaryContent(bytes)
                }
            };
            let last_modified_epoch_millis = fc
                .last_modified
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64);
            FileContextProto {
                file_name: fc.file_name,
                content: Some(content),
                line_range_start: fc.line_range.as_ref().map(|r| r.start as u32),
                line_range_end: fc.line_range.as_ref().map(|r| r.end as u32),
                last_modified_epoch_millis,
                line_count: fc.line_count as u32,
            }
        })
        .collect();

    let failed_files = result
        .missing_files
        .into_iter()
        .map(|path| FailedFileRead {
            path,
            error: Some(FileOperationError {
                message: "File not found or could not be read".to_string(),
            }),
        })
        .collect();

    ReadFileContextResponse {
        file_contexts,
        failed_files,
    }
}

#[cfg(test)]
#[path = "server_model_tests.rs"]
mod tests;
