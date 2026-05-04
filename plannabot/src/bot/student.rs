use crate::api;
use crate::bot::{callback_command_handler, get_username};
use crate::models::{TelegramName, UserEffectiveRole, UserRole};
use crate::state::BotState;
use std::sync::Arc;
use telluride::command::CallbackKey;
use telluride::data_store::InMemStore;
use teloxide::prelude::*;
use teloxide::types::ChatId;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug, Hash, bitcode::Encode, bitcode::Decode)]
#[command(rename_rule = "lowercase", description = "Student commands:")]
pub enum StudentCommand {
    #[command(description = "start the bot")]
    Start,
    #[command(description = "display help")]
    Help,
    #[command(description = "show your planned lessons")]
    Schedule,
    #[command(description = "book a lesson")]
    Book,
}

impl telluride::command::CallbackBitcode for StudentCommand {}

pub async fn is_student(username: TelegramName, chat_id: ChatId, state: Arc<BotState>) -> bool {
    matches!(
        state.get_effective_role(username.as_str(), chat_id).await,
        Some(UserEffectiveRole::Student(_))
    )
}

pub async fn student_command_handler(
    bot: Bot,
    msg: Message,
    cmd: StudentCommand,
    state: Arc<BotState>,
    _callback_storage: Arc<InMemStore<CallbackKey, StudentCommand>>,
) -> ResponseResult<()> {
    let _ = state.refresh_if_needed().await;

    let Some(username) = get_username(&msg) else {
        bot.send_message(
            msg.chat.id,
            "Please set a Telegram username to use this bot.",
        )
        .await?;
        return Ok(());
    };

    let Some(UserRole::Student(student)) = state.get_role(&username).await else {
        bot.send_message(
            msg.chat.id,
            "You are not authorized to use this bot. Please contact your teacher.",
        )
        .await?;
        return Ok(());
    };

    let result = match cmd {
        StudentCommand::Start => api::common::start(&bot, msg.chat.id, &username, &state).await,
        StudentCommand::Help => api::student::help(&bot, msg.chat.id).await,
        StudentCommand::Schedule => {
            api::student::schedule(&bot, msg.chat.id, &student, &state).await
        }
        StudentCommand::Book => api::student::book(&bot, msg.chat.id).await,
    };

    result.map_err(|e| {
        log::error!(
            "Error handling student command {:?} for @{}: {}",
            cmd,
            username,
            e
        );
        teloxide::RequestError::Io(Arc::new(std::io::Error::other(e.to_string())))
    })?;

    Ok(())
}

pub async fn student_callback_command_handler(
    bot: Bot,
    q: CallbackQuery,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, StudentCommand>>,
) -> ResponseResult<()> {
    callback_command_handler(bot, q, callback_storage, state, student_command_handler).await
}
