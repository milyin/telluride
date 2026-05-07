use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use chrono::{Datelike, NaiveDate, NaiveTime};
use telluride::utils::{screen_spaces, split_with_screened_spaces};

use crate::models::{LessonStatus, TelegramName};
use crate::types::Duration;

/// Subcommand discriminant for the `/book` command.
/// The bool field is the **force flag**: `false` = normal UI state, `true` = execute.
/// When force is true the Display representation gains a `"!"` suffix (e.g. `"create!"`).
pub enum BookSubcmd {
    Create(bool),
    Delete(bool),
    List,
    Reschedule(bool),
    Status(bool),
}

impl fmt::Display for BookSubcmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BookSubcmd::Create(false)    => write!(f, "create"),
            BookSubcmd::Create(true)     => write!(f, "create!"),
            BookSubcmd::Delete(false)    => write!(f, "delete"),
            BookSubcmd::Delete(true)     => write!(f, "delete!"),
            BookSubcmd::List             => write!(f, "list"),
            BookSubcmd::Reschedule(false) => write!(f, "reschedule"),
            BookSubcmd::Reschedule(true)  => write!(f, "reschedule!"),
            BookSubcmd::Status(false)    => write!(f, "status"),
            BookSubcmd::Status(true)     => write!(f, "status!"),
        }
    }
}

impl FromStr for BookSubcmd {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "create"      => Ok(BookSubcmd::Create(false)),
            "create!"     => Ok(BookSubcmd::Create(true)),
            "delete"      => Ok(BookSubcmd::Delete(false)),
            "delete!"     => Ok(BookSubcmd::Delete(true)),
            "list"        => Ok(BookSubcmd::List),
            "reschedule"  => Ok(BookSubcmd::Reschedule(false)),
            "reschedule!" => Ok(BookSubcmd::Reschedule(true)),
            "status"      => Ok(BookSubcmd::Status(false)),
            "status!"     => Ok(BookSubcmd::Status(true)),
            _             => Err(anyhow::anyhow!("unknown subcommand: {}", s)),
        }
    }
}

pub enum BookParams {
    // Top-level menu
    M0,

    // Create flow
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

    // List flow
    L0,
    L1(TelegramName, TelegramName, NaiveDate, NaiveTime),

    // Delete flow (entry already identified via L1)
    D0(TelegramName, TelegramName, NaiveDate, NaiveTime),
    DF(TelegramName, TelegramName, NaiveDate, NaiveTime),

    // Reschedule flow (entry already identified via L1)
    R1(TelegramName, TelegramName, NaiveDate, NaiveTime, i32, u32),
    R2(TelegramName, TelegramName, NaiveDate, NaiveTime, NaiveDate),
    R3(TelegramName, TelegramName, NaiveDate, NaiveTime, NaiveDate, NaiveTime),
    RF(TelegramName, TelegramName, NaiveDate, NaiveTime, NaiveDate, NaiveTime),

    // Status flow (teacher only)
    S0(TelegramName, TelegramName, NaiveDate, NaiveTime),
    SF(TelegramName, TelegramName, NaiveDate, NaiveTime, LessonStatus),
}

pub enum BookingActor {
    Student(TelegramName),
    Teacher(TelegramName),
}

impl fmt::Display for BookParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c  = BookSubcmd::Create(false);
        let cf = BookSubcmd::Create(true);
        let d  = BookSubcmd::Delete(false);
        let df = BookSubcmd::Delete(true);
        let l  = BookSubcmd::List;
        let r  = BookSubcmd::Reschedule(false);
        let rf = BookSubcmd::Reschedule(true);
        let s  = BookSubcmd::Status(false);
        let sf = BookSubcmd::Status(true);
        match self {
            BookParams::M0 => write!(f, "m0"),
            BookParams::C0() => write!(f, "{c}"),
            BookParams::C1(t) => write!(f, "{c} {}", screen_spaces(t.as_str())),
            BookParams::C2(t, s2) => write!(f, "{c} {} {}", screen_spaces(t.as_str()), screen_spaces(s2.as_str())),
            BookParams::C3(t, s2, y) => write!(f, "{c} {} {} {y}", screen_spaces(t.as_str()), screen_spaces(s2.as_str())),
            BookParams::C4(t, s2, y, m) => write!(f, "{c} {} {} {y} {m}", screen_spaces(t.as_str()), screen_spaces(s2.as_str())),
            BookParams::C5(t, s2, y, m, day) => write!(f, "{c} {} {} {y} {m} {day}", screen_spaces(t.as_str()), screen_spaces(s2.as_str())),
            BookParams::C6(t, s2, date) => write!(f, "{c} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s2.as_str()), screen_spaces(&date.to_string())),
            BookParams::C7(t, s2, date, time) => write!(f, "{c} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s2.as_str()), screen_spaces(&date.to_string()), screen_spaces(&time.to_string())),
            BookParams::C8(t, s2, date, time, dur) => write!(f, "{c} {} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s2.as_str()), screen_spaces(&date.to_string()), screen_spaces(&time.to_string()), screen_spaces(&dur.to_string())),
            BookParams::CF(t, s2, date, time, dur) => write!(f, "{cf} {} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s2.as_str()), screen_spaces(&date.to_string()), screen_spaces(&time.to_string()), screen_spaces(&dur.to_string())),
            BookParams::L0 => write!(f, "{l}"),
            BookParams::L1(t, s2, date, time) => write!(f, "{l} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s2.as_str()), screen_spaces(&date.to_string()), screen_spaces(&time.to_string())),
            BookParams::D0(t, s2, date, time) => write!(f, "{d} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s2.as_str()), screen_spaces(&date.to_string()), screen_spaces(&time.to_string())),
            BookParams::DF(t, s2, date, time) => write!(f, "{df} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s2.as_str()), screen_spaces(&date.to_string()), screen_spaces(&time.to_string())),
            BookParams::R1(t, s2, od, ot, ny, nm) => write!(f, "{r} {} {} {} {} {ny} {nm}", screen_spaces(t.as_str()), screen_spaces(s2.as_str()), screen_spaces(&od.to_string()), screen_spaces(&ot.to_string())),
            BookParams::R2(t, s2, od, ot, nd) => write!(f, "{r} {} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s2.as_str()), screen_spaces(&od.to_string()), screen_spaces(&ot.to_string()), screen_spaces(&nd.to_string())),
            BookParams::R3(t, s2, od, ot, nd, nt) => write!(f, "{r} {} {} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s2.as_str()), screen_spaces(&od.to_string()), screen_spaces(&ot.to_string()), screen_spaces(&nd.to_string()), screen_spaces(&nt.to_string())),
            BookParams::RF(t, s2, od, ot, nd, nt) => write!(f, "{rf} {} {} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s2.as_str()), screen_spaces(&od.to_string()), screen_spaces(&ot.to_string()), screen_spaces(&nd.to_string()), screen_spaces(&nt.to_string())),
            BookParams::S0(t, s2, date, time) => write!(f, "{s} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s2.as_str()), screen_spaces(&date.to_string()), screen_spaces(&time.to_string())),
            BookParams::SF(t, s2, date, time, status) => write!(f, "{sf} {} {} {} {} {}", screen_spaces(t.as_str()), screen_spaces(s2.as_str()), screen_spaces(&date.to_string()), screen_spaces(&time.to_string()), status),
        }
    }
}

/// Sequential token consumer used by `BookParams::from_str`.
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

impl FromStr for BookParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);

        let Some(first) = parts.first() else {
            return Ok(BookParams::M0);
        };

        match first.as_str() {
            "m0" => return Ok(BookParams::M0),
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
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse()?;
                let date: NaiveDate       = p.next("date")?.parse()?;
                let time: NaiveTime       = p.next("time")?.parse()?;
                p.finish()?;
                Ok(BookParams::D0(teacher, student, date, time))
            }
            BookSubcmd::Delete(true) => {
                let mut p = ParamParser::new(&parts, 1);
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse()?;
                let date: NaiveDate       = p.next("date")?.parse()?;
                let time: NaiveTime       = p.next("time")?.parse()?;
                p.finish()?;
                Ok(BookParams::DF(teacher, student, date, time))
            }
            BookSubcmd::Reschedule(false) => {
                let mut p = ParamParser::new(&parts, 1);
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse()?;
                let od: NaiveDate = p.next("old-date")?.parse()?;
                let ot: NaiveTime = p.next("old-time")?.parse()?;

                if p.is_empty() {
                    let now = chrono::Local::now();
                    return Ok(BookParams::R1(teacher, student, od, ot, now.year(), now.month()));
                }

                // Distinguish new-date (≥2 dashes) from year (no dashes).
                let fifth = p.next("new-date or year")?;
                let fifth_dashes = fifth.chars().filter(|c| *c == '-').count();
                if fifth_dashes >= 2 {
                    let nd: NaiveDate = fifth.parse()?;
                    if p.is_empty() {
                        return Ok(BookParams::R2(teacher, student, od, ot, nd));
                    }
                    let nt: NaiveTime = p.next("new-time")?.parse()?;
                    p.finish()?;
                    Ok(BookParams::R3(teacher, student, od, ot, nd, nt))
                } else {
                    let ny: i32 = fifth.parse()?;
                    let nm: u32 = p.next("month")?.parse()?;
                    p.finish()?;
                    Ok(BookParams::R1(teacher, student, od, ot, ny, nm))
                }
            }
            BookSubcmd::Reschedule(true) => {
                let mut p = ParamParser::new(&parts, 1);
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse()?;
                let od: NaiveDate         = p.next("old-date")?.parse()?;
                let ot: NaiveTime         = p.next("old-time")?.parse()?;
                let nd: NaiveDate         = p.next("new-date")?.parse()?;
                let nt: NaiveTime         = p.next("new-time")?.parse()?;
                p.finish()?;
                Ok(BookParams::RF(teacher, student, od, ot, nd, nt))
            }
            BookSubcmd::Status(false) => {
                let mut p = ParamParser::new(&parts, 1);
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse()?;
                let date: NaiveDate       = p.next("date")?.parse()?;
                let time: NaiveTime       = p.next("time")?.parse()?;
                p.finish()?;
                Ok(BookParams::S0(teacher, student, date, time))
            }
            BookSubcmd::Status(true) => {
                let mut p = ParamParser::new(&parts, 1);
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse()?;
                let date: NaiveDate       = p.next("date")?.parse()?;
                let time: NaiveTime       = p.next("time")?.parse()?;
                let status: LessonStatus  = p.next("status")?.parse()?;
                p.finish()?;
                Ok(BookParams::SF(teacher, student, date, time, status))
            }
            BookSubcmd::List => parse_list(&parts[1..]),
        }
    }
}

fn parse_list(parts: &[String]) -> Result<BookParams> {
    let mut p = ParamParser::new(parts, 0);
    if p.is_empty() {
        return Ok(BookParams::L0);
    }
    let teacher: TelegramName = p.next("teacher")?.parse()?;
    let student: TelegramName = p.next("student")?.parse()?;
    let date: NaiveDate       = p.next("date")?.parse()?;
    let time: NaiveTime       = p.next("time")?.parse()?;
    p.finish()?;
    Ok(BookParams::L1(teacher, student, date, time))
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

    let year: i32 = third.parse()?;
    if p.is_empty() {
        return Ok(BookParams::C3(teacher, student, year));
    }
    let month: u32 = p.next("month")?.parse()?;
    if p.is_empty() {
        return Ok(BookParams::C4(teacher, student, year, month));
    }
    let day: u32 = p.next("day")?.parse()?;
    p.finish()?;
    Ok(BookParams::C5(teacher, student, year, month, day))
}
