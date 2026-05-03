//! A singleton model for storing conversations by ID to enable restoration across terminal views.

use std::collections::HashMap;
use warpui::{Entity, SingletonEntity};

use crate::{
    ai::{
        agent::conversation::{AIConversation, AIConversationId},
        blocklist::history_model::convert_persisted_conversation_to_ai_conversation_with_metadata,
    },
    persistence::model::AgentConversation,
};

/// Singleton model that holds restored agent conversations on app startup.
///
/// Loading restored conversations into this model is a means of propagating restored data from
/// sqlite (read at startup) to arbitrary consuming locations in the view/model hierarchy without
/// piping it all the way from the root view to the terminal view(s) that require it.
#[derive(Default)]
pub struct RestoredAgentConversations {
    /// All conversations stored by their ID, available for restoration
    conversations: HashMap<AIConversationId, AIConversation>,
}

impl RestoredAgentConversations {
    /// 转换持久化会话; 把转换失败的 conversation_id 收集起来,调用方负责把它们从 sqlite 中清理掉,
    /// 否则下次启动会重复尝试转换并打 warn,白白拖慢启动。
    pub fn new(conversations: Vec<AgentConversation>) -> (Self, Vec<String>) {
        let mut conversations_by_id = HashMap::new();
        let mut failed_to_restore = Vec::new();
        for conversation in conversations.into_iter() {
            let conversation_id = conversation.conversation.conversation_id.clone();
            let Some(conversation) =
                convert_persisted_conversation_to_ai_conversation_with_metadata(conversation)
            else {
                log::warn!(
                    "Failed to convert persisted conversation {conversation_id} to AIConversation; will purge from sqlite"
                );
                failed_to_restore.push(conversation_id);
                continue;
            };
            conversations_by_id.insert(conversation.id(), conversation);
        }

        (
            Self {
                conversations: conversations_by_id,
            },
            failed_to_restore,
        )
    }

    /// Gets a reference to a restored conversation without removing it.
    pub fn get_conversation(&self, id: &AIConversationId) -> Option<&AIConversation> {
        self.conversations.get(id)
    }

    /// Removes the restored conversation and returns it, if any.
    pub fn take_conversation(&mut self, id: &AIConversationId) -> Option<AIConversation> {
        self.conversations.remove(id)
    }
}

impl Entity for RestoredAgentConversations {
    type Event = ();
}

impl SingletonEntity for RestoredAgentConversations {}
