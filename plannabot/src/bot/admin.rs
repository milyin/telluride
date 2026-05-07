use crate::api;
use crate::api::context::BotCtx;
use crate::api::traits::{ImpersonateCommand, ImpersonateParams};
use crate::bot::{callback_command_handler, get_username, report_error};
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
    #[command(description = "view the bot as a student or teacher")]
    Impersonate(String),
    #[command(description = "exit admin mode")]
    Quit,
}

impl telluride::command::CallbackBitcode for AdminCommand {}

impl ImpersonateCommand for AdminCommand {
    fn impersonate(params: ImpersonateParams) -> Self {
        AdminCommand::Impersonate(params.to_string())
    }
}

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
    callback_storage: Arc<InMemStore<CallbackKey, AdminCommand>>,
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
    let ctx = BotCtx { bot, chat_id: msg.chat.id, state, user_id, callback_storage, message_id: None };

    let result = match cmd {
        AdminCommand::Start => api::common::start(&ctx, &username).await,
        AdminCommand::Help => api::admin::help(&ctx).await,
        AdminCommand::Status => api::admin::status(&ctx, &teacher).await,
        AdminCommand::Refresh => api::admin::refresh(&ctx).await,
        AdminCommand::Impersonate(ref params) => api::impersonate::impersonate(&ctx, params).await,
        AdminCommand::Quit => api::admin::quit(&ctx).await,
    };

    if let Err(e) = result {
        return report_error(&ctx.bot, ctx.chat_id, e).await;
    }

    Ok(())
}

pub async fn admin_callback_command_handler(
    bot: Bot,
    q: CallbackQuery,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, AdminCommand>>,
) -> ResponseResult<()> {
    callback_command_handler(bot, q, callback_storage, state, |bot, msg, cmd, state, cb, _| {
        Box::pin(admin_command_handler(bot, msg, cmd, state, cb))
    }).await
}
