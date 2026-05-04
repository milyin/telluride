use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use chrono::{NaiveDate, NaiveTime};
use humantime::Duration;
use telluride::utils::{screen_spaces, split_with_screened_spaces};

use crate::models::TelegramName;

pub enum SelectDate {
    YearMonth(i32, u32),
    Date(NaiveDate),
}

impl fmt::Display for SelectDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectDate::YearMonth(y, m) => write!(f, "{:04}-{:02}", y, m),
            SelectDate::Date(d) => write!(f, "{}", d),
        }
    }
}

impl FromStr for SelectDate {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let dash_count = s.chars().filter(|c| *c == '-').count();
        if dash_count == 1 {
            let mut parts = s.splitn(2, '-');
            let year: i32 = parts.next().unwrap().parse()?;
            let month: u32 = parts.next().unwrap().parse()?;
            Ok(SelectDate::YearMonth(year, month))
        } else if dash_count == 2 {
            Ok(SelectDate::Date(s.parse()?))
        } else {
            Err(anyhow::anyhow!("expected YYYY-MM or YYYY-MM-DD, got: {}", s))
        }
    }
}

pub enum BookParams {
    L0(),
    L1(TelegramName),
    L2(TelegramName, SelectDate),
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
            return Ok(BookParams::L1(teacher));
        };

        // 2 parts → L2 with SelectDate (YearMonth "YYYY-MM" or Date "YYYY-MM-DD")
        if parts.get(2).is_none() {
            let select: SelectDate = second.parse()?;
            return Ok(BookParams::L2(teacher, select));
        }

        // 3+ parts → second must be a full NaiveDate
        let date: NaiveDate = second.parse()?;
        let hour: NaiveTime = parts.get(2).unwrap().parse()?;
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
            BookParams::L1(t) => write!(f, "{}", screen_spaces(t.as_str())),
            BookParams::L2(t, sd) => write!(f, "{} {}", screen_spaces(t.as_str()), sd),
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
