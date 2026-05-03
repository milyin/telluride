pub mod impersonate;

pub use impersonate::impersonate;

use crate::models::Teacher;
use crate::state::BotState;
use anyhow::Result;
use chrono_tz::Tz;
use std::sync::Arc;
use telluride::markdown::{MarkdownString, MarkdownStringMessage};
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
        "Hello, {}\\! Use /help to see available commands\\.",
        teacher.telegram_name.to_string()
    );
    text.push(&greeting);

    // Check if this user is also a student
    if state
        .is_both_teacher_and_student(teacher.telegram_name.as_str())
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
pub async fn help(
    bot: &Bot,
    chat_id: ChatId,
    teacher: &Teacher,
    state: &Arc<BotState>,
) -> Result<()> {
    let spreadsheet_id = state.sheets.get_spreadsheet_id();
    let sheets_url = format!(
        "https://docs.google.com/spreadsheets/d/{}/edit",
        spreadsheet_id
    );

    let url_markdown = MarkdownString::escape(sheets_url);

    let mut text = markdown_string!(
        "*Available Commands \\(Teacher Mode\\):*\n\n\
        /start \\- Start the bot\n\
        /help \\- Display this help message\n\
        /schedule \\- Show your planned lessons\n\
        /impersonate \\- Choose a student to impersonate from a button list\n\
        /quit \\- Exit impersonation mode\n\
        /refresh \\- Forcedly refresh the data"
    );

    if teacher.admin {
        let admin_part = markdown_string!("\n/admin \\- Enter admin mode");
        text.push(&admin_part);
    }

    let footer = markdown_format!(
        "\n\n📊 *Master Schedule:* [View on Google Sheets]({})",
        @raw url_markdown
    );
    text.push(&footer);

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
        .get_teacher_schedule(teacher.telegram_name.as_str())
        .await?;

    let mut planned: Vec<_> = entries.into_iter().filter(|e| e.is_planned()).collect();
    planned.sort_by_key(|e| e.datetime);

    if planned.is_empty() {
        let text = markdown_string!("📅 No planned lessons found\\.");
        bot.send_markdown_message(chat_id, text).await?;
    } else {
        let tz: Tz = teacher.timezone;
        let mut text = markdown_string!("📅 *Your planned lessons:*\n\n");
        for entry in &planned {
            let local_time = entry.datetime.with_timezone(&tz);
            let date_str = local_time.format("%Y-%m-%d").to_string();
            let time_str = local_time.format("%H:%M").to_string();
            let duration_str = format_duration(entry.duration_minutes);
            let line = markdown_format!(
                "📚 {} \\| {} \\| {} — Student: {}\n",
                date_str,
                time_str,
                duration_str,
                entry.student_telegram.to_string()
            );
            text.push(&line);
        }
        bot.send_markdown_message(chat_id, text).await?;
    }

    Ok(())
}

/// Handle the /quit command for a teacher (exit impersonation or admin mode).
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
                "Exited impersonation mode. You are back as {} (teacher).",
                teacher.telegram_name
            ),
        )
        .await?;
    } else if state.is_in_admin_mode(chat_id).await {
        state.exit_admin_mode(chat_id).await;
        bot.send_message(chat_id, "Exited admin mode.").await?;
    } else {
        bot.send_message(chat_id, "You are not in impersonation or admin mode.")
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

/// Handle the /admin command — enter admin mode (only for teachers with admin = true).
pub async fn admin(
    bot: &Bot,
    chat_id: ChatId,
    teacher: &Teacher,
    state: &Arc<BotState>,
) -> Result<()> {
    if !teacher.admin {
        return Ok(());
    }
    if state.is_in_admin_mode(chat_id).await {
        bot.send_message(chat_id, "You are already in admin mode. Use /quit to exit.")
            .await?;
        return Ok(());
    }
    state.enter_admin_mode(chat_id).await;
    bot.send_message(chat_id, "Admin mode activated. Use /help to see available commands, use /quit for exit")
        .await?;
    Ok(())
}

