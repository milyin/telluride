use chrono::{Datelike, NaiveDate};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

const NOOP: &str = "noop";
const WEEKDAY_LABELS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

fn noop_button(label: &str) -> InlineKeyboardButton {
    InlineKeyboardButton::callback(label, NOOP)
}

/// Builds an inline keyboard calendar for the given month.
///
/// The first row is a day-of-week header (Mon–Sun). Each subsequent row
/// contains exactly 7 buttons, one per calendar cell. Cells before the
/// first day and after the last day are filled with dummy no-op buttons
/// so every button appears the same width.
///
/// `day_button_fn` is called for each valid day and must return the
/// `InlineKeyboardButton` to display for that date.
///
/// `padding_fn`, if provided, is called with `(leading_padding_len,
/// trailing_padding_len)` and returns `(leading_buttons, trailing_buttons)`.
/// Each vector is truncated or padded with noop buttons to fit exactly.
pub fn build_month_calendar<F, P>(
    year: i32,
    month: u32,
    day_button_fn: F,
    padding_fn: P,
) -> InlineKeyboardMarkup
where
    F: Fn(NaiveDate) -> InlineKeyboardButton,
    P: Fn(usize, usize) -> (Vec<InlineKeyboardButton>, Vec<InlineKeyboardButton>),
{
    let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    rows.push(WEEKDAY_LABELS.iter().map(|d| noop_button(d)).collect());

    let first_day = NaiveDate::from_ymd_opt(year, month, 1).expect("invalid year/month");
    let leading = first_day.weekday().num_days_from_monday() as usize;
    let num_days = last_day_of_month(year, month).day();
    let trailing = (7 - (leading + num_days as usize) % 7) % 7;

    let (mut leading_buttons, mut trailing_buttons) = padding_fn(leading, trailing);

    leading_buttons.truncate(leading);
    while leading_buttons.len() < leading {
        leading_buttons.push(noop_button(" "));
    }

    trailing_buttons.truncate(trailing);
    while trailing_buttons.len() < trailing {
        trailing_buttons.push(noop_button(" "));
    }

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
