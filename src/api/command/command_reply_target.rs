use std::sync::Arc;

use teloxide::{
    Bot,
    payloads::{EditMessageReplyMarkupSetters, SendMessage},
    prelude::{Requester, ResponseResult},
    requests::JsonRequest,
    types::{Chat, Message, MessageId},
};

use crate::{
    api::{data_store::data_store_trait::UserDataStoreTrait, markdown::string::MarkdownString},
    markdown::MarkdownStringMessage,
};

use serde::{Deserialize, Serialize};
use teloxide::types::InlineKeyboardMarkup;

#[derive(Clone)]
pub struct CommandReplyTarget<C = String>
where
    C: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    pub bot: Bot,
    pub chat: Chat,
    pub msg_id: Option<MessageId>,
    pub batch: bool,
    pub callback_data_store: Arc<dyn UserDataStoreTrait<C>>,
}

impl<C> CommandReplyTarget<C>
where
    C: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    /// Send a new or edit a current markdown message without a menu
    pub async fn markdown_message(&self, text: MarkdownString) -> ResponseResult<Message> {
        if let Some(message_id) = self.msg_id {
            self.bot
                .edit_markdown_message_text(self.chat.id, message_id, text)
                .await
        } else {
            self.bot.send_markdown_message(self.chat.id, text).await
        }
    }

    /// Send a new or edit a current markdown message with an inline keyboard menu
    pub async fn markdown_message_with_menu(
        &self,
        text: MarkdownString,
        menu: InlineKeyboardMarkup,
    ) -> ResponseResult<Message> {
        let msg = self.markdown_message(text).await?;

        self.bot
            .edit_message_reply_markup(self.chat.id, msg.id)
            .reply_markup(menu)
            .await?;

        Ok(msg)
    }

    /// Send a new markdown message without a menu
    pub fn send_markdown_message(&self, text: MarkdownString) -> JsonRequest<SendMessage> {
        self.bot.send_markdown_message(self.chat.id, text)
    }

    /// Send a markdown message with an inline keyboard menu using a request builder
    pub async fn send_markdown_message_with_menu(
        &self,
        text: MarkdownString,
        menu: InlineKeyboardMarkup,
    ) -> ResponseResult<Message> {
        let msg = self.bot.send_markdown_message(self.chat.id, text).await?;

        self.bot
            .edit_message_reply_markup(self.chat.id, msg.id)
            .reply_markup(menu)
            .await?;

        Ok(msg)
    }
}
