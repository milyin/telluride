use crate::api;
use crate::bot::{get_callback_telegram_name, get_telegram_name, get_username};
use crate::models::{TelegramName, UserRole};
use crate::state::BotState;
use std::sync::Arc;
use telluride::command::CallbackKey;
use telluride::data_store::InMemStore;
use teloxide::prelude::*;
use teloxide::types::ChatId;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug, Hash, bitcode::Encode, bitcode::Decode)]
#[command(rename_rule = "lowercase", description = "Admin commands:")]
pub enum AdminCommand {
    #[command(description = "exit admin mode and restart the bot")]
    Start,
    #[command(description = "display help")]
    Help,
    #[command(description = "show global stat information")]
    Status,
    #[command(description = "forcedly refresh the data")]
    Refresh,
    #[command(description = "exit admin mode")]
    Quit,
}

impl telluride::command::CallbackBitcode for AdminCommand {}

async fn check_is_admin(username: &TelegramName, chat_id: ChatId, state: &BotState) -> bool {
    matches!(
        state.get_role(username.as_str()).await,
        Some(UserRole::Teacher(ref t)) if t.admin
    ) && state.is_in_admin_mode(chat_id).await
}

/// Returns true if the message sender is a teacher with admin=true currently in admin mode.
pub async fn filter_message_by_admin(msg: Message, state: Arc<BotState>) -> bool {
    let Some(username) = get_telegram_name(&msg) else {
        return false;
    };
    check_is_admin(&username, msg.chat.id, &state).await
}

/// Returns true if the callback query sender is a teacher with admin=true in admin mode.
pub async fn filter_callback_by_admin(q: CallbackQuery, state: Arc<BotState>) -> bool {
    let Some(username) = get_callback_telegram_name(&q) else {
        return false;
    };
    let Some(chat_id) = q.message.as_ref().map(|m| m.chat().id) else {
        return false;
    };
    check_is_admin(&username, chat_id, &state).await
}

pub async fn admin_command_handler(
    bot: Bot,
    msg: Message,
    cmd: AdminCommand,
    state: Arc<BotState>,
    _callback_storage: Arc<InMemStore<CallbackKey, AdminCommand>>,
) -> ResponseResult<()> {
    let _ = state.refresh_if_needed().await;

    let Some(username) = get_username(&msg) else {
        return Ok(());
    };
    let Some(UserRole::Teacher(teacher)) = state.get_role(&username).await else {
        return Ok(());
    };

    state
        .try_send_errors_to_teacher(&bot, msg.chat.id, &teacher.telegram_name)
        .await;

    let result = match cmd {
        AdminCommand::Start => {
            let role = UserRole::Teacher(teacher.clone());
            api::common::start(&bot, msg.chat.id, &role, &state).await
        }
        AdminCommand::Help => api::admin::help(&bot, msg.chat.id).await,
        AdminCommand::Status => api::admin::status(&bot, msg.chat.id, &teacher, &state).await,
        AdminCommand::Refresh => api::admin::refresh(&bot, msg.chat.id, &state).await,
        AdminCommand::Quit => api::admin::quit(&bot, msg.chat.id, &state).await,
    };

    result.map_err(|e| {
        log::error!(
            "Error handling admin command {:?} for @{}: {}",
            cmd,
            username,
            e
        );
        teloxide::RequestError::Io(Arc::new(std::io::Error::other(e.to_string())))
    })?;

    Ok(())
}
