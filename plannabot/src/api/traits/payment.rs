use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use telluride::utils::{format_screen_spaces, split_with_screened_spaces};

use crate::models::TelegramName;

pub enum PaymentParams {
    /// List all students with their balances.
    P0,
    /// Show balance breakdown for a specific student.
    P1(TelegramName),
    /// Confirm recording a payment (amount stored in cents).
    P2(TelegramName, i64),
    /// Execute: write the payment to the Payments sheet.
    PF(TelegramName, i64),
}

impl fmt::Display for PaymentParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaymentParams::P0 => format_screen_spaces!(),
            PaymentParams::P1(s) => format_screen_spaces!(s),
            PaymentParams::P2(s, amt) => format_screen_spaces!("confirm", s, amt),
            PaymentParams::PF(s, amt) => format_screen_spaces!("confirm!", s, amt),
        }
        .fmt(f)
    }
}

impl FromStr for PaymentParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);

        let Some(first) = parts.first() else {
            return Ok(PaymentParams::P0);
        };

        match first.as_str() {
            "confirm" | "confirm!" => {
                let execute = first == "confirm!";
                let student: TelegramName = parts
                    .get(1)
                    .ok_or_else(|| anyhow::anyhow!("missing student"))?
                    .parse()?;
                let amt: i64 = parts
                    .get(2)
                    .ok_or_else(|| anyhow::anyhow!("missing amount"))?
                    .parse()?;
                if execute {
                    Ok(PaymentParams::PF(student, amt))
                } else {
                    Ok(PaymentParams::P2(student, amt))
                }
            }
            _ => {
                // First token is a telegram name → P1
                let student: TelegramName = first.parse()?;
                if parts.len() == 1 {
                    return Ok(PaymentParams::P1(student));
                }
                // Second token is an amount (in major currency units, e.g. "100" or "100.50")
                let amt_str = &parts[1];
                let amt_cents = parse_amount_cents(amt_str)?;
                Ok(PaymentParams::P2(student, amt_cents))
            }
        }
    }
}

/// Parses a decimal amount string (e.g. "100" or "100.50") into integer cents.
pub fn parse_amount_cents(s: &str) -> Result<i64> {
    let f: f64 = s
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid amount: '{}'", s))?;
    if f < 0.0 {
        return Err(anyhow::anyhow!("amount must be positive"));
    }
    Ok((f * 100.0).round() as i64)
}

/// Formats integer cents as a human-readable amount string (e.g. 15050 → "150.50").
pub fn format_amount(cents: i64) -> String {
    let abs = cents.abs();
    let sign = if cents < 0 { "-" } else { "" };
    let major = abs / 100;
    let minor = abs % 100;
    if minor == 0 {
        format!("{}{}", sign, major)
    } else {
        format!("{}{}.{:02}", sign, major, minor)
    }
}
