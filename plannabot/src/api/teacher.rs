use crate::api::context::BotCtx;
use crate::models::Teacher;
use anyhow::Result;
use telluride::markdown::{MarkdownString, MarkdownStringMessage};
use telluride::markdown_format;
use teloxide::prelude::*;

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
        /balance \\- View student balances\n\
        /book \\- Book a lesson for a student\n\
        /payment \\- View student balances and record payments\n\
        /student \\- Manage your students\n\
        /worktime \\- Manage your working hours\n\
        /admin \\- Enter admin mode\n\
        /refresh \\- Forcedly refresh the data\n\n\
        📊 *Master Schedule:* [View on Google Sheets]({})",
        @raw url_markdown
    );
    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}

pub async fn admin(ctx: &BotCtx<impl Send + Sync + Clone>, teacher: &Teacher) -> Result<()> {
    if !teacher.admin {
        ctx.bot
            .send_message(ctx.chat_id, "You don't have admin privileges.")
            .await?;
        return Ok(());
    }
    if ctx.state.is_in_admin_mode(ctx.chat_id).await {
        ctx.bot
            .send_message(
                ctx.chat_id,
                "You are already in admin mode. Use /quit to exit.",
            )
            .await?;
        return Ok(());
    }
    ctx.state.clear_student_impersonation(ctx.chat_id).await;
    ctx.state.enter_admin_mode(ctx.chat_id).await;
    ctx.bot
        .send_message(
            ctx.chat_id,
            "Admin mode activated. Use /help to see available commands, use /quit to exit",
        )
        .await?;
    Ok(())
}
