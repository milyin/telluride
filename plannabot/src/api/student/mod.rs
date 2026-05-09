use crate::api::context::BotCtx;
use anyhow::Result;
use telluride::markdown::MarkdownStringMessage;
use telluride::markdown_string;

pub async fn help(ctx: &BotCtx<impl Send + Sync + Clone>) -> Result<()> {
    let text = markdown_string!(
        "*Available Commands:*\n\n\
        /start \\- Start the bot\n\
        /help \\- Display this help message\n\
        /schedule \\- Show your planned lessons\n\
        /balance \\- View your balance\n\
        /book \\- Book a lesson\n\
        /notification \\- Manage lesson reminders"
    );
    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}
