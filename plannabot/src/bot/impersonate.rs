use crate::api;
use crate::bot::{get_username, book_command::BookCommand};
use crate::models::{TelegramName, UserEffectiveRole};
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
    #[command(
        description = "book a lesson as the impersonated student (optionally provide: teacher_name date hour duration)"
    )]
    Book(String),
    #[command(description = "exit impersonation mode")]
    Quit,
}

impl telluride::command::CallbackBitcode for ImpersonateCommand {}

impl BookCommand for ImpersonateCommand {
    fn book(teacher_name: String) -> Self {
        ImpersonateCommand::Book(teacher_name)
    }
}

pub async fn is_impersonate(username: TelegramName, chat_id: ChatId, state: Arc<BotState>) -> bool {
    matches!(
        state.get_effective_role(username.as_str(), chat_id).await,
        Some(UserEffectiveRole::Impersonate(..))
    )
}

pub async fn impersonate_command_handler(
    bot: Bot,
    msg: Message,
    cmd: ImpersonateCommand,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, ImpersonateCommand>>,
) -> ResponseResult<()> {
    let _ = state.refresh_if_needed().await;

    let Some(username) = get_username(&msg) else {
        return Ok(());
    };
    let Some(UserEffectiveRole::Impersonate(teacher, _)) =
        state.get_effective_role(&username, msg.chat.id).await
    else {
        return Ok(());
    };

    state
        .try_send_errors_to_teacher(&bot, msg.chat.id, &teacher.telegram_name)
        .await;

    let result = match &cmd {
        ImpersonateCommand::Start => api::common::start(&bot, msg.chat.id, &username, &state).await,
        ImpersonateCommand::Help => api::impersonate::help(&bot, msg.chat.id).await,
        ImpersonateCommand::Schedule => api::impersonate::schedule(&bot, msg.chat.id, &state).await,
        ImpersonateCommand::Book(params) => {
            api::student::book(&bot, msg.chat.id, params, &state, msg.from.unwrap().id, callback_storage).await
        }
        ImpersonateCommand::Quit => {
            api::impersonate::quit(&bot, msg.chat.id, &teacher, &state).await
        }
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

pub async fn impersonate_callback_command_handler(
    bot: Bot,
    q: CallbackQuery,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, ImpersonateCommand>>,
) -> ResponseResult<()> {
    bot.answer_callback_query(q.id.clone()).await?;

    let data = match &q.data {
        Some(d) => d,
        None => return Ok(()),
    };

    let user_proxy = telluride::data_store::UserProxy::new(callback_storage.clone(), q.from.id);
    let cmd = match CallbackKey::unpack::<ImpersonateCommand, _>(data, &user_proxy).await {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to unpack callback: {}", e);
            return Ok(());
        }
    };

    let mut msg = match q.message.as_ref().and_then(|m| m.regular_message()) {
        Some(m) => m.clone(),
        None => return Ok(()),
    };
    msg.from = Some(q.from.clone());

    impersonate_command_handler(bot, msg, cmd, state, callback_storage).await
}
