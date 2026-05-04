use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate};
use telluride::calendar::build_month_calendar;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
use teloxide::prelude::*;
use teloxide::types::InlineKeyboardButton;

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
