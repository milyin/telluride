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

pub async fn show_teacher_selection<Cmd, F>(
    bot: &Bot,
    chat_id: ChatId,
    message_id: Option<MessageId>,
    message: &str,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    state: &Arc<BotState>,
    make_cmd: F,
) -> Result<()>
where
    Cmd: CallbackBitcode + 'static,
    F: Fn(TelegramName) -> Cmd,
{
    let teacher_names = state.get_teacher_names().await;

    if teacher_names.is_empty() {
        bot.send_message(chat_id, "No teachers are registered in the spreadsheet yet.")
            .await?;
        return Ok(());
    }

    let user_proxy = UserProxy::new(callback_storage, user_id);

    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    for name in teacher_names {
        let label = format!("@{}", name);
        let cmd = make_cmd(name.parse()?);
        let key = CallbackKey::pack(cmd, &user_proxy).await;
        let button = InlineKeyboardButton::callback_key(label, &key);
        buttons.push(vec![button]);
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

pub async fn show_date_selection<Cmd, F, G>(
    bot: &Bot,
    chat_id: ChatId,
    message_id: Option<MessageId>,
    year: i32,
    month: u32,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    make_date_cmd: F,
    make_month_nav: G,
) -> Result<()>
where
    Cmd: CallbackBitcode + 'static,
    F: Fn(NaiveDate) -> Cmd,
    G: Fn(i32, u32) -> Cmd,
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
    let prev_key = CallbackKey::pack(make_month_nav(prev_year, prev_month), &user_proxy).await;
    let next_key = CallbackKey::pack(make_month_nav(next_year, next_month), &user_proxy).await;
    let prev_btn = InlineKeyboardButton::callback_key("<", &prev_key);
    let next_btn = InlineKeyboardButton::callback_key(">", &next_key);

    let keyboard = build_month_calendar(
        year,
        month,
        |date| {
            let day = date.day();
            let label = if date == today {
                format!("[{}]", day)
            } else {
                format!("{}", day)
            };
            InlineKeyboardButton::callback_key(label, &date_keys[(day - 1) as usize])
        },
        |leading, trailing| {
            let total = leading + trailing;
            let mut all: Vec<InlineKeyboardButton> =
                (0..total).map(|_| InlineKeyboardButton::callback(" ", "noop")).collect();
            if total > 0 {
                all[0] = prev_btn.clone();
                all[total - 1] = next_btn.clone();
            }
            let trailing_btns = all.split_off(leading);
            (all, trailing_btns)
        },
    );

    let month_name = NaiveDate::from_ymd_opt(year, month, 1)
        .unwrap()
        .format("%B %Y")
        .to_string();
    let text = month_name;

    match message_id {
        Some(id) => {
            bot.edit_message_text(chat_id, id, text)
                .reply_markup(keyboard)
                .await?;
        }
        None => {
            bot.send_message(chat_id, text)
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
    date: NaiveDate,
    user_id: UserId,
    callback_storage: Arc<InMemStore<CallbackKey, Cmd>>,
    state: &Arc<BotState>,
    make_cmd: F,
) -> Result<()>
where
    Cmd: CallbackBitcode + 'static,
    F: Fn(NaiveTime) -> Cmd,
{
    let (worktime, _) = state.sheets.get_worktime().await?;
    let slots = available_slots(&worktime, teacher, date);

    let message = format!("@{} on {} — select a time slot:", teacher, date);

    if slots.is_empty() {
        let no_slots_msg = format!("{}\nNo available slots.", message);
        match message_id {
            Some(id) => {
                bot.edit_message_text(chat_id, id, no_slots_msg).await?;
            }
            None => {
                bot.send_message(chat_id, no_slots_msg).await?;
            }
        }
        return Ok(());
    }

    let user_proxy = UserProxy::new(callback_storage, user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    for slot in slots {
        let label = format!("{:02}:00 – {:02}:00", slot.hour(), slot.hour() + 1);
        let cmd = make_cmd(slot);
        let key = CallbackKey::pack(cmd, &user_proxy).await;
        let button = InlineKeyboardButton::callback_key(label, &key);
        buttons.push(vec![button]);
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
