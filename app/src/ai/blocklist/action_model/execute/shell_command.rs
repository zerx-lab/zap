use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures::channel::oneshot;
use futures::future::BoxFuture;
use futures::{select, FutureExt};
use futures_lite::pin;
use itertools::Itertools;
use parking_lot::FairMutex;
use warp_core::command::ExitCode;
use warp_core::execution_mode::AppExecutionMode;
use warp_util::path::ShellFamily;
use warpui::r#async::{Spawnable, Timer};
use warpui::{Entity, EntityId, ModelContext, ModelHandle, SingletonEntity};

use crate::ai::agent::{
    AIAgentActionId, AIAgentActionType, AIAgentPtyWriteMode, ReadShellCommandOutputResult,
    RequestCommandOutputResult, ShellCommandDelay, ShellCommandError,
    TransferShellCommandControlToUserResult, WriteToLongRunningShellCommandResult,
};
use crate::ai::blocklist::permissions::CommandExecutionPermission;
use crate::ai::blocklist::BlocklistAIPermissions;
use crate::ai::execution_profiles::WriteToPtyPermission;
use crate::terminal::event::BlockMetadataReceivedEvent;
use crate::terminal::model::block::{
    formatted_terminal_contents_for_input, Block, BlockId, CURSOR_MARKER,
};
use crate::terminal::shell::ShellType;
use crate::terminal::ssh::util::parse_interactive_ssh_command;
use crate::{
    ai::agent::AIAgentActionResultType,
    terminal::{
        model::session::active_session::ActiveSession,
        model_events::{ModelEvent, ModelEventDispatcher},
        TerminalModel,
    },
};
use crate::{send_telemetry_from_ctx, TelemetryEvent};

use super::{ActionExecution, AnyActionExecution, ExecuteActionInput, PreprocessActionInput};

/// Text returned to the agent for `run_shell_command` / related tools.
///
/// Prefer unobfuscated grid text; some shells / timing paths leave the primary serialization empty
/// while `output_to_string()` (displayed-output path) or the force-full path still has bytes.
///
/// Fallback: when output grids are empty (e.g. missing preexec / early-complete timing), extract
/// the stdout portion from the command grid (everything after the first line).
fn agent_shell_command_block_output(block: &Block) -> String {
    let primary = block.output_with_secrets_unobfuscated();
    if !primary.trim().is_empty() {
        return primary;
    }

    let displayed = block.output_to_string();
    if !displayed.trim().is_empty() {
        return displayed;
    }

    let forced = block.output_to_string_force_full_grid_contents();
    if !forced.trim().is_empty() {
        return forced;
    }

    let command_grid = block.command_with_secrets_unobfuscated(false);
    command_grid
        .split_once('\n')
        .map(|(_, output)| output.to_owned())
        .filter(|output| !output.trim().is_empty())
        .unwrap_or_default()
}

pub struct ShellCommandExecutor {
    active_session: ModelHandle<ActiveSession>,
    block_finished_senders: HashMap<BlockSelector, oneshot::Sender<()>>,
    /// Senders used by the `Check now` affordance to force a long-running shell command's
    /// pending poll future to resolve immediately with a fresh snapshot, bypassing the
    /// agent-set timeout.
    force_refresh_senders: HashMap<BlockSelector, oneshot::Sender<()>>,
    terminal_model: Arc<FairMutex<TerminalModel>>,
    terminal_view_id: EntityId,
    /// Sender to notify when user hands control back to agent after TransferShellCommandControlToUser.
    control_handback_sender: Option<oneshot::Sender<()>>,
}

impl ShellCommandExecutor {
    pub const MAX_WAIT_DURATION: Duration = Duration::from_secs(2);
    /// Maximum delay we will honor for any agent-requested wait. Applies both  
    /// to finite `ShellCommandDelay::Duration` requests and to  
    /// `ShellCommandDelay::OnCompletion`, which would otherwise wait indefinitely.  
    pub const MAX_AGENT_DELAY_DURATION: Duration = Duration::from_secs(120);

    pub fn new(
        active_session: ModelHandle<ActiveSession>,
        terminal_model: Arc<FairMutex<TerminalModel>>,
        model_event_dispatcher: &ModelHandle<ModelEventDispatcher>,
        terminal_view_id: EntityId,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        ctx.subscribe_to_model(model_event_dispatcher, Self::handle_terminal_model_event);

        Self {
            active_session,
            terminal_model,
            block_finished_senders: HashMap::new(),
            force_refresh_senders: HashMap::new(),
            terminal_view_id,
            control_handback_sender: None,
        }
    }

    fn handle_terminal_model_event(&mut self, event: &ModelEvent, _ctx: &mut ModelContext<Self>) {
        // We wait for precmd for the block _after_ the requested command's block so that
        // downstream checks for current working directory are fresh. The precmd hook is when
        // the shell relays current working directory to warp.
        if let ModelEvent::BlockMetadataReceived(BlockMetadataReceivedEvent { .. }) = event {
            let model = self.terminal_model.lock();
            let block_finished_senders = self.block_finished_senders.drain().collect_vec();
            for (block_selector, block_finished_tx) in block_finished_senders.into_iter() {
                if let Some(block) = block_selector.get_block(&model) {
                    if block.is_command_finished() {
                        if let Err(e) = block_finished_tx.send(()) {
                            log::warn!(
                                "Failed to notify block completion for running requested command: {e:?}"
                            )
                        }
                    } else {
                        self.block_finished_senders
                            .insert(block_selector, block_finished_tx);
                    }
                }
            }
        }
    }

    pub(super) fn should_autoexecute(
        &self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        let blocklist_permissions = BlocklistAIPermissions::as_ref(ctx);
        match &input.action.action {
            AIAgentActionType::RequestCommandOutput {
                command,
                is_read_only,
                is_risky,
                ..
            } => {
                let Some(escape_char) = self
                    .active_session
                    .as_ref(ctx)
                    .shell_type(ctx)
                    .map(|s| ShellFamily::from(s).escape_char())
                else {
                    return false;
                };
                let autoexecution_permission = blocklist_permissions.can_autoexecute_command(
                    &input.conversation_id,
                    command,
                    escape_char,
                    is_read_only.unwrap_or(false),
                    *is_risky,
                    Some(self.terminal_view_id),
                    ctx,
                );
                if let CommandExecutionPermission::Allowed(reason) = autoexecution_permission {
                    send_telemetry_from_ctx!(
                        TelemetryEvent::AutoexecutedAgentModeRequestedCommand { reason },
                        ctx
                    );
                } else if let CommandExecutionPermission::Denied(reason) = autoexecution_permission
                {
                    if AppExecutionMode::as_ref(ctx).is_autonomous() {
                        log::warn!(
                            "Command denied during autonomous execution, reason: {reason:?}"
                        );
                    }
                }
                autoexecution_permission.is_allowed()
            }
            AIAgentActionType::WriteToLongRunningShellCommand { block_id, .. } => {
                let terminal_model = self.terminal_model.lock();
                let block = terminal_model.block_list().block_with_id(block_id);

                if block.is_none_or(|block| block.finished()) {
                    // If the block is already finished, allow auto-execution - the finished output
                    // will be returned.
                    true
                } else {
                    let should_autoexecute = match blocklist_permissions.can_write_to_pty(
                        &input.conversation_id,
                        Some(self.terminal_view_id),
                        ctx,
                    ) {
                        WriteToPtyPermission::AlwaysAllow => true,
                        WriteToPtyPermission::AskOnFirstWrite => terminal_model
                            .block_list()
                            .active_block()
                            .has_agent_written_to_block(),
                        _ => false,
                    };

                    if should_autoexecute {
                        send_telemetry_from_ctx!(
                            TelemetryEvent::CLISubagentActionExecuted {
                                conversation_id: input.conversation_id,
                                block_id: block_id.clone(),
                                is_autoexecuted: true,
                            },
                            ctx
                        );
                    }

                    should_autoexecute
                }
            }
            AIAgentActionType::ReadShellCommandOutput { .. } => true,
            AIAgentActionType::TransferShellCommandControlToUser { .. } => false,
            _ => false,
        }
    }

    /// 用一组通用 pager 环境变量包裹命令,让命令在不进 pager 的同时**保留真实退出码**。
    ///
    /// 之前的实现是 `(cmd) | cat`,虽然能让 stdout 不再是 tty(从而 git/man/less 等不调 pager),
    /// 但 bash/zsh 下 `$?` 会被 `cat` 的退出码(几乎总是 0)覆盖,导致 agent 看到 `cargo check`
    /// 失败时仍然得到 exit_code=0,做出错误判断。
    ///
    /// 这里改用 `PAGER=cat GIT_PAGER=cat MANPAGER=cat` 并在子壳/script block 里执行,
    /// 既能覆盖 git/man/bat/kubectl/psql/gh 等绝大多数 CLI 的 pager 行为,又让外层 `$?` /
    /// `$LASTEXITCODE` 取自命令本身。
    fn turn_off_pager_for_command(&self, command: &String, ctx: &mut ModelContext<Self>) -> String {
        match self.active_session.as_ref(ctx).shell_type(ctx) {
            // 子壳里 export,子壳退出码 = 最后一条命令的退出码,从而保留真实 $?。
            Some(ShellType::Zsh) | Some(ShellType::Bash) => format!(
                "(export PAGER=cat GIT_PAGER=cat MANPAGER=cat; {command})"
            ),
            // fish: set -lx 在 begin/end 块内是局部 export, $status 取最后一条命令。
            Some(ShellType::Fish) => format!(
                "begin; set -lx PAGER cat; set -lx GIT_PAGER cat; set -lx MANPAGER cat; {command}; end"
            ),
            // pwsh: script block 局部 $env: 不污染外层会话, $LASTEXITCODE 透出。
            Some(ShellType::PowerShell) => format!(
                "& {{ $env:PAGER='cat'; $env:GIT_PAGER='cat'; $env:MANPAGER='cat'; {command} }}"
            ),
            // 未知 shell 无法安全装饰,直接放过。
            None => command.clone(),
        }
    }

    pub(super) fn execute(
        &mut self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> impl Into<AnyActionExecution> {
        let model = self.terminal_model.lock();

        // Determine the action we want to take based on the input.
        let action_id = input.action.id.clone();

        let handle = ctx.handle();
        match &input.action.action {
            AIAgentActionType::RequestCommandOutput {
                command,
                uses_pager,
                wait_until_completion,
                ..
            } => {
                if model
                    .block_list()
                    .active_block()
                    .is_active_and_long_running()
                {
                    // If there is an active block, we can't execute another command.
                    return ActionExecution::Sync(AIAgentActionResultType::RequestCommandOutput(
                        RequestCommandOutputResult::CancelledBeforeExecution,
                    ));
                }
                // OpenWarp:同步等待型命令(wait_until_completion=true)无条件禁用 pager。
                //
                // 模型自报的 `uses_pager` 不可靠 —— deepseek-v4-flash 等小模型几乎不会主动标,
                // 一旦命中 `git diff`/`git log`/`man` 等隐式 pager 就会卡在 less 提示符,
                // warp 把命令降级成 LongRunningCommandSnapshot 返回,但 agent 不知道这种契约
                // 切换、继续并行发新 tool call,导致 PTY 和 UI 双重锁死(输入框消失)。
                //
                // 治本逻辑:既然 agent 显式说"等到完成",pager 提示符违反这个契约,warp
                // 必须确保 pager 一定不被触发,而不是让模型来预判每个 CLI 的分页行为。
                //
                // 不影响显式异步路径(wait_until_completion=false),tail -f / dev server
                // 等真正长运行命令仍走原有 LongRunningCommandSnapshot 链路。
                let _ = uses_pager; // 字段保留作 API 兼容,但语义已不再依赖
                let decorated_command = if *wait_until_completion {
                    self.turn_off_pager_for_command(command, ctx)
                } else {
                    command.clone()
                };
                ctx.emit(ShellCommandExecutorEvent::ExecuteCommand {
                    action_id: action_id.clone(),
                    command: decorated_command,
                });

                let block_selector = BlockSelector::RequestedCommandId(action_id.clone());
                let command = command.clone();
                drop(model);

                ActionExecution::new_async(
                    self.action_result_future(
                        block_selector.clone(),
                        action_result_delay_for_requested_command(*wait_until_completion),
                    ),
                    move |result, ctx| {
                        // Remove the senders from the maps.
                        if let Some(handle) = handle.upgrade(ctx) {
                            handle.update(ctx, |me, _| {
                                me.block_finished_senders.remove(&block_selector);
                                me.force_refresh_senders.remove(&block_selector);
                            });
                        }

                        action_result_for_requested_command(command, result)
                    },
                )
            }
            AIAgentActionType::WriteToLongRunningShellCommand {
                block_id,
                input,
                mode,
            } => {
                let Some(block) = model.block_list().block_with_id(block_id) else {
                    return ActionExecution::Sync(
                        AIAgentActionResultType::WriteToLongRunningShellCommand(
                            WriteToLongRunningShellCommandResult::Error(
                                ShellCommandError::BlockNotFound,
                            ),
                        ),
                    );
                };
                if block.finished() {
                    let output: String = agent_shell_command_block_output(block);
                    let exit_code = block.exit_code();
                    return ActionExecution::Sync(
                        AIAgentActionResultType::WriteToLongRunningShellCommand(
                            WriteToLongRunningShellCommandResult::CommandFinished {
                                block_id: block.id().clone(),
                                output,
                                exit_code,
                            },
                        ),
                    );
                }
                // Drop immutable borrow.
                drop(model);

                let mut model = self.terminal_model.lock();
                if let Some(block) = model.block_list_mut().mut_block_from_id(block_id) {
                    block.mark_agent_written_to_block();
                }
                drop(model);

                ctx.emit(ShellCommandExecutorEvent::WriteToPty {
                    input: input.clone(),
                    mode: *mode,
                });

                let block_selector = BlockSelector::Id(block_id.clone());
                ActionExecution::new_async(
                    self.action_result_future(
                        block_selector.clone(),
                        ActionResultDelay::Duration(Duration::from_millis(200)),
                    ),
                    move |result, ctx| {
                        // Remove the senders from the maps.
                        if let Some(handle) = handle.upgrade(ctx) {
                            handle.update(ctx, |me, _| {
                                me.block_finished_senders.remove(&block_selector);
                                me.force_refresh_senders.remove(&block_selector);
                            });
                        }

                        action_result_for_write_to_long_running_shell_command(result)
                    },
                )
            }
            AIAgentActionType::ReadShellCommandOutput { block_id, delay } => {
                let Some(block) = model.block_list().block_with_id(block_id) else {
                    return ActionExecution::Sync(AIAgentActionResultType::ReadShellCommandOutput(
                        ReadShellCommandOutputResult::Error(ShellCommandError::BlockNotFound),
                    ));
                };
                if block.finished() {
                    let command = block.command_with_secrets_unobfuscated(false);
                    let output: String = block.output_with_secrets_unobfuscated();
                    let exit_code = block.exit_code();
                    return ActionExecution::Sync(AIAgentActionResultType::ReadShellCommandOutput(
                        ReadShellCommandOutputResult::CommandFinished {
                            command,
                            block_id: block_id.clone(),
                            output,
                            exit_code,
                        },
                    ));
                }
                let command = block.command_with_secrets_unobfuscated(false);
                // 仅在 `ReadShellCommandOutput` 路径上根据命令内容下调等待时长:此处
                // 是 agent 对一个**仍在运行**的 block 的二次轮询,默认走 `OnCompletion`
                // 时会等满 `MAX_AGENT_DELAY_DURATION`(120s)。对 ssh / mosh / sftp /
                // telnet 等永不主动退出的交互会话来说,这种等待没有意义。
                // `RequestCommandOutput`(首次发起)使用 `MAX_WAIT_DURATION = 2s` 的默
                // 认超时,本就不会卡 120s,因此无需同样处理。
                let delay = effective_read_shell_command_delay(&command, delay.clone());
                drop(model);

                let block_selector = BlockSelector::Id(block_id.clone());
                ActionExecution::new_async(
                    self.action_result_future(block_selector.clone(), delay),
                    move |result, ctx| {
                        // Remove the senders from the maps.
                        if let Some(handle) = handle.upgrade(ctx) {
                            handle.update(ctx, |me, _| {
                                me.block_finished_senders.remove(&block_selector);
                                me.force_refresh_senders.remove(&block_selector);
                            });
                        }

                        action_result_for_read_shell_command_output(command.clone(), result)
                    },
                )
            }
            AIAgentActionType::TransferShellCommandControlToUser { reason } => {
                let active_block = model.block_list().active_block();
                if !active_block.is_active_and_long_running() {
                    return ActionExecution::Sync(
                        AIAgentActionResultType::TransferShellCommandControlToUser(
                            TransferShellCommandControlToUserResult::Error(
                                ShellCommandError::BlockNotFound,
                            ),
                        ),
                    );
                }

                let block_id = active_block.id().clone();
                drop(model);

                // Emit event to transfer control to user.
                ctx.emit(ShellCommandExecutorEvent::TransferControlToUser {
                    action_id: action_id.clone(),
                    reason: reason.clone(),
                });

                // Create a channel to wait for control handback.
                let (handback_tx, handback_rx) = oneshot::channel();
                self.control_handback_sender = Some(handback_tx);

                let block_selector = BlockSelector::Id(block_id.clone());

                // Set up a future to also wait for block completion.
                let (block_finished_tx, block_finished_rx) = oneshot::channel();
                self.block_finished_senders
                    .insert(block_selector.clone(), block_finished_tx);

                // Build the future that captures terminal model and block data.
                let transfer_future = {
                    let terminal_model = self.terminal_model.clone();
                    let block_id = block_id.clone();
                    async move {
                        pin!(handback_rx);
                        pin!(block_finished_rx);

                        // Wait for either control handback or block completion.
                        let transfer_result = select! {
                            val = handback_rx => match val {
                                Ok(_) => TransferControlResult::ControlHandedBack,
                                Err(_) => TransferControlResult::Cancelled,
                            },
                            val = block_finished_rx => match val {
                                Ok(_) => TransferControlResult::BlockFinished,
                                Err(_) => TransferControlResult::Cancelled,
                            },
                        };

                        // Convert to ActionResult
                        let model = terminal_model.lock();
                        match transfer_result {
                            TransferControlResult::ControlHandedBack
                            | TransferControlResult::BlockFinished => {
                                match model.block_list().block_with_id(&block_id) {
                                    Some(block) => {
                                        if block.finished() {
                                            ActionResult::CommandFinished {
                                                block_id: block.id().clone(),
                                                output: agent_shell_command_block_output(block),
                                                exit_code: block.exit_code(),
                                            }
                                        } else {
                                            let grid_contents = if model.is_alt_screen_active() {
                                                formatted_terminal_contents_for_input(
                                                    model.alt_screen().grid_handler(),
                                                    None,
                                                    CURSOR_MARKER,
                                                )
                                            } else {
                                                formatted_terminal_contents_for_input(
                                                    block.output_grid().grid_handler(),
                                                    Some(1000),
                                                    CURSOR_MARKER,
                                                )
                                            };
                                            ActionResult::LongRunningCommandSnapshot {
                                                block_id: block.id().clone(),
                                                grid_contents,
                                                cursor: CURSOR_MARKER,
                                                is_alt_screen_active: model.is_alt_screen_active(),
                                                is_preempted: false,
                                            }
                                        }
                                    }
                                    None => ActionResult::BlockNotFound,
                                }
                            }
                            TransferControlResult::Cancelled => ActionResult::Cancelled,
                        }
                    }
                };

                ActionExecution::new_async(transfer_future, move |result, ctx| {
                    // Clean up.
                    if let Some(handle) = handle.upgrade(ctx) {
                        handle.update(ctx, |me, _| {
                            me.block_finished_senders.remove(&block_selector);
                            me.control_handback_sender = None;
                        });
                    }

                    action_result_for_transfer_shell_command_control_to_user(result)
                })
            }
            _ => ActionExecution::InvalidAction,
        }
    }

    /// Called when user hands control back to agent after TransferShellCommandControlToUser.
    pub fn notify_control_handed_back(&mut self) {
        if let Some(sender) = self.control_handback_sender.take() {
            let _ = sender.send(());
        }
    }

    /// Produces a future which resolves when the action is complete and
    /// we have a result to send to the agent.
    fn action_result_future(
        &mut self,
        block_selector: BlockSelector,
        delay: ActionResultDelay,
    ) -> impl Spawnable<Output = ActionResult> {
        // Create a channel to notify us when we receive block metadata.
        let (block_metadata_received_tx, block_metadata_received_rx) = oneshot::channel();
        self.block_finished_senders
            .insert(block_selector.clone(), block_metadata_received_tx);

        // Create a channel so the `Check now` affordance can short-circuit the timeout
        // and deliver the agent a fresh snapshot immediately.
        let (force_refresh_tx, force_refresh_rx) = oneshot::channel();
        self.force_refresh_senders
            .insert(block_selector.clone(), force_refresh_tx);

        // Create a future that resolves when we should send a result to the agent.
        let terminal_model = self.terminal_model.clone();

        #[derive(Debug, Clone, Copy)]
        enum WakeReason {
            BlockFinished,
            Timeout,
            /// User clicked `Check now` in the warping indicator, short-circuiting  
            /// the agent-set poll timer. Treated as a preemption so the server does  
            /// not interpret the early snapshot as a completion.  
            ForceRefresh,
        }

        async move {
            pin!(block_metadata_received_rx);
            pin!(force_refresh_rx);

            let timeout_duration = match delay {
                ActionResultDelay::UntilCompletion => None,
                ActionResultDelay::Duration(duration) => {
                    // Enforce a maximum allowed delay that the agent may request, never waiting longer than MAX_AGENT_DELAY_DURATION.
                    // If the requested duration exceeds this cap, we'll still behave as if the agent may expect a running command,
                    // so there's no need to signal preemption (the agent already anticipates an incomplete command state).
                    Some(duration.min(Self::MAX_AGENT_DELAY_DURATION))
                }
                ActionResultDelay::OnCompletion { timeout } => {
                    Some(timeout.min(Self::MAX_AGENT_DELAY_DURATION))
                }
                ActionResultDelay::Default => Some(Self::MAX_WAIT_DURATION),
            };

            let wake_reason = if let Some(timeout_duration) = timeout_duration {
                let timeout = Timer::after(timeout_duration).fuse();
                pin!(timeout);
                select! {
                    val = block_metadata_received_rx => match val {
                        Ok(_) => WakeReason::BlockFinished,
                        Err(_) => return ActionResult::Cancelled,
                    },
                    val = force_refresh_rx => match val {
                        // User asked the agent to check now; fall through to the snapshot
                        // code path below. Treated as a preemption (snapshot arrives before
                        // the agent's own timer would have fired).
                        Ok(_) => WakeReason::ForceRefresh,
                        // Sender was dropped (e.g. because the executor is being torn down).
                        Err(_) => return ActionResult::Cancelled,
                    },
                    _ = timeout => WakeReason::Timeout,
                }
            } else {
                select! {
                    val = block_metadata_received_rx => match val {
                        Ok(_) => WakeReason::BlockFinished,
                        Err(_) => return ActionResult::Cancelled,
                    },
                    val = force_refresh_rx => match val {
                        Ok(_) => WakeReason::ForceRefresh,
                        Err(_) => return ActionResult::Cancelled,
                    },
                }
            };

            // Mark the snapshot as preempted if woken early, allowing the server to distinguish
            // true completion from a forced client poll (`ForceRefresh`) or a timeout during `on_completion`.
            let is_preempted = matches!(wake_reason, WakeReason::ForceRefresh)
                || matches!(
                    (wake_reason, delay),
                    (WakeReason::Timeout, ActionResultDelay::OnCompletion { .. })
                );

            // At this point, we've either received block metadata or we've timed out.
            // Check the current state of the block and produce a result accordingly.
            let model = terminal_model.lock();
            let result = match block_selector.get_block(&model) {
                Some(block) => {
                    if block.finished() {
                        ActionResult::CommandFinished {
                            block_id: block.id().clone(),
                            output: agent_shell_command_block_output(block),
                            exit_code: block.exit_code(),
                        }
                    } else {
                        let grid_contents = if model.is_alt_screen_active() {
                            formatted_terminal_contents_for_input(
                                model.alt_screen().grid_handler(),
                                None,
                                CURSOR_MARKER,
                            )
                        } else {
                            formatted_terminal_contents_for_input(
                                block.output_grid().grid_handler(),
                                // TODO(vorporeal): This is probably too large.
                                Some(1000),
                                CURSOR_MARKER,
                            )
                        };
                        ActionResult::LongRunningCommandSnapshot {
                            block_id: block.id().clone(),
                            grid_contents,
                            cursor: CURSOR_MARKER,
                            is_alt_screen_active: model.is_alt_screen_active(),
                            is_preempted,
                        }
                    }
                }
                None => ActionResult::BlockNotFound,
            };

            result
        }
    }

    pub(super) fn cancel_execution(&mut self, id: &AIAgentActionId, _ctx: &mut ModelContext<Self>) {
        let terminal_model = self.terminal_model.lock();
        let active_block = terminal_model.block_list().active_block();
        if !active_block.is_active_and_long_running() {
            return;
        }

        let selector = if active_block
            .requested_command_action_id()
            .is_some_and(|requested_command_id| requested_command_id == id)
        {
            BlockSelector::RequestedCommandId(id.clone())
        } else {
            BlockSelector::Id(active_block.id().clone())
        };
        self.block_finished_senders.remove(&selector);
        self.force_refresh_senders.remove(&selector);
    }

    /// Force any in-flight poll for the given long-running command block to resolve
    /// immediately with a fresh snapshot, bypassing the agent-set timeout.
    ///
    /// Called by the `Check now` affordance in the warping indicator. No-ops if there
    /// is no matching in-flight poll (e.g. because the block already finished or the
    /// agent has transferred control to the user).
    pub fn force_refresh_block(&mut self, block_id: &BlockId) {
        let terminal_model = self.terminal_model.lock();
        // Find a sender whose selector resolves to this block. In practice there is at
        // most one: a given block can have at most one in-flight `action_result_future`
        // at a time.
        let matching_selector = self
            .force_refresh_senders
            .keys()
            .find(|selector| {
                selector
                    .get_block(&terminal_model)
                    .is_some_and(|block| block.id() == block_id)
            })
            .cloned();
        drop(terminal_model);

        if let Some(selector) = matching_selector {
            if let Some(sender) = self.force_refresh_senders.remove(&selector) {
                let _ = sender.send(());
            }
        }
    }

    pub(super) fn preprocess_action(
        &mut self,
        _action: PreprocessActionInput,
        _ctx: &mut ModelContext<Self>,
    ) -> BoxFuture<'static, ()> {
        futures::future::ready(()).boxed()
    }
}

/// `action_result_future` 内部使用的等待策略。
///
/// 相比对外的 `Option<ShellCommandDelay>`,这里把 `OnCompletion` 的 timeout
/// 从隐式常量提升为显式字段,便于按命令场景动态调整(见
/// `effective_read_shell_command_delay`)。
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ActionResultDelay {
    UntilCompletion,
    Default,
    Duration(Duration),
    OnCompletion { timeout: Duration },
}

impl ActionResultDelay {
    fn from_shell_command_delay(delay: Option<ShellCommandDelay>) -> Self {
        match delay {
            Some(ShellCommandDelay::Duration(duration)) => Self::Duration(duration),
            Some(ShellCommandDelay::OnCompletion) => Self::OnCompletion {
                timeout: ShellCommandExecutor::MAX_AGENT_DELAY_DURATION,
            },
            None => Self::Default,
        }
    }
}

fn action_result_delay_for_requested_command(wait_until_completion: bool) -> ActionResultDelay {
    if wait_until_completion {
        ActionResultDelay::UntilCompletion
    } else {
        ActionResultDelay::Default
    }
}

/// 把 agent 请求的 `ShellCommandDelay` 映射成内部使用的 `ActionResultDelay`,
/// 并对**永不主动退出**的交互会话(ssh / mosh / sftp / telnet 等)做特殊处理:
///
/// 1. `Some(OnCompletion)` —— 把 timeout 从 `MAX_AGENT_DELAY_DURATION`(120s)
///    缩短到 `MAX_WAIT_DURATION`(2s),避免 agent 在永不结束的命令上死等。
/// 2. `None`(默认)—— 主动**升级**为 `OnCompletion { 2s }`,而不是保留
///    `Default`。注意这会同步改变 `action_result_future` 内 `is_preempted` 的
///    取值:`Default` + `Timeout` 不算抢占,而 `OnCompletion` + `Timeout` 会
///    标记为抢占,从而让 server 把这次快照理解为"先看一眼"而非"命令完成"。
///    对交互会话来说这是正确语义。
/// 3. `Some(Duration(d))` —— 保留 agent 的显式请求,不做改写。
///
/// 非交互命令一律走 `from_shell_command_delay` 的原始映射。
fn effective_read_shell_command_delay(
    command: &str,
    delay: Option<ShellCommandDelay>,
) -> ActionResultDelay {
    if command_starts_non_terminating_session(command)
        && matches!(delay, None | Some(ShellCommandDelay::OnCompletion))
    {
        return ActionResultDelay::OnCompletion {
            timeout: ShellCommandExecutor::MAX_WAIT_DURATION,
        };
    }

    ActionResultDelay::from_shell_command_delay(delay)
}

/// 判断 `command` 是否会启动一个**永不主动退出**的交互会话。命中规则:
/// - 被 Warp generator wrapper 包裹的命令,递归判断内部命令。
/// - 裸 `ssh ...`(走 `parse_interactive_ssh_command`,会正确排除 `-T` / `-W`
///   等非交互形式)。
/// - 带路径或带 `.exe` 的 ssh(改写为裸 `ssh` 后再判)。
/// - `mosh` / `sftp` / `telnet`(含 `.exe`)等没有 ssh 那样的非交互旗标,直接按
///   可执行名匹配即可。
///
/// 仅供 `effective_read_shell_command_delay` 使用的启发式检测,误判后果有限。
fn command_starts_non_terminating_session(command: &str) -> bool {
    let command = command.trim_start();
    in_band_generator_command(command)
        .as_deref()
        .is_some_and(command_starts_non_terminating_session)
        || parse_interactive_ssh_command(command).is_some()
        || normalized_ssh_command(command)
            .as_deref()
            .is_some_and(|command| parse_interactive_ssh_command(command).is_some())
        || first_executable_name(command).is_some_and(|name| {
            matches!(
                name.as_str(),
                "mosh" | "mosh.exe" | "sftp" | "sftp.exe" | "telnet" | "telnet.exe"
            )
        })
}

/// 解开 Warp 自身的 generator wrapper,把里面真正要跑的命令抽出来。
///
/// wrapper 协议形如:`<wrapper> <generator_id> '<inner_command>' [extra flags...]`
/// 其中:
/// - `<wrapper>` 是 `warp_run_generator_command`(POSIX shell)或
///   `Warp-Run-GeneratorCommand`(PowerShell,大小写不敏感)。
/// - `<generator_id>` 是数字 id,这里不解析,直接跳过。
/// - `<inner_command>` 是被单引号包裹的真实命令字符串,也就是我们要返回的内容。
///
/// 协议固定按位置取 `tokens[2]`;若后续 wrapper 新增可选参数破坏了位置假设,这
/// 里会静默失配(返回 None),最坏情况只是退回到旧的 120s 等待,不会引入错误行为。
fn in_band_generator_command(command: &str) -> Option<String> {
    let tokens = shell_words::split(command.trim_start()).ok()?;
    if tokens.len() >= 3
        && (tokens[0].eq_ignore_ascii_case("Warp-Run-GeneratorCommand")
            || tokens[0] == "warp_run_generator_command")
    {
        Some(tokens[2].clone())
    } else {
        None
    }
}

/// 当命令的可执行入口是带路径或带 `.exe` 后缀的 ssh 时,把它改写为裸 `ssh`,
/// 以便复用 `parse_interactive_ssh_command` 这套只接受 `^ssh\s+...` 的解析。
///
/// 例如 `"C:\Windows\System32\OpenSSH\ssh.exe" host -p 22` 会被改写为
/// `ssh host -p 22`。其余参数原样保留(`rest` 是 `first_executable_token` 切
/// 完第一个 token 后的剩余字符串)。
///
/// 命名上只做"前缀改写",不规范化路径中的反斜杠/引号,也不展开转义。
fn normalized_ssh_command(command: &str) -> Option<String> {
    let (token, rest) = first_executable_token(command)?;
    let name = command_basename(token);
    if name.eq_ignore_ascii_case("ssh") || name.eq_ignore_ascii_case("ssh.exe") {
        Some(format!("ssh{rest}"))
    } else {
        None
    }
}

fn first_executable_name(command: &str) -> Option<String> {
    let (token, _) = first_executable_token(command)?;
    Some(command_basename(token).to_ascii_lowercase())
}

/// 返回命令真正的"可执行入口" token,跳过常见的调用前缀:
/// - PowerShell 调用运算符 `&`(必须是独立 token,如 `& "C:\...\ssh.exe" host`)。
/// - POSIX `command` 内建(`command ssh host`),用于绕过 alias/function。
///
/// 只剥离一层前缀,够覆盖实际场景;不支持 `&&` 链、`call`、`exec` 等其他形态。
fn first_executable_token(command: &str) -> Option<(&str, &str)> {
    let (token, rest) = first_command_token(command)?;
    if token == "&" || token.eq_ignore_ascii_case("command") {
        first_command_token(rest)
    } else {
        Some((token, rest))
    }
}

/// 启发式 tokenization:取出命令的第一个 token,以及它后面剩余的原始字符串。
///
/// 故意**不**使用 `shell_words::split`,原因有二:
/// 1. `shell_words` 会因 PowerShell 调用运算符 `&` 等非 POSIX 字符直接 fail,
///    而我们需要识别这类形态。
/// 2. 我们只需要"第一个 token + rest"两段,不需要完整 token 流,手写更直接。
///
/// 引号处理只识别字符串**起始位置**的 `"` 或 `'`,且不处理转义。为了避免
/// `"foo"bar`、`"ssh"hello-world` 这类"右引号后还粘着字符"的输入被错切成
/// `foo` / `ssh` 进而触发**误检**(把普通命令认成交互会话),这里要求右引号
/// 后必须紧跟空白或字符串结尾;不满足就返回 `None`,让上层走"退回到旧的等待
/// 行为"这条安全分支,而不是冒着 false-positive 风险硬切。
fn first_command_token(command: &str) -> Option<(&str, &str)> {
    let command = command.trim_start();
    if command.is_empty() {
        return None;
    }

    let mut chars = command.char_indices();
    let (_, first) = chars.next()?;
    if first == '"' || first == '\'' {
        for (idx, ch) in chars {
            if ch == first {
                let token = &command[first.len_utf8()..idx];
                let rest = &command[idx + ch.len_utf8()..];
                // 右引号后必须是空白或字符串末尾;否则视为无法 tokenize,
                // 让调用方退回到不抢占的安全路径。
                if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
                    return None;
                }
                return Some((token, rest));
            }
        }

        // 没找到配对的右引号:同样视为无法 tokenize。
        return None;
    }

    let end = command.find(char::is_whitespace).unwrap_or(command.len());
    Some((&command[..end], &command[end..]))
}

fn command_basename(command_token: &str) -> &str {
    command_token
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(command_token)
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum BlockSelector {
    Id(BlockId),
    RequestedCommandId(AIAgentActionId),
}

impl BlockSelector {
    fn get_block<'a>(&self, model: &'a TerminalModel) -> Option<&'a Block> {
        match self {
            BlockSelector::Id(block_id) => model.block_list().block_with_id(block_id),
            BlockSelector::RequestedCommandId(requested_command_id) => model
                .block_list()
                .block_for_ai_action_id(requested_command_id),
        }
    }
}

/// Returns the result from executing a requested command.
fn action_result_for_requested_command(
    command: String,
    result: ActionResult,
) -> AIAgentActionResultType {
    match result {
        ActionResult::CommandFinished {
            block_id,
            output,
            exit_code,
        } => AIAgentActionResultType::RequestCommandOutput(RequestCommandOutputResult::Completed {
            command,
            block_id,
            output,
            exit_code,
        }),
        ActionResult::LongRunningCommandSnapshot {
            block_id,
            grid_contents,
            cursor,
            is_alt_screen_active,
            ..
        } => AIAgentActionResultType::RequestCommandOutput(
            RequestCommandOutputResult::LongRunningCommandSnapshot {
                command,
                block_id,
                grid_contents,
                cursor: cursor.to_owned(),
                is_alt_screen_active,
            },
        ),
        ActionResult::BlockNotFound | ActionResult::Cancelled => {
            AIAgentActionResultType::RequestCommandOutput(
                RequestCommandOutputResult::CancelledBeforeExecution,
            )
        }
    }
}

/// Returns the result from writing to a long-running shell command.
fn action_result_for_write_to_long_running_shell_command(
    result: ActionResult,
) -> AIAgentActionResultType {
    match result {
        ActionResult::CommandFinished {
            block_id,
            output,
            exit_code,
        } => AIAgentActionResultType::WriteToLongRunningShellCommand(
            WriteToLongRunningShellCommandResult::CommandFinished {
                block_id,
                output,
                exit_code,
            },
        ),
        ActionResult::LongRunningCommandSnapshot {
            block_id,
            grid_contents,
            cursor,
            is_alt_screen_active,
            is_preempted,
        } => AIAgentActionResultType::WriteToLongRunningShellCommand(
            WriteToLongRunningShellCommandResult::Snapshot {
                block_id,
                grid_contents,
                cursor: cursor.to_owned(),
                is_alt_screen_active,
                is_preempted,
            },
        ),
        ActionResult::Cancelled => AIAgentActionResultType::WriteToLongRunningShellCommand(
            WriteToLongRunningShellCommandResult::Cancelled,
        ),
        ActionResult::BlockNotFound => AIAgentActionResultType::WriteToLongRunningShellCommand(
            WriteToLongRunningShellCommandResult::Error(ShellCommandError::BlockNotFound),
        ),
    }
}

/// Returns the result from reading shell command output.
fn action_result_for_read_shell_command_output(
    command: String,
    result: ActionResult,
) -> AIAgentActionResultType {
    match result {
        ActionResult::CommandFinished {
            output,
            exit_code,
            block_id,
        } => AIAgentActionResultType::ReadShellCommandOutput(
            ReadShellCommandOutputResult::CommandFinished {
                command,
                block_id,
                output,
                exit_code,
            },
        ),
        ActionResult::LongRunningCommandSnapshot {
            block_id,
            grid_contents,
            cursor,
            is_alt_screen_active,
            is_preempted,
        } => AIAgentActionResultType::ReadShellCommandOutput(
            ReadShellCommandOutputResult::LongRunningCommandSnapshot {
                command,
                block_id,
                grid_contents,
                cursor: cursor.to_owned(),
                is_alt_screen_active,
                is_preempted,
            },
        ),
        ActionResult::Cancelled => {
            AIAgentActionResultType::ReadShellCommandOutput(ReadShellCommandOutputResult::Cancelled)
        }
        ActionResult::BlockNotFound => AIAgentActionResultType::ReadShellCommandOutput(
            ReadShellCommandOutputResult::Error(ShellCommandError::BlockNotFound),
        ),
    }
}

/// Returns the result from transferring shell command control to user.
fn action_result_for_transfer_shell_command_control_to_user(
    result: ActionResult,
) -> AIAgentActionResultType {
    match result {
        ActionResult::CommandFinished {
            block_id,
            output,
            exit_code,
        } => AIAgentActionResultType::TransferShellCommandControlToUser(
            TransferShellCommandControlToUserResult::CommandFinished {
                block_id,
                output,
                exit_code,
            },
        ),
        ActionResult::LongRunningCommandSnapshot {
            block_id,
            grid_contents,
            cursor,
            is_alt_screen_active,
            is_preempted,
        } => AIAgentActionResultType::TransferShellCommandControlToUser(
            TransferShellCommandControlToUserResult::Snapshot {
                block_id,
                grid_contents,
                cursor: cursor.to_owned(),
                is_alt_screen_active,
                is_preempted,
            },
        ),
        ActionResult::Cancelled => AIAgentActionResultType::TransferShellCommandControlToUser(
            TransferShellCommandControlToUserResult::Cancelled,
        ),
        ActionResult::BlockNotFound => AIAgentActionResultType::TransferShellCommandControlToUser(
            TransferShellCommandControlToUserResult::Error(ShellCommandError::BlockNotFound),
        ),
    }
}

#[derive(Debug, Clone)]
pub enum ShellCommandExecutorEvent {
    ExecuteCommand {
        action_id: AIAgentActionId,
        command: String,
    },
    WriteToPty {
        input: Bytes,
        mode: AIAgentPtyWriteMode,
    },
    CancelExecution,
    /// Emitted when the agent requests to transfer control of a long-running command to the user.
    TransferControlToUser {
        action_id: AIAgentActionId,
        reason: String,
    },
}

impl Entity for ShellCommandExecutor {
    type Event = ShellCommandExecutorEvent;
}

/// Result from waiting for control transfer.
#[derive(Debug, Clone)]
enum TransferControlResult {
    ControlHandedBack,
    BlockFinished,
    Cancelled,
}

/// The possible results of taking an action.
#[derive(Debug, Clone)]
enum ActionResult {
    CommandFinished {
        block_id: BlockId,
        output: String,
        exit_code: ExitCode,
    },
    LongRunningCommandSnapshot {
        block_id: BlockId,
        grid_contents: String,
        cursor: &'static str,
        is_alt_screen_active: bool,
        is_preempted: bool,
    },
    Cancelled,
    BlockNotFound,
}

#[cfg(test)]
#[path = "shell_command_tests.rs"]
mod tests;
