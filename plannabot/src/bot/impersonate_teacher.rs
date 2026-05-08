use crate::api;
use crate::api::context::BotCtx;
use crate::api::traits::{BalanceActor, BalanceCommand, BalanceParams, BookCommand, BookParams, BookingActor, PaymentActor, PaymentCommand, PaymentParams, ScheduleCommand, ScheduleParams};
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
#[command(rename_rule = "lowercase", description = "Teacher impersonation commands:")]
pub enum ImpersonateTeacherCommand {
    #[command(description = "exit impersonation and restart the bot")]
    Start,
    #[command(description = "display help")]
    Help,
    #[command(description = "show the impersonated teacher's planned lessons")]
    Schedule(String),
    #[command(
        description = "book a lesson as the impersonated teacher (optionally provide: student_name date hour duration)"
    )]
    Book(String),
    #[command(description = "view student balances as the impersonated teacher")]
    Balance(String),
    #[command(description = "view student balances as the impersonated teacher")]
    Payment(String),
    #[command(description = "exit impersonation mode")]
    Quit,
}

impl telluride::command::CallbackBitcode for ImpersonateTeacherCommand {}

impl BookCommand for ImpersonateTeacherCommand {
    fn book(params: BookParams) -> Self {
        ImpersonateTeacherCommand::Book(params.to_string())
    }
}

impl PaymentCommand for ImpersonateTeacherCommand {
    fn payment(params: PaymentParams) -> Self {
        ImpersonateTeacherCommand::Payment(params.to_string())
    }
}

impl ScheduleCommand for ImpersonateTeacherCommand {
    fn schedule(params: ScheduleParams) -> Self {
        ImpersonateTeacherCommand::Schedule(params.to_string())
    }
}

impl BalanceCommand for ImpersonateTeacherCommand {
    fn balance(params: BalanceParams) -> Self {
        ImpersonateTeacherCommand::Balance(params.to_string())
    }
}

pub async fn is_impersonate_teacher(
    username: TelegramName,
    chat_id: ChatId,
    state: Arc<BotState>,
) -> bool {
    matches!(
        state.get_effective_role(username.as_str(), chat_id).await,
        Some(UserEffectiveRole::ImpersonateTeacher(..))
    )
}

async fn impersonate_teacher_handle(
    bot: Bot,
    msg: Message,
    cmd: ImpersonateTeacherCommand,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, ImpersonateTeacherCommand>>,
    message_id: Option<MessageId>,
) -> ResponseResult<()> {
    let _ = state.refresh_if_needed().await;

    let Some(username) = get_username(&msg) else {
        return Ok(());
    };
    let Some(UserEffectiveRole::ImpersonateTeacher(acting_teacher, impersonated_name)) =
        state.get_effective_role(&username, msg.chat.id).await
    else {
        return Ok(());
    };

    state
        .try_send_errors_to_teacher(&bot, msg.chat.id, &acting_teacher.telegram_name)
        .await;

    let user_id = msg.from.as_ref().unwrap().id;
    let ctx = BotCtx { bot, chat_id: msg.chat.id, state, user_id, callback_storage, message_id };

    let Some(impersonated_teacher) = ctx.state.get_teacher(&impersonated_name).await else {
        ctx.state.clear_teacher_impersonation(ctx.chat_id).await;
        let text = markdown_format!(
            "Teacher {} was not found in the spreadsheet\\. Impersonation mode has been deactivated\\.",
            impersonated_name.to_string()
        );
        ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
        return Ok(());
    };

    let result = match &cmd {
        ImpersonateTeacherCommand::Start => api::common::start(&ctx, &username).await,
        ImpersonateTeacherCommand::Help => api::impersonate_teacher::help(&ctx).await,
        ImpersonateTeacherCommand::Schedule(params) => {
            api::schedule::schedule(&ctx, params, &BookingActor::Teacher(impersonated_name.clone()), impersonated_teacher.timezone).await
        }
        ImpersonateTeacherCommand::Balance(params) => {
            api::balance::balance(&ctx, params, &BalanceActor::Teacher(impersonated_name.clone())).await
        }
        ImpersonateTeacherCommand::Book(params) => {
            api::book::book(
                &ctx,
                params,
                &BookingActor::Teacher(impersonated_name.clone()),
            )
            .await
        }
        ImpersonateTeacherCommand::Payment(params) => {
            api::payment::payment(&ctx, params, &PaymentActor::Teacher(impersonated_name.clone())).await
        }
        ImpersonateTeacherCommand::Quit => {
            api::impersonate_teacher::quit(&ctx, &acting_teacher).await
        }
    };

    if let Err(e) = result {
        return report_error(&ctx.bot, ctx.chat_id, e).await;
    }

    Ok(())
}

pub async fn impersonate_teacher_command_handler(
    bot: Bot,
    msg: Message,
    cmd: ImpersonateTeacherCommand,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, ImpersonateTeacherCommand>>,
) -> ResponseResult<()> {
    impersonate_teacher_handle(bot, msg, cmd, state, callback_storage, None).await
}

pub async fn impersonate_teacher_callback_command_handler(
    bot: Bot,
    q: CallbackQuery,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, ImpersonateTeacherCommand>>,
) -> ResponseResult<()> {
    callback_command_handler(
        bot,
        q,
        callback_storage,
        state,
        |bot, msg, cmd, state, cb, message_id| {
            Box::pin(impersonate_teacher_handle(bot, msg, cmd, state, cb, message_id))
        },
    )
    .await
}
