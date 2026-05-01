use super::{CommonCommand, TeacherCommand};
use crate::models::{Teacher, UserRole};
use crate::state::BotState;
use anyhow::Result;
use chrono_tz::Tz;
use std::sync::Arc;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
use teloxide::prelude::*;

pub async fn handle_command(
    bot: &Bot,
    msg: &Message,
    cmd: &CommonCommand,
    teacher: &Teacher,
    state: &Arc<BotState>,
) -> Result<()> {
    match cmd {
        CommonCommand::Start => {
            let mut text = markdown_string!("👋 *Welcome to Plannabot\\!*\n\n");
            let greeting = markdown_format!(
                "Hello, @{}\\! Use /help to see available commands\\.",
                teacher.telegram_name.as_str()
            );
            text.push(&greeting);
            bot.send_markdown_message(msg.chat.id, text).await?;
        }

        CommonCommand::Help => {
            let text = markdown_string!(
                "*Available Commands \\(Teacher Mode\\):*\n\n\
                /start \\- Start the bot\n\
                /help \\- Display this help message\n\
                /schedule \\- Show your planned lessons\n\
                /impersonate <username> \\- View the bot as a student\n\
                /quit \\- Exit impersonation mode"
            );
            bot.send_markdown_message(msg.chat.id, text).await?;
        }

        CommonCommand::Schedule => {
            let entries = state
                .sheets
                .get_teacher_schedule(&teacher.telegram_name)
                .await?;

            let mut planned: Vec<_> = entries.into_iter().filter(|e| e.is_planned()).collect();
            planned.sort_by_key(|e| e.datetime);

            if planned.is_empty() {
                let text = markdown_string!("📅 No planned lessons found\\.");
                bot.send_markdown_message(msg.chat.id, text).await?;
            } else {
                let tz: Tz = teacher.timezone.parse().unwrap_or(chrono_tz::UTC);
                let mut text = markdown_string!("📅 *Your planned lessons:*\n\n");
                for entry in &planned {
                    let local_time = entry.datetime.with_timezone(&tz);
                    let date_str = local_time.format("%Y-%m-%d").to_string();
                    let time_str = local_time.format("%H:%M").to_string();
                    let duration_str = super::format_duration(entry.duration_minutes);
                    let line = markdown_format!(
                        "📚 {} \\| {} \\| {} — Student: @{}\n",
                        date_str,
                        time_str,
                        duration_str,
                        entry.student_telegram.as_str()
                    );
                    text.push(&line);
                }
                bot.send_markdown_message(msg.chat.id, text).await?;
            }
        }
    }

    Ok(())
}

pub async fn handle_teacher_command(
    bot: &Bot,
    msg: &Message,
    cmd: &TeacherCommand,
    teacher: &Teacher,
    state: &Arc<BotState>,
) -> Result<()> {
    match cmd {
        TeacherCommand::Impersonate(username) => {
            if state.get_impersonation(msg.chat.id).await.is_some() {
                bot.send_message(
                    msg.chat.id,
                    "You are already in impersonation mode. Use /quit first.",
                )
                .await?;
                return Ok(());
            }

            let normalised = username.trim_start_matches('@').to_lowercase();
            if normalised.is_empty() {
                bot.send_message(msg.chat.id, "Usage: /impersonate <student_username>")
                    .await?;
                return Ok(());
            }

            match state.get_role(&normalised).await {
                Some(UserRole::Student(_)) => {
                    state.impersonate(msg.chat.id, normalised.clone()).await;
                    bot.send_message(
                        msg.chat.id,
                        format!(
                            "Now impersonating @{}. All commands will behave as if you were that student. \
                             Use /quit to return to teacher mode.",
                            normalised
                        ),
                    )
                    .await?;
                }
                Some(UserRole::Teacher(_)) => {
                    bot.send_message(
                        msg.chat.id,
                        format!("@{} is a teacher, not a student.", normalised),
                    )
                    .await?;
                }
                None => {
                    bot.send_message(
                        msg.chat.id,
                        format!("Student @{} was not found in the spreadsheet.", normalised),
                    )
                    .await?;
                }
            }
        }

        TeacherCommand::Quit => {
            if state.get_impersonation(msg.chat.id).await.is_some() {
                state.clear_impersonation(msg.chat.id).await;
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "Exited impersonation mode. You are back as @{} (teacher).",
                        teacher.telegram_name
                    ),
                )
                .await?;
            } else {
                bot.send_message(msg.chat.id, "You are not in impersonation mode.")
                    .await?;
            }
        }
    }

    Ok(())
}
