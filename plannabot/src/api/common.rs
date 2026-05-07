use crate::api::context::BotCtx;
use crate::models::UserRole;
use anyhow::Result;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};

pub async fn start(ctx: &BotCtx<impl Send + Sync + Clone>, username: &str) -> Result<()> {
    ctx.state.clear_student_impersonation(ctx.chat_id).await;
    ctx.state.clear_teacher_impersonation(ctx.chat_id).await;
    ctx.state.exit_admin_mode(ctx.chat_id).await;

    let role = ctx.state.get_role(username).await;

    let mut text = markdown_string!("👋 *Welcome to Plannabot\\!*\n\n");
    let greeting = markdown_format!(
        "Hello, {}\\! Use /help to see available commands\\.",
        role.as_ref().map(|r| r.name()).unwrap_or(username)
    );
    text.push(&greeting);

    if let Some(UserRole::Teacher(teacher)) = &role {
        if ctx.state
            .is_both_teacher_and_student(teacher.telegram_name.as_str())
            .await
        {
            let info = markdown_string!(
                "\n\n📌 *Note:* You are registered as both a teacher and a student\\."
            );
            text.push(&info);
        }
    }

    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}

pub fn format_duration(minutes: i64) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    match (hours, mins) {
        (0, m) => format!("{}m", m),
        (h, 0) => format!("{}h", h),
        (h, m) => format!("{}h{}m", h, m),
    }
}
