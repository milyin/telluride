use crate::models::Teacher;
use crate::state::BotState;
use anyhow::Result;
use chrono_tz::Tz;
use std::sync::Arc;
use telluride::markdown::{MarkdownString, MarkdownStringMessage};
use telluride::{markdown_format, markdown_string};
use teloxide::prelude::*;

/// Handle the /refresh command.
pub async fn refresh(bot: &Bot, chat_id: ChatId, state: &Arc<BotState>) -> Result<()> {
    bot.send_message(chat_id, "🔄 Refreshing data from Google Sheets...")
        .await?;

    match state.refresh().await {
        Ok(errors) => {
            let stats = state.get_stats().await;
            let mut text = markdown_format!(
                "✅ *Data successfully refreshed\\!*\n\n\
                • Students: {}\n\
                • Teachers: {}\n",
                stats.students_count.to_string(),
                stats.teachers_count.to_string()
            );

            if !errors.is_empty() {
                let err_text = markdown_format!(
                    "\n⚠️ *Encountered {} parse error\\(s\\)\\.*",
                    errors.len().to_string()
                );
                text.push(&err_text);
            }

            bot.send_markdown_message(chat_id, text).await?;
        }
        Err(e) => {
            bot.send_message(chat_id, format!("❌ Failed to refresh data: {}", e))
                .await?;
        }
    }

    Ok(())
}

/// Handle the /help command in admin mode.
pub async fn help(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let text = markdown_string!(
        "*Available Commands \\(Admin Mode\\):*\n\n\
        /start \\- Exit admin mode and restart the bot\n\
        /help \\- Display this help message\n\
        /status \\- Show global stat information\n\
        /refresh \\- Forcedly refresh the data\n\
        /quit \\- Exit admin mode"
    );
    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}

/// Handle the /status command (admin mode only).
pub async fn status(
    bot: &Bot,
    chat_id: ChatId,
    teacher: &Teacher,
    state: &Arc<BotState>,
) -> Result<()> {
    let stats = state.get_stats().await;
    let tz: Tz = teacher.timezone;

    let last_reload_str = stats
        .last_reload
        .map(|t| {
            let local_time = t.with_timezone(&tz);
            local_time.format("%Y-%m-%d %H:%M:%S %Z").to_string()
        })
        .unwrap_or_else(|| "Never".to_string());

    let last_modified_str = stats
        .last_modified
        .map(|t| {
            let local_time = t.with_timezone(&tz);
            local_time.format("%Y-%m-%d %H:%M:%S %Z").to_string()
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let spreadsheet_url = format!(
        "https://docs.google.com/spreadsheets/d/{}/edit",
        stats.spreadsheet_id
    );
    let url_markdown = MarkdownString::escape(spreadsheet_url);

    let text = markdown_format!(
        "📊 *Bot Status*\n\n\
        • *Students:* {}\n\
        • *Teachers:* {}\n\
        • *Sheet Modified:* {}\n\
        • *Last Refresh:* {}\n\
        • *Refresh Delay:* {}s\n\n\
        You can use /refresh to forcefully update the data\\.\n\n\
        🔗 [View Google Sheet]({})",
        stats.students_count.to_string(),
        stats.teachers_count.to_string(),
        MarkdownString::escape(last_modified_str),
        MarkdownString::escape(last_reload_str),
        stats.refresh_delay.as_secs().to_string(),
        @raw url_markdown
    );

    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}

/// Handle the /quit command in admin mode.
pub async fn quit(bot: &Bot, chat_id: ChatId, state: &Arc<BotState>) -> Result<()> {
    state.exit_admin_mode(chat_id).await;
    bot.send_message(chat_id, "Exited admin mode.").await?;
    Ok(())
}
