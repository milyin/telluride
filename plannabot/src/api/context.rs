use std::sync::Arc;

use anyhow::Result;
use telluride::command::CallbackKey;
use telluride::data_store::InMemStore;
use telluride::markdown::{MarkdownString, MarkdownStringMessage};
use teloxide::payloads::{EditMessageTextSetters, SendMessageSetters};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, MessageId, UserId};

use crate::state::BotState;

pub struct BotCtx<Cmd: Send + Sync + Clone> {
    pub bot: Bot,
    pub chat_id: ChatId,
    pub state: Arc<BotState>,
    pub user_id: UserId,
    pub callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    pub message_id: Option<MessageId>,
}

impl<Cmd: Send + Sync + Clone> BotCtx<Cmd> {
    pub async fn update_markdown_message(
        &self,
        text: MarkdownString,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<()> {
        match self.message_id {
            Some(id) => {
                let req = self.bot.edit_markdown_message_text(self.chat_id, id, text);
                let result = if let Some(kb) = keyboard {
                    req.reply_markup(kb).await.map(|_| ())
                } else {
                    req.await.map(|_| ())
                };
                match result {
                    Ok(_) => {}
                    Err(teloxide::RequestError::Api(teloxide::ApiError::MessageNotModified)) => {}
                    Err(e) => return Err(anyhow::anyhow!(e)),
                }
            }
            None => {
                let req = self.bot.send_markdown_message(self.chat_id, text);
                if let Some(kb) = keyboard {
                    req.reply_markup(kb).await?;
                } else {
                    req.await?;
                }
            }
        }
        Ok(())
    }
}
