use crate::models::Student;
use crate::state::BotState;
use anyhow::Result;
use chrono_tz::Tz;
use std::sync::Arc;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
use teloxide::prelude::*;

/// Handle the /start command for a student.
pub async fn start(bot: &Bot, chat_id: ChatId, student: &Student) -> Result<()> {
    let mut text = markdown_string!("👋 *Welcome to Plannabot\\!*\n\n");
    let greeting = markdown_format!(
        "Hello, {}\\! Use /help to see available commands\\.",
        student.name.as_str()
    );
    text.push(&greeting);
    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}

/// Handle the /help command for a student.
pub async fn help(bot: &Bot, chat_id: ChatId, is_impersonating: bool) -> Result<()> {
    let mut text = markdown_string!(
        "*Available Commands:*\n\n\
        /start \\- Start the bot\n\
        /help \\- Display this help message\n\
        /schedule \\- Show your planned lessons"
    );
    if is_impersonating {
        let quit_cmd = markdown_string!("\n/quit \\- Exit impersonation mode");
        text.push(&quit_cmd);
    }
    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}

/// Handle the /schedule command for a student.
pub async fn schedule(
    bot: &Bot,
    chat_id: ChatId,
    student: &Student,
    state: &Arc<BotState>,
) -> Result<()> {
    let entries = state
        .sheets
        .get_student_schedule(&student.telegram_name)
        .await?;

    let mut planned: Vec<_> = entries.into_iter().filter(|e| e.is_planned()).collect();
    planned.sort_by_key(|e| e.datetime);

    if planned.is_empty() {
        let text = markdown_string!("📅 No planned lessons found\\.");
        bot.send_markdown_message(chat_id, text).await?;
    } else {
        let tz: Tz = student.timezone.parse().unwrap_or(chrono_tz::UTC);
        let mut text = markdown_string!("📅 *Your planned lessons:*\n\n");
        for entry in &planned {
            let local_time = entry.datetime.with_timezone(&tz);
            let date_str = local_time.format("%Y-%m-%d").to_string();
            let time_str = local_time.format("%H:%M").to_string();
            let duration_str = format_duration(entry.duration_minutes);
            let line = markdown_format!(
                "📚 {} \\| {} \\| {} — Teacher: @{}\n",
                date_str,
                time_str,
                duration_str,
                entry.teacher_telegram.as_str()
            );
            text.push(&line);
        }
        bot.send_markdown_message(chat_id, text).await?;
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
