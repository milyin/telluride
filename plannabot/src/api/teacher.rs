use crate::models::{Teacher, UserRole};
use crate::state::BotState;
use anyhow::Result;
use chrono_tz::Tz;
use std::sync::Arc;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
use teloxide::prelude::*;

/// Handle the /start command for a teacher.
pub async fn start(
    bot: &Bot,
    chat_id: ChatId,
    teacher: &Teacher,
    state: &Arc<BotState>,
) -> Result<()> {
    let mut text = markdown_string!("👋 *Welcome to Plannabot\\!*\n\n");
    let greeting = markdown_format!(
        "Hello, @{}\\! Use /help to see available commands\\.",
        teacher.telegram_name.as_str()
    );
    text.push(&greeting);

    // Check if this user is also a student
    if state
        .is_both_teacher_and_student(&teacher.telegram_name)
        .await
    {
        let info = markdown_string!(
            "\n\n📌 *Note:* You are registered as both a teacher and a student\\."
        );
        text.push(&info);
    }

    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}

/// Handle the /help command for a teacher.
pub async fn help(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let text = markdown_string!(
        "*Available Commands \\(Teacher Mode\\):*\n\n\
        /start \\- Start the bot\n\
        /help \\- Display this help message\n\
        /schedule \\- Show your planned lessons\n\
        /impersonate \\<username\\> \\- View the bot as a student\n\
        /quit \\- Exit impersonation mode"
    );
    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}

/// Handle the /schedule command for a teacher.
pub async fn schedule(
    bot: &Bot,
    chat_id: ChatId,
    teacher: &Teacher,
    state: &Arc<BotState>,
) -> Result<()> {
    let entries = state
        .sheets
        .get_teacher_schedule(&teacher.telegram_name)
        .await?;

    let mut planned: Vec<_> = entries.into_iter().filter(|e| e.is_planned()).collect();
    planned.sort_by_key(|e| e.datetime);

    if planned.is_empty() {
        let text = markdown_string!("📅 No planned lessons found\\.");
        bot.send_markdown_message(chat_id, text).await?;
    } else {
        let tz: Tz = teacher.timezone.parse().unwrap_or(chrono_tz::UTC);
        let mut text = markdown_string!("📅 *Your planned lessons:*\n\n");
        for entry in &planned {
            let local_time = entry.datetime.with_timezone(&tz);
            let date_str = local_time.format("%Y-%m-%d").to_string();
            let time_str = local_time.format("%H:%M").to_string();
            let duration_str = format_duration(entry.duration_minutes);
            let line = markdown_format!(
                "📚 {} \\| {} \\| {} — Student: @{}\n",
                date_str,
                time_str,
                duration_str,
                entry.student_telegram.as_str()
            );
            text.push(&line);
        }
        bot.send_markdown_message(chat_id, text).await?;
    }

    Ok(())
}

/// Handle the /impersonate command for a teacher.
pub async fn impersonate(
    bot: &Bot,
    chat_id: ChatId,
    username: &str,
    state: &Arc<BotState>,
) -> Result<()> {
    if state.get_impersonation(chat_id).await.is_some() {
        bot.send_message(
            chat_id,
            "You are already in impersonation mode. Use /quit first.",
        )
        .await?;
        return Ok(());
    }

    let normalised = username.trim_start_matches('@').to_lowercase();
    if normalised.is_empty() {
        bot.send_message(chat_id, "Usage: /impersonate <student_username>")
            .await?;
        return Ok(());
    }

    match state.get_role(&normalised).await {
        Some(UserRole::Student(_)) => {
            state.impersonate(chat_id, normalised.clone()).await;
            bot.send_message(
                chat_id,
                format!(
                    "Now impersonating @{}. All commands will behave as if you were that student. \
                     Use /quit to return to teacher mode.",
                    normalised
                ),
            )
            .await?;
        }
        Some(UserRole::Teacher(_)) => {
            // Check if this teacher is also a student (dual role)
            if state.is_both_teacher_and_student(&normalised).await {
                state.impersonate(chat_id, normalised.clone()).await;
                bot.send_message(
                    chat_id,
                    format!(
                        "Now impersonating @{} (who is also a teacher). All commands will behave as if you were that student. \
                         Use /quit to return to teacher mode.",
                        normalised
                    ),
                )
                .await?;
            } else {
                bot.send_message(
                    chat_id,
                    format!("@{} is a teacher, not a student.", normalised),
                )
                .await?;
            }
        }
        None => {
            bot.send_message(
                chat_id,
                format!("Student @{} was not found in the spreadsheet.", normalised),
            )
            .await?;
        }
    }

    Ok(())
}

/// Handle the /quit command for a teacher (exit impersonation mode).
pub async fn quit(
    bot: &Bot,
    chat_id: ChatId,
    teacher: &Teacher,
    state: &Arc<BotState>,
) -> Result<()> {
    if state.get_impersonation(chat_id).await.is_some() {
        state.clear_impersonation(chat_id).await;
        bot.send_message(
            chat_id,
            format!(
                "Exited impersonation mode. You are back as @{} (teacher).",
                teacher.telegram_name
            ),
        )
        .await?;
    } else {
        bot.send_message(chat_id, "You are not in impersonation mode.")
            .await?;
    }

    Ok(())
}

/// Formats a lesson duration in minutes to a human-readable string.
///
/// - `90`  → `"1h 30m"`
/// - `120` → `"2h"`
/// - `45`  → `"45m"`
fn format_duration(minutes: i64) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    match (hours, mins) {
        (0, m) => format!("{}m", m),
        (h, 0) => format!("{}h", h),
        (h, m) => format!("{}h {}m", h, m),
    }
}
