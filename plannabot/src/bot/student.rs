use crate::api;
use crate::api::context::BotCtx;
use crate::api::traits::{
    BalanceActor, BalanceCommand, BalanceParams, BookCommand, BookParams, BookingActor,
    NotificationCommand, NotificationParams, ScheduleCommand, ScheduleParams,
};
use crate::bot::{callback_command_handler, get_username, report_error};
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
    Schedule(String),
    #[command(description = "view your balance")]
    Balance(String),
    #[command(description = "book a lesson (optionally provide: teacher_name date hour duration)")]
    Book(String),
    #[command(description = "manage lesson notifications")]
    Notification(String),
}

impl telluride::command::CallbackBitcode for StudentCommand {}

impl BookCommand for StudentCommand {
    fn book(params: BookParams) -> Self {
        StudentCommand::Book(params.to_string())
    }
}

impl ScheduleCommand for StudentCommand {
    fn schedule(params: ScheduleParams) -> Self {
        StudentCommand::Schedule(params.to_string())
    }
}

impl BalanceCommand for StudentCommand {
    fn balance(params: BalanceParams) -> Self {
        StudentCommand::Balance(params.to_string())
    }
}

impl NotificationCommand for StudentCommand {
    fn notification(params: NotificationParams) -> Self {
        StudentCommand::Notification(params.to_string())
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

    if student.chat_id != msg.chat.id.0 {
        let _ = state
            .sheets
            .update_student_chat_id(&student.telegram_name, msg.chat.id.0)
            .await;
    }

    let user_id = msg.from.as_ref().unwrap().id;
    let ctx = BotCtx {
        bot,
        chat_id: msg.chat.id,
        state,
        user_id,
        callback_storage,
        message_id,
    };

    let result = match &cmd {
        StudentCommand::Start => api::common::start(&ctx, &username).await,
        StudentCommand::Help => api::student::help(&ctx).await,
        StudentCommand::Schedule(params) => {
            api::schedule::schedule(
                &ctx,
                params,
                &BookingActor::Student(student.telegram_name.clone()),
                student.timezone,
            )
            .await
        }
        StudentCommand::Balance(params) => {
            api::balance::balance(
                &ctx,
                params,
                &BalanceActor::Student(student.telegram_name.clone()),
            )
            .await
        }
        StudentCommand::Book(params) => {
            api::book::book(
                &ctx,
                params,
                &BookingActor::Student(student.telegram_name.clone()),
            )
            .await
        }
        StudentCommand::Notification(params) => {
            api::notification::notification(&ctx, params, &student).await
        }
    };

    if let Err(e) = result {
        return report_error(&ctx.bot, ctx.chat_id, e).await;
    }

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
    callback_command_handler(
        bot,
        q,
        callback_storage,
        state,
        |bot, msg, cmd, state, cb, message_id| {
            Box::pin(student_handle(bot, msg, cmd, state, cb, message_id))
        },
    )
    .await
}
