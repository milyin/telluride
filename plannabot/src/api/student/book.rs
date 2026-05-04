use std::sync::Arc;

use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate, NaiveTime};
use humantime::Duration;
use telluride::command::{CallbackBitcode, CallbackKey};
use telluride::data_store::InMemStore;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
use teloxide::prelude::*;
use teloxide::types::UserId;

use crate::api::menus::{show_date_selection, show_teacher_selection};
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
        BookParams::L1(teacher, year, month) => {
            book_1(bot, chat_id, teacher, year, month, user_id, callback_storage).await
        }
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
    let now = Local::now();
    let year = now.year();
    let month = now.month();
    show_teacher_selection(
        bot,
        chat_id,
        None,
        "📅 Select a teacher to book a lesson:",
        user_id,
        callback_storage,
        state,
        move |name| Cmd::book(BookParams::L1(name, year, month)),
    )
    .await
}

async fn book_1<Cmd: BookCommand + CallbackBitcode + 'static>(
    bot: &Bot,
    chat_id: ChatId,
    teacher: TelegramName,
    year: i32,
    month: u32,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
) -> Result<()> {
    let t1 = teacher.clone();
    let t2 = teacher;
    show_date_selection(
        bot,
        chat_id,
        None,
        year,
        month,
        user_id,
        callback_storage,
        move |date| Cmd::book(BookParams::L2(t1.clone(), date)),
        move |y, m| Cmd::book(BookParams::L1(t2.clone(), y, m)),
    )
    .await
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
