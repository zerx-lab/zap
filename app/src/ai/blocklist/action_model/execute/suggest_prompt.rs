use futures::{channel::oneshot, future::BoxFuture, FutureExt};
use warpui::{Entity, ModelContext};

use crate::{
    ai::{
        agent::{
            conversation::AIConversationId, AIAgentAction, AIAgentActionId, AIAgentActionType,
            SuggestPromptRequest, SuggestPromptResult,
        },
        blocklist::action_model::execute::{
            ActionExecution, AnyActionExecution, ExecuteActionInput, PreprocessActionInput,
        },
    },
    AIAgentActionResultType,
};

pub struct PromptSuggestionExecutor {
    suggest_prompt_result_tx: Option<oneshot::Sender<SuggestPromptResult>>,
}

impl Default for PromptSuggestionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptSuggestionExecutor {
    pub fn new() -> Self {
        Self {
            suggest_prompt_result_tx: None,
        }
    }

    pub(super) fn should_autoexecute(
        &self,
        _input: ExecuteActionInput,
        _ctx: &mut ModelContext<Self>,
    ) -> bool {
        true
    }

    pub(super) fn execute(
        &mut self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> impl Into<AnyActionExecution> {
        let AIAgentAction {
            action: AIAgentActionType::SuggestPrompt(request),
            ..
        } = input.action
        else {
            return ActionExecution::InvalidAction;
        };

        // OpenWarp:无条件 emit(原版 gate 在 PromptSuggestionsViaMAA cargo feature 下,
        // OSS 默认不开 → chip 永远不显示 → oneshot 永挂)。BYOP 场景模型主动调
        // suggest_prompt 时,需要 view 层订阅本 event 把 chip 显示给用户,接受后
        // 调 complete_suggest_prompt_action 关 channel。详见
        // app/src/terminal/view.rs::handle_prompt_suggestion_executor_event。
        if let SuggestPromptRequest::PromptSuggestion { prompt, label } = request {
            ctx.emit(PromptSuggestionExecutorEvent::NewPromptSuggestion {
                prompt: prompt.clone(),
                label: label.clone(),
                conversation_id: input.conversation_id,
                action_id: input.action.id.clone(),
            });
        }

        let (result_tx, result_rx) = oneshot::channel();
        self.suggest_prompt_result_tx = Some(result_tx);

        ActionExecution::new_async(result_rx, |result, _ctx| match result {
            Ok(SuggestPromptResult::Accepted { query }) => {
                AIAgentActionResultType::SuggestPrompt(SuggestPromptResult::Accepted { query })
            }
            Ok(SuggestPromptResult::Cancelled) | Err(_) => {
                AIAgentActionResultType::SuggestPrompt(SuggestPromptResult::Cancelled)
            }
        })
    }

    pub(super) fn preprocess_action(
        &mut self,
        _action: PreprocessActionInput,
        _ctx: &mut ModelContext<Self>,
    ) -> BoxFuture<'static, ()> {
        futures::future::ready(()).boxed()
    }

    pub fn complete_suggest_prompt_action(&mut self, result: SuggestPromptResult) {
        if let Some(sender) = self.suggest_prompt_result_tx.take() {
            let _ = sender.send(result);
        }
    }
}

impl Entity for PromptSuggestionExecutor {
    type Event = PromptSuggestionExecutorEvent;
}

#[derive(Debug)]
pub enum PromptSuggestionExecutorEvent {
    NewPromptSuggestion {
        prompt: String,
        label: Option<String>,
        conversation_id: AIConversationId,
        action_id: AIAgentActionId,
    },
}
