use std::sync::Arc;

use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate, NaiveTime};
use humantime::Duration;
use telluride::command::{CallbackBitcode, CallbackKey};
use telluride::data_store::InMemStore;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId, UserId};

use crate::api::common::format_duration;
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
        BookParams::L3(teacher, date, hour) => {
            book_3(bot, chat_id, teacher, date, hour, state, student_name).await
        }
        BookParams::L4(teacher, date, hour, duration) => {
            book_4(bot, chat_id, teacher, date, hour, duration, state, student_name).await
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
    state: &Arc<BotState>,
    student_name: &TelegramName,
) -> Result<()> {
    let Some(pairing) = state.get_pairing(student_name, &teacher).await else {
        let text = markdown_string!(
            "⚠️ No pairing found for this teacher\\. Please contact your teacher\\."
        );
        bot.send_markdown_message(chat_id, text).await?;
        return Ok(());
    };

    let duration =
        Duration::from(std::time::Duration::from_secs(pairing.duration_minutes * 60));

    let mut text = markdown_string!("📅 *Book a Lesson*\n\n");
    let teacher_str = teacher.as_str();
    text.push(&markdown_format!("👨\\-🏫 Teacher: {}\n", teacher_str));
    let date_str = date.to_string();
    text.push(&markdown_format!("📆 Date: {}\n", date_str));
    let hour_str = hour.format("%H:%M").to_string();
    text.push(&markdown_format!("⏰ Time: {}\n", hour_str));
    let dur_str = format_duration(pairing.duration_minutes as i64);
    text.push(&markdown_format!("⏱ Duration: {}\n", dur_str));
    let cost_str = pairing.cost.to_string();
    text.push(&markdown_format!("💰 Cost: {}\n", cost_str));

    let l4_params = BookParams::L4(teacher, date, hour, duration).to_string();
    let button =
        InlineKeyboardButton::switch_inline_query_current_chat("✏️ Edit & Book", l4_params);
    let keyboard = InlineKeyboardMarkup::new(vec![vec![button]]);

    bot.send_markdown_message(chat_id, text)
        .reply_markup(keyboard)
        .await?;
    Ok(())
}

async fn book_4(
    bot: &Bot,
    chat_id: ChatId,
    teacher: TelegramName,
    date: NaiveDate,
    hour: NaiveTime,
    duration: Duration,
    state: &Arc<BotState>,
    student_name: &TelegramName,
) -> Result<()> {
    let Some(pairing) = state.get_pairing(student_name, &teacher).await else {
        let text = markdown_string!(
            "⚠️ Cannot book: you are not paired with this teacher\\."
        );
        bot.send_markdown_message(chat_id, text).await?;
        return Ok(());
    };

    let new_start = date.and_time(hour).and_utc();
    let duration_secs = duration.as_secs() as i64;
    let new_end = new_start + chrono::Duration::seconds(duration_secs);

    let (all_entries, _) = state.sheets.get_schedule().await?;
    let has_overlap = all_entries
        .iter()
        .filter(|e| e.is_planned())
        .filter(|e| e.student_telegram == *student_name || e.teacher_telegram == teacher)
        .any(|e| {
            let existing_start = e.datetime;
            let existing_end =
                existing_start + chrono::Duration::minutes(e.duration_minutes as i64);
            new_start < existing_end && existing_start < new_end
        });

    if has_overlap {
        let text = markdown_string!(
            "⚠️ Cannot book: this time slot conflicts with an existing lesson\\."
        );
        bot.send_markdown_message(chat_id, text).await?;
        return Ok(());
    }

    let duration_minutes = duration.as_secs() / 60;
    state
        .sheets
        .add_schedule_entry(student_name, &teacher, new_start, duration_minutes, pairing.cost)
        .await?;

    let mut text = markdown_string!("✅ *Lesson Booked\\!*\n\n");
    let teacher_str = teacher.as_str();
    text.push(&markdown_format!("👨\\-🏫 Teacher: {}\n", teacher_str));
    let date_str = date.to_string();
    text.push(&markdown_format!("📆 Date: {}\n", date_str));
    let hour_str = hour.format("%H:%M").to_string();
    text.push(&markdown_format!("⏰ Time: {}\n", hour_str));
    let dur_str = format_duration(duration_minutes as i64);
    text.push(&markdown_format!("⏱ Duration: {}\n", dur_str));
    let cost_str = pairing.cost.to_string();
    text.push(&markdown_format!("💰 Cost: {}\n", cost_str));

    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}
