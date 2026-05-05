use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use chrono::{NaiveDate, NaiveTime};
use humantime::Duration;
use telluride::utils::{screen_spaces, split_with_screened_spaces};

use crate::models::TelegramName;

pub enum BookParams {
    L0(),
    L1(TelegramName),
    L2(TelegramName, i32),                             // year selection
    L3(TelegramName, i32, u32),                         // month selection
    L4(TelegramName, i32, u32, u32),                    // calendar / date selection
    L5(TelegramName, NaiveDate),                        // slot selection for a date
    L6(TelegramName, NaiveDate, NaiveTime),             // booking preview
    L7(TelegramName, NaiveDate, NaiveTime, Duration),   // confirm booking
}

impl fmt::Display for BookParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BookParams::L0() => Ok(()),
            BookParams::L1(t) => write!(f, "{}", screen_spaces(t.as_str())),
            BookParams::L2(t, y) => write!(f, "{} {}", screen_spaces(t.as_str()), y),
            BookParams::L3(t, y, m) => write!(f, "{} {} {}", screen_spaces(t.as_str()), y, m),
            BookParams::L4(t, y, m, d) => {
                write!(f, "{} {} {} {}", screen_spaces(t.as_str()), y, m, d)
            }
            BookParams::L5(t, d) => {
                write!(f, "{} {}", screen_spaces(t.as_str()), screen_spaces(&d.to_string()))
            }
            BookParams::L6(t, d, h) => write!(
                f,
                "{} {} {}",
                screen_spaces(t.as_str()),
                screen_spaces(&d.to_string()),
                screen_spaces(&h.to_string())
            ),
            BookParams::L7(t, d, h, dur) => write!(
                f,
                "{} {} {} {}",
                screen_spaces(t.as_str()),
                screen_spaces(&d.to_string()),
                screen_spaces(&h.to_string()),
                screen_spaces(&dur.to_string())
            ),
        }
    }
}

impl FromStr for BookParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);
        let Some(teacher_str) = parts.get(0) else {
            return Ok(BookParams::L0());
        };
        let teacher: TelegramName = teacher_str.parse()?;

        let Some(second) = parts.get(1) else {
            return Ok(BookParams::L1(teacher));
        };

        // Distinguish by dashes: a NaiveDate "YYYY-MM-DD" has 2 dashes; a plain year has none.
        let second_dash_count = second.chars().filter(|c| *c == '-').count();
        if second_dash_count >= 2 {
            // L5, L6, or L7: second part is a NaiveDate
            let date: NaiveDate = second.parse()?;
            let Some(time_str) = parts.get(2) else {
                return Ok(BookParams::L5(teacher, date));
            };
            let time: NaiveTime = time_str.parse()?;
            let Some(duration_str) = parts.get(3) else {
                return Ok(BookParams::L6(teacher, date, time));
            };
            let duration: Duration = duration_str.parse()?;
            if let Some(extra) = parts.get(4) {
                return Err(anyhow::anyhow!("extra parameter: {}", extra));
            }
            return Ok(BookParams::L7(teacher, date, time, duration));
        }

        // L2, L3, or L4: second part is an integer year
        let year: i32 = second.parse()?;
        let Some(month_str) = parts.get(2) else {
            return Ok(BookParams::L2(teacher, year));
        };
        let month: u32 = month_str.parse()?;
        let Some(day_str) = parts.get(3) else {
            return Ok(BookParams::L3(teacher, year, month));
        };
        let day: u32 = day_str.parse()?;
        if let Some(extra) = parts.get(4) {
            return Err(anyhow::anyhow!("extra parameter: {}", extra));
        }
        Ok(BookParams::L4(teacher, year, month, day))
    }
}
