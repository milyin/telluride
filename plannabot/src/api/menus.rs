use std::sync::Arc;

use anyhow::Result;
use telluride::command::{CallbackBitcode, CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::{InMemStore, UserProxy};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId, UserId};

use crate::models::TelegramName;
use crate::state::BotState;

pub async fn show_teacher_selection<Cmd, F>(
    bot: &Bot,
    chat_id: ChatId,
    message_id: Option<MessageId>,
    message: &str,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    state: &Arc<BotState>,
    make_cmd: F,
) -> Result<()>
where
    Cmd: CallbackBitcode + 'static,
    F: Fn(TelegramName) -> Cmd,
{
    let teacher_names = state.get_teacher_names().await;

    if teacher_names.is_empty() {
        bot.send_message(chat_id, "No teachers are registered in the spreadsheet yet.")
            .await?;
        return Ok(());
    }

    let user_proxy = UserProxy::new(callback_storage, user_id);

    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    for name in teacher_names {
        let label = format!("@{}", name);
        let cmd = make_cmd(name.parse()?);
        let key = CallbackKey::pack(cmd, &user_proxy).await;
        let button = InlineKeyboardButton::callback_key(label, &key);
        buttons.push(vec![button]);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    match message_id {
        Some(id) => {
            bot.edit_message_text(chat_id, id, message)
                .reply_markup(keyboard)
                .await?;
        }
        None => {
            bot.send_message(chat_id, message)
                .reply_markup(keyboard)
                .await?;
        }
    }

    Ok(())
}
