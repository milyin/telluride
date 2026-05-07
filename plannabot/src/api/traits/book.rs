use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use chrono::{NaiveDate, NaiveTime};
use telluride::utils::{screen_spaces, split_with_screened_spaces};

use crate::models::TelegramName;
use crate::types::Duration;

/// Subcommand discriminant for the `/book` command.
/// The bool field is the **force flag**: `false` = normal UI state, `true` = execute.
/// When force is true the Display representation gains a `"!"` suffix (e.g. `"create!"`).
/// `FromStr` returns an error if the force form appears without a complete parameter set.
pub enum BookSubcmd {
    Create(bool),
    Edit(bool),
    Delete(bool),
}

impl fmt::Display for BookSubcmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BookSubcmd::Create(false) => write!(f, "create"),
            BookSubcmd::Create(true)  => write!(f, "create!"),
            BookSubcmd::Edit(false)   => write!(f, "edit"),
            BookSubcmd::Edit(true)    => write!(f, "edit!"),
            BookSubcmd::Delete(false) => write!(f, "delete"),
            BookSubcmd::Delete(true)  => write!(f, "delete!"),
        }
    }
}

impl FromStr for BookSubcmd {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "create"  => Ok(BookSubcmd::Create(false)),
            "create!" => Ok(BookSubcmd::Create(true)),
            "edit"    => Ok(BookSubcmd::Edit(false)),
            "edit!"   => Ok(BookSubcmd::Edit(true)),
            "delete"  => Ok(BookSubcmd::Delete(false)),
            "delete!" => Ok(BookSubcmd::Delete(true)),
            _         => Err(anyhow::anyhow!("unknown subcommand: {}", s)),
        }
    }
}

pub enum BookParams {
    // Menu states
    M0,
    M1(BookSubcmd),
    // Create flow (C0-C7 = former L0-L7; C8 shows summary + force button; CF executes)
    C0(),
    C1(TelegramName),
    C2(TelegramName, TelegramName),
    C3(TelegramName, TelegramName, i32),
    C4(TelegramName, TelegramName, i32, u32),
    C5(TelegramName, TelegramName, i32, u32, u32),
    C6(TelegramName, TelegramName, NaiveDate),
    C7(TelegramName, TelegramName, NaiveDate, NaiveTime),
    C8(TelegramName, TelegramName, NaiveDate, NaiveTime, Duration),
    CF(TelegramName, TelegramName, NaiveDate, NaiveTime, Duration),
    // Delete flow
    D0(),
    D1(TelegramName, TelegramName, NaiveDate, NaiveTime),
    DF(TelegramName, TelegramName, NaiveDate, NaiveTime),
    // Edit flow
    E0(),
    E1(TelegramName, TelegramName, NaiveDate, NaiveTime, i32, u32),
    E2(TelegramName, TelegramName, NaiveDate, NaiveTime, NaiveDate),
    E3(TelegramName, TelegramName, NaiveDate, NaiveTime, NaiveDate, NaiveTime),
    EF(TelegramName, TelegramName, NaiveDate, NaiveTime, NaiveDate, NaiveTime),
}

pub enum BookingActor {
    Student(TelegramName),
    Teacher(TelegramName),
}

impl fmt::Display for BookParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = BookSubcmd::Create(false);
        let cf = BookSubcmd::Create(true);
        let d = BookSubcmd::Delete(false);
        let df = BookSubcmd::Delete(true);
        let e = BookSubcmd::Edit(false);
        let ef = BookSubcmd::Edit(true);
        match self {
            BookParams::M0 => write!(f, "m0"),
            BookParams::M1(s) => write!(f, "m1 {s}"),
            BookParams::C0() => write!(f, "{c}"),
            BookParams::C1(t) => write!(f, "{c} {}", screen_spaces(t.as_str())),
            BookParams::C2(t, s) => write!(f, "{c} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str())),
            BookParams::C3(t, s, y) => write!(f, "{c} {} {} {y}", screen_spaces(t.as_str()), screen_spaces(s.as_str())),
            BookParams::C4(t, s, y, m) => write!(f, "{c} {} {} {y} {m}", screen_spaces(t.as_str()), screen_spaces(s.as_str())),
            BookParams::C5(t, s, y, m, day) => write!(f, "{c} {} {} {y} {m} {day}", screen_spaces(t.as_str()), screen_spaces(s.as_str())),
            BookParams::C6(t, s, date) => write!(f, "{c} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), screen_spaces(&date.to_string())),
            BookParams::C7(t, s, date, time) => write!(f, "{c} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), screen_spaces(&date.to_string()), screen_spaces(&time.to_string())),
            BookParams::C8(t, s, date, time, dur) => write!(f, "{c} {} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), screen_spaces(&date.to_string()), screen_spaces(&time.to_string()), screen_spaces(&dur.to_string())),
            BookParams::CF(t, s, date, time, dur) => write!(f, "{cf} {} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), screen_spaces(&date.to_string()), screen_spaces(&time.to_string()), screen_spaces(&dur.to_string())),
            BookParams::D0() => write!(f, "{d}"),
            BookParams::D1(t, s, od, ot) => write!(f, "{d} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), screen_spaces(&od.to_string()), screen_spaces(&ot.to_string())),
            BookParams::DF(t, s, od, ot) => write!(f, "{df} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), screen_spaces(&od.to_string()), screen_spaces(&ot.to_string())),
            BookParams::E0() => write!(f, "{e}"),
            BookParams::E1(t, s, od, ot, ny, nm) => write!(f, "{e} {} {} {} {} {ny} {nm}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), screen_spaces(&od.to_string()), screen_spaces(&ot.to_string())),
            BookParams::E2(t, s, od, ot, nd) => write!(f, "{e} {} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), screen_spaces(&od.to_string()), screen_spaces(&ot.to_string()), screen_spaces(&nd.to_string())),
            BookParams::E3(t, s, od, ot, nd, nt) => write!(f, "{e} {} {} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), screen_spaces(&od.to_string()), screen_spaces(&ot.to_string()), screen_spaces(&nd.to_string()), screen_spaces(&nt.to_string())),
            BookParams::EF(t, s, od, ot, nd, nt) => write!(f, "{ef} {} {} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s.as_str()), screen_spaces(&od.to_string()), screen_spaces(&ot.to_string()), screen_spaces(&nd.to_string()), screen_spaces(&nt.to_string())),
        }
    }
}

/// Sequential token consumer used by `BookParams::from_str`.
/// Eliminates magic count checks: each field is named, missing fields and
/// extra fields are both detected without hardcoding a total count.
struct ParamParser<'a> {
    parts: &'a [String],
    pos: usize,
}

impl<'a> ParamParser<'a> {
    fn new(parts: &'a [String], start: usize) -> Self {
        Self { parts, pos: start }
    }
    fn is_empty(&self) -> bool {
        self.pos >= self.parts.len()
    }
    /// Consume and return the next token, or error with the field name.
    fn next(&mut self, name: &str) -> Result<&str> {
        self.parts
            .get(self.pos)
            .map(|s| {
                self.pos += 1;
                s.as_str()
            })
            .ok_or_else(|| anyhow::anyhow!("missing parameter: {}", name))
    }
    /// Error if any tokens remain unconsumed.
    fn finish(&self) -> Result<()> {
        match self.parts.get(self.pos) {
            None => Ok(()),
            Some(extra) => Err(anyhow::anyhow!("extra parameter: {}", extra)),
        }
    }
}

impl FromStr for BookParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);

        let Some(first) = parts.first() else {
            return Ok(BookParams::M0);
        };

        match first.as_str() {
            "m0" => return Ok(BookParams::M0),
            "m1" => {
                let subcmd = parts
                    .get(1)
                    .ok_or_else(|| anyhow::anyhow!("m1 requires a subcommand"))?
                    .parse::<BookSubcmd>()?;
                return Ok(BookParams::M1(subcmd));
            }
            _ => {}
        }

        let subcmd: BookSubcmd = first.parse()?;

        match subcmd {
            BookSubcmd::Create(false) => parse_create(&parts[1..]),
            BookSubcmd::Create(true) => {
                let mut p = ParamParser::new(&parts, 1);
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse()?;
                let date: NaiveDate      = p.next("date")?.parse()?;
                let time: NaiveTime      = p.next("time")?.parse()?;
                let dur: Duration        = p.next("duration")?.parse()?;
                p.finish()?;
                Ok(BookParams::CF(teacher, student, date, time, dur))
            }
            BookSubcmd::Delete(false) => {
                let mut p = ParamParser::new(&parts, 1);
                if p.is_empty() {
                    return Ok(BookParams::D0());
                }
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse()?;
                let od: NaiveDate         = p.next("date")?.parse()?;
                let ot: NaiveTime         = p.next("time")?.parse()?;
                p.finish()?;
                Ok(BookParams::D1(teacher, student, od, ot))
            }
            BookSubcmd::Delete(true) => {
                let mut p = ParamParser::new(&parts, 1);
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse()?;
                let od: NaiveDate         = p.next("date")?.parse()?;
                let ot: NaiveTime         = p.next("time")?.parse()?;
                p.finish()?;
                Ok(BookParams::DF(teacher, student, od, ot))
            }
            BookSubcmd::Edit(false) => {
                let mut p = ParamParser::new(&parts, 1);
                if p.is_empty() {
                    return Ok(BookParams::E0());
                }
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse().map_err(|_| {
                    anyhow::anyhow!("expected student TelegramName")
                })?;
                let od: NaiveDate = p.next("old-date")?.parse()?;
                let ot: NaiveTime = p.next("old-time")?.parse()?;

                // Distinguish new-date (YYYY-MM-DD, ≥2 dashes) from year (integer, no dashes).
                let fifth = p.next("new-date or year")?;
                let fifth_dashes = fifth.chars().filter(|c| *c == '-').count();
                if fifth_dashes >= 2 {
                    let nd: NaiveDate = fifth.parse()?;
                    if p.is_empty() {
                        return Ok(BookParams::E2(teacher, student, od, ot, nd));
                    }
                    let nt: NaiveTime = p.next("new-time")?.parse()?;
                    p.finish()?;
                    Ok(BookParams::E3(teacher, student, od, ot, nd, nt))
                } else {
                    let ny: i32  = fifth.parse()?;
                    let nm: u32  = p.next("month")?.parse()?;
                    p.finish()?;
                    Ok(BookParams::E1(teacher, student, od, ot, ny, nm))
                }
            }
            BookSubcmd::Edit(true) => {
                let mut p = ParamParser::new(&parts, 1);
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse()?;
                let od: NaiveDate         = p.next("old-date")?.parse()?;
                let ot: NaiveTime         = p.next("old-time")?.parse()?;
                let nd: NaiveDate         = p.next("new-date")?.parse()?;
                let nt: NaiveTime         = p.next("new-time")?.parse()?;
                p.finish()?;
                Ok(BookParams::EF(teacher, student, od, ot, nd, nt))
            }
        }
    }
}

fn parse_create(parts: &[String]) -> Result<BookParams> {
    let mut p = ParamParser::new(parts, 0);
    if p.is_empty() {
        return Ok(BookParams::C0());
    }
    let teacher: TelegramName = p.next("teacher")?.parse()?;
    if p.is_empty() {
        return Ok(BookParams::C1(teacher));
    }
    let student: TelegramName = p.next("student")?.parse().map_err(|_| {
        anyhow::anyhow!("expected student TelegramName")
    })?;
    if p.is_empty() {
        return Ok(BookParams::C2(teacher, student));
    }

    // Distinguish date (YYYY-MM-DD, ≥2 dashes) from year (plain integer, no dashes).
    let third = p.next("date or year")?;
    let third_dashes = third.chars().filter(|c| *c == '-').count();
    if third_dashes >= 2 {
        let date: NaiveDate = third.parse()?;
        if p.is_empty() {
            return Ok(BookParams::C6(teacher, student, date));
        }
        let time: NaiveTime = p.next("time")?.parse()?;
        if p.is_empty() {
            return Ok(BookParams::C7(teacher, student, date, time));
        }
        let duration: Duration = p.next("duration")?.parse()?;
        p.finish()?;
        return Ok(BookParams::C8(teacher, student, date, time, duration));
    }

    // Year branch
    let year: i32 = third.parse()?;
    if p.is_empty() {
        return Ok(BookParams::C3(teacher, student, year));
    }
    let month: u32 = p.next("month")?.parse()?;
    if p.is_empty() {
        return Ok(BookParams::C4(teacher, student, year, month));
    }
    let day: u32 = p.next("day")?.parse()?;
    if let Some(extra) = parts.get(p.pos) {
        return Err(anyhow::anyhow!("extra parameter: {}", extra));
    }
    Ok(BookParams::C5(teacher, student, year, month, day))
}
