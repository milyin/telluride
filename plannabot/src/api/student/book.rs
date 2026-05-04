use std::str::FromStr;

use anyhow::Result;
use chrono::{NaiveDate, NaiveTime};
use humantime::Duration;
use telluride::markdown::MarkdownStringMessage;
use telluride::utils::split_with_screened_spaces;
use telluride::{markdown_format, markdown_string};
use teloxide::prelude::*;

use crate::models::TelegramName;

enum BookParams {
    L0(),
    L1(TelegramName),
    L2(TelegramName, NaiveDate),
    L3(TelegramName, NaiveDate, NaiveTime),
    L4(TelegramName, NaiveDate, NaiveTime, Duration),
}

impl FromStr for BookParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);
        let Some(teacher) = parts.get(0) else {
            return Ok(BookParams::L0());
        };
        let teacher = teacher.parse()?;
        let Some(date) = parts.get(1) else {
            return Ok(BookParams::L1(teacher));
        };
        let date = date.parse()?;
        let Some(hour) = parts.get(2) else {
            return Ok(BookParams::L2(teacher, date));
        };
        let hour = hour.parse()?;
        let Some(duration) = parts.get(3) else {
            return Ok(BookParams::L3(teacher, date, hour));
        };
        let duration = duration.parse()?;
        let Some(extra) = parts.get(4) else {
            return Ok(BookParams::L4(teacher, date, hour, duration));
        };
        Err(anyhow::anyhow!("extra parameter: {}", extra))
    }
}

pub async fn book(bot: &Bot, chat_id: ChatId, params: &str) -> Result<()> {
    let params: BookParams = params.parse()?;
    match params {
        BookParams::L0() => book_0(bot, chat_id).await,
        BookParams::L1(teacher) => book_1(bot, chat_id, teacher).await,
        BookParams::L2(teacher, date) => book_2(bot, chat_id, teacher, date).await,
        BookParams::L3(teacher, date, hour) => book_3(bot, chat_id, teacher, date, hour).await,
        BookParams::L4(teacher, date, hour, duration) => {
            book_4(bot, chat_id, teacher, date, hour, duration).await
        }
    }
}

async fn book_0(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let text = markdown_string!("📅 *Book a Lesson*\n\nNo parameters provided\\.");
    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}

async fn book_1(bot: &Bot, chat_id: ChatId, teacher: TelegramName) -> Result<()> {
    let mut text = markdown_string!("📅 *Book a Lesson*\n\n");
    let teacher_str = teacher.as_str();
    let line = markdown_format!("👨\\-🏫 Teacher: {}\n", teacher_str);
    text.push(&line);

    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}

async fn book_2(
    bot: &Bot,
    chat_id: ChatId,
    teacher: TelegramName,
    date: NaiveDate,
) -> Result<()> {
    let mut text = markdown_string!("📅 *Book a Lesson*\n\n");
    let teacher_str = teacher.as_str();
    let line = markdown_format!("👨\\-🏫 Teacher: {}\n", teacher_str);
    text.push(&line);
    let date_str = date.to_string();
    let line = markdown_format!("📆 Date: {}\n", date_str);
    text.push(&line);

    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}

async fn book_3(
    bot: &Bot,
    chat_id: ChatId,
    teacher: TelegramName,
    date: NaiveDate,
    hour: NaiveTime,
) -> Result<()> {
    let mut text = markdown_string!("📅 *Book a Lesson*\n\n");
    let teacher_str = teacher.as_str();
    let line = markdown_format!("👨\\-🏫 Teacher: {}\n", teacher_str);
    text.push(&line);
    let date_str = date.to_string();
    let line = markdown_format!("📆 Date: {}\n", date_str);
    text.push(&line);
    let hour_str = hour.to_string();
    let line = markdown_format!("⏰ Time: {}\n", hour_str);
    text.push(&line);

    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}

async fn book_4(
    bot: &Bot,
    chat_id: ChatId,
    teacher: TelegramName,
    date: NaiveDate,
    hour: NaiveTime,
    duration: Duration,
) -> Result<()> {
    let mut text = markdown_string!("📅 *Book a Lesson*\n\n");
    let teacher_str = teacher.as_str();
    let line = markdown_format!("👨\\-🏫 Teacher: {}\n", teacher_str);
    text.push(&line);
    let date_str = date.to_string();
    let line = markdown_format!("📆 Date: {}\n", date_str);
    text.push(&line);
    let hour_str = hour.to_string();
    let line = markdown_format!("⏰ Time: {}\n", hour_str);
    text.push(&line);
    let duration_str = duration.to_string();
    let line = markdown_format!("⏱ Duration: {}\n", duration_str);
    text.push(&line);

    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}
