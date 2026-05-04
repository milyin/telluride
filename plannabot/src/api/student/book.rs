use std::sync::Arc;

use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate, NaiveTime};
use humantime::Duration;
use telluride::command::{CallbackBitcode, CallbackKey};
use telluride::data_store::InMemStore;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
use teloxide::prelude::*;
use teloxide::types::{MessageId, UserId};

use crate::api::menus::{show_date_selection, show_slot_selection, show_teacher_selection};
use crate::api::traits::{BookCommand, BookParams, SelectDate};
use crate::models::TelegramName;
use crate::state::BotState;

pub async fn book<Cmd: BookCommand + CallbackBitcode + 'static>(
    bot: &Bot,
    chat_id: ChatId,
    params: &str,
    state: &Arc<BotState>,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    message_id: Option<MessageId>,
    student_name: &TelegramName,
) -> Result<()> {
    let params: BookParams = params.parse()?;
    match params {
        BookParams::L0() => book_0(bot, chat_id, state, user_id, callback_storage, message_id, student_name).await,
        BookParams::L1(teacher) => {
            book_1(bot, chat_id, teacher, user_id, callback_storage, state, message_id).await
        }
        BookParams::L2(teacher, select) => {
            book_2(bot, chat_id, teacher, select, user_id, callback_storage, state, message_id)
                .await
        }
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
    message_id: Option<MessageId>,
    student_name: &TelegramName,
) -> Result<()> {
    let pairings = state.get_pairings_for_student(student_name).await;
    let teacher_filter: Vec<_> = pairings.iter().map(|p| p.teacher_telegram.clone()).collect();
    show_teacher_selection(
        bot,
        chat_id,
        message_id,
        "📅 Select a teacher to book a lesson:",
        user_id,
        callback_storage,
        state,
        |name| Cmd::book(BookParams::L1(name)),
        None,
        teacher_filter,
    )
    .await
}

async fn book_1<Cmd: BookCommand + CallbackBitcode + 'static>(
    bot: &Bot,
    chat_id: ChatId,
    teacher: TelegramName,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    state: &Arc<BotState>,
    message_id: Option<MessageId>,
) -> Result<()> {
    let now = Local::now();
    let select = SelectDate::YearMonth(now.year(), now.month());
    book_2(bot, chat_id, teacher, select, user_id, callback_storage, state, message_id).await
}

async fn book_2<Cmd: BookCommand + CallbackBitcode + 'static>(
    bot: &Bot,
    chat_id: ChatId,
    teacher: TelegramName,
    select: SelectDate,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    state: &Arc<BotState>,
    message_id: Option<MessageId>,
) -> Result<()> {
    match select {
        SelectDate::YearMonth(year, month) => {
            let t1 = teacher.clone();
            let t2 = teacher;
            show_date_selection(
                bot,
                chat_id,
                message_id,
                year,
                month,
                user_id,
                callback_storage,
                move |date| Cmd::book(BookParams::L2(t1.clone(), SelectDate::Date(date))),
                move |y, m| Cmd::book(BookParams::L2(t2.clone(), SelectDate::YearMonth(y, m))),
                Some(Cmd::book(BookParams::L0())),
            )
            .await
        }
        SelectDate::Date(date) => {
            let teacher_for_cmd = teacher.clone();
            let teacher_for_back = teacher.clone();
            show_slot_selection(
                bot,
                chat_id,
                message_id,
                &teacher,
                date,
                user_id,
                callback_storage,
                state,
                move |time| Cmd::book(BookParams::L3(teacher_for_cmd.clone(), date, time)),
                Some(Cmd::book(BookParams::L2(
                    teacher_for_back,
                    SelectDate::YearMonth(date.year(), date.month()),
                ))),
            )
            .await
        }
    }
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
