use std::sync::Arc;

use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate, NaiveTime};
use crate::types::Duration;
use telluride::command::{CallbackBitcode, CallbackKey};
use telluride::data_store::InMemStore;
use telluride::markdown::MarkdownStringMessage;
use telluride::{markdown_format, markdown_string};
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId, UserId};

use crate::api::common::format_duration;
use crate::api::menus::{
    show_date_selection, show_month_selection, show_name_list, show_slot_selection,
    show_year_selection,
};
use crate::sheets::worktime::worktime_periods;
use crate::api::traits::{BookCommand, BookParams, BookingActor};
use crate::models::{TelegramName, TimePeriod};
use crate::state::BotState;

pub async fn book<Cmd: BookCommand + CallbackBitcode + 'static>(
    bot: &Bot,
    chat_id: ChatId,
    params: &str,
    state: &Arc<BotState>,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    message_id: Option<MessageId>,
    actor: &BookingActor,
) -> Result<()> {
    let params: BookParams = params.parse()?;
    match params {
        BookParams::L0() => {
            book_0(bot, chat_id, state, user_id, callback_storage, message_id, actor).await
        }
        BookParams::L1(teacher) => {
            book_1(bot, chat_id, teacher, user_id, callback_storage, state, message_id, actor).await
        }
        BookParams::L2(teacher, student) => {
            book_2(bot, chat_id, teacher, student, user_id, callback_storage, message_id).await
        }
        BookParams::L3(teacher, student, year) => {
            book_3(bot, chat_id, teacher, student, year, user_id, callback_storage, message_id).await
        }
        BookParams::L4(teacher, student, year, month) => {
            book_4(bot, chat_id, teacher, student, year, month, user_id, callback_storage, message_id).await
        }
        BookParams::L5(teacher, student, year, month, day) => {
            book_5(bot, chat_id, teacher, student, year, month, day, user_id, callback_storage, message_id).await
        }
        BookParams::L6(teacher, student, date) => {
            book_6(bot, chat_id, teacher, student, date, user_id, callback_storage, state, message_id).await
        }
        BookParams::L7(teacher, student, date, hour) => {
            book_7(bot, chat_id, teacher, student, date, hour, state, actor).await
        }
        BookParams::L8(teacher, student, date, hour, duration) => {
            book_8(bot, chat_id, teacher, student, date, hour, duration, state, actor).await
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
    actor: &BookingActor,
) -> Result<()> {
    match actor {
        BookingActor::Student(student) => {
            let pairings = state.get_pairings_for_student(student).await;
            if pairings.len() == 1 {
                return book_1(
                    bot, chat_id, pairings[0].teacher_telegram.clone(),
                    user_id, callback_storage, state, message_id, actor,
                ).await;
            }
            let names = pairings.into_iter().map(|p| p.teacher_telegram).collect();
            show_name_list(
                bot, chat_id, message_id,
                markdown_string!("📅 Select a teacher to book a lesson:"),
                markdown_string!("No paired teachers found\\."),
                user_id, callback_storage,
                names,
                |t| Cmd::book(BookParams::L1(t)),
                None,
            ).await
        }
        BookingActor::Teacher(teacher) => {
            book_1(bot, chat_id, teacher.clone(), user_id, callback_storage, state, message_id, actor).await
        }
    }
}

async fn book_1<Cmd: BookCommand + CallbackBitcode + 'static>(
    bot: &Bot,
    chat_id: ChatId,
    teacher: TelegramName,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    state: &Arc<BotState>,
    message_id: Option<MessageId>,
    actor: &BookingActor,
) -> Result<()> {
    match actor {
        BookingActor::Student(student) => {
            book_2(bot, chat_id, teacher, student.clone(), user_id, callback_storage, message_id).await
        }
        BookingActor::Teacher(teacher_actor) => {
            let pairings = state.get_pairings_for_teacher(teacher_actor).await;
            if pairings.len() == 1 {
                return book_2(
                    bot, chat_id, teacher, pairings[0].student_telegram.clone(),
                    user_id, callback_storage, message_id,
                ).await;
            }
            let t = teacher.clone();
            let names = pairings.into_iter().map(|p| p.student_telegram).collect();
            show_name_list(
                bot, chat_id, message_id,
                markdown_string!("📅 Select a student to book a lesson:"),
                markdown_string!("No paired students found\\."),
                user_id, callback_storage,
                names,
                move |s| Cmd::book(BookParams::L2(t.clone(), s)),
                None,
            ).await
        }
    }
}

async fn book_2<Cmd: BookCommand + CallbackBitcode + 'static>(
    bot: &Bot,
    chat_id: ChatId,
    teacher: TelegramName,
    student: TelegramName,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    message_id: Option<MessageId>,
) -> Result<()> {
    let now = Local::now();
    book_5(bot, chat_id, teacher, student, now.year(), now.month(), now.day(), user_id, callback_storage, message_id).await
}

async fn book_3<Cmd: BookCommand + CallbackBitcode + 'static>(
    bot: &Bot,
    chat_id: ChatId,
    teacher: TelegramName,
    student: TelegramName,
    year: i32,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    message_id: Option<MessageId>,
) -> Result<()> {
    show_year_selection(
        bot, chat_id, message_id, year, user_id, callback_storage,
        move |y| Cmd::book(BookParams::L5(teacher.clone(), student.clone(), y, 1, 1)),
    ).await
}

async fn book_4<Cmd: BookCommand + CallbackBitcode + 'static>(
    bot: &Bot,
    chat_id: ChatId,
    teacher: TelegramName,
    student: TelegramName,
    year: i32,
    _month: u32,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    message_id: Option<MessageId>,
) -> Result<()> {
    show_month_selection(
        bot, chat_id, message_id, year, user_id, callback_storage,
        move |m| Cmd::book(BookParams::L5(teacher.clone(), student.clone(), year, m, 1)),
    ).await
}

async fn book_5<Cmd: BookCommand + CallbackBitcode + 'static>(
    bot: &Bot,
    chat_id: ChatId,
    teacher: TelegramName,
    student: TelegramName,
    year: i32,
    month: u32,
    _day: u32,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    message_id: Option<MessageId>,
) -> Result<()> {
    let t_date = teacher.clone();
    let s_date = student.clone();
    let t_prev = teacher.clone();
    let s_prev = student.clone();
    let t_next = teacher.clone();
    let s_next = student.clone();
    let t_year = teacher.clone();
    let s_year = student.clone();
    let t_month = teacher.clone();
    let s_month = student.clone();
    let t_back = teacher.clone();
    let s_back = student.clone();
    let message = markdown_format!("📅 Book a Lesson\n{} ↔ {} — select a date:", teacher.to_string(), student.to_string());
    show_date_selection(
        bot, chat_id, message_id, message, year, month, user_id, callback_storage,
        move |date| Cmd::book(BookParams::L6(t_date.clone(), s_date.clone(), date)),
        move |py, pm| Cmd::book(BookParams::L5(t_prev.clone(), s_prev.clone(), py, pm, 1)),
        move |ny, nm| Cmd::book(BookParams::L5(t_next.clone(), s_next.clone(), ny, nm, 1)),
        move || Cmd::book(BookParams::L3(t_year.clone(), s_year.clone(), year)),
        move || Cmd::book(BookParams::L4(t_month.clone(), s_month.clone(), year, month)),
        Some(Cmd::book(BookParams::L2(t_back, s_back))),
    ).await
}

async fn book_6<Cmd: BookCommand + CallbackBitcode + 'static>(
    bot: &Bot,
    chat_id: ChatId,
    teacher: TelegramName,
    student: TelegramName,
    date: NaiveDate,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    state: &Arc<BotState>,
    message_id: Option<MessageId>,
) -> Result<()> {
    let t_cmd = teacher.clone();
    let s_cmd = student.clone();
    let t_back = teacher.clone();
    let s_back = student.clone();
    show_slot_selection(
        bot, chat_id, message_id,
        &teacher, &student, date, user_id, callback_storage, state,
        move |time| Cmd::book(BookParams::L7(t_cmd.clone(), s_cmd.clone(), date, time)),
        Some(Cmd::book(BookParams::L5(
            t_back, s_back,
            date.year(), date.month(), date.day(),
        ))),
    ).await
}

async fn book_7(
    bot: &Bot,
    chat_id: ChatId,
    teacher: TelegramName,
    student: TelegramName,
    date: NaiveDate,
    hour: NaiveTime,
    state: &Arc<BotState>,
    _actor: &BookingActor,
) -> Result<()> {
    let Some(pairing) = state.get_pairing(&student, &teacher).await else {
        let text = markdown_string!(
            "⚠️ No pairing found for this teacher\\. Please contact your teacher\\."
        );
        bot.send_markdown_message(chat_id, text).await?;
        return Ok(());
    };

    let duration = Duration::from(std::time::Duration::from_secs(pairing.duration_minutes * 60));

    let student_data = state.get_student(&student).await;
    let currency = student_data.as_ref().map(|s| s.currency.as_str()).unwrap_or("");

    let mut text = markdown_string!("📅 *Book a Lesson*\n\n");
    let teacher_str = teacher.to_string();
    text.push(&markdown_format!("👨\\-🏫 Teacher: {}\n", teacher_str));
    let student_str = student.to_string();
    text.push(&markdown_format!("👨\\-🎓 Student: {}\n", student_str));
    let date_str = date.to_string();
    text.push(&markdown_format!("📆 Date: {}\n", date_str));
    let hour_str = hour.format("%H:%M").to_string();
    text.push(&markdown_format!("⏰ Time: {}\n", hour_str));
    let dur_str = format_duration(pairing.duration_minutes as i64);
    text.push(&markdown_format!("⏱ Duration: {}\n", dur_str));
    let cost_str = if currency.is_empty() {
        pairing.cost.to_string()
    } else {
        format!("{} {}", pairing.cost, currency)
    };
    text.push(&markdown_format!("💰 Cost: {}\n", cost_str));

    let l8_params = format!("/book {}", BookParams::L8(teacher, student, date, hour, duration));
    let button = InlineKeyboardButton::switch_inline_query_current_chat("📅 Book", l8_params);
    let keyboard = InlineKeyboardMarkup::new(vec![vec![button]]);

    bot.send_markdown_message(chat_id, text)
        .reply_markup(keyboard)
        .await?;
    Ok(())
}

async fn book_8(
    bot: &Bot,
    chat_id: ChatId,
    teacher: TelegramName,
    student: TelegramName,
    date: NaiveDate,
    hour: NaiveTime,
    duration: Duration,
    state: &Arc<BotState>,
    actor: &BookingActor,
) -> Result<()> {
    match actor {
        BookingActor::Student(s) if s != &student => {
            let text = markdown_string!("⚠️ You can only book as yourself\\.");
            bot.send_markdown_message(chat_id, text).await?;
            return Ok(());
        }
        BookingActor::Teacher(t) if t != &teacher => {
            let text = markdown_string!("⚠️ You can only book for your own students\\.");
            bot.send_markdown_message(chat_id, text).await?;
            return Ok(());
        }
        _ => {}
    }

    let Some(pairing) = state.get_pairing(&student, &teacher).await else {
        let text = markdown_string!("⚠️ Cannot book: you are not paired with this teacher\\.");
        bot.send_markdown_message(chat_id, text).await?;
        return Ok(());
    };

    let new_start = date.and_time(hour).and_utc();
    let new_period = TimePeriod::new(new_start, chrono::Duration::seconds(duration.as_secs() as i64));

    let (worktime_entries, _) = state.sheets.get_worktime().await?;
    let fits_in_worktime = worktime_periods(&worktime_entries, &teacher, date)
        .iter()
        .any(|wp| wp.contains(&new_period));
    if !fits_in_worktime {
        let text = markdown_string!(
            "⚠️ Cannot book: the lesson extends outside the teacher's working hours\\."
        );
        bot.send_markdown_message(chat_id, text).await?;
        return Ok(());
    }

    let (all_entries, _) = state.sheets.get_schedule().await?;
    let has_overlap = all_entries
        .iter()
        .filter(|e| e.is_planned())
        .filter(|e| e.student_telegram == student || e.teacher_telegram == teacher)
        .any(|e| e.time_period().overlaps(&new_period));

    if has_overlap {
        let text = markdown_string!(
            "⚠️ Cannot book: this time slot conflicts with an existing lesson\\."
        );
        bot.send_markdown_message(chat_id, text).await?;
        return Ok(());
    }

    let duration_minutes = duration.as_secs() / 60;
    let actual_cost = if pairing.duration_minutes > 0 {
        pairing.cost * duration_minutes as i64 / pairing.duration_minutes as i64
    } else {
        pairing.cost
    };

    let student_data = state.get_student(&student).await;
    let currency = student_data.as_ref().map(|s| s.currency.as_str()).unwrap_or("");

    state
        .sheets
        .add_schedule_entry(&student, &teacher, new_start, duration_minutes, actual_cost)
        .await?;

    let mut text = markdown_string!("✅ *Lesson Booked\\!*\n\n");
    let teacher_str = teacher.to_string();
    text.push(&markdown_format!("👨\\-🏫 Teacher: {}\n", teacher_str));
    let student_str = student.to_string();
    text.push(&markdown_format!("👨\\-🎓 Student: {}\n", student_str));
    let date_str = date.to_string();
    text.push(&markdown_format!("📆 Date: {}\n", date_str));
    let hour_str = hour.format("%H:%M").to_string();
    text.push(&markdown_format!("⏰ Time: {}\n", hour_str));
    let dur_str = format_duration(duration_minutes as i64);
    text.push(&markdown_format!("⏱ Duration: {}\n", dur_str));
    let cost_str = if currency.is_empty() {
        actual_cost.to_string()
    } else {
        format!("{} {}", actual_cost, currency)
    };
    text.push(&markdown_format!("💰 Cost: {}\n", cost_str));

    bot.send_markdown_message(chat_id, text).await?;
    Ok(())
}
