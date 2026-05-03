use crate::api;
use crate::bot::{get_username, Action};
use crate::models::UserRole;
use crate::state::BotState;
use std::sync::Arc;
use telluride::command::CallbackKey;
use telluride::data_store::InMemStore;
use teloxide::prelude::*;
use teloxide::types::UserId;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Teacher commands:")]
pub enum TeacherCommand {
    #[command(description = "start the bot")]
    Start,
    #[command(description = "display help")]
    Help,
    #[command(description = "show your planned lessons")]
    Schedule,
    #[command(
        description = "view the bot as a student (shows student list or impersonates if username provided)"
    )]
    Impersonate(String),
    #[command(description = "enter admin mode")]
    Admin,
    #[command(description = "forcedly refresh the data")]
    Refresh,
}

/// Returns true if the message sender is a teacher in normal mode
/// (not in impersonation mode and not in admin mode).
pub async fn is_teacher(msg: Message, state: Arc<BotState>) -> bool {
    let Some(username) = get_username(&msg) else {
        return false;
    };
    if !matches!(state.get_role(&username).await, Some(UserRole::Teacher(_))) {
        return false;
    }
    let chat_id = msg.chat.id;
    !state.is_in_admin_mode(chat_id).await && state.get_impersonation(chat_id).await.is_none()
}

pub async fn teacher_command_handler(
    bot: Bot,
    msg: Message,
    cmd: TeacherCommand,
    state: Arc<BotState>,
    callback_storage: Arc<InMemStore<CallbackKey, Action>>,
) -> ResponseResult<()> {
    let _ = state.refresh_if_needed().await;

    let Some(username) = get_username(&msg) else {
        return Ok(());
    };
    let Some(UserRole::Teacher(teacher)) = state.get_role(&username).await else {
        return Ok(());
    };

    state
        .try_send_errors_to_teacher(&bot, msg.chat.id, &teacher.telegram_name)
        .await;

    let user_id: UserId = msg.from.as_ref().unwrap().id;

    let result = match cmd {
        TeacherCommand::Start => {
            let role = UserRole::Teacher(teacher.clone());
            api::common::start(&bot, msg.chat.id, &role, &state).await
        }
        TeacherCommand::Help => api::teacher::help(&bot, msg.chat.id, &state).await,
        TeacherCommand::Schedule => api::teacher::schedule(&bot, msg.chat.id, &teacher, &state).await,
        TeacherCommand::Impersonate(ref student_param) => {
            api::impersonate::impersonate(
                &bot,
                msg.chat.id,
                if student_param.is_empty() {
                    None
                } else {
                    Some(student_param.as_str())
                },
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
        log::error!("Error handling teacher command {:?} for @{}: {}", cmd, username, e);
        teloxide::RequestError::Io(Arc::new(std::io::Error::other(e.to_string())))
    })?;

    Ok(())
}
