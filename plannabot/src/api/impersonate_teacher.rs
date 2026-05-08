use crate::api::context::BotCtx;
use crate::models::Teacher;
use anyhow::Result;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};

pub async fn help(ctx: &BotCtx<impl Send + Sync + Clone>) -> Result<()> {
    let text = markdown_string!(
        "*Available Commands \\(Teacher Impersonation Mode\\):*\n\n\
        /start \\- Exit impersonation and restart the bot\n\
        /help \\- Display this help message\n\
        /schedule \\- Show the impersonated teacher's planned lessons\n\
        /balance \\- View student balances as the impersonated teacher\n\
        /book \\- Book a lesson as the impersonated teacher\n\
        /payment \\- View student balances as the impersonated teacher\n\
        /quit \\- Exit impersonation mode"
    );
    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}

pub async fn quit(ctx: &BotCtx<impl Send + Sync + Clone>, acting_teacher: &Teacher) -> Result<()> {
    ctx.state.clear_teacher_impersonation(ctx.chat_id).await;
    let text = markdown_format!(
        "Exited impersonation mode\\. You are back as {} \\(teacher\\)\\.",
        acting_teacher.telegram_name.to_string()
    );
    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}
