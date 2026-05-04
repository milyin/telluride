use std::sync::Arc;

use anyhow::Result;
use chrono::{NaiveDate, NaiveTime};
use humantime::Duration;
use telluride::command::{CallbackBitcode, CallbackKey};
use telluride::data_store::InMemStore;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
use teloxide::prelude::*;
use teloxide::types::UserId;

use crate::api::menus::show_teacher_selection;
use crate::api::traits::{BookCommand, BookParams};
use crate::models::TelegramName;
use crate::state::BotState;

pub async fn book<Cmd: BookCommand + CallbackBitcode + 'static>(
    bot: &Bot,
    chat_id: ChatId,
    params: &str,
    state: &Arc<BotState>,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
) -> Result<()> {
    let params: BookParams = params.parse()?;
    match params {
        BookParams::L0() => book_0(bot, chat_id, state, user_id, callback_storage).await,
        BookParams::L1(teacher) => book_1(bot, chat_id, teacher).await,
        BookParams::L2(teacher, date) => book_2(bot, chat_id, teacher, date).await,
        BookParams::L3(teacher, date, hour) => book_3(bot, chat_id, teacher, date, hour).await,
        BookParams::L4(teacher, date, hour, duration) => {
            book_4(bot, chat_id, teacher, date, hour, duration).await
        }
    }
}

async fn book_0<Cmd: BookCommand + CallbackBitcode + 'static>(
    bot: &Bot,
    chat_id: ChatId,
    state: &Arc<BotState>,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
) -> Result<()> {
    show_teacher_selection(
        bot,
        chat_id,
        None,
        "📅 Select a teacher to book a lesson:",
        user_id,
        callback_storage,
        state,
        |name| Cmd::book(BookParams::L1(name)),
    )
    .await
}

async fn book_1(bot: &Bot, chat_id: ChatId, teacher: TelegramName) -> Result<()> {
    let mut text = markdown_string!("📅 *Book a Lesson*\n\n");
    let teacher_str = teacher.as_str();
    let line = markdown_format!("👨\\-🏫 Teacher: {}\n", teacher_str);
    text.push(&line);

    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}

async fn book_2(bot: &Bot, chat_id: ChatId, teacher: TelegramName, date: NaiveDate) -> Result<()> {
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
