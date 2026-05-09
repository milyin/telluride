use crate::api;
use crate::api::context::BotCtx;
use crate::api::traits::{BalanceActor, BalanceCommand, BalanceParams, ImpersonateCommand, ImpersonateParams, PaymentActor, PaymentCommand, PaymentParams, UserCommand, UserParams};
use crate::bot::{callback_command_handler, get_username, report_error};
use crate::models::{TelegramName, UserEffectiveRole};
use crate::state::BotState;
use std::sync::Arc;
use telluride::command::CallbackKey;
use telluride::data_store::InMemStore;
use teloxide::prelude::*;
use teloxide::types::{ChatId, MessageId};
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
    #[command(description = "view student balances")]
    Balance(String),
    #[command(description = "view the bot as a student or teacher")]
    Impersonate(String),
    #[command(description = "view student payments")]
    Payment(String),
    #[command(description = "manage students and teachers")]
    User(String),
    #[command(description = "exit admin mode")]
    Quit,
}

impl telluride::command::CallbackBitcode for AdminCommand {}

impl ImpersonateCommand for AdminCommand {
    fn impersonate(params: ImpersonateParams) -> Self {
        AdminCommand::Impersonate(params.to_string())
    }
}

impl BalanceCommand for AdminCommand {
    fn balance(params: BalanceParams) -> Self {
        AdminCommand::Balance(params.to_string())
    }
}

impl PaymentCommand for AdminCommand {
    fn payment(params: PaymentParams) -> Self {
        AdminCommand::Payment(params.to_string())
    }
}

impl UserCommand for AdminCommand {
    fn user(params: UserParams) -> Self {
        AdminCommand::User(params.to_string())
    }
}

pub async fn is_admin(username: TelegramName, chat_id: ChatId, state: Arc<BotState>) -> bool {
    matches!(
        state.get_effective_role(username.as_str(), chat_id).await,
        Some(UserEffectiveRole::Admin(_))
    )
}

async fn admin_handle(
    bot: Bot,
    msg: Message,
    cmd: AdminCommand,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, AdminCommand>>,
    message_id: Option<MessageId>,
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

    let user_id = msg.from.as_ref().unwrap().id;
    let ctx = BotCtx { bot, chat_id: msg.chat.id, state, user_id, callback_storage, message_id };

    let result = match cmd {
        AdminCommand::Start => api::common::start(&ctx, &username).await,
        AdminCommand::Help => api::admin::help(&ctx).await,
        AdminCommand::Status => api::admin::status(&ctx, &teacher).await,
        AdminCommand::Refresh => api::admin::refresh(&ctx).await,
        AdminCommand::Balance(ref params) => {
            api::balance::balance(&ctx, params, &BalanceActor::Admin).await
        }
        AdminCommand::Impersonate(ref params) => api::impersonate::impersonate(&ctx, params).await,
        AdminCommand::Payment(ref params) => {
            api::payment::payment(&ctx, params, &PaymentActor::Admin).await
        }
        AdminCommand::User(ref params) => api::user::user(&ctx, params).await,
        AdminCommand::Quit => api::admin::quit(&ctx).await,
    };

    if let Err(e) = result {
        return report_error(&ctx.bot, ctx.chat_id, e).await;
    }

    Ok(())
}

pub async fn admin_command_handler(
    bot: Bot,
    msg: Message,
    cmd: AdminCommand,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, AdminCommand>>,
) -> ResponseResult<()> {
    admin_handle(bot, msg, cmd, state, callback_storage, None).await
}

pub async fn admin_callback_command_handler(
    bot: Bot,
    q: CallbackQuery,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, AdminCommand>>,
) -> ResponseResult<()> {
    callback_command_handler(bot, q, callback_storage, state, |bot, msg, cmd, state, cb, message_id| {
        Box::pin(admin_handle(bot, msg, cmd, state, cb, message_id))
    }).await
}
