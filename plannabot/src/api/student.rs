use crate::api::common::format_duration;
use crate::models::Student;
use crate::state::BotState;
use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate};
use chrono_tz::Tz;
use std::sync::Arc;
use telluride::calendar::build_month_calendar;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
use teloxide::prelude::*;
use teloxide::types::InlineKeyboardButton;

/// Handle the /help command for a student.
pub async fn help(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let text = markdown_string!(
        "*Available Commands:*\n\n\
        /start \\- Start the bot\n\
        /help \\- Display this help message\n\
        /schedule \\- Show your planned lessons\n\
        /book \\- Book a lesson"
    );
    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}

/// Handle the /schedule command for a student.
pub async fn schedule(
    bot: &Bot,
    chat_id: ChatId,
    student: &Student,
    state: &Arc<BotState>,
) -> Result<()> {
    let entries = state
        .sheets
        .get_student_schedule(student.telegram_name.as_str())
        .await?;

    let mut planned: Vec<_> = entries.into_iter().filter(|e| e.is_planned()).collect();
    planned.sort_by_key(|e| e.datetime);

    if planned.is_empty() {
        let text = markdown_string!("📅 No planned lessons found\\.");
        bot.send_markdown_message(chat_id, text).await?;
    } else {
        let tz: Tz = student.timezone;
        let mut text = markdown_string!("📅 *Your planned lessons:*\n\n");
        for entry in &planned {
            let local_time = entry.datetime.with_timezone(&tz);
            let date_str = local_time.format("%Y-%m-%d").to_string();
            let time_str = local_time.format("%H:%M").to_string();
            let duration_str = format_duration(entry.duration_minutes);
            let line = markdown_format!(
                "📚 {} \\| {} \\| {} — Teacher: {}\n",
                date_str,
                time_str,
                duration_str,
                entry.teacher_telegram.to_string()
            );
            text.push(&line);
        }
        bot.send_markdown_message(chat_id, text).await?;
    }

    Ok(())
}

/// Handle the /book command for a student.
pub async fn book(bot: &Bot, chat_id: ChatId, params: &str) -> Result<()> {
    // Parse parameters: teacher_name date hour duration
    // All are optional, space-separated
    let parts: Vec<&str> = params.split_whitespace().collect();

    let teacher = parts.get(0).and_then(|s| if s.is_empty() { None } else { Some(*s) });
    let date = parts.get(1).and_then(|s| if s.is_empty() { None } else { Some(*s) });
    let hour = parts.get(2).and_then(|s| if s.is_empty() { None } else { Some(*s) });
    let duration = parts.get(3).and_then(|s| if s.is_empty() { None } else { Some(*s) });

    // Build summary of provided parameters
    let mut text = markdown_string!("📅 *Book a Lesson*\n\n");

    if let Some(t) = teacher {
        let line = markdown_format!("👨\\-🏫 Teacher: {}\n", t);
        text.push(&line);
    }
    if let Some(d) = date {
        let line = markdown_format!("📆 Date: {}\n", d);
        text.push(&line);
    }
    if let Some(h) = hour {
        let line = markdown_format!("⏰ Time: {}\n", h);
        text.push(&line);
    }
    if let Some(dur) = duration {
        let line = markdown_format!("⏱ Duration: {} min\n", dur);
        text.push(&line);
    }

    if teacher.is_none() && date.is_none() && hour.is_none() && duration.is_none() {
        text.push(&markdown_string!("No parameters provided\\.\n"));
    }

    text.push(&markdown_string!("\n*Select a date from the calendar:*\n"));

    // Determine which month/year to show
    let (year, month) = if let Some(d) = date {
        // Try to parse the date to show its month
        if let Ok(parsed) = NaiveDate::parse_from_str(d, "%Y-%m-%d") {
            (parsed.year(), parsed.month())
        } else {
            let now = Local::now();
            (now.year(), now.month())
        }
    } else {
        let now = Local::now();
        (now.year(), now.month())
    };

    let keyboard = build_month_calendar(
        year,
        month,
        |d| InlineKeyboardButton::callback(d.day().to_string(), "noop"),
        |_leading, _trailing| (Vec::new(), Vec::new()),
    );

    bot.send_markdown_message(chat_id, text)
        .reply_markup(keyboard)
        .await?;
    Ok(())
}
