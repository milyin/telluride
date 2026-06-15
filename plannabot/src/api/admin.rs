use crate::api::context::BotCtx;
use crate::models::Teacher;
use anyhow::Result;
use chrono_tz::Tz;
use telluride::markdown::{MarkdownString, MarkdownStringMessage};
use telluride::{markdown_format, markdown_string};
use teloxide::prelude::*;

pub async fn refresh(ctx: &BotCtx<impl Send + Sync + Clone>) -> Result<()> {
    ctx.bot
        .send_message(ctx.chat_id, "🔄 Refreshing data from Google Sheets...")
        .await?;

    match ctx.state.refresh().await {
        Ok(errors) => {
            let stats = ctx.state.get_stats().await;
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

            ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
        }
        Err(e) => {
            let text = markdown_format!(
                "❌ *Failed to refresh data:* {}",
                MarkdownString::escape(e.to_string())
            );
            ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
        }
    }

    Ok(())
}

pub async fn help(ctx: &BotCtx<impl Send + Sync + Clone>) -> Result<()> {
    let text = markdown_string!(
        "*Available Commands \\(Admin Mode\\):*\n\n\
        /start \\- Exit admin mode and restart the bot\n\
        /help \\- Display this help message\n\
        /status \\- Show global stat information\n\
        /balance \\- View student balances\n\
        /refresh \\- Forcedly refresh the data\n\
        /impersonate \\- View the bot as a student or teacher\n\
        /user \\- Manage students and teachers\n\
        /quit \\- Exit admin mode"
    );
    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}

pub async fn status(ctx: &BotCtx<impl Send + Sync + Clone>, teacher: &Teacher) -> Result<()> {
    let stats = ctx.state.get_stats().await;
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

    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}

pub async fn quit(ctx: &BotCtx<impl Send + Sync + Clone>) -> Result<()> {
    ctx.state.exit_admin_mode(ctx.chat_id).await;
    ctx.bot
        .send_message(ctx.chat_id, "Exited admin mode.")
        .await?;
    Ok(())
}
