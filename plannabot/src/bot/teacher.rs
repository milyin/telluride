use super::{Action, CommonCommand, TeacherCommand};
use crate::api;
use crate::models::Teacher;
use crate::state::BotState;
use anyhow::Result;
use std::sync::Arc;
use telluride::command::CallbackKey;
use telluride::data_store::InMemStore;
use teloxide::prelude::*;
use teloxide::types::UserId;

/// Route common commands for a teacher to the appropriate API functions.
pub async fn handle_command(
    bot: &Bot,
    msg: &Message,
    cmd: &CommonCommand,
    teacher: &Teacher,
    state: &Arc<BotState>,
) -> Result<()> {
    match cmd {
        CommonCommand::Start => api::teacher::start(bot, msg.chat.id, teacher, state).await,
        CommonCommand::Help => {
            if state.is_in_admin_mode(msg.chat.id).await {
                api::admin::help(bot, msg.chat.id).await
            } else {
                api::teacher::help(bot, msg.chat.id, teacher, state).await
            }
        }
        CommonCommand::Schedule => api::teacher::schedule(bot, msg.chat.id, teacher, state).await,
    }
}

/// Route teacher-specific commands to the appropriate API functions.
///
/// `user_id` is the Telegram user ID of the teacher, used to namespace the
/// per-user callback storage so that each teacher's button actions are
/// isolated from other teachers' pending selections.
pub async fn handle_teacher_command(
    bot: &Bot,
    msg: &Message,
    cmd: &TeacherCommand,
    teacher: &Teacher,
    state: &Arc<BotState>,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Action>>,
) -> Result<()> {
    match cmd {
        TeacherCommand::Impersonate(student_param) => {
            api::teacher::impersonate(
                bot,
                msg.chat.id,
                if student_param.is_empty() {
                    None
                } else {
                    Some(student_param.as_str())
                },
                state,
                user_id,
                callback_storage,
            )
            .await
        }
        TeacherCommand::Quit => api::teacher::quit(bot, msg.chat.id, teacher, state).await,
        TeacherCommand::Admin => api::teacher::admin(bot, msg.chat.id, teacher, state).await,
        TeacherCommand::Refresh => api::teacher::refresh(bot, msg.chat.id, state).await,
    }
}
