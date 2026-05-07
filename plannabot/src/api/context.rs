use std::sync::Arc;

use telluride::command::CallbackKey;
use telluride::data_store::InMemStore;
use teloxide::prelude::*;
use teloxide::types::{MessageId, UserId};

use crate::state::BotState;

pub struct BotCtx<Cmd: Send + Sync + Clone> {
    pub bot: Bot,
    pub chat_id: ChatId,
    pub state: Arc<BotState>,
    pub user_id: UserId,
    pub callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    pub message_id: Option<MessageId>,
}
