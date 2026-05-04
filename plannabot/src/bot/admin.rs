use crate::api;
use crate::bot::{callback_command_handler, get_username};
use crate::models::{TelegramName, UserEffectiveRole};
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

pub async fn is_admin(username: TelegramName, chat_id: ChatId, state: Arc<BotState>) -> bool {
    matches!(
        state.get_effective_role(username.as_str(), chat_id).await,
        Some(UserEffectiveRole::Admin(_))
    )
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
    let Some(UserEffectiveRole::Admin(teacher)) =
        state.get_effective_role(&username, msg.chat.id).await
    else {
        return Ok(());
    };

    state
        .try_send_errors_to_teacher(&bot, msg.chat.id, &teacher.telegram_name)
        .await;

    let result = match cmd {
        AdminCommand::Start => api::common::start(&bot, msg.chat.id, &username, &state).await,
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

pub async fn admin_callback_command_handler(
    bot: Bot,
    q: CallbackQuery,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, AdminCommand>>,
) -> ResponseResult<()> {
    callback_command_handler(bot, q, callback_storage, state, admin_command_handler).await
}
