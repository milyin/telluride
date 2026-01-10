use std::sync::Arc;

use teloxide::{Bot, payloads::{EditMessageReplyMarkupSetters, SendMessage}, prelude::{Requester, ResponseResult}, requests::JsonRequest, types::{Chat, Message, MessageId}};

use crate::{api::{command::command_button::{ButtonData, CallbackDataStorageTrait, pack_callback_data}, markdown::string::MarkdownString}, markdown::MarkdownStringMessage};


#[derive(Clone)]
pub struct CommandReplyTarget {
    pub bot: Bot,
    pub chat: Chat,
    pub msg_id: Option<MessageId>,
    pub batch: bool,
    pub callback_data_storage: Arc<dyn CallbackDataStorageTrait>,
}

impl CommandReplyTarget {
    /// Send a new or edit a current markdown message without a menu
    pub async fn markdown_message(&self, text: MarkdownString) -> ResponseResult<Message> {
        if let Some(message_id) = self.msg_id {
            self.bot.edit_markdown_message_text(self.chat.id, message_id, text)
                .await
        } else {
            self.bot.send_markdown_message(self.chat.id, text)
                .await
        }
    }

    /// Send a new or edit a current markdown message with an inline keyboard menu
    /// The menu is automatically packed using pack_callback_data to handle long callback data
    pub async fn markdown_message_with_menu<R, B>(
        &self,
        text: MarkdownString,
        menu: impl IntoIterator<Item = R>,
    ) -> ResponseResult<Message>
    where
        R: IntoIterator<Item = B>,
        B: Into<ButtonData>,
    {
        let msg = self
            .markdown_message(text)
            .await?;

        Self::attach_menu_to_message(
            &self.bot,
            &self.callback_data_storage,
            self.chat.id,
            msg.id,
            menu,
        )
        .await?;

        Ok(msg)
    }

    /// Send a new markdown message without a menu
    pub fn send_markdown_message(&self, text: MarkdownString) -> JsonRequest<SendMessage> {
        self.bot.send_markdown_message(self.chat.id, text)
    }

    /// Send a markdown message with an inline keyboard menu using a request builder
    /// The menu is automatically packed using pack_callback_data to handle long callback data
    pub async fn send_markdown_message_with_menu<R, B>(
        &self,
        text: MarkdownString,
        menu: impl IntoIterator<Item = R>,
    ) -> ResponseResult<Message>
    where
        R: IntoIterator<Item = B>,
        B: Into<ButtonData>,
    {
        let msg = self.bot.send_markdown_message(self.chat.id, text).await?;

        Self::attach_menu_to_message(
            &self.bot,
            &self.callback_data_storage,
            self.chat.id,
            msg.id,
            menu,
        )
        .await?;

        Ok(msg)
    }

    /// Internal helper function to attach a menu to an existing message
    /// Extracted to avoid code duplication between different send methods
    async fn attach_menu_to_message<R, B>(
        bot: &Bot,
        callback_data_storage: &Arc<dyn CallbackDataStorageTrait>,
        chat_id: teloxide::types::ChatId,
        message_id: MessageId,
        menu: impl IntoIterator<Item = R>,
    ) -> ResponseResult<()>
    where
        R: IntoIterator<Item = B>,
        B: Into<ButtonData>,
    {
        // Pack callback data and attach keyboard to the message
        let keyboard = pack_callback_data(callback_data_storage, message_id.0, menu).await;
        bot.edit_message_reply_markup(chat_id, message_id)
            .reply_markup(keyboard)
            .await?;
        Ok(())
    }
}
