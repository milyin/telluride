use crate::api;
use crate::bot::get_username;
use crate::models::UserRole;
use crate::state::BotState;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Student commands:")]
pub enum StudentCommand {
    #[command(description = "start the bot")]
    Start,
    #[command(description = "display help")]
    Help,
    #[command(description = "show your planned lessons")]
    Schedule,
}

/// Returns true if the message sender has a Student role.
pub async fn is_student(msg: Message, state: Arc<BotState>) -> bool {
    let Some(username) = get_username(&msg) else {
        return false;
    };
    matches!(state.get_role(&username).await, Some(UserRole::Student(_)))
}

pub async fn student_command_handler(
    bot: Bot,
    msg: Message,
    cmd: StudentCommand,
    state: Arc<BotState>,
) -> ResponseResult<()> {
    let _ = state.refresh_if_needed().await;

    let Some(username) = get_username(&msg) else {
        bot.send_message(
            msg.chat.id,
            "Please set a Telegram username to use this bot.",
        )
        .await?;
        return Ok(());
    };

    let Some(UserRole::Student(student)) = state.get_role(&username).await else {
        bot.send_message(
            msg.chat.id,
            "You are not authorized to use this bot. Please contact your teacher.",
        )
        .await?;
        return Ok(());
    };

    let result = match cmd {
        StudentCommand::Start => {
            let role = UserRole::Student(student.clone());
            api::common::start(&bot, msg.chat.id, &role, &state).await
        }
        StudentCommand::Help => api::student::help(&bot, msg.chat.id).await,
        StudentCommand::Schedule => api::student::schedule(&bot, msg.chat.id, &student, &state).await,
    };

    result.map_err(|e| {
        log::error!("Error handling student command {:?} for @{}: {}", cmd, username, e);
        teloxide::RequestError::Io(Arc::new(std::io::Error::other(e.to_string())))
    })?;

    Ok(())
}
