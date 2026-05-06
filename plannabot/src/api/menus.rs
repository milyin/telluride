use std::sync::Arc;

use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate, NaiveTime, Timelike};
use telluride::calendar::build_month_calendar;
use telluride::command::{CallbackBitcode, CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::{InMemStore, UserProxy};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId, UserId};

use crate::models::TelegramName;
use crate::sheets::worktime::available_slots;
use crate::state::BotState;

/// Core list-of-names keyboard: builds inline buttons for each name and sends or edits a message.
/// Callers supply pre-filtered names; empty-list handling uses `empty_msg`.
pub(crate) async fn show_name_list<Cmd, F>(
    bot: &Bot,
    chat_id: ChatId,
    message_id: Option<MessageId>,
    message: &str,
    empty_msg: &str,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    names: Vec<TelegramName>,
    make_cmd: F,
    back: Option<Cmd>,
) -> Result<()>
where
    Cmd: CallbackBitcode + 'static,
    F: Fn(TelegramName) -> Cmd,
{
    if names.is_empty() {
        bot.send_message(chat_id, empty_msg).await?;
        return Ok(());
    }

    let user_proxy = UserProxy::new(callback_storage, user_id);

    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    for name in names {
        let label = name.to_string();
        let cmd = make_cmd(name);
        let key = CallbackKey::pack(cmd, &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    if let Some(back_cmd) = back {
        let key = CallbackKey::pack(back_cmd, &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key("↩ Back", &key)]);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    match message_id {
        Some(id) => {
            bot.edit_message_text(chat_id, id, message)
                .reply_markup(keyboard)
                .await?;
        }
        None => {
            bot.send_message(chat_id, message)
                .reply_markup(keyboard)
                .await?;
        }
    }

    Ok(())
}


pub async fn show_year_selection<Cmd, F>(
    bot: &Bot,
    chat_id: ChatId,
    message_id: Option<MessageId>,
    current_year: i32,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    make_cmd: F,
) -> Result<()>
where
    Cmd: CallbackBitcode + 'static,
    F: Fn(i32) -> Cmd,
{
    let user_proxy = UserProxy::new(callback_storage, user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for year in (current_year - 1)..=(current_year + 3) {
        let label = year.to_string();
        let key = CallbackKey::pack(make_cmd(year), &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    let text = "Select a year:";
    match message_id {
        Some(id) => {
            bot.edit_message_text(chat_id, id, text).reply_markup(keyboard).await?;
        }
        None => {
            bot.send_message(chat_id, text).reply_markup(keyboard).await?;
        }
    }
    Ok(())
}

pub async fn show_month_selection<Cmd, F>(
    bot: &Bot,
    chat_id: ChatId,
    message_id: Option<MessageId>,
    year: i32,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    make_cmd: F,
) -> Result<()>
where
    Cmd: CallbackBitcode + 'static,
    F: Fn(u32) -> Cmd,
{
    const MONTH_NAMES: [&str; 12] = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ];

    let user_proxy = UserProxy::new(callback_storage, user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for (i, name) in MONTH_NAMES.iter().enumerate() {
        let month = (i + 1) as u32;
        let key = CallbackKey::pack(make_cmd(month), &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key(*name, &key)]);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    let text = format!("Select a month for {}:", year);
    match message_id {
        Some(id) => {
            bot.edit_message_text(chat_id, id, &text).reply_markup(keyboard).await?;
        }
        None => {
            bot.send_message(chat_id, &text).reply_markup(keyboard).await?;
        }
    }
    Ok(())
}

pub async fn show_date_selection<Cmd, FDate, FPrev, FNext, FYear, FMonth>(
    bot: &Bot,
    chat_id: ChatId,
    message_id: Option<MessageId>,
    message: &str,
    year: i32,
    month: u32,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    make_date_cmd: FDate,
    make_prev_month: FPrev,
    make_next_month: FNext,
    make_year_cmd: FYear,
    make_month_cmd: FMonth,
    back: Option<Cmd>,
) -> Result<()>
where
    Cmd: CallbackBitcode + 'static,
    FDate: Fn(NaiveDate) -> Cmd,
    FPrev: FnOnce(i32, u32) -> Cmd,
    FNext: FnOnce(i32, u32) -> Cmd,
    FYear: FnOnce() -> Cmd,
    FMonth: FnOnce() -> Cmd,
{
    let user_proxy = UserProxy::new(callback_storage, user_id);

    let num_days = {
        let next_first = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .unwrap();
        next_first.pred_opt().unwrap().day()
    };

    let today = Local::now().date_naive();

    let mut date_keys: Vec<CallbackKey> = Vec::with_capacity(num_days as usize);
    for day in 1..=num_days {
        let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        let key = CallbackKey::pack(make_date_cmd(date), &user_proxy).await;
        date_keys.push(key);
    }

    let (prev_year, prev_month) = if month == 1 { (year - 1, 12u32) } else { (year, month - 1) };
    let (next_year, next_month) = if month == 12 { (year + 1, 1u32) } else { (year, month + 1) };

    let prev_key = CallbackKey::pack(make_prev_month(prev_year, prev_month), &user_proxy).await;
    let next_key = CallbackKey::pack(make_next_month(next_year, next_month), &user_proxy).await;
    let year_key = CallbackKey::pack(make_year_cmd(), &user_proxy).await;
    let month_key = CallbackKey::pack(make_month_cmd(), &user_proxy).await;

    let prev_btn = InlineKeyboardButton::callback_key("<", &prev_key);
    let next_btn = InlineKeyboardButton::callback_key(">", &next_key);
    let year_btn = InlineKeyboardButton::callback_key(year.to_string(), &year_key);
    let month_btn = InlineKeyboardButton::callback_key(
        NaiveDate::from_ymd_opt(year, month, 1).unwrap().format("%B").to_string(),
        &month_key,
    );

    let mut keyboard = build_month_calendar(
        year,
        month,
        |date| {
            let day = date.day();
            let label = if date == today { format!("[{}]", day) } else { format!("{}", day) };
            InlineKeyboardButton::callback_key(label, &date_keys[(day - 1) as usize])
        },
    );

    // Insert navigation row (<, year, month, >) above the calendar
    keyboard.inline_keyboard.insert(0, vec![prev_btn, year_btn, month_btn, next_btn]);

    if let Some(back_cmd) = back {
        let key = CallbackKey::pack(back_cmd, &user_proxy).await;
        keyboard.inline_keyboard.push(vec![InlineKeyboardButton::callback_key("↩ Back", &key)]);
    }

    match message_id {
        Some(id) => {
            bot.edit_message_text(chat_id, id, message)
                .reply_markup(keyboard)
                .await?;
        }
        None => {
            bot.send_message(chat_id, message)
                .reply_markup(keyboard)
                .await?;
        }
    }

    Ok(())
}

pub async fn show_slot_selection<Cmd, F>(
    bot: &Bot,
    chat_id: ChatId,
    message_id: Option<MessageId>,
    teacher: &TelegramName,
    student: &TelegramName,
    date: NaiveDate,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    state: &Arc<BotState>,
    make_cmd: F,
    back: Option<Cmd>,
) -> Result<()>
where
    Cmd: CallbackBitcode + 'static,
    F: Fn(NaiveTime) -> Cmd,
{
    let Some(pairing) = state.get_pairing(student, teacher).await else {
        let message = format!("{} on {} — no available slots.", teacher, date);
        match message_id {
            Some(id) => { bot.edit_message_text(chat_id, id, message).await?; }
            None => { bot.send_message(chat_id, message).await?; }
        }
        return Ok(());
    };
    let lesson_duration = chrono::Duration::minutes(pairing.duration_minutes as i64);

    let (worktime, _) = state.sheets.get_worktime().await?;
    let (schedule, _) = state.sheets.get_schedule().await?;
    let slots = available_slots(&worktime, &schedule, teacher, student, date, lesson_duration);

    let message = if slots.is_empty() {
        format!("{} on {} — no available slots.", teacher, date)
    } else {
        format!("{} on {} — select a time slot:", teacher, date)
    };

    let user_proxy = UserProxy::new(callback_storage, user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for slot in slots {
        let end_time = slot + lesson_duration;
        let label = format!("{:02}:{:02} – {:02}:{:02}", slot.hour(), slot.minute(), end_time.hour(), end_time.minute());
        let cmd = make_cmd(slot);
        let key = CallbackKey::pack(cmd, &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    if let Some(back_cmd) = back {
        let key = CallbackKey::pack(back_cmd, &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key("↩ Back", &key)]);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    match message_id {
        Some(id) => {
            bot.edit_message_text(chat_id, id, message)
                .reply_markup(keyboard)
                .await?;
        }
        None => {
            bot.send_message(chat_id, message)
                .reply_markup(keyboard)
                .await?;
        }
    }

    Ok(())
}
