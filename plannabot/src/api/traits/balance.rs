use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use telluride::utils::{ParamParser, format_screen_spaces, split_with_screened_spaces};

use crate::models::TelegramName;

pub enum BalanceParams {
    /// Paginated student list (teacher/admin only). Page is 0-based.
    L0(i32),
    /// Entry point for a specific student: redirects to L2 with the current month.
    L1(TelegramName),
    /// Monthly balance view for a specific student.
    L2(TelegramName, i32, u32),
}

pub enum BalanceActor {
    Student(TelegramName),
    Teacher(TelegramName),
    Admin,
}

impl fmt::Display for BalanceParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BalanceParams::L0(page) => format_screen_spaces!(page),
            BalanceParams::L1(s) => format_screen_spaces!(s),
            BalanceParams::L2(s, year, mo) => format_screen_spaces!(s, year, mo),
        }
        .fmt(f)
    }
}

impl FromStr for BalanceParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);
        let mut p = ParamParser::new(&parts, 0);
        let Some(first) = p.next_opt() else {
            return Ok(BalanceParams::L0(0));
        };
        if let Ok(page) = first.parse::<i32>() {
            p.finish()?;
            return Ok(BalanceParams::L0(page));
        }
        let student: TelegramName = first.parse()?;
        if p.is_empty() {
            return Ok(BalanceParams::L1(student));
        }
        let year: i32 = p.next("year")?.parse()?;
        let month: u32 = p.next("month")?.parse()?;
        p.finish()?;
        Ok(BalanceParams::L2(student, year, month))
    }
}
