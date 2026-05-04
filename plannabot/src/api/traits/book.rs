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
    L2(TelegramName, NaiveDate),
    L3(TelegramName, NaiveDate, NaiveTime),
    L4(TelegramName, NaiveDate, NaiveTime, Duration),
}

impl FromStr for BookParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);
        let Some(teacher) = parts.get(0) else {
            return Ok(BookParams::L0());
        };
        let teacher = teacher.parse()?;
        let Some(date) = parts.get(1) else {
            return Ok(BookParams::L1(teacher));
        };
        let date = date.parse()?;
        let Some(hour) = parts.get(2) else {
            return Ok(BookParams::L2(teacher, date));
        };
        let hour = hour.parse()?;
        let Some(duration) = parts.get(3) else {
            return Ok(BookParams::L3(teacher, date, hour));
        };
        let duration = duration.parse()?;
        let Some(extra) = parts.get(4) else {
            return Ok(BookParams::L4(teacher, date, hour, duration));
        };
        Err(anyhow::anyhow!("extra parameter: {}", extra))
    }
}

impl fmt::Display for BookParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BookParams::L0() => Ok(()),
            BookParams::L1(t) => write!(f, "{}", screen_spaces(t.as_str())),
            BookParams::L2(t, d) => write!(f, "{} {}", screen_spaces(t.as_str()), screen_spaces(&d.to_string())),
            BookParams::L3(t, d, h) => write!(f, "{} {} {}", screen_spaces(t.as_str()), screen_spaces(&d.to_string()), screen_spaces(&h.to_string())),
            BookParams::L4(t, d, h, dur) => write!(f, "{} {} {} {}", screen_spaces(t.as_str()), screen_spaces(&d.to_string()), screen_spaces(&h.to_string()), screen_spaces(&dur.to_string())),
        }
    }
}
