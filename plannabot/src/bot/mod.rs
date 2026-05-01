pub mod student;
pub mod teacher;

use crate::models::UserRole;
use crate::state::BotState;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Supported commands:")]
pub enum Command {
    #[command(description = "start the bot")]
    Start,
    #[command(description = "display help")]
    Help,
    #[command(description = "show your scheduled lessons")]
    Schedule,
}

/// Extracts the Telegram username from a message sender.
/// Returns the username without '@', converted to lowercase.
pub fn get_username(msg: &Message) -> Option<String> {
    msg.from
        .as_ref()
        .and_then(|user| user.username.as_ref())
        .map(|username| username.trim_start_matches('@').to_lowercase())
}

/// Formats a lesson duration in minutes to a human-readable string.
///
/// - `90` → `"1h 30m"`
/// - `120` → `"2h"`
/// - `45` → `"45m"`
pub fn format_duration(minutes: i64) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    match (hours, mins) {
        (0, m) => format!("{}m", m),
        (h, 0) => format!("{}h", h),
        (h, m) => format!("{}h {}m", h, m),
    }
}

/// Teloxide endpoint handler for bot commands.
///
/// Before resolving the user's role, calls [`BotState::refresh_if_needed`] so
/// that any spreadsheet edits made since the last check are picked up
/// automatically (subject to the 15-second throttle).
pub async fn command_handler(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: Arc<BotState>,
) -> ResponseResult<()> {
    // --- Refresh cache if the spreadsheet was modified ----------------------
    state.refresh_if_needed().await;

    // --- Gate: require a Telegram username ----------------------------------
    let Some(username) = get_username(&msg) else {
        bot.send_message(
            msg.chat.id,
            "Please set a Telegram username to use this bot.",
        )
        .await?;
        return Ok(());
    };

    // --- Gate: require authorisation ----------------------------------------
    let Some(role) = state.get_role(&username).await else {
        bot.send_message(
            msg.chat.id,
            "You are not authorized to use this bot. Please contact your teacher.",
        )
        .await?;
        return Ok(());
    };

    // --- Dispatch to role-specific handler ----------------------------------
    let result = match role {
        UserRole::Student(ref student) => {
            student::handle_command(&bot, &msg, &cmd, student, &state).await
        }
        UserRole::Teacher(ref teacher) => {
            teacher::handle_command(&bot, &msg, &cmd, teacher, &state).await
        }
    };

    result.map_err(|e| {
        log::error!("Error handling command {:?} for @{}: {}", cmd, username, e);
        teloxide::RequestError::Io(Arc::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string(),
        )))
    })?;

    Ok(())
}

/// Teloxide endpoint handler for plain (non-command) text messages.
///
/// Refreshes the cache (same throttled check as [`command_handler`]) and
/// nudges authorised users toward the command interface.
pub async fn message_handler(bot: Bot, msg: Message, state: Arc<BotState>) -> ResponseResult<()> {
    // --- Refresh cache if the spreadsheet was modified ----------------------
    state.refresh_if_needed().await;

    // --- Gate: require a Telegram username ----------------------------------
    let Some(username) = get_username(&msg) else {
        bot.send_message(
            msg.chat.id,
            "Please set a Telegram username to use this bot.",
        )
        .await?;
        return Ok(());
    };

    // --- Gate: require authorisation ----------------------------------------
    if state.get_role(&username).await.is_none() {
        bot.send_message(
            msg.chat.id,
            "You are not authorized to use this bot. Please contact your teacher.",
        )
        .await?;
        return Ok(());
    }

    bot.send_message(msg.chat.id, "Use /help to see available commands.")
        .await?;

    Ok(())
}
