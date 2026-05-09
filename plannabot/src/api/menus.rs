use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate, NaiveTime, Timelike};
use std::collections::HashMap;
use telluride::calendar::build_month_calendar;
use telluride::command::{CallbackBitcode, CallbackKey, InlineKeyboardButtonPackedExt};
use telluride::data_store::UserProxy;
use telluride::markdown::MarkdownString;
use telluride::{markdown_format, markdown_string};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::api::common::PageInfo;
use crate::api::context::BotCtx;
use crate::models::TelegramName;
use crate::sheets::worktime::{available_slots, DayAvailability};

/// Page size used by [`show_annotated_list`].
pub const ANNOTATED_LIST_PAGE_SIZE: usize = 3;

/// Shows a paginated list where each item's detail appears in the message text
/// and buttons carry only the username (for navigation to a detail view).
///
/// `items` — all items as `(name, annotation)` pairs, pre-computed by the caller.
/// `make_item_cmd` — called with the clicked name to produce the detail command.
/// `make_page_cmd` — called with a page index to produce the prev/next command.
pub async fn show_annotated_list<Cmd, FItem, FPage>(
    ctx: &BotCtx<Cmd>,
    header: MarkdownString,
    empty_msg: MarkdownString,
    items: Vec<(TelegramName, MarkdownString)>,
    page: i32,
    make_item_cmd: FItem,
    make_page_cmd: FPage,
    back: Option<Cmd>,
) -> Result<()>
where
    Cmd: CallbackBitcode + 'static,
    FItem: Fn(TelegramName) -> Cmd,
    FPage: Fn(i32) -> Cmd,
{
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);

    if items.is_empty() {
        let keyboard = if let Some(back_cmd) = back {
            let key = CallbackKey::pack(back_cmd, &user_proxy).await;
            Some(InlineKeyboardMarkup::new(vec![vec![
                InlineKeyboardButton::callback_key("↩ Back", &key),
            ]]))
        } else {
            None
        };
        return ctx.update_markdown_message(empty_msg, keyboard).await;
    }

    let info = PageInfo::with_size(page, items.len(), ANNOTATED_LIST_PAGE_SIZE);
    let total_pages = (items.len() + ANNOTATED_LIST_PAGE_SIZE - 1) / ANNOTATED_LIST_PAGE_SIZE;

    let mut text = header;
    text.push(&markdown_format!(" \\(page {}/{}\\):\n", info.page + 1, total_pages));
    for (name, annotation) in &items[info.start..info.end] {
        let mut line = markdown_format!("\n• {}: ", name.to_string());
        line.push(annotation);
        text.push(&line);
    }

    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    for (name, _) in &items[info.start..info.end] {
        let key = CallbackKey::pack(make_item_cmd(name.clone()), &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key(name.to_string(), &key)]);
    }

    let mut nav_row: Vec<InlineKeyboardButton> = Vec::new();
    if info.has_prev() {
        let k = CallbackKey::pack(make_page_cmd(info.page - 1), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("◀", &k));
    }
    if info.has_next() {
        let k = CallbackKey::pack(make_page_cmd(info.page + 1), &user_proxy).await;
        nav_row.push(InlineKeyboardButton::callback_key("▶", &k));
    }
    if !nav_row.is_empty() {
        buttons.push(nav_row);
    }
    if let Some(back_cmd) = back {
        let key = CallbackKey::pack(back_cmd, &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key("↩ Back", &key)]);
    }

    ctx.update_markdown_message(text, Some(InlineKeyboardMarkup::new(buttons))).await
}

pub(crate) async fn show_name_list<Cmd, F>(
    ctx: &BotCtx<Cmd>,
    message: MarkdownString,
    empty_msg: MarkdownString,
    names: Vec<TelegramName>,
    make_cmd: F,
    back: Option<Cmd>,
) -> Result<()>
where
    Cmd: CallbackBitcode + 'static,
    F: Fn(TelegramName) -> Cmd,
{
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);

    if names.is_empty() {
        let keyboard = if let Some(back_cmd) = back {
            let key = CallbackKey::pack(back_cmd, &user_proxy).await;
            Some(InlineKeyboardMarkup::new(vec![vec![
                InlineKeyboardButton::callback_key("↩ Back", &key),
            ]]))
        } else {
            None
        };
        ctx.update_markdown_message(empty_msg, keyboard).await?;
        return Ok(());
    }

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
    ctx.update_markdown_message(message, Some(keyboard)).await
}

pub async fn show_year_selection<Cmd, F>(
    ctx: &BotCtx<Cmd>,
    message: MarkdownString,
    current_year: i32,
    make_cmd: F,
) -> Result<()>
where
    Cmd: CallbackBitcode + 'static,
    F: Fn(i32) -> Cmd,
{
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for year in (current_year - 1)..=(current_year + 3) {
        let label = year.to_string();
        let key = CallbackKey::pack(make_cmd(year), &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key(label, &key)]);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    ctx.update_markdown_message(message, Some(keyboard)).await
}

pub async fn show_month_selection<Cmd, F>(
    ctx: &BotCtx<Cmd>,
    message: MarkdownString,
    _year: i32,
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

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for (i, name) in MONTH_NAMES.iter().enumerate() {
        let month = (i + 1) as u32;
        let key = CallbackKey::pack(make_cmd(month), &user_proxy).await;
        buttons.push(vec![InlineKeyboardButton::callback_key(*name, &key)]);
    }

    let keyboard = InlineKeyboardMarkup::new(buttons);
    ctx.update_markdown_message(message, Some(keyboard)).await
}

pub async fn show_date_selection<Cmd, FDate, FPrev, FNext, FYear, FMonth>(
    ctx: &BotCtx<Cmd>,
    message: MarkdownString,
    year: i32,
    month: u32,
    make_date_cmd: FDate,
    make_prev_month: FPrev,
    make_next_month: FNext,
    make_year_cmd: FYear,
    make_month_cmd: FMonth,
    back: Option<Cmd>,
    day_availability: &HashMap<NaiveDate, DayAvailability>,
) -> Result<()>
where
    Cmd: CallbackBitcode + 'static,
    FDate: Fn(NaiveDate) -> Cmd,
    FPrev: FnOnce(i32, u32) -> Cmd,
    FNext: FnOnce(i32, u32) -> Cmd,
    FYear: FnOnce() -> Cmd,
    FMonth: FnOnce() -> Cmd,
{
    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);

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
            let marker = match day_availability.get(&date) {
                Some(DayAvailability::Free)    => "🟢",
                Some(DayAvailability::Partial) => "🟡",
                Some(DayAvailability::Busy)    => "🔴",
                Some(DayAvailability::Marked)  => "📆",
                Some(DayAvailability::Planned) => "📅",
                Some(DayAvailability::Done)    => "✅",
                None => "",
            };
            let base = if date == today { format!("[{}]", day) } else { format!("{}", day) };
            let label = if marker.is_empty() { base } else { format!("{}{}", marker, base) };
            InlineKeyboardButton::callback_key(label, &date_keys[(day - 1) as usize])
        },
    );

    keyboard.inline_keyboard.insert(0, vec![prev_btn, year_btn, month_btn, next_btn]);

    if let Some(back_cmd) = back {
        let key = CallbackKey::pack(back_cmd, &user_proxy).await;
        keyboard.inline_keyboard.push(vec![InlineKeyboardButton::callback_key("↩ Back", &key)]);
    }

    ctx.update_markdown_message(message, Some(keyboard)).await
}

pub async fn show_slot_selection<Cmd, F>(
    ctx: &BotCtx<Cmd>,
    teacher: &TelegramName,
    student: &TelegramName,
    date: NaiveDate,
    header: MarkdownString,
    make_cmd: F,
    back: Option<Cmd>,
) -> Result<()>
where
    Cmd: CallbackBitcode + 'static,
    F: Fn(NaiveTime) -> Cmd,
{
    let Some(pairing) = ctx.state.get_pairing(student, teacher).await else {
        let mut message = header;
        message.push(&markdown_string!("No available slots\\."));
        return ctx.update_markdown_message(message, None).await;
    };
    let lesson_duration = chrono::Duration::minutes(pairing.duration_minutes as i64);

    let (worktime, _) = ctx.state.sheets.get_worktime().await?;
    let (schedule, _) = ctx.state.sheets.get_schedule().await?;
    let slots = available_slots(&worktime, &schedule, teacher, student, date, lesson_duration);

    let mut message = header;
    if slots.is_empty() {
        message.push(&markdown_string!("No available slots\\."));
    } else {
        message.push(&markdown_string!("Select a time slot:"));
    }

    let user_proxy = UserProxy::new(ctx.callback_storage.clone(), ctx.user_id);
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
    ctx.update_markdown_message(message, Some(keyboard)).await
}
