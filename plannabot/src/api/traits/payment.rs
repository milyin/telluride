use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use telluride::utils::{format_screen_spaces, split_with_screened_spaces};

use crate::models::TelegramName;

pub enum PaymentActor {
    Teacher(TelegramName),
    Admin,
}

pub enum PaymentParams {
    /// Paginated student list. Page is 0-based.
    P0(i32),
    /// Show detail for a specific student.
    P1(TelegramName),
    // P2 and PF are reserved for future payment recording.
    // P2(TelegramName, i64),
    // PF(TelegramName, i64),
}

impl fmt::Display for PaymentParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaymentParams::P0(page) => format_screen_spaces!(page),
            PaymentParams::P1(s) => format_screen_spaces!(s),
            // PaymentParams::P2(s, amt) => format_screen_spaces!("confirm", s, amt),
            // PaymentParams::PF(s, amt) => format_screen_spaces!("confirm!", s, amt),
        }
        .fmt(f)
    }
}

impl FromStr for PaymentParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);

        let Some(first) = parts.first() else {
            return Ok(PaymentParams::P0(0));
        };

        // Integer first token → page number → P0
        if let Ok(page) = first.parse::<i32>() {
            return Ok(PaymentParams::P0(page));
        }

        // Commented out until payment recording is implemented:
        // match first.as_str() {
        //     "confirm" | "confirm!" => { ... }
        //     _ => {}
        // }

        let student: TelegramName = first.parse()?;
        Ok(PaymentParams::P1(student))
    }
}
