use crate::api;
use crate::api::context::BotCtx;
use crate::api::traits::{
    BalanceActor, BalanceCommand, BalanceParams, BookCommand, BookParams, BookingActor,
    NotificationCommand, NotificationParams, ScheduleCommand, ScheduleParams,
};
use crate::bot::{callback_command_handler, get_username, report_error};
use crate::models::{TelegramName, UserEffectiveRole};
use crate::state::BotState;
use std::sync::Arc;
use telluride::command::CallbackKey;
use telluride::data_store::InMemStore;
use telluride::markdown::MarkdownStringMessage;
use telluride::markdown_format;
use teloxide::prelude::*;
use teloxide::types::{ChatId, MessageId};
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug, Hash, bitcode::Encode, bitcode::Decode)]
#[command(
    rename_rule = "lowercase",
    description = "Student impersonation commands:"
)]
pub enum ImpersonateStudentCommand {
    #[command(description = "exit impersonation and restart the bot")]
    Start,
    #[command(description = "display help")]
    Help,
    #[command(description = "show the impersonated student's planned lessons")]
    Schedule(String),
    #[command(description = "view the impersonated student's balance")]
    Balance(String),
    #[command(
        description = "book a lesson as the impersonated student (optionally provide: teacher_name date hour duration)"
    )]
    Book(String),
    #[command(description = "manage lesson notifications for the impersonated student")]
    Notification(String),
    #[command(description = "exit impersonation mode")]
    Quit,
}

impl telluride::command::CallbackBitcode for ImpersonateStudentCommand {}

impl BookCommand for ImpersonateStudentCommand {
    fn book(params: BookParams) -> Self {
        ImpersonateStudentCommand::Book(params.to_string())
    }
}

impl ScheduleCommand for ImpersonateStudentCommand {
    fn schedule(params: ScheduleParams) -> Self {
        ImpersonateStudentCommand::Schedule(params.to_string())
    }
}

impl BalanceCommand for ImpersonateStudentCommand {
    fn balance(params: BalanceParams) -> Self {
        ImpersonateStudentCommand::Balance(params.to_string())
    }
}

impl NotificationCommand for ImpersonateStudentCommand {
    fn notification(params: NotificationParams) -> Self {
        ImpersonateStudentCommand::Notification(params.to_string())
    }
}

pub async fn is_impersonate_student(
    username: TelegramName,
    chat_id: ChatId,
    state: Arc<BotState>,
) -> bool {
    matches!(
        state.get_effective_role(username.as_str(), chat_id).await,
        Some(UserEffectiveRole::ImpersonateStudent(..))
    )
}

async fn impersonate_student_handle(
    bot: Bot,
    msg: Message,
    cmd: ImpersonateStudentCommand,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, ImpersonateStudentCommand>>,
    message_id: Option<MessageId>,
) -> ResponseResult<()> {
    let _ = state.refresh_if_needed().await;

    let Some(username) = get_username(&msg) else {
        return Ok(());
    };
    let Some(UserEffectiveRole::ImpersonateStudent(acting_teacher, impersonated_name)) =
        state.get_effective_role(&username, msg.chat.id).await
    else {
        return Ok(());
    };

    state
        .try_send_errors_to_teacher(&bot, msg.chat.id, &acting_teacher.telegram_name)
        .await;

    let user_id = msg.from.as_ref().unwrap().id;
    let ctx = BotCtx {
        bot,
        chat_id: msg.chat.id,
        state,
        user_id,
        callback_storage,
        message_id,
    };

    let Some(impersonated_student) = ctx.state.get_student(&impersonated_name).await else {
        ctx.state.clear_student_impersonation(ctx.chat_id).await;
        let text = markdown_format!(
            "Student {} was not found in the spreadsheet\\. Impersonation mode has been deactivated\\.",
            impersonated_name.to_string()
        );
        ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
        return Ok(());
    };

    let result = match &cmd {
        ImpersonateStudentCommand::Start => api::common::start(&ctx, &username).await,
        ImpersonateStudentCommand::Help => api::impersonate_student::help(&ctx).await,
        ImpersonateStudentCommand::Schedule(params) => {
            api::schedule::schedule(
                &ctx,
                params,
                &BookingActor::Student(impersonated_name.clone()),
                impersonated_student.timezone,
            )
            .await
        }
        ImpersonateStudentCommand::Balance(params) => {
            api::balance::balance(
                &ctx,
                params,
                &BalanceActor::Student(impersonated_name.clone()),
            )
            .await
        }
        ImpersonateStudentCommand::Book(params) => {
            api::book::book(
                &ctx,
                params,
                &BookingActor::Student(impersonated_name.clone()),
            )
            .await
        }
        ImpersonateStudentCommand::Notification(params) => {
            api::notification::notification(&ctx, params, &impersonated_student).await
        }
        ImpersonateStudentCommand::Quit => {
            api::impersonate_student::quit(&ctx, &acting_teacher).await
        }
    };

    if let Err(e) = result {
        return report_error(&ctx.bot, ctx.chat_id, e).await;
    }

    Ok(())
}

pub async fn impersonate_student_command_handler(
    bot: Bot,
    msg: Message,
    cmd: ImpersonateStudentCommand,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, ImpersonateStudentCommand>>,
) -> ResponseResult<()> {
    impersonate_student_handle(bot, msg, cmd, state, callback_storage, None).await
}

pub async fn impersonate_student_callback_command_handler(
    bot: Bot,
    q: CallbackQuery,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, ImpersonateStudentCommand>>,
) -> ResponseResult<()> {
    callback_command_handler(
        bot,
        q,
        callback_storage,
        state,
        |bot, msg, cmd, state, cb, message_id| {
            Box::pin(impersonate_student_handle(
                bot, msg, cmd, state, cb, message_id,
            ))
        },
    )
    .await
}
