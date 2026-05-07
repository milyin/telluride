use crate::api::common::format_duration;
use crate::api::context::BotCtx;
use crate::models::Teacher;
use teloxide::prelude::*;
use anyhow::Result;
use chrono_tz::Tz;
use telluride::markdown::{MarkdownString, MarkdownStringMessage};
use telluride::{markdown_format, markdown_string};

pub async fn help(ctx: &BotCtx<impl Send + Sync + Clone>) -> Result<()> {
    let spreadsheet_id = ctx.state.sheets.get_spreadsheet_id();
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
        /book \\- Book a lesson for a student\n\
        /impersonate \\- Impersonate a student to make actions on their behalf\n\
        /admin \\- Enter admin mode\n\
        /refresh \\- Forcedly refresh the data\n\n\
        📊 *Master Schedule:* [View on Google Sheets]({})",
        @raw url_markdown
    );
    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}

pub async fn schedule(ctx: &BotCtx<impl Send + Sync + Clone>, teacher: &Teacher) -> Result<()> {
    let entries = ctx.state
        .sheets
        .get_teacher_schedule(teacher.telegram_name.as_str())
        .await?;

    let mut planned: Vec<_> = entries.into_iter().filter(|e| e.is_planned()).collect();
    planned.sort_by_key(|e| e.datetime);

    if planned.is_empty() {
        let text = markdown_string!("📅 No planned lessons found\\.");
        ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    } else {
        let tz: Tz = teacher.timezone;
        let mut text = markdown_string!("📅 *Your planned lessons:*\n\n");
        for entry in &planned {
            let local_time = entry.datetime.with_timezone(&tz);
            let date_str = local_time.format("%Y-%m-%d").to_string();
            let time_str = local_time.format("%H:%M").to_string();
            let duration_str = format_duration(entry.duration_minutes as i64);
            let line = markdown_format!(
                "📚 {} \\| {} \\| {} — Student: {}\n",
                date_str,
                time_str,
                duration_str,
                entry.student_telegram.to_string()
            );
            text.push(&line);
        }
        ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    }

    Ok(())
}

pub async fn admin(ctx: &BotCtx<impl Send + Sync + Clone>, teacher: &Teacher) -> Result<()> {
    if !teacher.admin {
        ctx.bot.send_message(ctx.chat_id, "You don't have admin privileges.")
            .await?;
        return Ok(());
    }
    if ctx.state.is_in_admin_mode(ctx.chat_id).await {
        ctx.bot.send_message(ctx.chat_id, "You are already in admin mode. Use /quit to exit.")
            .await?;
        return Ok(());
    }
    ctx.state.clear_impersonation(ctx.chat_id).await;
    ctx.state.enter_admin_mode(ctx.chat_id).await;
    ctx.bot.send_message(
        ctx.chat_id,
        "Admin mode activated. Use /help to see available commands, use /quit to exit",
    )
    .await?;
    Ok(())
}
