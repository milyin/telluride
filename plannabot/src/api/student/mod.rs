use crate::api::common::format_duration;
use crate::api::context::BotCtx;
use crate::models::Student;
use anyhow::Result;
use chrono_tz::Tz;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};

pub async fn help(ctx: &BotCtx<impl Send + Sync + Clone>) -> Result<()> {
    let text = markdown_string!(
        "*Available Commands:*\n\n\
        /start \\- Start the bot\n\
        /help \\- Display this help message\n\
        /schedule \\- Show your planned lessons\n\
        /book \\- Book a lesson"
    );
    ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    Ok(())
}

pub async fn schedule(ctx: &BotCtx<impl Send + Sync + Clone>, student: &Student) -> Result<()> {
    let entries = ctx.state
        .sheets
        .get_student_schedule(student.telegram_name.as_str())
        .await?;

    let mut planned: Vec<_> = entries.into_iter().filter(|e| e.is_planned()).collect();
    planned.sort_by_key(|e| e.datetime);

    if planned.is_empty() {
        let text = markdown_string!("📅 No planned lessons found\\.");
        ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    } else {
        let tz: Tz = student.timezone;
        let mut text = markdown_string!("📅 *Your planned lessons:*\n\n");
        for entry in &planned {
            let local_time = entry.datetime.with_timezone(&tz);
            let date_str = local_time.format("%Y-%m-%d").to_string();
            let time_str = local_time.format("%H:%M").to_string();
            let duration_str = format_duration(entry.duration_minutes as i64);
            let line = markdown_format!(
                "📚 {} \\| {} \\| {} — Teacher: {}\n",
                date_str,
                time_str,
                duration_str,
                entry.teacher_telegram.to_string()
            );
            text.push(&line);
        }
        ctx.bot.send_markdown_message(ctx.chat_id, text).await?;
    }

    Ok(())
}
