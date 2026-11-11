//! This module contains core business logic for Agent Mode, primarily sending input to an AI
//! model and receiving output.
//!
//! The `BlocklistAIController` orchestrates state updates and service calls to power the
//! Agent Mode UI.
pub mod input_context;
mod pending_response_streams;
pub mod response_stream;
pub(super) mod shared_session;
mod slash_command;
use input_context::{input_context_for_request, parse_context_attachments};
pub use slash_command::*;

use self::response_stream::{PendingTitleGeneration, ResponseStream, ResponseStreamEvent};
use super::agent_view::AgentViewEntryOrigin;
use super::ResponseStreamId;
use super::{
    action_model::{BlocklistAIActionEvent, BlocklistAIActionModel},
    agent_view::{AgentViewController, AgentViewControllerEvent},
    context_model::BlocklistAIContextModel,
    history_model::{BlocklistAIHistoryEvent, BlocklistAIHistoryModel},
    input_model::InputConfig,
    BlocklistAIInputModel, InputType,
};
use crate::ai::agent::api::{self, ServerConversationToken};
use crate::ai::agent::conversation::{AIConversation, ConversationStatus};
use crate::ai::agent::task::TaskId;
use crate::ai::agent::{
    AIAgentActionResult, CancellationReason, PassiveSuggestionResultType, PassiveSuggestionTrigger,
    PassiveSuggestionTriggerType, RunningCommand,
};
use crate::ai::agent::{DocumentContentAttachmentSource, FileContext};
use crate::ai::ambient_agents::AmbientAgentTaskId;
use crate::ai::api_error::AIApiError;
use crate::ai::byop_readiness::{
    BlockedByopReadinessError, PendingByopToolResultsError, ReadinessCategory,
    ReadinessDiagnosticCoalescer, ReadinessDiagnosticContext, ReadinessDiagnosticLevel,
    ReadinessTriggerLayer, BLOCKED_BYOP_REQUEST_MESSAGE,
};
use crate::ai::document::ai_document_model::{
    AIDocumentId, AIDocumentModel, AIDocumentUserEditStatus,
};
use crate::ai::llms::LLMId;
use crate::ai::{
    agent::{
        conversation::AIConversationId, AIAgentActionResultType, AIAgentAttachment, AIAgentContext,
        AIAgentExchangeId, AIAgentInput, AIAgentOutputStatus, AIIdentifiers, EntrypointType,
        FinishedAIAgentOutput, MessageId, RenderableAIError, RequestCost, RequestMetadata,
        StaticQueryType, UserQueryMode,
    },
    llms::LLMPreferences,
};
use crate::features::FeatureFlag;
use crate::global_resource_handles::GlobalResourceHandlesProvider;
use crate::network::NetworkStatus;
use crate::notebooks::editor::model::FileLinkResolutionContext;
use crate::persistence::ModelEvent;
use crate::search::slash_command_menu::static_commands::commands;
use crate::terminal::model::block::{
    formatted_terminal_contents_for_input, BlockId, CURSOR_MARKER,
};
use crate::terminal::ssh::util::InteractiveSshCommand;
use crate::terminal::view::inline_banner::ZeroStatePromptSuggestionType;
use crate::terminal::{
    model::session::{active_session::ActiveSession, SessionType},
    model::terminal_model::TerminalModel,
    shared_session::ParticipantId,
    ShellLaunchData,
};
use crate::{send_telemetry_from_ctx, server::telemetry::TelemetryEvent};
use anyhow::anyhow;
use chrono::{DateTime, Local};
use itertools::Itertools;
use parking_lot::FairMutex;
use pending_response_streams::PendingResponseStreams;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use warp_core::assertions::safe_assert;
use warp_multi_agent_api::{
    client_action::{Action, UpdateTaskDescription},
    message, ClientAction, Task, ToolType,
};
use warpui::r#async::SpawnedFutureHandle;

use warpui::{AppContext, Entity, EntityId, ModelContext, ModelHandle, SingletonEntity};

#[derive(Debug, Clone)]
pub struct SessionContext {
    session_type: Option<SessionType>,
    shell: Option<ShellLaunchData>,
    current_working_directory: Option<String>,
    /// Zap:legacy SSH session(用户在本地 PTY 手敲 `ssh xxx@yyy`,
    /// 远端没装 warp shell hook)的连接信息。`session_type` 仍是 `Local`,
    /// 但 PTY 实际跑在远端,需要在 prompt 里告知 LLM,否则模型默认在本地 OS。
    ssh_connection_info: Option<InteractiveSshCommand>,
    /// 是否为 legacy SSH 会话(`IsLegacySSHSession::Yes`)。
    is_legacy_ssh: bool,
}

impl SessionContext {
    pub fn from_session(session: &ActiveSession, app: &AppContext) -> Self {
        let session_arc = session.session(app);
        let ssh_connection_info = session_arc
            .as_ref()
            .and_then(|s| s.subshell_info().as_ref())
            .and_then(|info| info.ssh_connection_info.clone());
        let is_legacy_ssh = session_arc
            .as_ref()
            .map(|s| s.is_legacy_ssh_session())
            .unwrap_or(false);
        SessionContext {
            session_type: session.session_type(app),
            shell: session.shell_launch_data(app),
            current_working_directory: session.current_working_directory().cloned(),
            ssh_connection_info,
            is_legacy_ssh,
        }
    }

    pub fn session_type(&self) -> &Option<SessionType> {
        &self.session_type
    }

    pub fn shell(&self) -> &Option<ShellLaunchData> {
        &self.shell
    }

    pub fn current_working_directory(&self) -> &Option<String> {
        &self.current_working_directory
    }

    /// Returns the remote host ID if this is a `WarpifiedRemote` session with
    /// a connected `RemoteServerClient`.
    pub fn host_id(&self) -> Option<&warp_core::HostId> {
        match &self.session_type {
            Some(SessionType::WarpifiedRemote { host_id }) => host_id.as_ref(),
            Some(SessionType::Local) | None => None,
        }
    }

    /// Returns `true` if this is a remote session (regardless of whether
    /// the remote server client is connected).
    pub fn is_remote(&self) -> bool {
        matches!(self.session_type, Some(SessionType::WarpifiedRemote { .. }))
    }

    /// Zap:legacy SSH 连接信息(host/port),仅在 `is_legacy_ssh()` 为 true 时有意义。
    pub fn ssh_connection_info(&self) -> Option<&InteractiveSshCommand> {
        self.ssh_connection_info.as_ref()
    }

    /// Zap:本会话是否为 legacy SSH(用户手敲 ssh,远端无 warp hook)。
    /// 这种会话 `session_type` 仍是 `Local`,但 PTY 实际跑在远端,
    /// `host_info`/`shell` 等画像反映的是本地客户端而非远端 shell。
    pub fn is_legacy_ssh(&self) -> bool {
        self.is_legacy_ssh
    }

    #[cfg(test)]
    pub fn new_for_test() -> Self {
        SessionContext {
            session_type: None,
            shell: None,
            current_working_directory: None,
            ssh_connection_info: None,
            is_legacy_ssh: false,
        }
    }
}

pub enum BlocklistAIControllerEvent {
    /// Emitted when a request is sent to the AI agent API.
    SentRequest {
        contains_user_query: bool,
        /// True when this request is the first send of a previously queued prompt (e.g.
        /// via `/queue` or the auto-queue toggle) rather than a direct user submission.
        /// Subscribers that perform user-submission side effects (e.g. clearing the input
        /// buffer) should skip those effects when this is true — the user may have typed
        /// new input while the agent was busy and we don't want to wipe it.
        is_queued_prompt: bool,
        /// The model ID used for this request. None for slash commands that don't
        /// send a model request (e.g., /fork).
        model_id: LLMId,
        /// The ID of the response stream for this request.
        stream_id: ResponseStreamId,
    },

    /// Emitted when an AI output response is fully received, particularly relevant when output is
    /// being streamed.
    FinishedReceivingOutput {
        stream_id: ResponseStreamId,
        conversation_id: AIConversationId,
    },

    /// Emitted when the export-to-file slash command is executed.
    ExportConversationToFile { filename: Option<String> },
}

#[derive(Debug, Clone)]
pub struct RequestInput {
    pub conversation_id: AIConversationId,
    pub input_messages: HashMap<TaskId, Vec<AIAgentInput>>,
    pub working_directory: Option<String>,
    pub model_id: LLMId,
    pub coding_model_id: LLMId,
    pub cli_agent_model_id: LLMId,
    pub computer_use_model_id: LLMId,
    pub shared_session_response_initiator: Option<ParticipantId>,
    pub request_start_ts: DateTime<Local>,
    pub supported_tools_override: Option<Vec<ToolType>>,
}

struct RequestConversationSnapshot {
    conversation_data: api::ConversationData,
    parent_agent_id: Option<String>,
    agent_name: Option<String>,
}

/// preflight 在每轮迭代里只修复"一种类型"的问题(已持久化结果回收 / 当前输入结果 / finished
/// results / cancellation result),正常一轮请求最多涉及 2~3 种,但极端并发或一次累积多 task 的
/// 结果时需要更多轮才能收敛。这里保留较小的上界以便在真正的死循环时尽早返回 blocked,而不是
/// 静默自旋。
const BYOP_PREFLIGHT_MAX_ITERATIONS: usize = 6;

impl RequestInput {
    fn for_task(
        inputs: Vec<AIAgentInput>,
        task_id: TaskId,
        active_session: &ModelHandle<ActiveSession>,
        shared_session_response_initiator: Option<ParticipantId>,
        conversation_id: AIConversationId,
        terminal_view_id: EntityId,
        app: &AppContext,
    ) -> Self {
        let mut me = Self::new_with_common_fields(
            conversation_id,
            active_session,
            shared_session_response_initiator,
            terminal_view_id,
            app,
        );
        me.input_messages.insert(task_id, inputs);
        me
    }

    fn for_actions_results(
        action_results: Vec<AIAgentActionResult>,
        context: Arc<[AIAgentContext]>,
        active_session: &ModelHandle<ActiveSession>,
        shared_session_response_initiator: Option<ParticipantId>,
        conversation_id: AIConversationId,
        terminal_view_id: EntityId,
        app: &AppContext,
    ) -> Self {
        let mut me = Self::new_with_common_fields(
            conversation_id,
            active_session,
            shared_session_response_initiator,
            terminal_view_id,
            app,
        );
        for result in action_results.into_iter() {
            me.input_messages
                .entry(result.task_id.clone())
                .or_default()
                .push(AIAgentInput::ActionResult {
                    result,
                    context: context.clone(),
                });
        }
        me
    }

    pub fn all_inputs(&self) -> impl Iterator<Item = &AIAgentInput> {
        self.input_messages.values().flatten()
    }

    pub fn with_supported_tools(mut self, tools: Vec<ToolType>) -> Self {
        self.supported_tools_override = Some(tools);
        self
    }

    fn remove_action_results_by_tool_call_id(&mut self, keys: &HashSet<(String, String)>) -> usize {
        let mut removed = 0;
        for inputs in self.input_messages.values_mut() {
            let before = inputs.len();
            inputs.retain(|input| {
                let AIAgentInput::ActionResult { result, .. } = input else {
                    return true;
                };
                !keys.contains(&(result.task_id.to_string(), result.id.to_string()))
            });
            removed += before - inputs.len();
        }
        removed
    }

    fn new_with_common_fields(
        conversation_id: AIConversationId,
        active_session: &ModelHandle<ActiveSession>,
        shared_session_response_initiator: Option<ParticipantId>,
        terminal_view_id: EntityId,
        app: &AppContext,
    ) -> Self {
        let llm_prefs = LLMPreferences::as_ref(app);
        let model_id = llm_prefs
            .get_active_base_model(app, Some(terminal_view_id))
            .id
            .clone();
        let coding_model_id = llm_prefs
            .get_active_coding_model(app, Some(terminal_view_id))
            .id
            .clone();
        let cli_agent_model_id = llm_prefs
            .get_active_cli_agent_model(app, Some(terminal_view_id))
            .id
            .clone();
        let computer_use_model_id = llm_prefs
            .get_active_computer_use_model(app, Some(terminal_view_id))
            .id
            .clone();
        let working_directory = active_session
            .as_ref(app)
            .current_working_directory()
            .cloned();

        Self {
            conversation_id,
            input_messages: Default::default(),
            working_directory,
            model_id,
            coding_model_id,
            cli_agent_model_id,
            computer_use_model_id,
            shared_session_response_initiator,
            request_start_ts: Local::now(),
            supported_tools_override: None,
        }
    }
}

/// Controller for Blocklist AI.
///
/// This is responsible for managing and updating blocklist AI state in a single terminal pane.
pub struct BlocklistAIController {
    active_session: ModelHandle<ActiveSession>,
    input_model: ModelHandle<BlocklistAIInputModel>,
    context_model: ModelHandle<BlocklistAIContextModel>,
    action_model: ModelHandle<BlocklistAIActionModel>,
    terminal_model: Arc<FairMutex<TerminalModel>>,

    in_flight_response_streams: PendingResponseStreams,

    /// The ID of the terminal view this controller is associated with.
    terminal_view_id: EntityId,

    should_refresh_available_llms_on_stream_finish: bool,

    shared_session_state: shared_session::SharedSessionState,

    /// Ambient agent task ID attached to this controller. This is a property of the controller, and not an individual
    /// conversation, because the ambient agent task driver owns the entire Zap window working on a task, and any
    /// sessions within it. In the future, one task may span several sessions with background processes.
    ambient_agent_task_id: Option<AmbientAgentTaskId>,

    /// Per-session directory for downloading file attachments.
    /// Set by the agent driver based on the workspace directory (e.g. `{working_dir}/.warp/attachments`).
    attachments_download_dir: Option<std::path::PathBuf>,

    /// Pending auto-resume tasks that are waiting for network connectivity.
    /// These should be cancelled when a new request is sent for the same conversation.
    pending_auto_resume_handles: HashMap<AIConversationId, SpawnedFutureHandle>,
    /// Passive conversations explicitly requested to follow up after actions complete.
    pending_passive_follow_ups: HashSet<AIConversationId>,
    /// Passive suggestion results that should be included with the next request
    /// for a given conversation (e.g. accepted/iterated code diffs that weren't
    /// auto-resumed).
    pending_passive_suggestion_results: HashMap<
        AIConversationId,
        Vec<(
            PassiveSuggestionResultType,
            Option<PassiveSuggestionTrigger>,
        )>,
    >,
    /// BYOP 请求因 readiness 报 PendingToolResults 被暂存的 RequestInput,
    /// 等同 conversation 的 live action 完成、触发 FinishedAction 后由
    /// `flush_pending_byop_request_after_finished_action` 取出并重发,
    /// 避免用户提交的 prompt 被静默丢弃。
    /// 每个 conversation 最多保留 1 条;用户后续直接提交新请求会覆盖。
    pending_byop_requests: HashMap<AIConversationId, PendingByopRequest>,
}

/// 见 `pending_byop_requests`。
struct PendingByopRequest {
    request_input: RequestInput,
    query_metadata: Option<RequestMetadata>,
    default_to_follow_up_on_success: bool,
    can_attempt_resume_on_error: bool,
    is_queued_prompt: bool,
}

enum InputQueryType {
    /// The user submitted query from the input. This may map to [`AIAgentInput::UserQuery`] but may
    /// map to other `AIAgentInput` types depending on various factors.
    UserSubmittedQueryFromInput {
        query: String,
        static_query_type: Option<StaticQueryType>,
        running_command: Option<RunningCommand>,
    },
    /// A custom [`AIInputType`].
    AIInputType { ai_input: AIAgentInput },
}

enum WhichTask {
    NewConversation,
    Task {
        conversation_id: AIConversationId,
        task_id: TaskId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FollowUpTrigger {
    Auto,
    UserRequested,
}

struct InputQuery {
    which_task: WhichTask,
    input_query: InputQueryType,
    /// Additional referenced attachments to include in the query
    /// (e.g. file path references from shared session file uploads).
    additional_attachments: HashMap<String, AIAgentAttachment>,
}

impl InputQuery {
    fn query(&self) -> String {
        match &self.input_query {
            InputQueryType::UserSubmittedQueryFromInput { query, .. } => query.clone(),
            InputQueryType::AIInputType { ai_input } => ai_input.user_query().unwrap_or_default(),
        }
    }
}

impl BlocklistAIController {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        input_model: ModelHandle<BlocklistAIInputModel>,
        context_model: ModelHandle<BlocklistAIContextModel>,
        action_model: ModelHandle<BlocklistAIActionModel>,
        active_session: ModelHandle<ActiveSession>,
        agent_view_controller: ModelHandle<AgentViewController>,
        terminal_model: Arc<FairMutex<TerminalModel>>,
        terminal_view_id: EntityId,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        ctx.subscribe_to_model(&action_model, move |me, event, ctx| {
            let BlocklistAIActionEvent::FinishedAction {
                conversation_id,
                cancellation_reason,
                ..
            } = event
            else {
                return;
            };
            let action_model = me.action_model.as_ref(ctx);
            if action_model.has_unfinished_actions_for_conversation(*conversation_id) {
                return;
            }

            let history_model = BlocklistAIHistoryModel::handle(ctx);
            let Some((is_viewing_shared_session, is_entirely_passive_code_diff)) = history_model
                .as_ref(ctx)
                .conversation(conversation_id)
                .map(|conversation| {
                    (
                        conversation.is_viewing_shared_session(),
                        conversation.is_entirely_passive_code_diff(),
                    )
                })
            else {
                return;
            };

            // Viewer sessions should not send follow-ups.
            // They only act as passive viewers of the action stream.
            if is_viewing_shared_session {
                return;
            }

            let Some(finished_action_results) =
                action_model.get_finished_action_results(*conversation_id)
            else {
                return;
            };
            let is_passive_code_diff = is_entirely_passive_code_diff
                && finished_action_results.last().is_some_and(|result| {
                    matches!(result.result, AIAgentActionResultType::RequestFileEdits(_))
                });
            let has_manual_follow_up = me.pending_passive_follow_ups.contains(conversation_id);

            let is_lrc_command_completed =
                cancellation_reason.is_some_and(|reason| reason.is_lrc_command_completed());
            let should_trigger_follow_up_request = (!is_passive_code_diff
                && !is_lrc_command_completed
                && finished_action_results
                    .iter()
                    .any(|result| result.result.should_trigger_request_upon_completion()))
                || has_manual_follow_up;
            if !should_trigger_follow_up_request {
                // We also check if there's an in-flight req, because it's possible that this
                // subscription callback was queued in response to auto-cancelling pending actions
                // in the process of constructing a request. In such cases, we don't want to update
                // conversation status to Cancelled/Success.
                if !me
                    .in_flight_response_streams
                    .has_active_stream_for_conversation(*conversation_id, ctx)
                {
                    // If the completed actions do not trigger a follow-up request, update conversation
                    // status based on the outcome of the actions.
                    //
                    // (It would otherwise remain `InProgress`, which would be correct, since we'd be
                    // immediately triggering a follow-up request).
                    //
                    // In practice, the only time where this codepath gets triggered is upon completion
                    // of a passive code diff action, where we don't autosend the next request.
                    //
                    // With passive code diffs, its most appropriate to mark the conversation
                    // successful if the passive diff was accepted. In practice, there's only ever
                    // one RequestFileEdits action, so `finished_action_results` at this point
                    // should only have a single element.
                    //
                    // If the user does end up following up on the passive diff-originated conversation,
                    // the status will once again be updated to `InProgress`.
                    let updated_conversation_status = if finished_action_results
                        .iter()
                        .all(|result| result.result.is_successful())
                        || is_lrc_command_completed
                    {
                        ConversationStatus::Success
                    } else {
                        // This is an imperfect heuristic that practically speaking should have no effect.
                        //
                        // If we actually need to differentiate between the state of a conversation
                        // where actions completed with mixed result statuses (e.g. a mix of
                        // cancelled, error, and success) _and_ we don't automatically send back action
                        // results to the agent, then it'd be worth considering adding a new status
                        // variant.
                        ConversationStatus::Cancelled
                    };
                    history_model.update(ctx, |history_model, ctx| {
                        history_model.update_conversation_status(
                            me.terminal_view_id,
                            *conversation_id,
                            updated_conversation_status,
                            ctx,
                        );
                    });
                }
                return;
            }
            let trigger = if has_manual_follow_up {
                FollowUpTrigger::UserRequested
            } else {
                FollowUpTrigger::Auto
            };
            // 优先重发被 readiness 暂存的 BYOP 请求(其中含用户的原始 prompt);
            // 没有则走原 follow-up 路径(只发 finished_action_results)。
            if me.flush_pending_byop_request_after_finished_action(*conversation_id, ctx) {
                return;
            }
            me.send_follow_up_for_conversation(*conversation_id, trigger, ctx);
        });

        ctx.subscribe_to_model(&agent_view_controller, |me, event, ctx| {
            let AgentViewControllerEvent::ExitedAgentView {
                conversation_id,
                final_exchange_count,
                ..
            } = event
            else {
                return;
            };

            // If we exited a brand-new empty conversation, there's nothing meaningful to cancel.
            if *final_exchange_count == 0 {
                return;
            }

            let history = BlocklistAIHistoryModel::handle(ctx);
            let Some(conversation) = history.as_ref(ctx).conversation(conversation_id) else {
                return;
            };

            // Viewer sessions should not send cancellations.
            if conversation.is_viewing_shared_session() {
                return;
            }

            if conversation.status().is_in_progress() {
                me.cancel_conversation_progress(
                    *conversation_id,
                    CancellationReason::ManuallyCancelled,
                    ctx,
                );
            }
        });

        Self {
            input_model,
            context_model,
            action_model,
            active_session,
            terminal_model,
            in_flight_response_streams: PendingResponseStreams::new(),
            terminal_view_id,
            should_refresh_available_llms_on_stream_finish: false,
            shared_session_state: shared_session::SharedSessionState::default(),
            ambient_agent_task_id: None,
            attachments_download_dir: None,
            pending_auto_resume_handles: HashMap::new(),
            pending_passive_follow_ups: HashSet::new(),
            pending_passive_suggestion_results: HashMap::new(),
            pending_byop_requests: HashMap::new(),
        }
    }

    /// Internal method to send a query to the AI model. External callers should use either
    /// `send_user_query_in_conversation`, `send_user_in_conversation`, or
    /// `send_custom_ai_input_query` instead.
    ///
    /// When the request is sent, a `BlocklistAIEvent::SentRequest` event is emitted containing the
    /// query itself as well as a oneshot `Receiver` that can be `await`-ed to receive the response
    /// from the AI.
    fn send_query(
        &mut self,
        input_query: InputQuery,
        entrypoint_type: EntrypointType,
        // The shared session participant who initiated this query
        // (None if this is not a shared session).
        shared_session_participant_id: Option<ParticipantId>,
        is_queued_prompt: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        // Store the participant who initiated this query before sending
        // so that send_query can use it when creating the exchange.
        if let Some(participant_id) = shared_session_participant_id {
            self.set_current_response_initiator(participant_id);
        }

        let query = input_query.query().to_owned();
        let (conversation_id, task_id) = match input_query.which_task {
            WhichTask::NewConversation => {
                let conversation = self.start_new_conversation_for_request(ctx);
                (conversation.id(), conversation.get_root_task_id().clone())
            }
            WhichTask::Task {
                conversation_id,
                task_id,
            } => (conversation_id, task_id),
        };

        // Drain any queued passive suggestion results for this conversation
        // *before* cancelling progress, since cancel_conversation_progress
        // clears the pending map.
        let pending_passive_results = self
            .pending_passive_suggestion_results
            .remove(&conversation_id)
            .unwrap_or_default();

        let cancellation_reason =
            self.cancel_active_conversation_for_follow_up(conversation_id, ctx);

        if let Some(slash_command_request) = SlashCommandRequest::from_query(query.as_str()) {
            slash_command_request.send_request(self, is_queued_prompt, ctx);
            return;
        }

        let (query, user_query_mode) =
            if let Some(q) = commands::strip_command_prefix(&query, commands::PLAN_NAME) {
                (q, UserQueryMode::Plan)
            } else {
                (query, UserQueryMode::Normal)
            };

        let should_prepend_finished_action_results = matches!(
            input_query.input_query,
            InputQueryType::UserSubmittedQueryFromInput { .. }
        );

        let completed_action_results = self.action_model.update(ctx, |action_model, ctx| {
            action_model.cancel_all_pending_actions(
                conversation_id,
                Some(cancellation_reason),
                ctx,
            );
            action_model.drain_finished_action_results(conversation_id)
        });

        let context = input_context_for_request(
            false,
            self.context_model.as_ref(ctx),
            self.active_session.as_ref(ctx),
            Some(conversation_id),
            vec![],
            ctx,
        );
        let mut inputs = if should_prepend_finished_action_results {
            completed_action_results
                .into_iter()
                .map(|result| AIAgentInput::ActionResult {
                    result,
                    context: context.clone(),
                })
                .collect_vec()
        } else {
            // Custom AI inputs like CodeReview and FetchReviewComments are encoded as
            // top-level request variants (`request::input::Type::CodeReview`,
            // `request::input::Type::FetchReviewComments`, etc.), and `convert_input`
            // only emits those variants in the single-input path.
            //
            // Tool call results are encoded differently: they only exist inside
            // `request::input::Type::UserInputs` as `user_input::Input::ToolCallResult`.
            // There is no proto request shape that can represent both a top-level
            // CodeReview-style input and a ToolCallResult in the same request.
            //
            // So if we prepend an ActionResult here, `convert_input` has to fall back
            // to the multi-input `UserInputs` path, where CodeReview / FetchReviewComments
            // are ignored entirely. The stale tool result is preserved, but the custom
            // AI input disappears from the request.
            vec![]
        };

        // Append any queued passive suggestion results that were drained
        // earlier (before cancel_conversation_progress).
        for (suggestion, trigger) in pending_passive_results {
            inputs.push(AIAgentInput::PassiveSuggestionResult {
                trigger,
                suggestion,
                context: context.clone(),
            });
        }

        let additional_attachments = input_query.additional_attachments;
        let ai_input = match input_query.input_query {
            InputQueryType::UserSubmittedQueryFromInput {
                static_query_type,
                running_command,
                ..
            } => input_for_query(
                query,
                &task_id,
                conversation_id,
                static_query_type,
                user_query_mode,
                running_command,
                additional_attachments,
                self.context_model.as_ref(ctx),
                self.active_session.as_ref(ctx),
                ctx,
            ),
            InputQueryType::AIInputType { ai_input } => ai_input,
        };
        inputs.push(ai_input);

        if let Err(e) = self.send_request_input(
            RequestInput::for_task(
                inputs,
                task_id,
                &self.active_session,
                self.get_current_response_initiator(),
                conversation_id,
                self.terminal_view_id,
                ctx,
            ),
            Some(RequestMetadata {
                is_autodetected_user_query: !self.input_model.as_ref(ctx).is_input_type_locked(),
                entrypoint: entrypoint_type,
                is_auto_resume_after_error: false,
            }),
            /*default_to_follow_up_on_success*/ true,
            /*can_attempt_resume_on_error*/ true,
            is_queued_prompt,
            ctx,
        ) {
            if let Some(pending) = e.downcast_ref::<PendingByopToolResultsError>() {
                log::debug!(
                    "[byop-readiness] request is waiting for pending tool result(s) \
                     category={:?} tool_calls={}",
                    pending.category(),
                    pending.tool_call_count()
                );
            } else {
                log::error!("Failed to send agent request: {e:?}");
            }
        }
    }

    /// Populates plan documents from user query to AIDocumentModel if not already present.
    /// Parses attachments from query and creates AI documents for any user-attached plans.
    /// This is split from parse_context_attachments to run later in the pipeline when new conversations are created.
    fn maybe_populate_plans_for_ai_document_model(
        &self,
        referenced_attachments: &HashMap<String, AIAgentAttachment>,
        conversation_id: AIConversationId,
        ctx: &mut ModelContext<Self>,
    ) {
        // Get file link resolution context from active session
        let session = self.active_session.as_ref(ctx);
        let file_link_resolution_context =
            session
                .current_working_directory()
                .cloned()
                .map(|working_directory| FileLinkResolutionContext {
                    working_directory,
                    shell_launch_data: session.shell_launch_data(ctx),
                });

        for attachment in referenced_attachments.values() {
            let AIAgentAttachment::DocumentContent {
                document_id,
                content,
                source,
                ..
            } = attachment
            else {
                continue;
            };
            if !matches!(*source, DocumentContentAttachmentSource::UserAttached) {
                continue;
            }
            let document_id = match AIDocumentId::try_from(document_id.as_str()) {
                Ok(id) => id,
                Err(_) => {
                    log::warn!("Invalid ai_document_id in document content: {document_id}");
                    continue;
                }
            };

            // Skip if document already exists in the model
            let ai_document_model = AIDocumentModel::as_ref(ctx);
            if ai_document_model
                .get_current_document(&document_id)
                .is_some()
            {
                continue;
            }

            // openWarp 本地化:plan 只存本地 SQLite。会话恢复时如果 plan 不在内存中,
            // 从 attachment 携带的 content 重建。title 从 attachment 调用方没携带,
            // 先用默认 planning 标题占位；apply_persisted_content 恢复时会被 SQLite 中存的 title 覆盖。
            AIDocumentModel::handle(ctx).update(ctx, |model, model_ctx| {
                model.create_document_with_id(
                    document_id,
                    crate::ai::ai_document_view::DEFAULT_PLANNING_DOCUMENT_TITLE,
                    content.clone(),
                    conversation_id,
                    file_link_resolution_context.clone(),
                    model_ctx,
                );
            });
        }
    }

    pub fn send_user_query_in_new_conversation(
        &mut self,
        query: String,
        static_query_type: Option<StaticQueryType>,
        entrypoint_type: EntrypointType,
        participant_id: Option<ParticipantId>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.send_user_query_in_new_conversation_internal(
            query,
            static_query_type,
            entrypoint_type,
            participant_id,
            /*is_queued_prompt*/ false,
            ctx,
        );
    }

    /// Sends the first submission of a previously queued user prompt into a new conversation.
    /// Same as [`Self::send_user_query_in_new_conversation`] but marks the emitted
    /// `SentRequest` event so UI subscribers (e.g. the input editor) know not to treat
    /// this as a direct user submission and therefore not clear the input buffer.
    pub fn send_queued_user_query_in_new_conversation(
        &mut self,
        query: String,
        static_query_type: Option<StaticQueryType>,
        entrypoint_type: EntrypointType,
        participant_id: Option<ParticipantId>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.send_user_query_in_new_conversation_internal(
            query,
            static_query_type,
            entrypoint_type,
            participant_id,
            /*is_queued_prompt*/ true,
            ctx,
        );
    }

    fn send_user_query_in_new_conversation_internal(
        &mut self,
        query: String,
        static_query_type: Option<StaticQueryType>,
        entrypoint_type: EntrypointType,
        participant_id: Option<ParticipantId>,
        is_queued_prompt: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        let participant_id = participant_id.or_else(|| self.get_sharer_participant_id());
        let running_command = {
            let terminal_model = self.terminal_model.lock();
            get_running_command(&terminal_model)
        };
        if let Some(running_command) = running_command {
            let conversation_id = self.start_new_conversation_for_request(ctx).id();
            let history_model = BlocklistAIHistoryModel::handle(ctx);
            let task_id = match history_model.update(ctx, |history_model, ctx| {
                history_model.create_cli_subagent_task_for_conversation(
                    running_command.block_id.clone(),
                    conversation_id,
                    self.terminal_view_id,
                    ctx,
                )
            }) {
                Ok(task_id) => task_id,
                Err(e) => {
                    log::error!("Could not create CLI subagent task optimistically: {e:?}");
                    return;
                }
            };
            self.send_query(
                InputQuery {
                    which_task: WhichTask::Task {
                        conversation_id,
                        task_id,
                    },
                    input_query: InputQueryType::UserSubmittedQueryFromInput {
                        query,
                        static_query_type,
                        running_command: Some(running_command),
                    },
                    additional_attachments: HashMap::new(),
                },
                entrypoint_type,
                participant_id,
                is_queued_prompt,
                ctx,
            );
        } else {
            self.send_query(
                InputQuery {
                    which_task: WhichTask::NewConversation,
                    input_query: InputQueryType::UserSubmittedQueryFromInput {
                        query,
                        static_query_type,
                        running_command: None,
                    },
                    additional_attachments: HashMap::new(),
                },
                entrypoint_type,
                participant_id,
                is_queued_prompt,
                ctx,
            );
        }
    }

    /// Sends a query into an existing conversation as an agent-initiated request.
    /// This is the agent-initiated counterpart to `send_user_query_in_conversation`.
    pub fn send_agent_query_in_conversation(
        &mut self,
        query: String,
        conversation_id: AIConversationId,
        ctx: &mut ModelContext<Self>,
    ) {
        self.send_user_query_in_conversation_internal(
            query,
            conversation_id,
            None,
            false,
            HashMap::new(),
            EntrypointType::AgentInitiated,
            /*is_queued_prompt*/ false,
            ctx,
        );
    }

    /// Sends the given user query to the AI model.
    pub fn send_user_query_in_conversation(
        &mut self,
        query: String,
        conversation_id: AIConversationId,
        participant_id: Option<ParticipantId>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.send_user_query_in_conversation_internal(
            query,
            conversation_id,
            participant_id,
            false, // skip_running_command_detection
            HashMap::new(),
            EntrypointType::UserInitiated,
            /*is_queued_prompt*/ false,
            ctx,
        );
    }

    /// Sends the first submission of a previously queued user prompt into an existing conversation.
    /// Same as [`Self::send_user_query_in_conversation`] but marks the emitted `SentRequest`
    /// event so UI subscribers (e.g. the input editor) know not to treat this as a direct
    /// user submission and therefore not clear the input buffer.
    pub fn send_queued_user_query_in_conversation(
        &mut self,
        query: String,
        conversation_id: AIConversationId,
        participant_id: Option<ParticipantId>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.send_user_query_in_conversation_internal(
            query,
            conversation_id,
            participant_id,
            false, // skip_running_command_detection
            HashMap::new(),
            EntrypointType::UserInitiated,
            /*is_queued_prompt*/ true,
            ctx,
        );
    }

    /// Sends the given user query to the AI model, with additional referenced attachments.
    pub fn send_user_query_in_conversation_with_attachments(
        &mut self,
        query: String,
        conversation_id: AIConversationId,
        participant_id: Option<ParticipantId>,
        additional_attachments: HashMap<String, AIAgentAttachment>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.send_user_query_in_conversation_internal(
            query,
            conversation_id,
            participant_id,
            false, // skip_running_command_detection
            additional_attachments,
            EntrypointType::UserInitiated,
            /*is_queued_prompt*/ false,
            ctx,
        );
    }

    /// Sends the given user query to the AI model, skipping long running command detection.
    /// We use this when we fork a conversation and immediately send an initial query, to avoid
    /// a race condition where restored command blocks may appear long running when the initial query is sent,
    /// causing the query to go to the lrc subagent.
    pub fn send_user_query_in_conversation_no_lrc_subagent(
        &mut self,
        query: String,
        conversation_id: AIConversationId,
        participant_id: Option<ParticipantId>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.send_user_query_in_conversation_internal(
            query,
            conversation_id,
            participant_id,
            true, // skip_running_command_detection
            HashMap::new(),
            EntrypointType::UserInitiated,
            /*is_queued_prompt*/ false,
            ctx,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn send_user_query_in_conversation_internal(
        &mut self,
        query: String,
        conversation_id: AIConversationId,
        participant_id: Option<ParticipantId>,
        skip_running_command_detection: bool,
        additional_attachments: HashMap<String, AIAgentAttachment>,
        entrypoint_type: EntrypointType,
        is_queued_prompt: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        let is_viewer = self
            .terminal_model
            .lock()
            .shared_session_status()
            .is_viewer();
        if is_viewer {
            log::error!("Viewers should never attempt to send queries directly");
        }

        // Ensure we capture all pending context blocks before promoting and attaching them to the conversation.
        let context_block_ids = self
            .context_model
            .as_ref(ctx)
            .pending_context_block_ids()
            .clone();

        let (promoted_blocks, task_id, running_command) = {
            let mut terminal_model = self.terminal_model.lock();
            terminal_model
                .block_list_mut()
                .associate_blocks_with_conversation(context_block_ids.iter(), conversation_id);

            // Promote all blocks that are pending for this conversation to attached.
            // This happens at query submission time, making blocks permanently associated with the conversation.
            let promoted_blocks = terminal_model
                .block_list_mut()
                .promote_blocks_to_attached_from_conversation(conversation_id);

            let active_block = terminal_model.block_list().active_block();
            let running_command_opt = if !skip_running_command_detection {
                get_running_command(&terminal_model)
            } else {
                None
            };

            let (task_id, running_command) = if let Some(running_command) = running_command_opt {
                let history_model = BlocklistAIHistoryModel::handle(ctx);
                match history_model.update(ctx, |history_model, ctx| {
                    history_model.create_cli_subagent_task_for_conversation(
                        running_command.block_id.clone(),
                        conversation_id,
                        self.terminal_view_id,
                        ctx,
                    )
                }) {
                    Ok(task_id) => (task_id, Some(running_command)),
                    Err(e) => {
                        log::error!("Could not create CLI subagent task optimistically: {e:?}");
                        return;
                    }
                }
            } else if let Some(task_id) = active_block
                .is_agent_monitoring()
                .then(|| active_block.agent_interaction_metadata())
                .flatten()
                .filter(|metadata| metadata.conversation_id() == &conversation_id)
                .and_then(|metadata| metadata.subagent_task_id().cloned())
            {
                (task_id, None)
            } else {
                let history_model = BlocklistAIHistoryModel::as_ref(ctx);
                let Some(conversation) = history_model.conversation(&conversation_id) else {
                    log::error!(
                        "Tried to send follow-up query for non-existent conversation: {conversation_id:?}"
                    );
                    return;
                };

                (conversation.get_root_task_id().clone(), None)
            };

            (promoted_blocks, task_id, running_command)
        };

        // Persist the updated visibility for each promoted block
        if !promoted_blocks.is_empty() {
            if let Some(sender) = GlobalResourceHandlesProvider::as_ref(ctx)
                .get()
                .model_event_sender
                .as_ref()
            {
                for (block_id, agent_view_visibility) in promoted_blocks {
                    if let Err(e) = sender.send(ModelEvent::UpdateBlockAgentViewVisibility {
                        block_id: block_id.to_string(),
                        agent_view_visibility: agent_view_visibility.into(),
                    }) {
                        log::error!("Error sending UpdateBlockAgentViewVisibility event: {e:?}");
                    }
                }
            }
        }

        let participant_id = participant_id.or_else(|| self.get_sharer_participant_id());
        self.send_query(
            InputQuery {
                which_task: WhichTask::Task {
                    conversation_id,
                    task_id,
                },
                input_query: InputQueryType::UserSubmittedQueryFromInput {
                    query,
                    static_query_type: None,
                    running_command,
                },
                additional_attachments,
            },
            entrypoint_type,
            participant_id,
            is_queued_prompt,
            ctx,
        );
    }

    /// Sends a request triggered by a zero-state prompt suggestion.
    pub fn send_zero_state_prompt_suggestion(
        &mut self,
        query_type: ZeroStatePromptSuggestionType,
        ctx: &mut ModelContext<Self>,
    ) {
        let participant_id = self.get_sharer_participant_id();
        self.send_query(
            InputQuery {
                which_task: WhichTask::NewConversation,
                input_query: InputQueryType::UserSubmittedQueryFromInput {
                    query: query_type.query().to_string(),
                    static_query_type: query_type.static_query_type(),
                    running_command: None,
                },
                additional_attachments: HashMap::new(),
            },
            EntrypointType::ZeroStateAgentModePromptSuggestion,
            participant_id,
            /*is_queued_prompt*/ false,
            ctx,
        );
    }

    /// Sends a custom [`AIAgentInput`] query.
    pub fn send_custom_ai_input_query(
        &mut self,
        ai_input: AIAgentInput,
        ctx: &mut ModelContext<Self>,
    ) {
        let participant_id = self.get_sharer_participant_id();
        let which_task = match self.context_model.as_ref(ctx).selected_conversation_id(ctx) {
            Some(id) => {
                let Some(conversation) = BlocklistAIHistoryModel::as_ref(ctx).conversation(&id)
                else {
                    log::error!(
                        "Tried to send custom AI input query as follow-up in non-existent conversation"
                    );
                    return;
                };
                WhichTask::Task {
                    conversation_id: conversation.id(),
                    task_id: conversation.get_root_task_id().clone(),
                }
            }
            None => WhichTask::NewConversation,
        };
        self.send_query(
            InputQuery {
                which_task,
                input_query: InputQueryType::AIInputType { ai_input },
                additional_attachments: HashMap::new(),
            },
            EntrypointType::UserInitiated,
            participant_id,
            /*is_queued_prompt*/ false,
            ctx,
        )
    }

    pub fn send_slash_command_request(
        &mut self,
        slash_command: SlashCommandRequest,
        ctx: &mut ModelContext<Self>,
    ) {
        // Slash commands are a fresh user turn; mirror `send_query`'s
        // cancel-and-resend so we don't trip `send_request_input`'s in-flight
        // invariant.
        if let Some(conversation_id) = slash_command.conversation_id(self, ctx) {
            self.cancel_active_conversation_for_follow_up(conversation_id, ctx);
        }
        slash_command.send_request(self, /*is_queued_prompt*/ false, ctx);
    }

    /// Cancel any in-flight progress on the active conversation in preparation
    /// for sending a follow-up turn that will land on `target_conversation_id`.
    /// Without this pre-cancel, [`Self::send_request_input`] would trip its
    /// in-flight invariant when the new turn re-uses an existing conversation.
    ///
    /// Returns the [`CancellationReason::FollowUpSubmitted`] reason used so
    /// callers can reuse it for downstream side effects (e.g. cancelling
    /// pending actions on the target conversation).
    fn cancel_active_conversation_for_follow_up(
        &mut self,
        target_conversation_id: AIConversationId,
        ctx: &mut ModelContext<Self>,
    ) -> CancellationReason {
        let active_conversation_id =
            BlocklistAIHistoryModel::as_ref(ctx).active_conversation_id(self.terminal_view_id);
        let reason = CancellationReason::FollowUpSubmitted {
            is_for_same_conversation: active_conversation_id
                .is_some_and(|id| id == target_conversation_id),
        };
        if let Some(active_conversation_id) = active_conversation_id {
            self.cancel_conversation_progress(active_conversation_id, reason, ctx);
        }
        reason
    }

    /// Same as [`Self::send_slash_command_request`] but marks the emitted `SentRequest`
    /// event as a queued prompt submission so UI subscribers (e.g. the input editor)
    /// don't clear the input buffer on the auto-send.
    pub fn send_queued_slash_command_request(
        &mut self,
        slash_command: SlashCommandRequest,
        ctx: &mut ModelContext<Self>,
    ) {
        slash_command.send_request(self, /*is_queued_prompt*/ true, ctx);
    }

    /// Mark a conversation to follow up after its actions complete and attempt to send immediately
    /// if results are already available.
    pub fn request_follow_up_after_actions(
        &mut self,
        conversation_id: AIConversationId,
        ctx: &mut ModelContext<Self>,
    ) {
        self.pending_passive_follow_ups.insert(conversation_id);

        if self
            .in_flight_response_streams
            .has_active_stream_for_conversation(conversation_id, ctx)
        {
            return;
        }

        let has_pending_actions = self
            .action_model
            .as_ref(ctx)
            .get_pending_actions_for_conversation(&conversation_id)
            .next()
            .is_some();
        if has_pending_actions {
            return;
        }

        let finished_action_results = self
            .action_model
            .as_ref(ctx)
            .get_finished_action_results(conversation_id);
        if finished_action_results.is_some_and(|results| !results.is_empty()) {
            self.send_follow_up_for_conversation(
                conversation_id,
                FollowUpTrigger::UserRequested,
                ctx,
            );
        }
    }

    /// Sends a custom AI input, building context from the current session.
    pub fn send_ai_input_with_context(
        &mut self,
        build_input: impl FnOnce(Arc<[AIAgentContext]>) -> AIAgentInput,
        ctx: &mut ModelContext<Self>,
    ) {
        let context = input_context_for_request(
            false,
            self.context_model.as_ref(ctx),
            self.active_session.as_ref(ctx),
            None,
            vec![],
            ctx,
        );
        self.send_custom_ai_input_query(build_input(context), ctx);
    }

    /// Sends the result of a passive suggestion (accepted/rejected code diff or
    /// prompt) back to the model so it can continue with accurate context.
    pub fn send_passive_suggestion_result(
        &mut self,
        conversation_id: Option<AIConversationId>,
        suggestion: PassiveSuggestionResultType,
        trigger: Option<PassiveSuggestionTrigger>,
        ctx: &mut ModelContext<Self>,
    ) {
        let which_task = match conversation_id {
            Some(id) => {
                let Some(conversation) = BlocklistAIHistoryModel::as_ref(ctx).conversation(&id)
                else {
                    log::error!("[passive-suggestion-result] conversation not found for id {id:?}");
                    return;
                };
                WhichTask::Task {
                    conversation_id: conversation.id(),
                    task_id: conversation.get_root_task_id().clone(),
                }
            }
            None => WhichTask::NewConversation,
        };

        let context = input_context_for_request(
            false,
            self.context_model.as_ref(ctx),
            self.active_session.as_ref(ctx),
            conversation_id,
            vec![],
            ctx,
        );

        let participant_id = self.get_sharer_participant_id();
        let trigger_type = trigger.as_ref().map(PassiveSuggestionTriggerType::from);
        log::debug!(
            "[passive-suggestions] sending result: trigger={}, trigger_type={:?}",
            if trigger.is_some() { "Some" } else { "None" },
            trigger_type,
        );
        self.send_query(
            InputQuery {
                which_task,
                input_query: InputQueryType::AIInputType {
                    ai_input: AIAgentInput::PassiveSuggestionResult {
                        trigger,
                        suggestion,
                        context,
                    },
                },
                additional_attachments: HashMap::new(),
            },
            EntrypointType::TriggerPassiveSuggestion {
                trigger: trigger_type,
            },
            participant_id,
            /*is_queued_prompt*/ false,
            ctx,
        );
    }

    /// Queues a passive suggestion result to be included with the next request
    /// for the given conversation. Use this instead of `send_passive_suggestion_result`
    /// when the result should not trigger an immediate server request (e.g. the user
    /// accepted a code diff without auto-resuming).
    pub fn queue_passive_suggestion_result(
        &mut self,
        conversation_id: AIConversationId,
        suggestion: PassiveSuggestionResultType,
        trigger: Option<PassiveSuggestionTrigger>,
    ) {
        self.pending_passive_suggestion_results
            .entry(conversation_id)
            .or_default()
            .push((suggestion, trigger));
    }

    fn send_follow_up_for_conversation(
        &mut self,
        conversation_id: AIConversationId,
        trigger: FollowUpTrigger,
        ctx: &mut ModelContext<Self>,
    ) {
        if self
            .in_flight_response_streams
            .has_active_stream_for_conversation(conversation_id, ctx)
        {
            return;
        }

        BlocklistAIHistoryModel::handle(ctx).update(ctx, |history, ctx| {
            history.mark_active_conversation_id(conversation_id, self.terminal_view_id, ctx);
        });

        if !FeatureFlag::AgentView.is_enabled() && trigger == FollowUpTrigger::Auto {
            // If `AgentView` is enabled, the conversation is guaranteed to be active while the
            // conversation is in-progress and thus while actions are executing/finishing.
            self.context_model.update(ctx, |context_model, ctx| {
                context_model.set_pending_query_state_for_existing_conversation(
                    conversation_id,
                    AgentViewEntryOrigin::AutoFollowUp,
                    ctx,
                );
            });
        }

        let finished_results = self.action_model.update(ctx, |action_model, _| {
            action_model.drain_finished_action_results(conversation_id)
        });
        if finished_results.is_empty() {
            return;
        }

        let context = input_context_for_request(
            false,
            self.context_model.as_ref(ctx),
            self.active_session.as_ref(ctx),
            Some(conversation_id),
            vec![],
            ctx,
        );
        let request_input = RequestInput::for_actions_results(
            finished_results,
            context,
            &self.active_session,
            self.get_current_response_initiator(),
            conversation_id,
            self.terminal_view_id,
            ctx,
        );

        let _ = self.send_request_input(
            request_input,
            None,
            /*default_to_follow_up_on_success*/ false,
            /*can_attempt_resume_on_error*/ true,
            /*is_queued_prompt*/ false,
            ctx,
        );

        self.pending_passive_follow_ups.remove(&conversation_id);
    }

    /// 当 conversation 的 live action 完成时,如果该 conversation 之前有 BYOP 请求因
    /// readiness 报 PendingToolResults 被暂存,优先重发该请求(以便用户的原始 prompt 不被丢失),
    /// 并把 action 完成产出的 finished_action_results 合入同一个 RequestInput,避免发出两次请求。
    /// 返回 true 表示已消费 pending 请求并尝试发送;false 表示没有 pending 请求需要 flush。
    fn flush_pending_byop_request_after_finished_action(
        &mut self,
        conversation_id: AIConversationId,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        let Some(pending) = self.pending_byop_requests.remove(&conversation_id) else {
            return false;
        };
        let PendingByopRequest {
            mut request_input,
            query_metadata,
            default_to_follow_up_on_success,
            can_attempt_resume_on_error,
            is_queued_prompt,
        } = pending;

        let finished_results = self.action_model.update(ctx, |action_model, _| {
            action_model.drain_finished_action_results(conversation_id)
        });
        if !finished_results.is_empty() {
            let context = input_context_for_request(
                false,
                self.context_model.as_ref(ctx),
                self.active_session.as_ref(ctx),
                Some(conversation_id),
                vec![],
                ctx,
            );
            for result in finished_results {
                request_input
                    .input_messages
                    .entry(result.task_id.clone())
                    .or_default()
                    .push(AIAgentInput::ActionResult {
                        result,
                        context: context.clone(),
                    });
            }
        }

        log::debug!(
            "[byop-readiness] flushing pending BYOP request after finished action \
             conversation_id={conversation_id:?}"
        );
        if let Err(e) = self.send_request_input(
            request_input,
            query_metadata,
            default_to_follow_up_on_success,
            can_attempt_resume_on_error,
            is_queued_prompt,
            ctx,
        ) {
            // 二次失败时不再循环排队:若仍报 Pending,Err 已被 send_request_input 内部重新
            // 写入 pending_byop_requests;若是其它错误则按 controller 已有路径处理。
            if e.downcast_ref::<PendingByopToolResultsError>().is_none() {
                log::error!("[byop-readiness] failed to flush pending BYOP request: {e:?}");
            }
        }
        true
    }

    pub fn resume_conversation(
        &mut self,
        conversation_id: AIConversationId,
        can_attempt_resume_on_error: bool,
        is_auto_resume_after_error: bool,
        additional_context: Vec<AIAgentContext>,
        ctx: &mut ModelContext<Self>,
    ) {
        let Some(conversation) =
            BlocklistAIHistoryModel::as_ref(ctx).conversation(&conversation_id)
        else {
            log::error!("Tried to resume non-existent conversation: {conversation_id:?}");
            return;
        };
        let task_id = {
            let terminal_model = self.terminal_model.lock();
            let active_block = terminal_model.block_list().active_block();
            if let Some(agent_interaction_metadata) = active_block
                .agent_interaction_metadata()
                .filter(|metadata| {
                    metadata.conversation_id() == &conversation_id && metadata.is_agent_in_control()
                })
            {
                agent_interaction_metadata
                    .subagent_task_id()
                    .cloned()
                    .unwrap_or_else(|| conversation.get_root_task_id().clone())
            } else {
                conversation.get_root_task_id().clone()
            }
        };

        let context = input_context_for_request(
            false,
            self.context_model.as_ref(ctx),
            self.active_session.as_ref(ctx),
            Some(conversation_id),
            additional_context,
            ctx,
        );

        let inputs = vec![AIAgentInput::ResumeConversation { context }];
        let metadata = if is_auto_resume_after_error {
            Some(RequestMetadata {
                is_autodetected_user_query: false,
                entrypoint: EntrypointType::ResumeConversation,
                is_auto_resume_after_error: true,
            })
        } else {
            None
        };
        let _ = self.send_request_input(
            RequestInput::for_task(
                inputs,
                task_id,
                &self.active_session,
                self.get_current_response_initiator(),
                conversation_id,
                self.terminal_view_id,
                ctx,
            ),
            metadata,
            /*default_to_follow_up_on_success*/ true,
            can_attempt_resume_on_error,
            /*is_queued_prompt*/ false,
            ctx,
        );
    }

    pub fn send_passive_code_diff_request(
        &mut self,
        query: String,
        block_id: &BlockId,
        file_contexts: Vec<FileContext>,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<(AIConversationId, ResponseStreamId)> {
        let mut input_context = file_contexts
            .into_iter()
            .map(AIAgentContext::File)
            .collect_vec();
        if let Some(block_context) = self
            .context_model
            .as_ref(ctx)
            .transform_block_to_context(block_id, false)
        {
            input_context.push(block_context);
        }

        let new_conversation = self.start_new_conversation_for_request(ctx);
        self.send_request_input(
            RequestInput::for_task(
                vec![AIAgentInput::AutoCodeDiffQuery {
                    query,
                    context: input_context.into(),
                }],
                new_conversation.get_root_task_id().clone(),
                &self.active_session,
                self.get_current_response_initiator(),
                new_conversation.id(),
                self.terminal_view_id,
                ctx,
            ),
            Some(RequestMetadata {
                is_autodetected_user_query: false,
                entrypoint: EntrypointType::PromptSuggestion {
                    is_static: false,
                    is_coding: true,
                },
                is_auto_resume_after_error: false,
            }),
            /*default_to_follow_up_on_success=*/ false,
            /*can_attempt_resume_on_error*/ true,
            /*is_queued_prompt*/ false,
            ctx,
        )
    }

    /// Builds request params for an out-of-band passive suggestions request.
    ///
    /// This reads conversation state read-only and does NOT create exchanges,
    /// register response streams, or modify conversation status. The caller
    /// is responsible for spawning the API call and handling the response.
    ///
    /// If `followup_conversation_id` is provided, the conversation's task context
    /// and server token are included so the server can use prior context.
    /// Otherwise, a new conversation is created to anchor the request.
    /// Builds request params for an out-of-band passive suggestions request.
    ///
    /// This is read-only and does NOT create exchanges, register response
    /// streams, or modify conversation history. The caller is responsible for
    /// spawning the API call and handling the response.
    ///
    /// If `followup_conversation_id` is provided, the conversation's task
    /// context and server token are included so the server can use prior
    /// context. Otherwise a fresh, ephemeral conversation ID is generated
    /// without touching the history model.
    pub fn build_passive_suggestions_request_params(
        &self,
        followup_conversation_id: Option<AIConversationId>,
        trigger: PassiveSuggestionTrigger,
        supported_tools: Vec<ToolType>,
        ctx: &ModelContext<Self>,
    ) -> anyhow::Result<(AIConversationId, api::RequestParams)> {
        let history_model = BlocklistAIHistoryModel::as_ref(ctx);

        // Resolve conversation state. For follow-ups we read from history;
        // for new triggers we generate a fresh ID without persisting anything.
        let (conversation_id, task_id, conversation_data) = if let Some(conversation_id) =
            followup_conversation_id
        {
            let Some(conversation) = history_model.conversation(&conversation_id) else {
                return Err(anyhow!(
                    "Tried to build passive suggestions request params for non-existent conversation with ID {conversation_id:?}"
                ));
            };
            let task_id = conversation.get_root_task_id().clone();
            let conversation_data = api::ConversationData {
                id: conversation_id,
                tasks: conversation.compute_active_tasks(),
                server_conversation_token: conversation.server_conversation_token().cloned(),
                forked_from_conversation_token: conversation
                    .forked_from_server_conversation_token()
                    .cloned(),
                ambient_agent_task_id: self.ambient_agent_task_id,
                existing_suggestions: None,
            };
            (conversation_id, task_id, conversation_data)
        } else if !matches!(
            trigger,
            PassiveSuggestionTrigger::AgentResponseCompleted { .. }
        ) {
            // Generate a fresh, ephemeral conversation ID without mutating history.
            let conversation_id = AIConversationId::new();
            let task_id = TaskId::new(uuid::Uuid::new_v4().to_string());
            let conversation_data = api::ConversationData {
                id: conversation_id,
                tasks: vec![],
                server_conversation_token: None,
                forked_from_conversation_token: None,
                ambient_agent_task_id: self.ambient_agent_task_id,
                existing_suggestions: None,
            };
            (conversation_id, task_id, conversation_data)
        } else {
            return Err(anyhow!(
                "Tried to use agent response completed trigger to generate passive suggestions without a conversation ID"
            ));
        };

        let inputs = vec![AIAgentInput::TriggerPassiveSuggestion {
            context: input_context_for_request(
                false,
                self.context_model.as_ref(ctx),
                self.active_session.as_ref(ctx),
                Some(conversation_id),
                vec![],
                ctx,
            ),
            attachments: vec![],
            trigger: trigger.clone(),
        }];

        let request_input = RequestInput::for_task(
            inputs,
            task_id,
            &self.active_session,
            self.get_current_response_initiator(),
            conversation_id,
            self.terminal_view_id,
            ctx,
        )
        .with_supported_tools(supported_tools);

        let metadata = Some(RequestMetadata {
            is_autodetected_user_query: false,
            entrypoint: EntrypointType::TriggerPassiveSuggestion {
                trigger: Some((&trigger).into()),
            },
            is_auto_resume_after_error: false,
        });

        let request_params = api::RequestParams::new(
            Some(self.terminal_view_id),
            SessionContext::from_session(self.active_session.as_ref(ctx), ctx),
            &request_input,
            conversation_data,
            metadata,
            ctx,
        );

        Ok((conversation_id, request_params))
    }

    pub fn send_unit_test_suggestions_request(
        &mut self,
        block_output: String,
        trigger: PassiveSuggestionTrigger,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<(AIConversationId, ResponseStreamId)> {
        let attachments = vec![AIAgentAttachment::PlainText(block_output.to_string())];
        let trigger_type = (&trigger).into();
        let inputs = vec![AIAgentInput::TriggerPassiveSuggestion {
            context: input_context_for_request(
                false,
                self.context_model.as_ref(ctx),
                self.active_session.as_ref(ctx),
                None,
                vec![],
                ctx,
            ),
            attachments,
            trigger,
        }];

        let new_conversation = self.start_new_conversation_for_request(ctx);
        self.send_request_input(
            RequestInput::for_task(
                inputs,
                new_conversation.get_root_task_id().clone(),
                &self.active_session,
                self.get_current_response_initiator(),
                new_conversation.id(),
                self.terminal_view_id,
                ctx,
            ),
            Some(RequestMetadata {
                is_autodetected_user_query: false,
                entrypoint: EntrypointType::TriggerPassiveSuggestion {
                    trigger: Some(trigger_type),
                },
                is_auto_resume_after_error: false,
            }),
            /*default_to_follow_up_on_success*/ false,
            /*can_attempt_resume_on_error*/ true,
            /*is_queued_prompt*/ false,
            ctx,
        )
    }

    /// Set the ID of the ambient agent task which owns this controller and its backing session.
    pub fn set_ambient_agent_task_id(
        &mut self,
        id: Option<AmbientAgentTaskId>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.ambient_agent_task_id = id;
        self.action_model.update(ctx, |action_model, ctx| {
            action_model.set_ambient_agent_task_id(id, ctx);
        });
    }

    /// Set the per-session directory for downloading file attachments.
    pub fn set_attachments_download_dir(&mut self, dir: std::path::PathBuf) {
        self.attachments_download_dir = Some(dir);
    }

    fn start_new_conversation_for_request<'a>(
        &self,
        ctx: &'a mut ModelContext<Self>,
    ) -> &'a AIConversation {
        let is_autoexecute_override = self
            .context_model
            .as_ref(ctx)
            .pending_query_autoexecute_override(ctx)
            .is_autoexecute_any_action();
        let history_model = BlocklistAIHistoryModel::handle(ctx);
        let id = history_model.update(ctx, |history_model, ctx| {
            // We don't mark passive conversations as "the active conversation" (at least when they first appear).
            history_model.start_new_conversation(
                self.terminal_view_id,
                is_autoexecute_override,
                false,
                ctx,
            )
        });
        history_model
            .as_ref(ctx)
            .conversation(&id)
            .expect("Conversation exists- was just created.")
    }

    fn conversation_snapshot_for_request(
        &self,
        request_input: &RequestInput,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<RequestConversationSnapshot> {
        let history_model = BlocklistAIHistoryModel::handle(ctx);
        let Some(conversation) = history_model
            .as_ref(ctx)
            .conversation(&request_input.conversation_id)
        else {
            return Err(anyhow!(
                "Tried to send request for non-existent conversation with ID {:?}",
                request_input.conversation_id
            ));
        };

        let active_tasks = conversation.compute_active_tasks();
        let conversation_id = conversation.id();
        let conversation_data = api::ConversationData {
            id: conversation_id,
            tasks: active_tasks,
            server_conversation_token: conversation.server_conversation_token().cloned(),
            forked_from_conversation_token: conversation
                .forked_from_server_conversation_token()
                .cloned(),
            ambient_agent_task_id: self.ambient_agent_task_id,
            existing_suggestions: history_model
                .as_ref(ctx)
                .existing_suggestions_for_conversation(conversation_id)
                .cloned(),
        };

        Ok(RequestConversationSnapshot {
            conversation_data,
            parent_agent_id: conversation.parent_agent_id().map(str::to_string),
            agent_name: conversation.agent_name().map(str::to_string),
        })
    }

    fn build_request_params_for_input(
        &self,
        request_input: &RequestInput,
        conversation_data: api::ConversationData,
        query_metadata: Option<RequestMetadata>,
        parent_agent_id: Option<String>,
        agent_name: Option<String>,
        ctx: &mut ModelContext<Self>,
    ) -> api::RequestParams {
        let history_model = BlocklistAIHistoryModel::handle(ctx);
        let conversation_id = conversation_data.id;
        let mut request_params = api::RequestParams::new(
            Some(self.terminal_view_id),
            SessionContext::from_session(self.active_session.as_ref(ctx), ctx),
            request_input,
            conversation_data,
            query_metadata,
            ctx,
        );
        request_params.parent_agent_id = parent_agent_id;
        request_params.agent_name = agent_name;

        let compaction_cfg = crate::ai::byop_compaction::CompactionConfig::from_settings(ctx);
        history_model.update(ctx, |history_model, _ctx| {
            if let Some(convo) = history_model.conversation_mut(&conversation_id) {
                crate::ai::byop_compaction::commit::prune_now(convo, &compaction_cfg);
            }
        });
        if let Some(convo) = history_model.as_ref(ctx).conversation(&conversation_id) {
            request_params.compaction_state = Some(convo.compaction_state.clone());
            request_params.byop_repair_state = convo.byop_repair_state.clone();
        }

        self.populate_lrc_request_params(&mut request_params, conversation_id);
        request_params
    }

    fn populate_lrc_request_params(
        &self,
        request_params: &mut api::RequestParams,
        conversation_id: AIConversationId,
    ) {
        let terminal_model = self.terminal_model.lock();
        let active_block = terminal_model.block_list().active_block();
        let is_lrc_tagged_in = active_block.is_agent_tagged_in();
        let is_matching_lrc_agent = active_block.is_agent_in_control()
            && active_block
                .agent_interaction_metadata()
                .is_some_and(|metadata| metadata.conversation_id() == &conversation_id);
        if !is_lrc_tagged_in && !is_matching_lrc_agent {
            return;
        }

        request_params.lrc_command_id = Some(active_block.id().to_string());
        request_params.lrc_should_spawn_subagent = is_lrc_tagged_in;

        if let Some(running_command) = byop_get_running_command_for_lrc(&terminal_model) {
            request_params.lrc_running_command = Some(running_command.clone());
            let total_inputs = request_params.input.len();
            let mut filled_count = 0usize;
            for input in request_params.input.iter_mut() {
                if let crate::ai::agent::AIAgentInput::UserQuery {
                    running_command: rc_slot @ None,
                    ..
                } = input
                {
                    *rc_slot = Some(running_command.clone());
                    filled_count += 1;
                }
            }
            log::info!(
                "[byop-diag] LRC running_command filled: {filled_count}/{total_inputs} \
                 UserQuery slot(s); should_spawn={} grid_contents_len={} command={:?} is_alt_screen={}",
                request_params.lrc_should_spawn_subagent,
                running_command.grid_contents.len(),
                running_command.command,
                running_command.is_alt_screen_active
            );
        } else {
            log::warn!(
                "[byop-diag] LRC detected but byop_get_running_command_for_lrc \
                 returned None (active_block 状态不符)"
            );
        }
    }

    fn run_byop_request_preflight(
        &mut self,
        request_input: &mut RequestInput,
        conversation_data: &mut api::ConversationData,
        request_params: &mut api::RequestParams,
        query_metadata: Option<RequestMetadata>,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<()> {
        if crate::ai::agent_providers::lookup_byop(ctx, &request_params.model).is_none() {
            return Ok(());
        }

        let readiness_attempt_id = request_params
            .byop_readiness_attempt_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        request_params.byop_readiness_attempt_id = Some(readiness_attempt_id.clone());
        let conversation_id_for_log = conversation_data.id.to_string();
        let mut diagnostics = ReadinessDiagnosticCoalescer::default();

        for iteration in 0..BYOP_PREFLIGHT_MAX_ITERATIONS {
            let removed_persisted = self.remove_persisted_byop_action_results_from_input(
                conversation_data.id,
                request_input,
                ctx,
            );
            if removed_persisted > 0 {
                log::debug!(
                    "[byop-readiness] controller preflight removed {removed_persisted} \
                     already-persisted action result(s) from current input \
                     iteration={iteration}"
                );
                self.rebuild_request_after_byop_preflight(
                    request_input,
                    conversation_data,
                    request_params,
                    query_metadata.clone(),
                    ctx,
                )?;
                request_params.byop_readiness_attempt_id = Some(readiness_attempt_id.clone());
                continue;
            }

            let committed_current_results = self.commit_byop_current_action_results(
                conversation_data.id,
                request_input,
                request_params,
                ctx,
            )?;
            if committed_current_results > 0 {
                log::debug!(
                    "[byop-readiness] controller persisted current action result(s) \
                     progress={committed_current_results} iteration={iteration}"
                );
                self.rebuild_request_after_byop_preflight(
                    request_input,
                    conversation_data,
                    request_params,
                    query_metadata.clone(),
                    ctx,
                )?;
                request_params.byop_readiness_attempt_id = Some(readiness_attempt_id.clone());
                continue;
            }

            let committed_finished_results = self.commit_byop_finished_action_results(
                conversation_data.id,
                request_params,
                ctx,
            )?;
            if committed_finished_results > 0 {
                log::debug!(
                    "[byop-readiness] controller drained finished action result(s) \
                     progress={committed_finished_results} iteration={iteration}"
                );
                self.rebuild_request_after_byop_preflight(
                    request_input,
                    conversation_data,
                    request_params,
                    query_metadata.clone(),
                    ctx,
                )?;
                request_params.byop_readiness_attempt_id = Some(readiness_attempt_id.clone());
                continue;
            }

            let live_tool_calls =
                self.byop_unfinished_live_tool_calls(conversation_data.id, request_params, ctx);
            let report =
                crate::ai::agent_providers::chat_stream::classify_byop_controller_readiness_with_live_tool_calls(
                    request_params,
                    live_tool_calls,
                );
            match report.state {
                crate::ai::byop_readiness::ReadinessState::Ready
                | crate::ai::byop_readiness::ReadinessState::AcceptedHistoryRepair { .. } => {
                    return Ok(());
                }
                crate::ai::byop_readiness::ReadinessState::NeedsCancellationCommit {
                    tool_calls,
                } => {
                    let diagnostic_context = ReadinessDiagnosticContext::new(
                        &conversation_id_for_log,
                        &readiness_attempt_id,
                        ReadinessTriggerLayer::ControllerPreflight,
                    )
                    .with_iteration(iteration);
                    diagnostics.log_state(
                        &crate::ai::byop_readiness::ReadinessState::NeedsCancellationCommit {
                            tool_calls: tool_calls.clone(),
                        },
                        &diagnostic_context,
                        ReadinessDiagnosticLevel::Debug,
                    );
                    let progress = self.commit_byop_cancellation_results(
                        conversation_data.id,
                        request_input,
                        &tool_calls,
                        ctx,
                    )?;
                    if progress == 0 {
                        log::error!(
                            "[byop-readiness] controller preflight made no cancellation progress \
                             iteration={iteration} tool_calls={}",
                            tool_calls.len()
                        );
                        diagnostics.finish(&diagnostic_context, ReadinessDiagnosticLevel::Error);
                        return Err(BlockedByopReadinessError::new(
                            ReadinessCategory::NeedsCancellationCommit,
                        )
                        .into());
                    }
                    log::debug!(
                        "[byop-readiness] controller committed cancellation result(s) \
                         progress={progress} iteration={iteration}"
                    );
                    self.rebuild_request_after_byop_preflight(
                        request_input,
                        conversation_data,
                        request_params,
                        query_metadata.clone(),
                        ctx,
                    )?;
                    request_params.byop_readiness_attempt_id = Some(readiness_attempt_id.clone());
                }
                crate::ai::byop_readiness::ReadinessState::PendingToolResults { tool_calls } => {
                    let diagnostic_context = ReadinessDiagnosticContext::new(
                        &conversation_id_for_log,
                        &readiness_attempt_id,
                        ReadinessTriggerLayer::ControllerPreflight,
                    )
                    .with_iteration(iteration);
                    diagnostics.log_state(
                        &crate::ai::byop_readiness::ReadinessState::PendingToolResults {
                            tool_calls: tool_calls.clone(),
                        },
                        &diagnostic_context,
                        ReadinessDiagnosticLevel::Debug,
                    );
                    diagnostics.finish(&diagnostic_context, ReadinessDiagnosticLevel::Debug);
                    return Err(PendingByopToolResultsError::new(tool_calls.len()).into());
                }
                crate::ai::byop_readiness::ReadinessState::MissingResultWithoutRepairSource {
                    tool_calls,
                    reason: crate::ai::byop_readiness::MissingResultReason::NoResult,
                } => {
                    // 用户在工具执行中打断 AI(提交新对话)时,被打断的 tool call 可能因
                    // cancel 竞态 / long-running command future 被 drop(见
                    // action_model/execute/shell_command.rs cancel_execution)而永远不产生
                    // cancelled result,留下"有 tool_call 无 tool_result"的孤立缺口。
                    // 此前这里直接阻断,导致对话彻底卡死(issue #147 / #222)。
                    // 缺失结果的唯一安全语义就是"该调用已被取消",因此为这些确认无结果的
                    // tool call 合成一条 cancellation 占位结果落地历史,使对话自愈。
                    let diagnostic_context = ReadinessDiagnosticContext::new(
                        &conversation_id_for_log,
                        &readiness_attempt_id,
                        ReadinessTriggerLayer::ControllerPreflight,
                    )
                    .with_iteration(iteration);
                    diagnostics.log_state(
                        &crate::ai::byop_readiness::ReadinessState::MissingResultWithoutRepairSource {
                            tool_calls: tool_calls.clone(),
                            reason: crate::ai::byop_readiness::MissingResultReason::NoResult,
                        },
                        &diagnostic_context,
                        ReadinessDiagnosticLevel::Debug,
                    );
                    let progress = self.synthesize_byop_missing_cancellation_results(
                        conversation_data.id,
                        &tool_calls,
                        ctx,
                    )?;
                    if progress == 0 {
                        diagnostics.finish(&diagnostic_context, ReadinessDiagnosticLevel::Error);
                        log::error!(
                            "[byop-readiness] controller preflight could not synthesize \
                             cancellation result for missing tool call(s) \
                             iteration={iteration} tool_calls={} conversation_id={} \
                             request_attempt_id={}",
                            tool_calls.len(),
                            conversation_id_for_log,
                            readiness_attempt_id
                        );
                        return Err(BlockedByopReadinessError::new(
                            ReadinessCategory::MissingResultWithoutRepairSource,
                        )
                        .into());
                    }
                    log::info!(
                        "[byop-readiness] controller synthesized cancellation result(s) for \
                         interrupted tool call(s) progress={progress} iteration={iteration} \
                         conversation_id={conversation_id_for_log}"
                    );
                    self.rebuild_request_after_byop_preflight(
                        request_input,
                        conversation_data,
                        request_params,
                        query_metadata.clone(),
                        ctx,
                    )?;
                    request_params.byop_readiness_attempt_id = Some(readiness_attempt_id.clone());
                }
                state @ (crate::ai::byop_readiness::ReadinessState::DuplicateToolResults {
                    ..
                }
                | crate::ai::byop_readiness::ReadinessState::OrphanToolResult { .. }
                | crate::ai::byop_readiness::ReadinessState::OutOfOrderToolResult { .. }
                | crate::ai::byop_readiness::ReadinessState::MissingResultWithoutRepairSource {
                    ..
                }) => {
                    let category = state.category();
                    let diagnostic_context = ReadinessDiagnosticContext::new(
                        &conversation_id_for_log,
                        &readiness_attempt_id,
                        ReadinessTriggerLayer::ControllerPreflight,
                    )
                    .with_iteration(iteration);
                    diagnostics.log_state(
                        &state,
                        &diagnostic_context,
                        ReadinessDiagnosticLevel::Error,
                    );
                    diagnostics.finish(&diagnostic_context, ReadinessDiagnosticLevel::Error);
                    log::error!(
                        "[byop-readiness] controller blocked request category={category:?} \
                         conversation_id={} trigger_layer=controller_preflight \
                         request_attempt_id={} iteration={iteration}",
                        conversation_id_for_log,
                        readiness_attempt_id
                    );
                    return Err(BlockedByopReadinessError::new(category).into());
                }
            }
        }

        let diagnostic_context = ReadinessDiagnosticContext::new(
            &conversation_id_for_log,
            &readiness_attempt_id,
            ReadinessTriggerLayer::ControllerPreflight,
        );
        diagnostics.log_category(
            ReadinessCategory::ReadinessLoopDidNotConverge,
            &diagnostic_context,
            ReadinessDiagnosticLevel::Error,
        );
        diagnostics.finish(&diagnostic_context, ReadinessDiagnosticLevel::Error);
        log::error!(
            "[byop-readiness] controller preflight did not converge iterations={} \
             conversation_id={} request_attempt_id={}",
            BYOP_PREFLIGHT_MAX_ITERATIONS,
            conversation_id_for_log,
            readiness_attempt_id
        );
        Err(BlockedByopReadinessError::new(ReadinessCategory::ReadinessLoopDidNotConverge).into())
    }

    fn rebuild_request_after_byop_preflight(
        &self,
        request_input: &RequestInput,
        conversation_data: &mut api::ConversationData,
        request_params: &mut api::RequestParams,
        query_metadata: Option<RequestMetadata>,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<()> {
        let snapshot = self.conversation_snapshot_for_request(request_input, ctx)?;
        *conversation_data = snapshot.conversation_data.clone();
        *request_params = self.build_request_params_for_input(
            request_input,
            snapshot.conversation_data,
            query_metadata,
            snapshot.parent_agent_id,
            snapshot.agent_name,
            ctx,
        );
        Ok(())
    }

    fn remove_persisted_byop_action_results_from_input(
        &self,
        conversation_id: AIConversationId,
        request_input: &mut RequestInput,
        ctx: &mut ModelContext<Self>,
    ) -> usize {
        let keys = request_input
            .all_inputs()
            .filter_map(|input| {
                let AIAgentInput::ActionResult { result, .. } = input else {
                    return None;
                };
                let task_id = result.task_id.to_string();
                let tool_call_id = result.id.to_string();
                self.has_persisted_tool_result(conversation_id, &task_id, &tool_call_id, ctx)
                    .then_some((task_id, tool_call_id))
            })
            .collect::<HashSet<_>>();
        request_input.remove_action_results_by_tool_call_id(&keys)
    }

    fn commit_byop_current_action_results(
        &self,
        conversation_id: AIConversationId,
        request_input: &mut RequestInput,
        request_params: &api::RequestParams,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<usize> {
        let visible_tool_calls = Self::visible_byop_tool_result_keys(request_params);
        let results = request_input
            .all_inputs()
            .filter_map(|input| {
                let AIAgentInput::ActionResult { result, .. } = input else {
                    return None;
                };
                if result.result.is_cancelled() {
                    return None;
                }
                visible_tool_calls
                    .contains(&(result.task_id.to_string(), result.id.to_string()))
                    .then(|| result.clone())
            })
            .collect_vec();
        let keys_to_remove = Self::action_result_keys(&results);
        let appended = self.append_byop_action_result_messages(conversation_id, &results, ctx)?;
        let removed = request_input.remove_action_results_by_tool_call_id(&keys_to_remove);
        Ok(appended + removed)
    }

    fn commit_byop_finished_action_results(
        &self,
        conversation_id: AIConversationId,
        request_params: &api::RequestParams,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<usize> {
        let visible_tool_calls = Self::visible_byop_tool_result_keys(request_params);
        let results = self
            .action_model
            .as_ref(ctx)
            .finished_action_results_matching(conversation_id, &visible_tool_calls);
        let keys_to_remove = Self::action_result_keys(&results);
        let appended = self.append_byop_action_result_messages(conversation_id, &results, ctx)?;
        let removed = self.action_model.update(ctx, |action_model, _| {
            action_model.remove_finished_action_results_matching(conversation_id, &keys_to_remove)
        });
        Ok(appended + removed)
    }

    fn append_byop_action_result_messages(
        &self,
        conversation_id: AIConversationId,
        results: &[AIAgentActionResult],
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<usize> {
        let request_id = format!("byop-preflight:{}", uuid::Uuid::new_v4());
        let mut grouped_messages: HashMap<TaskId, Vec<warp_multi_agent_api::Message>> =
            HashMap::new();
        for result in results {
            let task_id = result.task_id.to_string();
            let tool_call_id = result.id.to_string();
            if self.has_persisted_tool_result(conversation_id, &task_id, &tool_call_id, ctx) {
                continue;
            }
            grouped_messages
                .entry(result.task_id.clone())
                .or_default()
                .push(Self::byop_action_result_message(&request_id, result));
        }

        let mut appended = 0;
        for (task_id, messages) in grouped_messages {
            appended +=
                BlocklistAIHistoryModel::handle(ctx).update(ctx, |history_model, ctx| {
                    history_model.append_byop_preflight_messages_to_task(
                        conversation_id,
                        task_id,
                        messages,
                        ctx,
                    )
                })?;
        }
        Ok(appended)
    }

    fn byop_unfinished_live_tool_calls(
        &self,
        conversation_id: AIConversationId,
        request_params: &api::RequestParams,
        ctx: &mut ModelContext<Self>,
    ) -> Vec<crate::ai::byop_readiness::LiveToolCall> {
        use crate::ai::agent::task::helper::{ToolCallExt, ToolExt};

        request_params
            .tasks
            .iter()
            .flat_map(|task| task.messages.iter())
            .filter_map(|msg| {
                let message::Message::ToolCall(tool_call) = msg.message.as_ref()? else {
                    return None;
                };
                if tool_call.subagent().is_some()
                    || !self
                        .action_model
                        .as_ref(ctx)
                        .has_unfinished_action_for_tool_call(
                            conversation_id,
                            &msg.task_id,
                            &tool_call.tool_call_id,
                            ctx,
                        )
                {
                    return None;
                }
                Some(crate::ai::byop_readiness::LiveToolCall::new(
                    crate::ai::byop_readiness::ToolCallRef::new(
                        crate::ai::byop_readiness::ToolCallKey::new(
                            &msg.task_id,
                            &msg.id,
                            &tool_call.tool_call_id,
                        ),
                        crate::ai::byop_readiness::RedactedToolKind::new(
                            tool_call
                                .tool
                                .as_ref()
                                .map(|tool| tool.name())
                                .unwrap_or("unknown"),
                        ),
                    ),
                    crate::ai::byop_readiness::LiveToolCallState::Running,
                ))
            })
            .collect()
    }

    fn visible_byop_tool_result_keys(
        request_params: &api::RequestParams,
    ) -> HashSet<(String, String)> {
        use crate::ai::agent::task::helper::ToolCallExt;

        request_params
            .tasks
            .iter()
            .flat_map(|task| task.messages.iter())
            .filter_map(|msg| {
                let message::Message::ToolCall(tool_call) = msg.message.as_ref()? else {
                    return None;
                };
                if tool_call.subagent().is_some() {
                    return None;
                }
                Some((msg.task_id.clone(), tool_call.tool_call_id.clone()))
            })
            .collect()
    }

    fn action_result_keys(results: &[AIAgentActionResult]) -> HashSet<(String, String)> {
        results
            .iter()
            .map(|result| (result.task_id.to_string(), result.id.to_string()))
            .collect()
    }

    fn commit_byop_cancellation_results(
        &self,
        conversation_id: AIConversationId,
        request_input: &mut RequestInput,
        tool_calls: &[crate::ai::byop_readiness::ToolCallRef],
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<usize> {
        // `ToolCallKey` 是三元组(task_id + assistant_tool_call_message_id + tool_call_id),
        // 但 BYOP `AIAgentActionResult` 与持久化的 `ToolCallResult` 协议层只携带 (task_id, tool_call_id),
        // 没有显式的 assistant_tool_call_message_id 反向引用。为了对齐 readiness 模块的 ToolCallKey
        // 语义,这里用 (task_id, tool_call_id) 作为外层匹配键,同时记录 readiness 期望的
        // assistant_tool_call_message_id 集合;在协议假设(同 conversation 内 tool_call_id 唯一)
        // 被打破时只记 warn 而不阻断,避免因边界场景导致用户流程卡死。
        let mut wanted: HashMap<(String, String), HashSet<String>> = HashMap::new();
        for tool_call in tool_calls {
            wanted
                .entry((
                    tool_call.key.task_id.clone(),
                    tool_call.key.tool_call_id.clone(),
                ))
                .or_default()
                .insert(tool_call.key.assistant_tool_call_message_id.clone());
        }
        for ((task_id, tool_call_id), assistant_message_ids) in &wanted {
            if assistant_message_ids.len() > 1 {
                log::warn!(
                    "[byop-readiness] commit_byop_cancellation_results saw duplicate tool_call_id \
                     across multiple assistant messages task_id={task_id} \
                     tool_call_id={tool_call_id} assistant_message_id_count={}",
                    assistant_message_ids.len()
                );
            }
        }
        let request_id = format!("byop-preflight:{}", uuid::Uuid::new_v4());
        let mut grouped_messages: HashMap<TaskId, Vec<warp_multi_agent_api::Message>> =
            HashMap::new();
        let mut keys_to_remove = HashSet::new();
        // 已经被持久化过的 cancellation result 也算"有进展",
        // 否则当 readiness 仍报 NeedsCancellationCommit 但 request_input 中已无对应 entry 时,
        // 会被错误判定为零进展而阻断;实际上下一轮 readiness 会自然 converge。
        let mut already_persisted = 0usize;

        for input in request_input.all_inputs() {
            let AIAgentInput::ActionResult { result, .. } = input else {
                continue;
            };
            if !result.result.is_cancelled() {
                continue;
            }

            let task_id = result.task_id.to_string();
            let tool_call_id = result.id.to_string();
            let key = (task_id.clone(), tool_call_id.clone());
            if !wanted.contains_key(&key) {
                continue;
            }

            keys_to_remove.insert(key.clone());
            if self.has_persisted_tool_result(conversation_id, &task_id, &tool_call_id, ctx) {
                already_persisted += 1;
                continue;
            }

            grouped_messages
                .entry(result.task_id.clone())
                .or_default()
                .push(Self::byop_action_result_message(&request_id, result));
        }

        let mut appended = 0;
        for (task_id, messages) in grouped_messages {
            appended +=
                BlocklistAIHistoryModel::handle(ctx).update(ctx, |history_model, ctx| {
                    history_model.append_byop_preflight_messages_to_task(
                        conversation_id,
                        task_id,
                        messages,
                        ctx,
                    )
                })?;
        }

        let removed = request_input.remove_action_results_by_tool_call_id(&keys_to_remove);
        Ok(appended + removed + already_persisted)
    }

    /// 为被用户打断、确认已无任何结果(既无 running action 也无 finished/persisted result)的
    /// tool call 合成一条 cancellation 占位 `ToolCallResult` 落地历史。
    ///
    /// 仅在 readiness 判定为 `MissingResultWithoutRepairSource { NoResult }` 时调用:此时 history
    /// 里存在孤立的 tool_call 但既不会再产生真实结果(future 已 drop / cancel 竞态丢失),
    /// 也没有 RepairRecord 授权。补一条"已取消"是唯一安全且语义正确的修复,使 BYOP 请求
    /// 能继续发送,而不是把整段对话永久阻断。
    fn synthesize_byop_missing_cancellation_results(
        &self,
        conversation_id: AIConversationId,
        tool_calls: &[crate::ai::byop_readiness::ToolCallRef],
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<usize> {
        let request_id = format!("byop-preflight:{}", uuid::Uuid::new_v4());
        let mut grouped_messages: HashMap<TaskId, Vec<warp_multi_agent_api::Message>> =
            HashMap::new();
        for tool_call in tool_calls {
            let task_id = &tool_call.key.task_id;
            let tool_call_id = &tool_call.key.tool_call_id;
            // 防御:若该 (task_id, tool_call_id) 已有持久化结果则跳过,避免补出重复 result。
            if self.has_persisted_tool_result(conversation_id, task_id, tool_call_id, ctx) {
                continue;
            }
            grouped_messages
                .entry(TaskId::new(task_id.clone()))
                .or_default()
                .push(Self::byop_synthetic_cancellation_message(
                    &request_id,
                    task_id,
                    tool_call_id,
                ));
        }

        let mut appended = 0;
        for (task_id, messages) in grouped_messages {
            appended +=
                BlocklistAIHistoryModel::handle(ctx).update(ctx, |history_model, ctx| {
                    history_model.append_byop_preflight_messages_to_task(
                        conversation_id,
                        task_id,
                        messages,
                        ctx,
                    )
                })?;
        }
        Ok(appended)
    }

    /// 构造一条不依赖 `AIAgentActionResult` 的 cancellation `ToolCallResult` message。
    /// 被打断的 tool call 已无对应 action,因此无法走 `byop_action_result_message`;
    /// 这里直接以 `server_message_data` 携带取消状态(与无结构化结果时的格式一致)。
    fn byop_synthetic_cancellation_message(
        request_id: &str,
        task_id: &str,
        tool_call_id: &str,
    ) -> warp_multi_agent_api::Message {
        let server_message_data = serde_json::json!({
            "status": "cancelled",
            "reason": "interrupted_by_user",
        })
        .to_string();
        warp_multi_agent_api::Message {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: task_id.to_owned(),
            server_message_data,
            citations: vec![],
            message: Some(message::Message::ToolCallResult(message::ToolCallResult {
                tool_call_id: tool_call_id.to_owned(),
                context: None,
                result: None,
            })),
            request_id: request_id.to_owned(),
            timestamp: None,
        }
    }

    fn has_persisted_tool_result(
        &self,
        conversation_id: AIConversationId,
        task_id: &str,
        tool_call_id: &str,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        // 持久化 protobuf `ToolCallResult` 协议只携带 `tool_call_id`,没有 assistant_message_id 反向引用,
        // 因此最严格的匹配只能做到 (task_id, tool_call_id) 二元组。外层 `get_task(&task_id)` 已经
        // 把搜索范围隔离到目标 task,这里再额外校验一次 `msg.task_id`,以防未来调用方拿到非隔离视图。
        let task_id_owned = TaskId::new(task_id.to_owned());
        BlocklistAIHistoryModel::as_ref(ctx)
            .conversation(&conversation_id)
            .and_then(|conversation| conversation.get_task(&task_id_owned))
            .is_some_and(|task| {
                task.messages().any(|msg| {
                    msg.task_id == task_id
                        && matches!(
                            msg.message.as_ref(),
                            Some(message::Message::ToolCallResult(result))
                                if result.tool_call_id == tool_call_id
                        )
                })
            })
    }

    fn byop_action_result_message(
        request_id: &str,
        result: &AIAgentActionResult,
    ) -> warp_multi_agent_api::Message {
        let structured_result =
            crate::ai::agent_providers::tools::action_result_to_msg_result(result);
        let server_message_data = if structured_result.is_none() {
            let status = if result.result.is_cancelled() {
                "cancelled"
            } else {
                "completed"
            };
            crate::ai::agent_providers::tools::serialize_action_result(result).unwrap_or_else(
                || {
                    serde_json::json!({
                        "status": status,
                        "result": result.result.to_string(),
                    })
                    .to_string()
                },
            )
        } else {
            String::new()
        };

        warp_multi_agent_api::Message {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: result.task_id.to_string(),
            server_message_data,
            citations: vec![],
            message: Some(message::Message::ToolCallResult(message::ToolCallResult {
                tool_call_id: result.id.to_string(),
                context: None,
                result: structured_result,
            })),
            request_id: request_id.to_owned(),
            timestamp: None,
        }
    }

    fn complete_byop_blocked_request(
        &mut self,
        request_input: RequestInput,
        conversation_id: AIConversationId,
        model_id: LLMId,
        is_queued_prompt: bool,
        error_message: String,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<(AIConversationId, ResponseStreamId)> {
        let response_stream_id = ResponseStreamId::new_local();
        let input_contains_user_query = request_input
            .all_inputs()
            .any(|input| input.is_user_query());
        let is_passive_request = request_input
            .all_inputs()
            .any(|input| input.is_passive_request());

        for input in request_input.all_inputs() {
            if let AIAgentInput::UserQuery {
                referenced_attachments,
                ..
            } = input
            {
                self.maybe_populate_plans_for_ai_document_model(
                    referenced_attachments,
                    conversation_id,
                    ctx,
                );
            }
        }

        BlocklistAIHistoryModel::handle(ctx).update(ctx, |history_model, ctx| {
            history_model.update_conversation_for_new_request_input(
                request_input,
                response_stream_id.clone(),
                self.terminal_view_id,
                ctx,
            )?;
            history_model.update_conversation_status(
                self.terminal_view_id,
                conversation_id,
                ConversationStatus::InProgress,
                ctx,
            );
            Ok::<(), anyhow::Error>(())
        })?;

        BlocklistAIHistoryModel::handle(ctx).update(ctx, |history_model, ctx| {
            history_model.mark_response_stream_completed_with_error(
                RenderableAIError::Other {
                    error_message,
                    will_attempt_resume: false,
                    waiting_for_network: false,
                },
                &response_stream_id,
                conversation_id,
                self.terminal_view_id,
                ctx,
            );
        });

        if input_contains_user_query {
            let pending_document_id = self.context_model.as_ref(ctx).pending_document_id();
            self.context_model.update(ctx, |context_model, ctx| {
                context_model.reset_context_to_default(ctx);
            });
            if let Some(doc_id) = pending_document_id {
                AIDocumentModel::handle(ctx).update(ctx, |model, mctx| {
                    model.set_user_edit_status(&doc_id, AIDocumentUserEditStatus::UpToDate, mctx);
                });
            }
        }

        ctx.emit(BlocklistAIControllerEvent::SentRequest {
            contains_user_query: input_contains_user_query,
            is_queued_prompt,
            model_id,
            stream_id: response_stream_id.clone(),
        });
        if !is_passive_request {
            BlocklistAIHistoryModel::handle(ctx).update(ctx, |history_model, ctx| {
                history_model.mark_active_conversation_id(
                    conversation_id,
                    self.terminal_view_id,
                    ctx,
                )
            });
        }
        if input_contains_user_query {
            ctx.dispatch_global_action("workspace:save_app", ());
        }

        Ok((conversation_id, response_stream_id))
    }

    /// Attempts to send a request to the AI model API. Adds context to the input if it
    /// contains a user query. Returns `Err` if the AI input was not able to be sent due to an
    /// existing in-flight request. Emits an event containing a receiver for the AI's output.
    /// If conversation_id is Some, we follow up in that conversation.
    /// If it's None or we can't find a conversation with that ID, we start a new one.
    /// Returns the conversation ID of affected conversation and response stream ID.
    ///
    ///  This function does not handle cancelling any in flight requests (and sending them back as
    /// input) for an existing conversation. Consider calling [`Self::send_custom_ai_input_query`] if
    /// you're trying to send a query with a custom [`AIAgentInput`] type where you'd like the "normal"
    /// flow that handles existing conversations properly.
    fn send_request_input(
        &mut self,
        mut request_input: RequestInput,
        query_metadata: Option<RequestMetadata>,
        default_to_follow_up_on_success: bool,
        can_attempt_resume_on_error: bool,
        is_queued_prompt: bool,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<(AIConversationId, ResponseStreamId)> {
        let history_model = BlocklistAIHistoryModel::handle(ctx);
        let request_snapshot = self.conversation_snapshot_for_request(&request_input, ctx)?;
        let mut conversation_data = request_snapshot.conversation_data;
        let conversation_id = conversation_data.id;

        // Cancel any pending auto-resume for this conversation, since the user is sending a new
        // request.
        if let Some(handle) = self
            .pending_auto_resume_handles
            .remove(&request_input.conversation_id)
        {
            handle.abort();
        }
        // 新一轮请求开始,先丢掉之前因 readiness Pending 暂存的旧请求;
        // 如果本次 preflight 又遇到 PendingToolResults,下面会重新写回。
        self.pending_byop_requests.remove(&conversation_id);

        // Make sure there's no existing response stream for the conversation. If
        // there is, something has gone wrong.
        if self
            .in_flight_response_streams
            .has_active_stream_for_conversation(conversation_id, ctx)
        {
            send_telemetry_from_ctx!(
                TelemetryEvent::AIInputNotSent {
                    entrypoint: query_metadata.as_ref().map(|metadata| metadata.entrypoint),
                    inputs: request_input
                        .all_inputs()
                        .cloned()
                        .map(|input| input.into())
                        .collect(),
                    active_server_conversation_id: conversation_data
                        .server_conversation_token
                        .clone(),
                    active_client_conversation_id: Some(conversation_id),
                },
                ctx
            );
            const AI_INPUT_NOT_SENT_ERROR_STR: &str =
                "Not sending AI input because there is an in-flight request";
            safe_assert!(false, "{}", AI_INPUT_NOT_SENT_ERROR_STR);
            return Err(anyhow::anyhow!(AI_INPUT_NOT_SENT_ERROR_STR));
        }

        // Log an error if tool call results do not have corresponding tool calls in task context
        validate_tool_call_results(
            request_input.all_inputs(),
            &conversation_data.tasks,
            &conversation_data.server_conversation_token,
        );

        let mut request_params = self.build_request_params_for_input(
            &request_input,
            conversation_data.clone(),
            query_metadata.clone(),
            request_snapshot.parent_agent_id,
            request_snapshot.agent_name,
            ctx,
        );
        let preflight_result = self.run_byop_request_preflight(
            &mut request_input,
            &mut conversation_data,
            &mut request_params,
            query_metadata.clone(),
            ctx,
        );
        if let Err(error) = preflight_result {
            if let Some(pending) = error.downcast_ref::<PendingByopToolResultsError>() {
                log::debug!(
                    "[byop-readiness] BYOP request not opened; waiting for pending tool \
                     result(s) category={:?} tool_calls={} conversation_id={} \
                     request_attempt_id={}",
                    pending.category(),
                    pending.tool_call_count(),
                    conversation_data.id,
                    request_params
                        .byop_readiness_attempt_id
                        .as_deref()
                        .unwrap_or("unknown")
                );
                // 把用户的输入暂存,等 conversation 内的 live action 完成、触发 FinishedAction
                // 后,在 `flush_pending_byop_request_after_finished_action` 中重发,避免静默丢失。
                self.pending_byop_requests.insert(
                    conversation_id,
                    PendingByopRequest {
                        request_input,
                        query_metadata,
                        default_to_follow_up_on_success,
                        can_attempt_resume_on_error,
                        is_queued_prompt,
                    },
                );
                return Err(error);
            }
            if let Some(blocked) = error.downcast_ref::<BlockedByopReadinessError>() {
                log::error!(
                    "[byop-readiness] rendering blocked request error category={:?} \
                     conversation_id={} request_attempt_id={}",
                    blocked.category(),
                    conversation_data.id,
                    request_params
                        .byop_readiness_attempt_id
                        .as_deref()
                        .unwrap_or("unknown")
                );
                return self.complete_byop_blocked_request(
                    request_input,
                    conversation_data.id,
                    request_params.model.clone(),
                    is_queued_prompt,
                    BLOCKED_BYOP_REQUEST_MESSAGE.to_owned(),
                    ctx,
                );
            }
            // 持久化路径失败(shared session 不可保存、settings 关闭持久化、SQLite sender 不可用
            // 或 channel 满)时,原本会作为通用 anyhow::Error 冒泡到调用方被 log::error! 静默吞掉,
            // 用户看不到任何反馈。这里把它也走 blocked request UI 路径,让用户能看到失败原因。
            if let Some(persistence_err) =
                error.downcast_ref::<crate::ai::agent::conversation::UpdateConversationError>()
            {
                if matches!(
                    persistence_err,
                    crate::ai::agent::conversation::UpdateConversationError::ByopPreflightPersistenceUnavailable(_)
                    | crate::ai::agent::conversation::UpdateConversationError::ByopPreflightPersistenceSend(_)
                ) {
                    log::error!(
                        "[byop-readiness] rendering blocked request due to persistence failure: \
                         {persistence_err:?} conversation_id={} request_attempt_id={}",
                        conversation_data.id,
                        request_params
                            .byop_readiness_attempt_id
                            .as_deref()
                            .unwrap_or("unknown")
                    );
                    return self.complete_byop_blocked_request(
                        request_input,
                        conversation_data.id,
                        request_params.model.clone(),
                        is_queued_prompt,
                        "Zap couldn't save the BYOP conversation state needed to send this \
                         request. Check that conversation persistence is enabled and that there \
                         is enough disk space, then try again."
                            .to_owned(),
                        ctx,
                    );
                }
            }
            return Err(error);
        }

        let server_conversation_token_for_identifiers =
            conversation_data.server_conversation_token.clone();

        let response_stream_id = ResponseStreamId::new_local();
        let input_contains_user_query = request_input
            .all_inputs()
            .any(|input| input.is_user_query());
        let is_passive_request = request_input
            .all_inputs()
            .any(|input| input.is_passive_request());
        let referenced_attachments_to_populate: Vec<_> = request_input
            .all_inputs()
            .filter_map(|input| match input {
                AIAgentInput::UserQuery {
                    referenced_attachments,
                    ..
                } => Some(referenced_attachments.clone()),
                _ => None,
            })
            .collect();

        // 先尝试把本轮 exchange 写入会话再启动 ResponseStream;
        // 若推入失败(例如 Conversation(NoActiveTask)),绝不能创建/注册无法被取消的孤儿流。
        let push_exchange_result = history_model.update(ctx, |history_model, ctx| {
            let res = history_model.update_conversation_for_new_request_input(
                request_input,
                response_stream_id.clone(),
                self.terminal_view_id,
                ctx,
            );
            if res.is_ok() {
                history_model.update_conversation_status(
                    self.terminal_view_id,
                    conversation_data.id,
                    ConversationStatus::InProgress,
                    ctx,
                );
            }
            res
        });
        if let Err(e) = push_exchange_result {
            log::warn!("Failed to push new exchange to AI conversation: {e:?}");
            return Err(anyhow::anyhow!(
                "Failed to push new exchange to AI conversation: {e:?}"
            ));
        }

        for referenced_attachments in &referenced_attachments_to_populate {
            self.maybe_populate_plans_for_ai_document_model(
                referenced_attachments,
                conversation_data.id,
                ctx,
            );
        }

        let response_stream = ctx.add_model(|ctx| {
            // Create AIIdentifiers for the response stream
            let ai_identifiers = AIIdentifiers {
                server_output_id: None, // Will be populated by the successful response
                server_conversation_id: server_conversation_token_for_identifiers.map(Into::into),
                client_conversation_id: Some(conversation_data.id),
                client_exchange_id: None,
                model_id: Some(request_params.model.clone()),
            };
            ResponseStream::new_with_id(
                response_stream_id.clone(),
                request_params.clone(),
                ai_identifiers,
                can_attempt_resume_on_error,
                ctx,
            )
        });
        let response_stream_clone = response_stream.clone();
        ctx.subscribe_to_model(&response_stream, move |me, event, ctx| {
            me.handle_response_stream_event(
                input_contains_user_query,
                event,
                &response_stream_clone,
                ctx,
            );
        });

        self.in_flight_response_streams.register_new_stream(
            response_stream_id.clone(),
            conversation_data.id,
            response_stream,
            CancellationReason::FollowUpSubmitted {
                is_for_same_conversation: true,
            },
            ctx,
        );

        if input_contains_user_query {
            // Get the pending document ID before clearing context
            let pending_document_id = self.context_model.as_ref(ctx).pending_document_id();

            // Reset the context state to the default.
            self.context_model.update(ctx, |context_model, ctx| {
                context_model.reset_context_to_default(ctx);
            });

            // Update the document status to UpToDate after query submission
            if let Some(doc_id) = pending_document_id {
                AIDocumentModel::handle(ctx).update(ctx, |model, mctx| {
                    model.set_user_edit_status(&doc_id, AIDocumentUserEditStatus::UpToDate, mctx);
                });
            }
        }

        ctx.emit(BlocklistAIControllerEvent::SentRequest {
            contains_user_query: input_contains_user_query,
            is_queued_prompt,
            model_id: request_params.model.clone(),
            stream_id: response_stream_id.clone(),
        });
        if !is_passive_request {
            BlocklistAIHistoryModel::handle(ctx).update(ctx, |history_model, ctx| {
                history_model.mark_active_conversation_id(
                    conversation_data.id,
                    self.terminal_view_id,
                    ctx,
                )
            });
        }

        // Trigger a snapshot save to persist the agent view state when a user query is sent.
        // This ensures the agent view is restored if the app restarts.
        if input_contains_user_query {
            ctx.dispatch_global_action("workspace:save_app", ());
        }

        // If `AgentView` is enabled, the agent view is guaranteed to be active when the agent
        // input is sent, so logic to ensure follow-ups is redundant.
        if !FeatureFlag::AgentView.is_enabled() && default_to_follow_up_on_success {
            // Set the input mode to AI but allow autodetection to run
            self.input_model.update(ctx, |input_model, ctx| {
                input_model.set_input_config_for_classic_mode(
                    InputConfig {
                        input_type: InputType::AI,
                        is_locked: false,
                    },
                    ctx,
                );
            });
            // After making an AI query, default to asking a follow up.
            self.context_model.update(ctx, |context_model, ctx| {
                context_model.set_pending_query_state_for_existing_conversation(
                    conversation_data.id,
                    AgentViewEntryOrigin::AutoFollowUp,
                    ctx,
                )
            });
        }

        Ok((conversation_data.id, response_stream_id))
    }

    /// Cancels a pending AI request response stream, given the exchange ID, if it exists.
    /// Returns true if a pending stream was found and canceled, false otherwise.
    pub fn try_cancel_pending_response_stream(
        &mut self,
        stream_id: &ResponseStreamId,
        reason: CancellationReason,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        self.in_flight_response_streams
            .try_cancel_stream(stream_id, reason, ctx)
    }

    /// Cancels 'progress' for the active conversation if there is one:
    ///  * If there is an in-flight request, cancels it.
    ///  * Else, if the request finished, but actions from the response are pending or mid-execution, cancels all of them.
    pub fn cancel_conversation_progress(
        &mut self,
        conversation_id: AIConversationId,
        reason: CancellationReason,
        ctx: &mut ModelContext<Self>,
    ) {
        // Cancel any pending auto-resume for this conversation.
        if let Some(handle) = self.pending_auto_resume_handles.remove(&conversation_id) {
            handle.abort();
        }

        // Discard any queued passive suggestion results for this conversation.
        self.pending_passive_suggestion_results
            .remove(&conversation_id);

        if !self
            .in_flight_response_streams
            .try_cancel_streams_for_conversation(conversation_id, reason, ctx)
        {
            // Otherwise, cancel pending actions and update the input state.
            let has_unfinished_actions = self.action_model.update(ctx, |action_model, ctx| {
                let has_unfinished =
                    action_model.has_unfinished_actions_for_conversation(conversation_id);
                action_model.cancel_all_pending_actions(conversation_id, Some(reason), ctx);
                has_unfinished
            });
            // 若既无在途请求又无 pending/running actions,但会话状态仍卡在 InProgress
            // (例如子任务/未映射流状态脱节),显式收敛为 Cancelled 以免 Stop 按钮永久无响应。
            if !has_unfinished_actions {
                BlocklistAIHistoryModel::handle(ctx).update(ctx, |history_model, ctx| {
                    if history_model
                        .conversation(&conversation_id)
                        .is_some_and(|conversation| conversation.status().is_in_progress())
                    {
                        history_model.update_conversation_status(
                            self.terminal_view_id,
                            conversation_id,
                            ConversationStatus::Cancelled,
                            ctx,
                        );
                    }
                });
            }
            self.set_input_mode_for_cancellation(ctx);
        }
    }

    /// Clears finished action results for a conversation. Used when reverting.
    pub fn clear_finished_action_results(
        &mut self,
        conversation_id: AIConversationId,
        ctx: &mut ModelContext<Self>,
    ) {
        self.action_model.update(ctx, |action_model, _| {
            action_model.clear_finished_action_results(conversation_id);
        });
    }

    /// Cancels the in-flight request for the given conversation, if there is one.
    ///
    /// Returns `true` if a request was actually cancelled.
    pub fn cancel_request(
        &mut self,
        response_stream_id: &ResponseStreamId,
        reason: CancellationReason,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        self.in_flight_response_streams
            .try_cancel_stream(response_stream_id, reason, ctx)
    }

    fn start_title_generation(
        &mut self,
        pending_title_generation: PendingTitleGeneration,
        stream_id: ResponseStreamId,
        conversation_id: AIConversationId,
        ctx: &mut ModelContext<Self>,
    ) {
        let terminal_view_id = self.terminal_view_id;
        let _ = ctx.spawn(
            async move {
                let result = crate::ai::agent_providers::chat_stream::generate_title_via_byop(
                    &pending_title_generation.input,
                    &pending_title_generation.user_query,
                )
                .await;
                (pending_title_generation.task_id, result)
            },
            move |_me, (task_id, result), ctx| match result {
                Ok(Some(title)) => {
                    log::info!("[byop] title generated: {title:?}");
                    let client_actions = vec![ClientAction {
                        action: Some(Action::UpdateTaskDescription(UpdateTaskDescription {
                            task_id,
                            description: title,
                        })),
                    }];
                    BlocklistAIHistoryModel::handle(ctx).update(ctx, |history_model, ctx| {
                        match history_model.apply_client_actions(
                            &stream_id,
                            client_actions,
                            conversation_id,
                            terminal_view_id,
                            ctx,
                        ) {
                            Ok(()) => {
                                ctx.emit(BlocklistAIHistoryEvent::UpdatedConversationMetadata {
                                    terminal_view_id: Some(terminal_view_id),
                                    conversation_id,
                                });
                            }
                            Err(e) => {
                                log::warn!("[byop] title update failed: {e:#}");
                            }
                        }
                    });
                }
                Ok(None) => {
                    log::warn!("[byop] title gen returned empty content; skip");
                }
                Err(e) => {
                    log::warn!("[byop] title gen failed: {e:#}; skip");
                }
            },
        );
    }

    fn handle_response_stream_event(
        &mut self,
        did_input_contain_user_query: bool,
        event: &ResponseStreamEvent,
        response_stream: &ModelHandle<ResponseStream>,
        ctx: &mut ModelContext<Self>,
    ) {
        let stream_id = response_stream.as_ref(ctx).id().clone();

        match event {
            ResponseStreamEvent::ReceivedEvent(event) => {
                // Dynamic lookup handles conversation splits mid-stream.
                let Some(conversation_id) = BlocklistAIHistoryModel::as_ref(ctx)
                    .conversation_for_response_stream(&stream_id)
                else {
                    log::warn!("Could not find conversation for response stream: {stream_id:?}");
                    return;
                };
                let Some(event) = event.consume() else {
                    debug_assert!(
                        false,
                        "This model should only have a single subscriber that takes ownership over the event."
                    );
                    return;
                };
                let history_model = BlocklistAIHistoryModel::handle(ctx);
                match event {
                    Ok(event) => {
                        let Some(event) = event.r#type else {
                            return;
                        };
                        match event {
                            warp_multi_agent_api::response_event::Type::Init(init_event) => {
                                history_model.update(ctx, |history_model, ctx| {
                                    history_model.initialize_output_for_response_stream(
                                        &stream_id,
                                        conversation_id,
                                        self.terminal_view_id,
                                        init_event,
                                        ctx,
                                    );

                                    // Clear the forked_from token after the first Init event.
                                    // For forked conversations, we only need to send this once so
                                    // viewers can update their conversation's server token. After
                                    // that, the viewer's conversation uses the new token directly.
                                    if let Some(conversation) =
                                        history_model.conversation_mut(&conversation_id)
                                    {
                                        conversation.clear_forked_from_server_conversation_token();
                                    }
                                });
                            }
                            warp_multi_agent_api::response_event::Type::Finished(
                                finished_event,
                            ) => {
                                let completed_successfully = matches!(
                                    finished_event.reason.as_ref(),
                                    Some(
                                        warp_multi_agent_api::response_event::stream_finished::Reason::Done(_)
                                    ) | None
                                );
                                if completed_successfully {
                                    if let Some(pending_title_generation) =
                                        response_stream.update(ctx, |response_stream, _| {
                                            response_stream.take_pending_title_generation()
                                        })
                                    {
                                        self.start_title_generation(
                                            pending_title_generation,
                                            stream_id.clone(),
                                            conversation_id,
                                            ctx,
                                        );
                                    }
                                }
                                // Zap BYOP 本地会话压缩:在 stream finished 前拿 summarization 标志
                                let summarize_overflow =
                                    response_stream.as_ref(ctx).summarization_overflow();
                                self.handle_response_stream_finished(
                                    &stream_id,
                                    finished_event,
                                    conversation_id,
                                    did_input_contain_user_query,
                                    summarize_overflow,
                                    ctx,
                                );
                            }
                            warp_multi_agent_api::response_event::Type::ClientActions(actions) => {
                                let client_actions = actions.actions;
                                let apply_result =
                                    history_model.update(ctx, |history_model, ctx| {
                                        history_model.apply_client_actions(
                                            &stream_id,
                                            client_actions,
                                            conversation_id,
                                            self.terminal_view_id,
                                            ctx,
                                        )
                                    });
                                if let Err(e) = apply_result {
                                    log::error!(
                                        "Failed to apply client actions to conversation: {e:?}"
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if matches!(e.as_ref(), AIApiError::QuotaLimit) {
                            // Zap(Phase 3c A1):删除
                            // `AIRequestUsageModel::enable_buy_credits_banner` 调用。
                            // 本地化后 BYOP 场景下不存在"购买额外 credits"业务。
                        }

                        let mut renderable_error: RenderableAIError = e.as_ref().into();
                        if let RenderableAIError::Other {
                            will_attempt_resume,
                            waiting_for_network,
                            ..
                        } = &mut renderable_error
                        {
                            let should_attempt_resume = response_stream
                                .as_ref(ctx)
                                .should_resume_conversation_after_stream_finished();
                            *will_attempt_resume |= should_attempt_resume;
                            if should_attempt_resume {
                                let network_status = NetworkStatus::as_ref(ctx);
                                *waiting_for_network = !network_status.is_online();
                            }
                        }

                        history_model.update(ctx, |history_model, ctx| {
                            history_model.mark_response_stream_completed_with_error(
                                renderable_error,
                                &stream_id,
                                conversation_id,
                                self.terminal_view_id,
                                ctx,
                            );
                        });
                    }
                }
            }
            ResponseStreamEvent::AfterStreamFinished { cancellation } => {
                // Cancellations provide conversation_id (survives truncation); otherwise use dynamic lookup.
                let conversation_id = match &cancellation {
                    Some(stream_cancellation) => stream_cancellation.conversation_id,
                    None => {
                        let Some(id) = BlocklistAIHistoryModel::as_ref(ctx)
                            .conversation_for_response_stream(&stream_id)
                        else {
                            log::warn!(
                                "Could not find conversation for response stream: {stream_id:?}"
                            );
                            return;
                        };
                        id
                    }
                };

                let history_model = BlocklistAIHistoryModel::handle(ctx);
                let Some(conversation) = history_model.as_ref(ctx).conversation(&conversation_id)
                else {
                    log::warn!("Conversation not found.");
                    return;
                };
                let new_exchange_ids = conversation.new_exchange_ids_for_response(&stream_id);
                let mut was_passive_request = false;
                let mut is_any_exchange_unfinished = false;
                let mut actions_to_queue = vec![];
                // Zap BYOP:收集本轮新加 message id,稍后用于在 EMPTY 分支检测
                // synthetic invalid_arguments 错误标记。**只看本轮 added** 才能避免
                // 在历史里反复命中导致 auto-resume 死循环(标记一旦持久化就永远在)。
                let mut newly_added_message_ids: std::collections::HashSet<MessageId> =
                    std::collections::HashSet::new();

                for new_exchange_id in new_exchange_ids {
                    let Some(exchange) = conversation.exchange_with_id(new_exchange_id) else {
                        log::warn!("Exchange not found.");
                        return;
                    };
                    was_passive_request |= exchange.has_passive_request();
                    is_any_exchange_unfinished |= !exchange.output_status.is_finished();
                    newly_added_message_ids.extend(exchange.added_message_ids.iter().cloned());

                    if let AIAgentOutputStatus::Finished {
                        finished_output: FinishedAIAgentOutput::Success { output },
                        ..
                    } = &exchange.output_status
                    {
                        actions_to_queue.extend(output.get().actions().cloned());
                    }
                }

                if let Some(stream_cancellation) = &cancellation {
                    history_model.update(ctx, |history_model, ctx| {
                        history_model.mark_response_stream_cancelled(
                            &stream_id,
                            conversation_id,
                            self.terminal_view_id,
                            stream_cancellation.reason,
                            ctx,
                        );
                    });

                    if !was_passive_request {
                        self.set_input_mode_for_cancellation(ctx);
                    }
                } else if is_any_exchange_unfinished {
                    log::warn!("AI response stream ended without emitting StreamFinished event.");

                    let error_message = "Request did not successfully complete";
                    history_model.update(ctx, |history_model, ctx| {
                        history_model.mark_response_stream_completed_with_error(
                            RenderableAIError::Other {
                                error_message: error_message.to_string(),
                                will_attempt_resume: false,
                                waiting_for_network: false,
                            },
                            &stream_id,
                            conversation_id,
                            self.terminal_view_id,
                            ctx,
                        );
                    });
                } else if !actions_to_queue.is_empty() {
                    log::info!(
                        "[byop-diag] queue_actions: count={} ids=[{}] conversation_id={:?}",
                        actions_to_queue.len(),
                        actions_to_queue
                            .iter()
                            .map(|a| format!("{}", a.id))
                            .collect::<Vec<_>>()
                            .join(", "),
                        conversation_id,
                    );
                    // Zap:LRC tag-in 首轮自动授权 agent 工具执行。
                    //
                    // 触发条件:发起本轮请求时 active_block 处于
                    // InteractionMode::User { did_user_tag_in_agent: true }。不能用当前
                    // active_block 的 monitored metadata 兜底,否则同一 CLI subagent 会话里的
                    // 后续普通请求也会被自动确认,导致确认 UI 不显示。
                    let auto_accept_for_lrc_tag_in =
                        response_stream.as_ref(ctx).is_lrc_tag_in_request();
                    if auto_accept_for_lrc_tag_in {
                        log::info!(
                            "[byop] LRC tag-in: queue with auto-accept ({} action(s))",
                            actions_to_queue.len()
                        );
                    }
                    self.action_model.update(ctx, |action_model, ctx| {
                        action_model.queue_actions_with_options(
                            actions_to_queue,
                            conversation_id,
                            auto_accept_for_lrc_tag_in,
                            ctx,
                        );
                    });
                } else {
                    // Zap BYOP:from_args 解析失败时,chat_stream 走 fallback emit
                    // carrier ToolCall(tool=None) + synthetic error ToolCallResult(result=None,
                    // server_message_data 是 invalid_arguments JSON)。两者都走 NoClientRepresentation,
                    // 不入 actions_to_queue,exchange 静默结束 → 模型永远收不到错误反馈,
                    // 用户必须手动再发消息才能让模型重试。
                    //
                    // 检测最近 ~16 条 messages 是否含 BYOP synthetic 错误标记;有的话复用
                    // line 2695+ 的 auto-resume 路径触发重发,让模型立即基于 error tool_result
                    // 修正参数重试。`can_attempt_resume_on_error=false` 防 LLM 持续输出坏 args 导致死循环。
                    // 只在本轮新加的 messages 里查找 synthetic 错误标记,避免历史持久化的
                    // 同标记反复命中导致死循环。
                    // Zap BYOP:没有进入 AIAgentAction 队列的 synthetic
                    // ToolCallResult 需要 auto-resume,否则 exchange 静默结束,模型卡死等结果。
                    // 1. invalid_arguments — from_args 解析失败兜底(原始)。
                    // 2. _byop_intercepted — todowrite / webfetch / websearch 等本地拦截
                    //    工具结果。这类工具不走 protobuf executor,直接合成 result,
                    //    没有 AIAgentAction 入队。
                    let needs_byop_local_resume = conversation.all_tasks().any(|task| {
                        task.messages().any(|msg| {
                            newly_added_message_ids.contains(&MessageId::new(msg.id.clone()))
                                && matches!(
                                    msg.message,
                                    Some(message::Message::ToolCallResult(
                                        message::ToolCallResult { result: None, .. },
                                    )),
                                )
                                && (msg
                                    .server_message_data
                                    .contains(r#""error":"invalid_arguments""#)
                                    || msg
                                        .server_message_data
                                        .contains(r#""_byop_intercepted":true"#))
                        })
                    });
                    if needs_byop_local_resume {
                        log::info!(
                            "[byop] detected synthetic local tool_result (invalid_arguments \
                             or _byop_intercepted) without queued action → schedule auto-resume. \
                             conversation_id={conversation_id:?}"
                        );
                        let network_status = NetworkStatus::handle(ctx);
                        let wait_for_online = network_status.as_ref(ctx).wait_until_online();
                        let handle = ctx.spawn(wait_for_online, move |me, _, ctx| {
                            me.pending_auto_resume_handles.remove(&conversation_id);
                            me.resume_conversation(
                                conversation_id,
                                /*can_attempt_resume_on_error*/
                                false,
                                /*is_auto_resume_after_error*/
                                true,
                                vec![],
                                ctx,
                            );
                        });
                        self.pending_auto_resume_handles
                            .insert(conversation_id, handle);
                    }
                }

                // Cancelled streams will handle pending_response_stream updates synchronously.
                if cancellation.is_none() {
                    self.in_flight_response_streams.cleanup_stream(&stream_id);
                }

                // Before cleaning up the response stream, check if we should attempt to resume.
                if response_stream
                    .as_ref(ctx)
                    .should_resume_conversation_after_stream_finished()
                {
                    let network_status = NetworkStatus::handle(ctx);
                    let wait_for_online = network_status.as_ref(ctx).wait_until_online();
                    let handle = ctx.spawn(wait_for_online, move |me, _, ctx| {
                        // Clean up the pending handle now that the resume is executing.
                        me.pending_auto_resume_handles.remove(&conversation_id);
                        me.resume_conversation(
                            conversation_id,
                            // Don't allow a second resume-on-error to prevent a persistent
                            // loop.
                            /*can_attempt_resume_on_error*/
                            false,
                            /*is_auto_resume_after_error*/
                            true,
                            vec![],
                            ctx,
                        );
                    });
                    self.pending_auto_resume_handles
                        .insert(conversation_id, handle);
                }

                // Clean up the response stream tracking entry now that the stream is complete.
                history_model.update(ctx, |history_model, _| {
                    if let Some(conversation) = history_model.conversation_mut(&conversation_id) {
                        conversation.cleanup_completed_response_stream(&stream_id);
                    }
                });
                ctx.unsubscribe_from_model(response_stream);

                if self.should_refresh_available_llms_on_stream_finish {
                    self.should_refresh_available_llms_on_stream_finish = false;
                    LLMPreferences::handle(ctx).update(ctx, |llm_preferences, ctx| {
                        llm_preferences.refresh_authed_models(ctx);
                    });
                }
                ctx.emit(BlocklistAIControllerEvent::FinishedReceivingOutput {
                    stream_id,
                    conversation_id,
                });
                // Zap(Phase 3c A1):删除
                // `AIRequestUsageModel::refresh_request_usage_async` 与
                // `maybe_refresh_ai_overages` 调用。两者本质都是服务端计量同步 RPC，
                // 本地化后无作用。
            }
        }
    }

    /// Sets the terminal input state after an AI request is cancelled.
    /// From the user perspective, we downgrade the level of autonomy so:
    /// * Executing a task automatically -> interactive AI input
    /// * Interactive AI input -> interactive shell input
    fn set_input_mode_for_cancellation(&mut self, ctx: &mut ModelContext<Self>) {
        // If the request was cancelled, default to shell mode with autodetection
        // enabled.
        self.input_model.update(ctx, |input_model, ctx| {
            input_model.set_input_config_for_classic_mode(
                input_model
                    .input_config()
                    .with_shell_type()
                    .unlocked_if_autodetection_enabled(false, ctx),
                ctx,
            );
        });
    }

    // Zap(Phase 3c A1):删除 `maybe_refresh_ai_overages` 函数。
    // 原实现是“本地 limit 耗尽时从服务端拉取最新 overage 状态”的调优路径，
    // BYOP 本地化后既无 limit 也无 overage，函数本体与唯一调用点都需要一起删除。

    pub(super) fn handle_response_stream_finished(
        &mut self,
        stream_id: &ResponseStreamId,
        mut finished_event: warp_multi_agent_api::response_event::StreamFinished,
        conversation_id: AIConversationId,
        did_request_contain_user_query: bool,
        summarize_overflow: Option<bool>,
        ctx: &mut ModelContext<Self>,
    ) {
        // Zap BYOP 本地会话压缩:在 token_usage move 进下面 closure 前先聚合,
        // 用于 auto overflow 检查(后面 Done 分支用)。
        let aggregate_token_count: usize = finished_event
            .token_usage
            .iter()
            .map(|u| (u.total_input + u.output + u.input_cache_read + u.input_cache_write) as usize)
            .max()
            .unwrap_or(0);

        let history_model = BlocklistAIHistoryModel::handle(ctx);
        history_model.update(ctx, |history_model, _| {
            // Update conversation cost and usage information before updating and
            // persisting the conversation.
            history_model.update_conversation_cost_and_usage_for_request(
                conversation_id,
                finished_event
                    .request_cost
                    .map(|cost| RequestCost::new(cost.exact.into())),
                finished_event.token_usage,
                finished_event.conversation_usage_metadata.take(),
                did_request_contain_user_query,
            );
        });

        let history_model = BlocklistAIHistoryModel::handle(ctx);
        match finished_event.reason {
            Some(warp_multi_agent_api::response_event::stream_finished::Reason::Done(_)) | None => {
                // Zap BYOP 本地会话压缩 - 写回 summary
                if let Some(overflow) = summarize_overflow {
                    let compaction_cfg = crate::ai::byop_compaction::CompactionConfig::from_settings(ctx);
                    history_model.update(ctx, |history_model, _ctx| {
                        if let Some(convo) = history_model.conversation_mut(&conversation_id) {
                            crate::ai::byop_compaction::commit::commit_summarization(
                                convo,
                                overflow,
                                &compaction_cfg,
                            );
                        }
                    });
                }
                history_model.update(ctx, |history_model, ctx| {
                    history_model.mark_response_stream_completed_successfully(
                        stream_id,
                        conversation_id,
                        self.terminal_view_id,
                        ctx,
                    );
                });

                // Zap BYOP 本地会话压缩 - auto overflow 触发(对齐 opencode `processor.ts:395-403`)
                // 仅在本流不是摘要本身时检查,防止递归。
                if summarize_overflow.is_none() {
                    let aggregate_count = aggregate_token_count;
                    if aggregate_count > 0 {
                        let cfg = crate::ai::byop_compaction::CompactionConfig::from_settings(ctx);
                        let model_limit =
                            crate::ai::byop_compaction::overflow::ModelLimit::FALLBACK;
                        let counts = crate::ai::byop_compaction::overflow::TokenCounts {
                            total: aggregate_count,
                            ..Default::default()
                        };
                        if crate::ai::byop_compaction::is_overflow(&cfg, counts, model_limit) {
                            log::info!(
                                "[byop-compaction] auto overflow detected: tokens={aggregate_count} usable={}",
                                crate::ai::byop_compaction::usable(&cfg, model_limit)
                            );
                            // 通过 SlashCommandRequest::Summarize 触发(与 /compact-and 同链路);
                            // overflow=true → chat_stream 拼摘要请求时携带 overflow 标记,
                            // commit_summarization 写回时也以 overflow=true 落 state(便于 UI 区分)。
                            self.send_slash_command_request(
                                crate::ai::blocklist::controller::SlashCommandRequest::Summarize {
                                    prompt: None,
                                    overflow: true,
                                },
                                ctx,
                            );
                        }
                    }
                }
            }
            Some(warp_multi_agent_api::response_event::stream_finished::Reason::Other(_)) => {
                let error_message = "Response stream finished unexpectedly (with finish reason `Other`).";
                history_model.update(ctx, |history_model, ctx| {
                    history_model.mark_response_stream_completed_with_error(
                        RenderableAIError::Other {
                            error_message: error_message.to_owned(),
                            will_attempt_resume: false,
                            waiting_for_network: false,
                        },
                        stream_id,
                        conversation_id,
                        self.terminal_view_id,
                        ctx,
                    );
                });
            }
            Some(warp_multi_agent_api::response_event::stream_finished::Reason::ContextWindowExceeded(_)) => {
                let error_message = "Input exceeded context window limit.";
                history_model.update(ctx, |history_model, ctx| {
                    history_model.mark_response_stream_completed_with_error(
                        RenderableAIError::ContextWindowExceeded(error_message.to_owned()),
                        stream_id,
                        conversation_id,
                        self.terminal_view_id,
                        ctx,
                    );
                });
            }
            Some(warp_multi_agent_api::response_event::stream_finished::Reason::QuotaLimit(_)) => {
                history_model.update(ctx, |history_model, ctx| {
                    history_model.mark_response_stream_completed_with_error(
                        RenderableAIError::QuotaLimit,
                        stream_id,
                        conversation_id,
                        self.terminal_view_id,
                        ctx,
                    );
                });
            }
            Some(warp_multi_agent_api::response_event::stream_finished::Reason::LlmUnavailable(_)) => {
                let error_message = "The LLM is currently unavailable.";
                history_model.update(ctx, |history_model, ctx| {
                    history_model.mark_response_stream_completed_with_error(
                        RenderableAIError::Other {
                            error_message: error_message.to_owned(),
                            will_attempt_resume: false,
                            waiting_for_network: false,
                        },
                        stream_id,
                        conversation_id,
                        self.terminal_view_id,
                        ctx,
                    );
                });
            }
            Some(warp_multi_agent_api::response_event::stream_finished::Reason::InvalidApiKey(details)) => {
                use warp_multi_agent_api::LlmProvider;
                let is_aws_bedrock = details
                    .provider
                    .try_into()
                    .ok()
                    .is_some_and(|p: LlmProvider| p == LlmProvider::AwsBedrock);

                let error = if is_aws_bedrock {
                    RenderableAIError::AwsBedrockCredentialsExpiredOrInvalid {
                        model_name: details.model_name,
                    }
                } else {
                    let provider = details.provider.try_into().ok().and_then(|p| match p {
                        LlmProvider::Google => Some("Google"),
                        LlmProvider::Anthropic => Some("Anthropic"),
                        LlmProvider::Openai => Some("OpenAI"),
                        LlmProvider::Xai => Some("xAI"),
                        LlmProvider::Openrouter => Some("OpenRouter"),
                        LlmProvider::AwsBedrock | LlmProvider::Unknown => None,
                    });
                    RenderableAIError::InvalidApiKey {
                        provider: provider.unwrap_or("Unknown").to_string(),
                        model_name: details.model_name,
                    }
                };

                history_model.update(ctx, |history_model, ctx| {
                    history_model.mark_response_stream_completed_with_error(
                        error,
                        stream_id,
                        conversation_id,
                        self.terminal_view_id,
                        ctx,
                    );
                });
            }
            Some(warp_multi_agent_api::response_event::stream_finished::Reason::InternalError(
                warp_multi_agent_api::response_event::stream_finished::InternalError{ message})) => {
                let error_message = format!(
                    "Response stream finished unexpectedly with internal error: {message}",
                );
                history_model.update(ctx, |history_model, ctx| {
                    history_model.mark_response_stream_completed_with_error(
                        RenderableAIError::Other {
                            error_message,
                            will_attempt_resume: false,
                            waiting_for_network: false,
                        },
                        stream_id,
                        conversation_id,
                        self.terminal_view_id,
                        ctx,
                    );
                });
            }
            Some(warp_multi_agent_api::response_event::stream_finished::Reason::MaxTokenLimit(_)) => {
                let error_message = "Input exceeded context window limit.";
                history_model.update(ctx, |history_model, ctx| {
                    history_model.mark_response_stream_completed_with_error(
                        RenderableAIError::ContextWindowExceeded(error_message.to_owned()),
                        stream_id,
                        conversation_id,
                        self.terminal_view_id,
                        ctx,
                    );
                });
            }
        }

        if finished_event.should_refresh_model_config {
            LLMPreferences::handle(ctx).update(ctx, |llm_preferences, ctx| {
                llm_preferences.refresh_authed_models(ctx);
            });
        }
    }
}

impl Entity for BlocklistAIController {
    type Event = BlocklistAIControllerEvent;
}

#[derive(Clone)]
pub struct ClientIdentifiers {
    pub conversation_id: AIConversationId,
    pub client_exchange_id: AIAgentExchangeId,
    /// Not populated for restored AI blocks.
    pub response_stream_id: Option<ResponseStreamId>,
}

#[allow(clippy::too_many_arguments)]
fn input_for_query(
    query: String,
    task_id: &TaskId,
    conversation_id: AIConversationId,
    static_query_type: Option<StaticQueryType>,
    user_query_mode: UserQueryMode,
    running_command: Option<RunningCommand>,
    additional_attachments: HashMap<String, AIAgentAttachment>,
    context_model: &BlocklistAIContextModel,
    active_session: &ActiveSession,
    app: &AppContext,
) -> AIAgentInput {
    let context = input_context_for_request(
        true,
        context_model,
        active_session,
        Some(conversation_id),
        vec![],
        app,
    );
    let intended_agent = BlocklistAIHistoryModel::as_ref(app)
        .conversation(&conversation_id)
        .and_then(|c| c.get_task(task_id))
        .and_then(|task| {
            if task.is_root_task() {
                Some(warp_multi_agent_api::AgentType::Primary)
            } else if task.is_cli_subagent() {
                Some(warp_multi_agent_api::AgentType::Cli)
            } else {
                None
            }
        });
    let mut referenced_attachments = parse_context_attachments(&query, context_model, app);
    referenced_attachments.extend(additional_attachments);
    AIAgentInput::UserQuery {
        query,
        context,
        static_query_type,
        referenced_attachments,
        user_query_mode,
        running_command,
        intended_agent,
    }
}

/// Validates that tool call results have corresponding tool calls in the task context, otherwise
/// logs a warning.
fn validate_tool_call_results<'a>(
    inputs: impl Iterator<Item = &'a AIAgentInput>,
    tasks: &[Task],
    server_conversation_token: &Option<ServerConversationToken>,
) {
    // Create a mapping from tool call IDs to their task IDs
    let mut tool_call_to_task_map: HashMap<String, String> = HashMap::new();
    for task in tasks {
        for message in &task.messages {
            if let Some(message::Message::ToolCall(tool_call)) = &message.message {
                tool_call_to_task_map
                    .insert(tool_call.tool_call_id.clone(), message.task_id.clone());
            }
        }
    }

    // Check each input for tool call results and validate they have corresponding tool calls
    for input in inputs {
        if let AIAgentInput::ActionResult { result, .. } = input {
            let action_id_str = result.id.to_string();
            let server_conversation_id = server_conversation_token
                .as_ref()
                .map(|token| token.as_str())
                .unwrap_or("None");

            if !tool_call_to_task_map.contains_key(&action_id_str) {
                log::warn!(
                    "Found tool call result with ID '{action_id_str}' but no corresponding tool \
                    call in task context. Server conversation ID: '{server_conversation_id}'"
                );
            }
        }
    }
}

fn get_running_command(terminal_model: &TerminalModel) -> Option<RunningCommand> {
    let active_block = terminal_model.block_list().active_block();
    if !active_block.is_active_and_long_running() || active_block.is_agent_monitoring() {
        return None;
    }
    let is_alt_screen_active = terminal_model.is_alt_screen_active();
    Some(RunningCommand {
        block_id: active_block.id().clone(),
        command: active_block.command_to_string(),
        grid_contents: if is_alt_screen_active {
            formatted_terminal_contents_for_input(
                terminal_model.alt_screen().grid_handler(),
                None,
                CURSOR_MARKER,
            )
        } else {
            formatted_terminal_contents_for_input(
                active_block.output_grid().grid_handler(),
                // TODO(vorporeal): This is probably too large.
                Some(1000),
                CURSOR_MARKER,
            )
        },
        cursor: CURSOR_MARKER.to_owned(),
        requested_command_id: active_block.requested_command_action_id().cloned(),
        is_alt_screen_active,
    })
}

/// Zap BYOP 专用:LRC tag-in / agent-monitored 场景下提取 RunningCommand。
///
/// 上游 `get_running_command` 在 `is_agent_monitoring()` 时返回 None — 因为 Zap 自家
/// 路径下 LRC 已 spawn cli subagent 后,server 端持久该状态,后续轮 client 不必重发
/// running_command。但 BYOP 直连模型无服务端持久,**每轮都要把当前 PTY grid 内容
/// 重新带给模型**(否则模型只能看到首轮 grid_contents 之后的盲区)。
///
/// 条件放宽为 `is_agent_in_control_or_tagged_in()` — 覆盖:
///   - tag-in:`InteractionMode::User { did_user_tag_in_agent: true }`(spawn 前)
///   - monitored:`InteractionMode::Agent { ... }`(spawn 后)
///
/// 提取逻辑严格对齐上游:alt-screen 时取 `terminal_model.alt_screen().grid_handler()`
/// 而不是 `active_block.output_grid()`(后者在 alt-screen 期间是空的,
/// 不要再用 `output_to_string_force_full_grid_contents()`,那条路在 nvim 等 TUI 下
/// 会得到空字符串导致 `<attached_running_command>` 块为空,模型抱怨"看不到 command_id")。
fn byop_get_running_command_for_lrc(terminal_model: &TerminalModel) -> Option<RunningCommand> {
    let active_block = terminal_model.block_list().active_block();
    if !active_block.is_active_and_long_running() {
        return None;
    }
    if !active_block.is_agent_in_control_or_tagged_in() {
        return None;
    }
    let is_alt_screen_active = terminal_model.is_alt_screen_active();
    Some(RunningCommand {
        block_id: active_block.id().clone(),
        command: active_block.command_to_string(),
        grid_contents: if is_alt_screen_active {
            formatted_terminal_contents_for_input(
                terminal_model.alt_screen().grid_handler(),
                None,
                CURSOR_MARKER,
            )
        } else {
            formatted_terminal_contents_for_input(
                active_block.output_grid().grid_handler(),
                Some(1000),
                CURSOR_MARKER,
            )
        },
        cursor: CURSOR_MARKER.to_owned(),
        requested_command_id: active_block.requested_command_action_id().cloned(),
        is_alt_screen_active,
    })
}
