use chrono::{Datelike, NaiveDate};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

const NOOP: &str = "noop";
const WEEKDAY_LABELS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

fn noop_button(label: &str) -> InlineKeyboardButton {
    InlineKeyboardButton::callback(label, NOOP)
}

/// Builds an inline keyboard calendar for the given month.
///
/// Each row contains exactly 7 buttons, one per calendar cell. Cells before
/// the first day and after the last day are filled with noop buttons labelled
/// with the corresponding weekday abbreviation (Mon, Tue, …).
///
/// `day_button_fn` is called for each valid day and must return the
/// `InlineKeyboardButton` to display for that date.
pub fn build_month_calendar<F>(
    year: i32,
    month: u32,
    day_button_fn: F,
) -> InlineKeyboardMarkup
where
    F: Fn(NaiveDate) -> InlineKeyboardButton,
{
    let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    let first_day = NaiveDate::from_ymd_opt(year, month, 1).expect("invalid year/month");
    let leading = first_day.weekday().num_days_from_monday() as usize;
    let num_days = last_day_of_month(year, month).day();
    let trailing = (7 - (leading + num_days as usize) % 7) % 7;

    let leading_buttons: Vec<InlineKeyboardButton> = (0..leading)
        .map(|i| noop_button(WEEKDAY_LABELS[i]))
        .collect();

    let first_trailing = (leading + num_days as usize) % 7;
    let trailing_buttons: Vec<InlineKeyboardButton> = (0..trailing)
        .map(|i| noop_button(WEEKDAY_LABELS[(first_trailing + i) % 7]))
        .collect();

    let mut row: Vec<InlineKeyboardButton> = leading_buttons;

    for day in 1..=num_days {
        let date = NaiveDate::from_ymd_opt(year, month, day).expect("invalid date");
        row.push(day_button_fn(date));
        if row.len() == 7 {
            rows.push(row);
            row = Vec::new();
        }
    }

    if trailing > 0 {
        row.extend(trailing_buttons);
        rows.push(row);
    } else if !row.is_empty() {
        rows.push(row);
    }

    InlineKeyboardMarkup::new(rows)
}

fn last_day_of_month(year: i32, month: u32) -> NaiveDate {
    let first_of_next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    first_of_next
        .expect("invalid year")
        .pred_opt()
        .expect("date underflow")
}
