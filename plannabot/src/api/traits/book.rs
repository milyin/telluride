use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use chrono::{NaiveDate, NaiveTime};
use telluride::utils::{screen_spaces, split_with_screened_spaces};

use crate::models::TelegramName;
use crate::types::Duration;

pub enum BookParams {
    L0(),
    L1(TelegramName),
    L2(TelegramName, TelegramName),
    L3(TelegramName, TelegramName, i32),
    L4(TelegramName, TelegramName, i32, u32),
    L5(TelegramName, TelegramName, i32, u32, u32),
    L6(TelegramName, TelegramName, NaiveDate),
    L7(TelegramName, TelegramName, NaiveDate, NaiveTime),
    L8(TelegramName, TelegramName, NaiveDate, NaiveTime, Duration),
}

pub enum BookingActor {
    Student(TelegramName),
    Teacher(TelegramName),
}

impl fmt::Display for BookParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BookParams::L0() => Ok(()),
            BookParams::L1(t) => write!(f, "{}", screen_spaces(t.as_str())),
            BookParams::L2(t, s) => write!(f, "{} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str())),
            BookParams::L3(t, s, y) => write!(f, "{} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), y),
            BookParams::L4(t, s, y, m) => write!(f, "{} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), y, m),
            BookParams::L5(t, s, y, m, d) => write!(f, "{} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), y, m, d),
            BookParams::L6(t, s, date) => write!(f, "{} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), screen_spaces(&date.to_string())),
            BookParams::L7(t, s, date, time) => write!(f, "{} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), screen_spaces(&date.to_string()), screen_spaces(&time.to_string())),
            BookParams::L8(t, s, date, time, dur) => write!(f, "{} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), screen_spaces(&date.to_string()), screen_spaces(&time.to_string()), screen_spaces(&dur.to_string())),
        }
    }
}

impl FromStr for BookParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);
        let Some(first_str) = parts.get(0) else {
            return Ok(BookParams::L0());
        };
        let teacher: TelegramName = first_str.parse()?;

        let Some(second_str) = parts.get(1) else {
            return Ok(BookParams::L1(teacher));
        };

        // If the second token starts with a letter it is a TelegramName (student).
        // Digits → year from an old-format callback; dashes → date — both are rejected.
        let second_first = second_str.chars().next().unwrap_or('0');
        if !second_first.is_alphabetic() {
            return Err(anyhow::anyhow!(
                "unexpected second parameter '{}': expected student TelegramName",
                second_str
            ));
        }
        let student: TelegramName = second_str.parse()?;

        let Some(third_str) = parts.get(2) else {
            return Ok(BookParams::L2(teacher, student));
        };

        // Distinguish by dashes: NaiveDate "YYYY-MM-DD" has 2 dashes; a plain year has none.
        let third_dash_count = third_str.chars().filter(|c| *c == '-').count();
        if third_dash_count >= 2 {
            let date: NaiveDate = third_str.parse()?;
            let Some(time_str) = parts.get(3) else {
                return Ok(BookParams::L6(teacher, student, date));
            };
            let time: NaiveTime = time_str.parse()?;
            let Some(duration_str) = parts.get(4) else {
                return Ok(BookParams::L7(teacher, student, date, time));
            };
            let duration: Duration = duration_str.parse()?;
            if let Some(extra) = parts.get(5) {
                return Err(anyhow::anyhow!("extra parameter: {}", extra));
            }
            return Ok(BookParams::L8(teacher, student, date, time, duration));
        }

        // Year branch
        let year: i32 = third_str.parse()?;
        let Some(month_str) = parts.get(3) else {
            return Ok(BookParams::L3(teacher, student, year));
        };
        let month: u32 = month_str.parse()?;
        let Some(day_str) = parts.get(4) else {
            return Ok(BookParams::L4(teacher, student, year, month));
        };
        let day: u32 = day_str.parse()?;
        if let Some(extra) = parts.get(5) {
            return Err(anyhow::anyhow!("extra parameter: {}", extra));
        }
        Ok(BookParams::L5(teacher, student, year, month, day))
    }
}
