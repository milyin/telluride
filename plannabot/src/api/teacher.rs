use crate::api::common::format_duration;
use crate::models::Teacher;
use crate::state::BotState;
use anyhow::Result;
use chrono_tz::Tz;
use std::sync::Arc;
use telluride::markdown::{MarkdownString, MarkdownStringMessage};
use telluride::{markdown_format, markdown_string};
use teloxide::prelude::*;

/// Handle the /help command for a teacher.
///
/// Always shows all teacher commands, including /admin (non-admin teachers see it
/// but receive "no admin privileges" if they try it).
pub async fn help(bot: &Bot, chat_id: ChatId, state: &Arc<BotState>) -> Result<()> {
    let spreadsheet_id = state.sheets.get_spreadsheet_id();
    let sheets_url = format!(
        "https://docs.google.com/spreadsheets/d/{}/edit",
        spreadsheet_id
    );

    let url_markdown = MarkdownString::escape(sheets_url);

    let text = markdown_format!(
        "*Available Commands \\(Teacher Mode\\):*\n\n\
        /start \\- Start the bot\n\
        /help \\- Display this help message\n\
        /schedule \\- Show your planned lessons\n\
        /impersonate \\- Choose a student to impersonate from a button list\n\
        /admin \\- Enter admin mode\n\
        /refresh \\- Forcedly refresh the data\n\n\
        📊 *Master Schedule:* [View on Google Sheets]({})",
        @raw url_markdown
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

/// Handle the /admin command — enter admin mode.
///
/// Clears impersonation mode first (modes are mutually exclusive).
/// Sends "no admin privileges" if the teacher does not have admin = true.
pub async fn admin(
    bot: &Bot,
    chat_id: ChatId,
    teacher: &Teacher,
    state: &Arc<BotState>,
) -> Result<()> {
    if !teacher.admin {
        bot.send_message(chat_id, "You don't have admin privileges.")
            .await?;
        return Ok(());
    }
    if state.is_in_admin_mode(chat_id).await {
        bot.send_message(chat_id, "You are already in admin mode. Use /quit to exit.")
            .await?;
        return Ok(());
    }
    state.clear_impersonation(chat_id).await;
    state.enter_admin_mode(chat_id).await;
    bot.send_message(
        chat_id,
        "Admin mode activated. Use /help to see available commands, use /quit to exit",
    )
    .await?;
    Ok(())
}
