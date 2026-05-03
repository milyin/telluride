use super::AdminCommand;
use crate::api;
use crate::models::Teacher;
use crate::state::BotState;
use anyhow::Result;
use std::sync::Arc;
use teloxide::prelude::*;

/// Route admin commands to the appropriate API functions.
///
/// The caller must ensure the user is in admin mode before calling this.
pub async fn handle_admin_command(
    bot: &Bot,
    msg: &Message,
    cmd: &AdminCommand,
    teacher: &Teacher,
    state: &Arc<BotState>,
) -> Result<()> {
    match cmd {
        AdminCommand::Status => api::admin::status(bot, msg.chat.id, teacher, state).await,
        AdminCommand::Refresh => api::admin::refresh(bot, msg.chat.id, state).await,
    }
}
