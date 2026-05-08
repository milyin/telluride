use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use chrono::{NaiveDate, NaiveTime};
use telluride::utils::{format_screen_spaces, split_with_screened_spaces};

use crate::models::TelegramName;

pub enum PaymentActor {
    Teacher(TelegramName),
    Admin,
}

pub enum PaymentSubcmd {
    List,
    Show,
    Delete(bool),
    New(bool),
}

impl fmt::Display for PaymentSubcmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaymentSubcmd::List => write!(f, "list"),
            PaymentSubcmd::Show => write!(f, "show"),
            PaymentSubcmd::Delete(false) => write!(f, "delete"),
            PaymentSubcmd::Delete(true) => write!(f, "delete!"),
            PaymentSubcmd::New(false) => write!(f, "new"),
            PaymentSubcmd::New(true) => write!(f, "new!"),
        }
    }
}

impl FromStr for PaymentSubcmd {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "list" => Ok(PaymentSubcmd::List),
            "show" => Ok(PaymentSubcmd::Show),
            "delete" => Ok(PaymentSubcmd::Delete(false)),
            "delete!" => Ok(PaymentSubcmd::Delete(true)),
            "new" => Ok(PaymentSubcmd::New(false)),
            "new!" => Ok(PaymentSubcmd::New(true)),
            _ => Err(anyhow::anyhow!("unknown payment subcommand: {}", s)),
        }
    }
}

pub enum PaymentParams {
    /// Paginated student list. Page is 0-based.
    P0(i32),
    /// Student menu (list / new payment / back).
    P1(TelegramName),
    /// Paginated payment list for a student. Page is 0-based.
    PL(TelegramName, i32),
    /// Show a specific payment identified by date+time (UTC).
    PS(TelegramName, NaiveDate, NaiveTime),
    /// Confirm delete of a payment.
    PD(TelegramName, NaiveDate, NaiveTime),
    /// Execute delete of a payment (force flag).
    PDF(TelegramName, NaiveDate, NaiveTime),
    /// New payment prompt (shows balance, prefills inline command).
    PN(TelegramName),
    /// Create a new payment (force flag).
    PNF(TelegramName, i64),
}

impl fmt::Display for PaymentParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let l = PaymentSubcmd::List;
        let sh = PaymentSubcmd::Show;
        let d = PaymentSubcmd::Delete(false);
        let df = PaymentSubcmd::Delete(true);
        let n = PaymentSubcmd::New(false);
        let nf = PaymentSubcmd::New(true);
        match self {
            PaymentParams::P0(page) => format_screen_spaces!(page),
            PaymentParams::P1(s) => format_screen_spaces!(s),
            PaymentParams::PL(s, page) => format_screen_spaces!(l, s, page),
            PaymentParams::PS(s, date, time) => format_screen_spaces!(sh, s, date, time),
            PaymentParams::PD(s, date, time) => format_screen_spaces!(d, s, date, time),
            PaymentParams::PDF(s, date, time) => format_screen_spaces!(df, s, date, time),
            PaymentParams::PN(s) => format_screen_spaces!(n, s),
            PaymentParams::PNF(s, amt) => format_screen_spaces!(nf, s, amt),
        }
        .fmt(f)
    }
}

struct ParamParser<'a> {
    parts: &'a [String],
    pos: usize,
}

impl<'a> ParamParser<'a> {
    fn new(parts: &'a [String], start: usize) -> Self {
        Self { parts, pos: start }
    }
    fn next(&mut self, name: &str) -> Result<&str> {
        self.parts
            .get(self.pos)
            .map(|s| {
                self.pos += 1;
                s.as_str()
            })
            .ok_or_else(|| anyhow::anyhow!("missing parameter: {}", name))
    }
    fn finish(&self) -> Result<()> {
        match self.parts.get(self.pos) {
            None => Ok(()),
            Some(extra) => Err(anyhow::anyhow!("extra parameter: {}", extra)),
        }
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

        // Try subcommand
        if let Ok(subcmd) = first.parse::<PaymentSubcmd>() {
            let mut p = ParamParser::new(&parts, 1);
            return match subcmd {
                PaymentSubcmd::List => {
                    let student: TelegramName = p.next("student")?.parse()?;
                    let page = if let Ok(pg_str) = p.next("page") {
                        pg_str.parse::<i32>().unwrap_or(0)
                    } else {
                        0
                    };
                    Ok(PaymentParams::PL(student, page))
                }
                PaymentSubcmd::Show => {
                    let student: TelegramName = p.next("student")?.parse()?;
                    let date: NaiveDate = p.next("date")?.parse()?;
                    let time: NaiveTime = p.next("time")?.parse()?;
                    p.finish()?;
                    Ok(PaymentParams::PS(student, date, time))
                }
                PaymentSubcmd::Delete(false) => {
                    let student: TelegramName = p.next("student")?.parse()?;
                    let date: NaiveDate = p.next("date")?.parse()?;
                    let time: NaiveTime = p.next("time")?.parse()?;
                    p.finish()?;
                    Ok(PaymentParams::PD(student, date, time))
                }
                PaymentSubcmd::Delete(true) => {
                    let student: TelegramName = p.next("student")?.parse()?;
                    let date: NaiveDate = p.next("date")?.parse()?;
                    let time: NaiveTime = p.next("time")?.parse()?;
                    p.finish()?;
                    Ok(PaymentParams::PDF(student, date, time))
                }
                PaymentSubcmd::New(false) => {
                    let student: TelegramName = p.next("student")?.parse()?;
                    p.finish()?;
                    Ok(PaymentParams::PN(student))
                }
                PaymentSubcmd::New(true) => {
                    let student: TelegramName = p.next("student")?.parse()?;
                    let amount: i64 = p.next("amount")?.parse()?;
                    p.finish()?;
                    Ok(PaymentParams::PNF(student, amount))
                }
            };
        }

        // Telegram name → student menu (P1)
        let student: TelegramName = first.parse()?;
        Ok(PaymentParams::P1(student))
    }
}
