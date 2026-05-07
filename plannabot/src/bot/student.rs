use crate::api;
use crate::api::context::BotCtx;
use crate::api::traits::{BookCommand, BookParams, BookingActor};
use crate::bot::{callback_command_handler, get_username};
use crate::models::{TelegramName, UserEffectiveRole, UserRole};
use crate::state::BotState;
use std::sync::Arc;
use telluride::command::CallbackKey;
use telluride::data_store::InMemStore;
use teloxide::prelude::*;
use teloxide::types::{ChatId, MessageId};
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
    #[command(
        description = "book a lesson (optionally provide: teacher_name date hour duration)"
    )]
    Book(String),
}

impl telluride::command::CallbackBitcode for StudentCommand {}

impl BookCommand for StudentCommand {
    fn book(params: BookParams) -> Self {
        StudentCommand::Book(params.to_string())
    }
}

pub async fn is_student(username: TelegramName, chat_id: ChatId, state: Arc<BotState>) -> bool {
    matches!(
        state.get_effective_role(username.as_str(), chat_id).await,
        Some(UserEffectiveRole::Student(_))
    )
}

async fn student_handle(
    bot: Bot,
    msg: Message,
    cmd: StudentCommand,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, StudentCommand>>,
    message_id: Option<MessageId>,
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

    let user_id = msg.from.as_ref().unwrap().id;
    let ctx = BotCtx { bot, chat_id: msg.chat.id, state, user_id, callback_storage, message_id };

    let result = match &cmd {
        StudentCommand::Start => api::common::start(&ctx.bot, ctx.chat_id, &username, &ctx.state).await,
        StudentCommand::Help => api::student::help(&ctx.bot, ctx.chat_id).await,
        StudentCommand::Schedule => {
            api::student::schedule(&ctx.bot, ctx.chat_id, &student, &ctx.state).await
        }
        StudentCommand::Book(params) => {
            api::book::book(&ctx, params, &BookingActor::Student(student.telegram_name.clone())).await
        }
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

pub async fn student_command_handler(
    bot: Bot,
    msg: Message,
    cmd: StudentCommand,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, StudentCommand>>,
) -> ResponseResult<()> {
    student_handle(bot, msg, cmd, state, callback_storage, None).await
}

pub async fn student_callback_command_handler(
    bot: Bot,
    q: CallbackQuery,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, StudentCommand>>,
) -> ResponseResult<()> {
    callback_command_handler(bot, q, callback_storage, state, |bot, msg, cmd, state, cb, message_id| {
        Box::pin(student_handle(bot, msg, cmd, state, cb, message_id))
    }).await
}
