use super::{CommonCommand, TeacherCommand};
use crate::api;
use crate::models::Teacher;
use crate::state::BotState;
use anyhow::Result;
use std::sync::Arc;
use teloxide::prelude::*;

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
        CommonCommand::Help => api::teacher::help(bot, msg.chat.id).await,
        CommonCommand::Schedule => api::teacher::schedule(bot, msg.chat.id, teacher, state).await,
    }
}

/// Route teacher-specific commands to the appropriate API functions.
pub async fn handle_teacher_command(
    bot: &Bot,
    msg: &Message,
    cmd: &TeacherCommand,
    teacher: &Teacher,
    state: &Arc<BotState>,
) -> Result<()> {
    match cmd {
        TeacherCommand::Impersonate(username) => {
            api::teacher::impersonate(bot, msg.chat.id, username, state).await
        }
        TeacherCommand::Quit => api::teacher::quit(bot, msg.chat.id, teacher, state).await,
    }
}
