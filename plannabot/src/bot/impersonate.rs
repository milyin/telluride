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
#[command(rename_rule = "lowercase", description = "Impersonation commands:")]
pub enum ImpersonateCommand {
    #[command(description = "exit impersonation and restart the bot")]
    Start,
    #[command(description = "display help")]
    Help,
    #[command(description = "show the impersonated student's planned lessons")]
    Schedule,
    #[command(description = "book a lesson as the impersonated student")]
    Book,
    #[command(description = "exit impersonation mode")]
    Quit,
}

impl telluride::command::CallbackBitcode for ImpersonateCommand {}

async fn check_is_impersonate(username: &TelegramName, chat_id: ChatId, state: &BotState) -> bool {
    matches!(state.get_role(username.as_str()).await, Some(UserRole::Teacher(_)))
        && state.get_impersonation(chat_id).await.is_some()
}

/// Returns true if the message sender is a teacher currently in impersonation mode.
pub async fn is_impersonate(msg: Message, state: Arc<BotState>) -> bool {
    let Some(username) = get_telegram_name(&msg) else { return false; };
    check_is_impersonate(&username, msg.chat.id, &state).await
}

/// Returns true if the callback query sender is a teacher currently in impersonation mode.
pub async fn is_impersonate_callback(q: CallbackQuery, state: Arc<BotState>) -> bool {
    let Some(username) = get_callback_telegram_name(&q) else { return false; };
    let Some(chat_id) = q.message.as_ref().map(|m| m.chat().id) else { return false; };
    check_is_impersonate(&username, chat_id, &state).await
}

pub async fn impersonate_command_handler(
    bot: Bot,
    msg: Message,
    cmd: ImpersonateCommand,
    state: Arc<BotState>,
    _callback_storage: Arc<InMemStore<CallbackKey, ImpersonateCommand>>,
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
        ImpersonateCommand::Start => {
            let role = UserRole::Teacher(teacher.clone());
            api::common::start(&bot, msg.chat.id, &role, &state).await
        }
        ImpersonateCommand::Help => api::impersonate::help(&bot, msg.chat.id).await,
        ImpersonateCommand::Schedule => api::impersonate::schedule(&bot, msg.chat.id, &state).await,
        ImpersonateCommand::Book => api::student::book(&bot, msg.chat.id).await,
        ImpersonateCommand::Quit => api::impersonate::quit(&bot, msg.chat.id, &teacher, &state).await,
    };

    result.map_err(|e| {
        log::error!(
            "Error handling impersonate command {:?} for @{}: {}",
            cmd,
            username,
            e
        );
        teloxide::RequestError::Io(Arc::new(std::io::Error::other(e.to_string())))
    })?;

    Ok(())
}
