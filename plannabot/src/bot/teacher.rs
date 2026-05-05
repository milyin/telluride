use crate::api;
use crate::bot::{callback_command_handler, get_username};
use crate::models::{TelegramName, UserEffectiveRole};
use crate::state::BotState;
use std::sync::Arc;
use telluride::command::CallbackKey;
use telluride::data_store::InMemStore;
use teloxide::prelude::*;
use teloxide::types::{ChatId, UserId};
use teloxide::utils::command::{BotCommands, ParseError};

fn parse_impersonate_arg(s: String) -> Result<(Option<TelegramName>,), ParseError> {
    if s.trim().is_empty() {
        Ok((None,))
    } else {
        let name = s.trim().parse::<TelegramName>()
            .map_err(|e| ParseError::IncorrectFormat(e.into()))?;
        Ok((Some(name),))
    }
}

#[derive(BotCommands, Clone, Debug, Hash, bitcode::Encode, bitcode::Decode)]
#[command(rename_rule = "lowercase", description = "Teacher commands:")]
pub enum TeacherCommand {
    #[command(description = "start the bot")]
    Start,
    #[command(description = "display help")]
    Help,
    #[command(description = "show your planned lessons")]
    Schedule,
    #[command(
        description = "view the bot as a student (shows student list or impersonates if username provided)",
        parse_with = parse_impersonate_arg
    )]
    Impersonate(Option<TelegramName>),
    #[command(description = "enter admin mode")]
    Admin,
    #[command(description = "forcedly refresh the data")]
    Refresh,
}

impl telluride::command::CallbackBitcode for TeacherCommand {}

pub async fn is_teacher(username: TelegramName, chat_id: ChatId, state: Arc<BotState>) -> bool {
    matches!(
        state.get_effective_role(username.as_str(), chat_id).await,
        Some(UserEffectiveRole::Teacher(_))
    )
}

pub async fn teacher_command_handler(
    bot: Bot,
    msg: Message,
    cmd: TeacherCommand,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, TeacherCommand>>,
) -> ResponseResult<()> {
    let _ = state.refresh_if_needed().await;

    let Some(username) = get_username(&msg) else {
        return Ok(());
    };
    let Some(UserEffectiveRole::Teacher(teacher)) =
        state.get_effective_role(&username, msg.chat.id).await
    else {
        return Ok(());
    };

    state
        .try_send_errors_to_teacher(&bot, msg.chat.id, &teacher.telegram_name)
        .await;

    let user_id: UserId = msg.from.as_ref().unwrap().id;

    let result = match cmd {
        TeacherCommand::Start => api::common::start(&bot, msg.chat.id, &username, &state).await,
        TeacherCommand::Help => api::teacher::help(&bot, msg.chat.id, &state).await,
        TeacherCommand::Schedule => {
            api::teacher::schedule(&bot, msg.chat.id, &teacher, &state).await
        }
        TeacherCommand::Impersonate(ref student_param) => {
            api::impersonate::impersonate(
                &bot,
                msg.chat.id,
                student_param.clone(),
                &state,
                user_id,
                callback_storage,
            )
            .await
        }
        TeacherCommand::Admin => api::teacher::admin(&bot, msg.chat.id, &teacher, &state).await,
        TeacherCommand::Refresh => api::admin::refresh(&bot, msg.chat.id, &state).await,
    };

    result.map_err(|e| {
        log::error!(
            "Error handling teacher command {:?} for @{}: {}",
            cmd,
            username,
            e
        );
        teloxide::RequestError::Io(Arc::new(std::io::Error::other(e.to_string())))
    })?;

    Ok(())
}

pub async fn teacher_callback_command_handler(
    bot: Bot,
    q: CallbackQuery,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, TeacherCommand>>,
) -> ResponseResult<()> {
    callback_command_handler(bot, q, callback_storage, state, |bot, msg, cmd, state, cb, _| {
        Box::pin(teacher_command_handler(bot, msg, cmd, state, cb))
    }).await
}
