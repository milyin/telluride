use super::CommonCommand;
use crate::api;
use crate::models::Student;
use crate::state::BotState;
use anyhow::Result;
use std::sync::Arc;
use teloxide::prelude::*;

/// Route common commands for a student to the appropriate API functions.
pub async fn handle_command(
    bot: &Bot,
    msg: &Message,
    cmd: &CommonCommand,
    student: &Student,
    state: &Arc<BotState>,
    is_impersonating: bool,
) -> Result<()> {
    match cmd {
        CommonCommand::Start => api::student::start(bot, msg.chat.id, student).await,
        CommonCommand::Help => api::student::help(bot, msg.chat.id, is_impersonating).await,
        CommonCommand::Schedule => api::student::schedule(bot, msg.chat.id, student, state).await,
    }
}
