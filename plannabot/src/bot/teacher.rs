use super::Command;
use crate::models::Teacher;
use crate::state::BotState;
use anyhow::Result;
use chrono_tz::Tz;
use std::sync::Arc;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
use teloxide::prelude::*;

pub async fn handle_command(
    bot: &Bot,
    msg: &Message,
    cmd: &Command,
    teacher: &Teacher,
    state: &Arc<BotState>,
) -> Result<()> {
    match cmd {
        Command::Start => {
            let mut text = markdown_string!("👋 *Welcome to Plannabot\\!*\n\n");
            let greeting = markdown_format!(
                "Hello, @{}\\! Use /help to see available commands\\.",
                teacher.telegram_name.as_str()
            );
            text.push(&greeting);
            bot.send_markdown_message(msg.chat.id, text).await?;
        }

        Command::Help => {
            let text = markdown_string!(
                "*Available Commands \\(Teacher Mode\\):*\n\n\
                /start \\- Start the bot\n\
                /help \\- Display this help message\n\
                /schedule \\- Show your planned lessons"
            );
            bot.send_markdown_message(msg.chat.id, text).await?;
        }

        Command::Schedule => {
            let entries = state
                .sheets
                .get_teacher_schedule(&teacher.telegram_name)
                .await?;

            let mut planned: Vec<_> = entries.into_iter().filter(|e| e.is_planned()).collect();
            planned.sort_by_key(|e| e.datetime);

            if planned.is_empty() {
                let text = markdown_string!("📅 No planned lessons found\\.");
                bot.send_markdown_message(msg.chat.id, text).await?;
            } else {
                let tz: Tz = teacher.timezone.parse().unwrap_or(chrono_tz::UTC);
                let mut text = markdown_string!("📅 *Your planned lessons:*\n\n");
                for entry in &planned {
                    let local_time = entry.datetime.with_timezone(&tz);
                    let date_str = local_time.format("%Y-%m-%d").to_string();
                    let time_str = local_time.format("%H:%M").to_string();
                    let duration_str = super::format_duration(entry.duration_minutes);
                    let line = markdown_format!(
                        "📚 {} \\| {} \\| {} — Student: @{}\n",
                        date_str,
                        time_str,
                        duration_str,
                        entry.student_telegram.as_str()
                    );
                    text.push(&line);
                }
                bot.send_markdown_message(msg.chat.id, text).await?;
            }
        }
    }

    Ok(())
}
