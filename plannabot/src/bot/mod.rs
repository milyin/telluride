pub mod student;
pub mod teacher;

use crate::models::UserRole;
use crate::state::BotState;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

/// Commands available to all authorised users.
#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Supported commands:")]
pub enum CommonCommand {
    #[command(description = "start the bot")]
    Start,
    #[command(description = "display help")]
    Help,
    #[command(description = "show your scheduled lessons")]
    Schedule,
}

/// Commands available to teachers only.
#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Teacher commands:")]
pub enum TeacherCommand {
    #[command(description = "view the bot as a student")]
    Impersonate(String),
    #[command(description = "exit impersonation mode")]
    Quit,
}

/// Extracts the Telegram username from a message sender.
/// Returns the username without '@', converted to lowercase.
pub fn get_username(msg: &Message) -> Option<String> {
    msg.from
        .as_ref()
        .and_then(|user| user.username.as_ref())
        .map(|username| username.trim_start_matches('@').to_lowercase())
}

/// Teloxide endpoint handler for common bot commands (Start, Help, Schedule).
///
/// Before resolving the user's role, calls [`BotState::refresh_if_needed`] so
/// that any spreadsheet edits made since the last check are picked up
/// automatically (subject to the 15-second throttle).
///
/// When a teacher is in impersonation mode, every common command is forwarded
/// to the student handler for the impersonated student.
pub async fn common_command_handler(
    bot: Bot,
    msg: Message,
    cmd: CommonCommand,
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
            student::handle_command(&bot, &msg, &cmd, student, &state, false).await
        }
        UserRole::Teacher(ref teacher) => {
            match state.get_impersonation(msg.chat.id).await {
                // ---- Impersonation mode: act as the impersonated student ----
                Some(ref student_name) => match state.get_student(student_name).await {
                    Some(ref student) => {
                        student::handle_command(&bot, &msg, &cmd, student, &state, true).await
                    }
                    None => {
                        // Student was removed from the spreadsheet.
                        state.clear_impersonation(msg.chat.id).await;
                        bot.send_message(
                            msg.chat.id,
                            format!(
                                "Student @{} was not found in the spreadsheet. \
                                 Impersonation mode has been deactivated.",
                                student_name
                            ),
                        )
                        .await?;
                        return Ok(());
                    }
                },
                // ---- Normal teacher mode ------------------------------------
                None => teacher::handle_command(&bot, &msg, &cmd, teacher, &state).await,
            }
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

/// Teloxide endpoint handler for teacher-only commands (Impersonate, Quit).
///
/// Rejects the command silently (no response) if the sender is not a teacher.
pub async fn teacher_command_handler(
    bot: Bot,
    msg: Message,
    cmd: TeacherCommand,
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

    // --- Gate: require a teacher role ---------------------------------------
    let role = state.get_role(&username).await;
    let teacher = match role {
        Some(UserRole::Teacher(ref t)) => t.clone(),
        _ => {
            // Students (or unknown users) cannot use teacher commands.
            return Ok(());
        }
    };

    // --- Dispatch -----------------------------------------------------------
    let result = teacher::handle_teacher_command(&bot, &msg, &cmd, &teacher, &state).await;

    result.map_err(|e| {
        log::error!(
            "Error handling teacher command {:?} for @{}: {}",
            cmd,
            username,
            e
        );
        teloxide::RequestError::Io(Arc::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string(),
        )))
    })?;

    Ok(())
}

/// Teloxide endpoint handler for plain (non-command) text messages.
///
/// Refreshes the cache (same throttled check as [`common_command_handler`])
/// and nudges authorised users toward the command interface.
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
