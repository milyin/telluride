use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use telluride::utils::{format_screen_spaces, split_with_screened_spaces};

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
            BalanceParams::L0(page)        => format_screen_spaces!(page),
            BalanceParams::L1(s)           => format_screen_spaces!(s),
            BalanceParams::L2(s, year, mo) => format_screen_spaces!(s, year, mo),
        }
        .fmt(f)
    }
}

impl FromStr for BalanceParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);
        let Some(first) = parts.first() else {
            return Ok(BalanceParams::L0(0));
        };
        // Integer first token → page number → L0
        if let Ok(page) = first.parse::<i32>() {
            return Ok(BalanceParams::L0(page));
        }
        let student: TelegramName = first.parse()?;
        match parts.len() {
            1 => Ok(BalanceParams::L1(student)),
            3 => {
                let year: i32 = parts[1]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid year: {}", parts[1]))?;
                let month: u32 = parts[2]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid month: {}", parts[2]))?;
                Ok(BalanceParams::L2(student, year, month))
            }
            n => Err(anyhow::anyhow!("unexpected token count {}", n)),
        }
    }
}
