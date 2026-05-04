use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use chrono::{NaiveDate, NaiveTime};
use humantime::Duration;
use telluride::utils::{screen_spaces, split_with_screened_spaces};

use crate::models::TelegramName;

pub enum BookParams {
    L0(),
    L1(TelegramName, i32, u32),
    L2(TelegramName, NaiveDate),
    L3(TelegramName, NaiveDate, NaiveTime),
    L4(TelegramName, NaiveDate, NaiveTime, Duration),
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
            return Err(anyhow::anyhow!("missing year-month after teacher name"));
        };

        // "YYYY-MM" (1 dash) → L1; "YYYY-MM-DD" (2 dashes) → L2+
        let dash_count = second.chars().filter(|c| *c == '-').count();
        if dash_count == 1 {
            let mut ym = second.splitn(2, '-');
            let year: i32 = ym.next().unwrap().parse()?;
            let month: u32 = ym.next().unwrap().parse()?;
            if let Some(extra) = parts.get(2) {
                return Err(anyhow::anyhow!("unexpected extra parameter: {}", extra));
            }
            return Ok(BookParams::L1(teacher, year, month));
        }

        let date: NaiveDate = second.parse()?;
        let Some(hour_str) = parts.get(2) else {
            return Ok(BookParams::L2(teacher, date));
        };
        let hour: NaiveTime = hour_str.parse()?;
        let Some(duration_str) = parts.get(3) else {
            return Ok(BookParams::L3(teacher, date, hour));
        };
        let duration: Duration = duration_str.parse()?;
        if let Some(extra) = parts.get(4) {
            return Err(anyhow::anyhow!("extra parameter: {}", extra));
        }
        Ok(BookParams::L4(teacher, date, hour, duration))
    }
}

impl fmt::Display for BookParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BookParams::L0() => Ok(()),
            BookParams::L1(t, y, m) => {
                write!(f, "{} {:04}-{:02}", screen_spaces(t.as_str()), y, m)
            }
            BookParams::L2(t, d) => {
                write!(f, "{} {}", screen_spaces(t.as_str()), screen_spaces(&d.to_string()))
            }
            BookParams::L3(t, d, h) => write!(
                f,
                "{} {} {}",
                screen_spaces(t.as_str()),
                screen_spaces(&d.to_string()),
                screen_spaces(&h.to_string())
            ),
            BookParams::L4(t, d, h, dur) => write!(
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
