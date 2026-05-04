pub mod admin;
pub mod impersonate;
pub mod student;
pub mod teacher;

use crate::models::{TelegramName, UserRole};
use crate::state::BotState;
use std::future::Future;
use std::sync::Arc;
use telluride::command::CallbackKey;
use telluride::data_store::{InMemStore, UserProxy};
use teloxide::prelude::*;
use teloxide::types::{ChatId, MessageId};

/// Extracts the Telegram username from a message sender.
/// Returns the username without '@', converted to lowercase.
pub fn get_username(msg: &Message) -> Option<String> {
    msg.from
        .as_ref()
        .and_then(|user| user.username.as_ref())
        .map(|username| username.trim_start_matches('@').to_lowercase())
}

/// Extracts and validates the sender's username from a message as a [`TelegramName`].
pub fn get_telegram_name(msg: &Message) -> Option<TelegramName> {
    msg.from
        .as_ref()
        .and_then(|u| u.username.as_deref())
        .and_then(|s| TelegramName::try_from(s).ok())
}

/// Extracts and validates the sender's username from a callback query as a [`TelegramName`].
pub fn get_callback_telegram_name(q: &CallbackQuery) -> Option<TelegramName> {
    q.from
        .username
        .as_deref()
        .and_then(|s| TelegramName::try_from(s).ok())
}

pub async fn filter_message_by<F, Fut>(msg: Message, state: Arc<BotState>, is_role: F) -> bool
where
    F: FnOnce(TelegramName, ChatId, Arc<BotState>) -> Fut,
    Fut: Future<Output = bool>,
{
    let Some(username) = get_telegram_name(&msg) else {
        return false;
    };
    is_role(username, msg.chat.id, state).await
}

pub async fn filter_callback_by<F, Fut>(q: CallbackQuery, state: Arc<BotState>, is_role: F) -> bool
where
    F: FnOnce(TelegramName, ChatId, Arc<BotState>) -> Fut,
    Fut: Future<Output = bool>,
{
    let Some(username) = get_callback_telegram_name(&q) else {
        return false;
    };
    let Some(chat_id) = q.message.as_ref().map(|m| m.chat().id) else {
        return false;
    };
    is_role(username, chat_id, state).await
}

/// Generic teloxide endpoint handler for inline keyboard callback queries.
///
/// Unpacks a command of type `Cmd` from the callback data, substitutes the
/// real user into the attached message (the bot sent it, so `msg.from` would
/// otherwise be the bot), and forwards to the supplied `handler` — the same
/// function used for the corresponding text-command branch.
pub async fn callback_command_handler<Cmd, F, Fut>(
    bot: Bot,
    q: CallbackQuery,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    state: Arc<BotState>,
    handler: F,
) -> ResponseResult<()>
where
    Cmd: telluride::command::CallbackBitcode + 'static,
    F: Fn(Bot, Message, Cmd, Arc<BotState>, Arc<InMemStore<CallbackKey, Cmd>>, Option<MessageId>) -> Fut,
    Fut: std::future::Future<Output = ResponseResult<()>> + Send,
{
    bot.answer_callback_query(q.id.clone()).await?;

    let data = match &q.data {
        Some(d) => d,
        None => return Ok(()),
    };

    let user_proxy = UserProxy::new(callback_storage.clone(), q.from.id);
    let cmd = match CallbackKey::unpack::<Cmd, _>(data, &user_proxy).await {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to unpack callback: {}", e);
            return Ok(());
        }
    };

    // The keyboard was sent by the bot, so q.message.from == bot.
    // Replace `from` with the real user so command handlers see the correct sender.
    let mut msg = match q.message.as_ref().and_then(|m| m.regular_message()) {
        Some(m) => m.clone(),
        None => return Ok(()),
    };
    let message_id = Some(msg.id);
    msg.from = Some(q.from.clone());

    handler(bot, msg, cmd, state, callback_storage, message_id).await
}

/// Teloxide endpoint handler for plain (non-command) text messages.
///
/// Refreshes the cache and nudges authorised users toward the command interface.
/// Unauthorized users receive an explicit rejection message.
pub async fn message_handler(bot: Bot, msg: Message, state: Arc<BotState>) -> ResponseResult<()> {
    let _ = state.refresh_if_needed().await;

    let Some(username) = get_username(&msg) else {
        bot.send_message(
            msg.chat.id,
            "Please set a Telegram username to use this bot.",
        )
        .await?;
        return Ok(());
    };

    let Some(role) = state.get_role(&username).await else {
        bot.send_message(
            msg.chat.id,
            "You are not authorized to use this bot. Please contact your teacher.",
        )
        .await?;
        return Ok(());
    };

    if let UserRole::Teacher(ref teacher) = role {
        state
            .try_send_errors_to_teacher(&bot, msg.chat.id, &teacher.telegram_name)
            .await;
    }

    bot.send_message(msg.chat.id, "Use /help to see available commands.")
        .await?;

    Ok(())
}
