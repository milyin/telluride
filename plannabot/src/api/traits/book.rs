use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use chrono::{Datelike, NaiveDate, NaiveTime};
use telluride::markdown::MarkdownString;
use telluride::markdown_format;
use telluride::utils::{format_screen_spaces, split_with_screened_spaces, ParamParser};

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
    Update,
}

impl fmt::Display for BookSubcmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BookSubcmd::Create(false) => write!(f, "create"),
            BookSubcmd::Create(true) => write!(f, "create!"),
            BookSubcmd::Delete(false) => write!(f, "delete"),
            BookSubcmd::Delete(true) => write!(f, "delete!"),
            BookSubcmd::List => write!(f, "list"),
            BookSubcmd::Reschedule(false) => write!(f, "reschedule"),
            BookSubcmd::Reschedule(true) => write!(f, "reschedule!"),
            BookSubcmd::Status(false) => write!(f, "status"),
            BookSubcmd::Status(true) => write!(f, "status!"),
            BookSubcmd::Update => write!(f, "update"),
        }
    }
}

impl FromStr for BookSubcmd {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "create" => Ok(BookSubcmd::Create(false)),
            "create!" => Ok(BookSubcmd::Create(true)),
            "delete" => Ok(BookSubcmd::Delete(false)),
            "delete!" => Ok(BookSubcmd::Delete(true)),
            "list" => Ok(BookSubcmd::List),
            "reschedule" => Ok(BookSubcmd::Reschedule(false)),
            "reschedule!" => Ok(BookSubcmd::Reschedule(true)),
            "status" => Ok(BookSubcmd::Status(false)),
            "status!" => Ok(BookSubcmd::Status(true)),
            "update" => Ok(BookSubcmd::Update),
            _ => Err(anyhow::anyhow!("unknown subcommand: {}", s)),
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

    // List flow (L0 = top menu; L1 = lesson detail)
    L0(i32),
    L1(TelegramName, TelegramName, NaiveDate, NaiveTime),

    // Update flow (calendar + day view)
    U0(i32, u32),
    U1(NaiveDate),

    // Delete flow (entry already identified via L1)
    D0(TelegramName, TelegramName, NaiveDate, NaiveTime),
    DF(TelegramName, TelegramName, NaiveDate, NaiveTime),

    // Reschedule flow (entry already identified via L1)
    R1(TelegramName, TelegramName, NaiveDate, NaiveTime, i32, u32),
    R2(TelegramName, TelegramName, NaiveDate, NaiveTime, NaiveDate),
    R3(
        TelegramName,
        TelegramName,
        NaiveDate,
        NaiveTime,
        NaiveDate,
        NaiveTime,
    ),
    RF(
        TelegramName,
        TelegramName,
        NaiveDate,
        NaiveTime,
        NaiveDate,
        NaiveTime,
    ),

    // Status flow (teacher only)
    S0(TelegramName, TelegramName, NaiveDate, NaiveTime),
    SF(
        TelegramName,
        TelegramName,
        NaiveDate,
        NaiveTime,
        LessonStatus,
    ),
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
        let l = BookSubcmd::List;
        let r = BookSubcmd::Reschedule(false);
        let rf = BookSubcmd::Reschedule(true);
        let s = BookSubcmd::Status(false);
        let sf = BookSubcmd::Status(true);
        let u = BookSubcmd::Update;
        match self {
            BookParams::M0 => format_screen_spaces!(),
            BookParams::C0() => format_screen_spaces!(c),
            BookParams::C1(t) => format_screen_spaces!(c, t),
            BookParams::C2(t, s2) => format_screen_spaces!(c, t, s2),
            BookParams::C3(t, s2, y) => format_screen_spaces!(c, t, s2, y),
            BookParams::C4(t, s2, y, m) => format_screen_spaces!(c, t, s2, y, m),
            BookParams::C5(t, s2, y, m, day) => format_screen_spaces!(c, t, s2, y, m, day),
            BookParams::C6(t, s2, date) => format_screen_spaces!(c, t, s2, date),
            BookParams::C7(t, s2, date, time) => format_screen_spaces!(c, t, s2, date, time),
            BookParams::C8(t, s2, date, time, dur) => {
                format_screen_spaces!(c, t, s2, date, time, dur)
            }
            BookParams::CF(t, s2, date, time, dur) => {
                format_screen_spaces!(cf, t, s2, date, time, dur)
            }
            BookParams::L0(w) => format_screen_spaces!(l, w),
            BookParams::L1(t, s2, date, time) => format_screen_spaces!(l, t, s2, date, time),
            BookParams::U0(y, m) => format_screen_spaces!(u, y, m),
            BookParams::U1(date) => format_screen_spaces!(u, date),
            BookParams::D0(t, s2, date, time) => format_screen_spaces!(d, t, s2, date, time),
            BookParams::DF(t, s2, date, time) => format_screen_spaces!(df, t, s2, date, time),
            BookParams::R1(t, s2, od, ot, ny, nm) => {
                format_screen_spaces!(r, t, s2, od, ot, ny, nm)
            }
            BookParams::R2(t, s2, od, ot, nd) => format_screen_spaces!(r, t, s2, od, ot, nd),
            BookParams::R3(t, s2, od, ot, nd, nt) => {
                format_screen_spaces!(r, t, s2, od, ot, nd, nt)
            }
            BookParams::RF(t, s2, od, ot, nd, nt) => {
                format_screen_spaces!(rf, t, s2, od, ot, nd, nt)
            }
            BookParams::S0(t, s2, date, time) => format_screen_spaces!(s, t, s2, date, time),
            BookParams::SF(t, s2, date, time, status) => {
                format_screen_spaces!(sf, t, s2, date, time, status)
            }
        }
        .fmt(f)
    }
}


impl FromStr for BookParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);

        let Some(first) = parts.first() else {
            return Ok(BookParams::M0);
        };

        let subcmd: BookSubcmd = first.parse()?;

        match subcmd {
            BookSubcmd::Create(false) => parse_create(&parts[1..]),
            BookSubcmd::Create(true) => {
                let mut p = ParamParser::new(&parts, 1);
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse()?;
                let date: NaiveDate = p.next("date")?.parse()?;
                let time: NaiveTime = p.next("time")?.parse()?;
                let dur: Duration = p.next("duration")?.parse()?;
                p.finish()?;
                Ok(BookParams::CF(teacher, student, date, time, dur))
            }
            BookSubcmd::Delete(false) => {
                let mut p = ParamParser::new(&parts, 1);
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse()?;
                let date: NaiveDate = p.next("date")?.parse()?;
                let time: NaiveTime = p.next("time")?.parse()?;
                p.finish()?;
                Ok(BookParams::D0(teacher, student, date, time))
            }
            BookSubcmd::Delete(true) => {
                let mut p = ParamParser::new(&parts, 1);
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse()?;
                let date: NaiveDate = p.next("date")?.parse()?;
                let time: NaiveTime = p.next("time")?.parse()?;
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
                    return Ok(BookParams::R1(
                        teacher,
                        student,
                        od,
                        ot,
                        now.year(),
                        now.month(),
                    ));
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
                let od: NaiveDate = p.next("old-date")?.parse()?;
                let ot: NaiveTime = p.next("old-time")?.parse()?;
                let nd: NaiveDate = p.next("new-date")?.parse()?;
                let nt: NaiveTime = p.next("new-time")?.parse()?;
                p.finish()?;
                Ok(BookParams::RF(teacher, student, od, ot, nd, nt))
            }
            BookSubcmd::Status(false) => {
                let mut p = ParamParser::new(&parts, 1);
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse()?;
                let date: NaiveDate = p.next("date")?.parse()?;
                let time: NaiveTime = p.next("time")?.parse()?;
                p.finish()?;
                Ok(BookParams::S0(teacher, student, date, time))
            }
            BookSubcmd::Status(true) => {
                let mut p = ParamParser::new(&parts, 1);
                let teacher: TelegramName = p.next("teacher")?.parse()?;
                let student: TelegramName = p.next("student")?.parse()?;
                let date: NaiveDate = p.next("date")?.parse()?;
                let time: NaiveTime = p.next("time")?.parse()?;
                let status: LessonStatus = p.next("status")?.parse()?;
                p.finish()?;
                Ok(BookParams::SF(teacher, student, date, time, status))
            }
            BookSubcmd::List => parse_list(&parts[1..]),
            BookSubcmd::Update => parse_update(&parts[1..]),
        }
    }
}

fn parse_list(parts: &[String]) -> Result<BookParams> {
    let mut p = ParamParser::new(parts, 0);
    if p.is_empty() {
        return Ok(BookParams::L0(0));
    }
    let first = p.next("week_offset or teacher")?;
    if let Ok(week_offset) = first.parse::<i32>() {
        p.finish()?;
        return Ok(BookParams::L0(week_offset));
    }
    let teacher: TelegramName = first.parse()?;
    let student: TelegramName = p.next("student")?.parse()?;
    let date: NaiveDate = p.next("date")?.parse()?;
    let time: NaiveTime = p.next("time")?.parse()?;
    p.finish()?;
    Ok(BookParams::L1(teacher, student, date, time))
}

fn parse_update(parts: &[String]) -> Result<BookParams> {
    let mut p = ParamParser::new(parts, 0);
    if p.is_empty() {
        let now = chrono::Local::now();
        return Ok(BookParams::U0(now.year(), now.month()));
    }
    let first = p.next("date or year")?;
    let first_dashes = first.chars().filter(|c| *c == '-').count();
    if first_dashes >= 2 {
        let date: NaiveDate = first.parse()?;
        p.finish()?;
        return Ok(BookParams::U1(date));
    }
    let year: i32 = first.parse()?;
    let month: u32 = p.next("month")?.parse()?;
    p.finish()?;
    Ok(BookParams::U0(year, month))
}

impl From<&BookParams> for MarkdownString {
    fn from(p: &BookParams) -> MarkdownString {
        let mut s = MarkdownString::new();
        match p {
            BookParams::M0
            | BookParams::C0()
            | BookParams::L0(_)
            | BookParams::U0(_, _)
            | BookParams::U1(_) => {}

            BookParams::C1(t) => {
                s.push(&markdown_format!("👨\\-🏫 Teacher: {}\n", t.to_string()));
            }

            BookParams::C2(t, st)
            | BookParams::C3(t, st, _)
            | BookParams::C4(t, st, _, _)
            | BookParams::C5(t, st, _, _, _) => {
                s.push(&markdown_format!("🏫 Teacher: {}\n", t.to_string()));
                s.push(&markdown_format!("🎓 Student: {}\n", st.to_string()));
            }

            BookParams::C6(t, st, date) => {
                s.push(&markdown_format!("🏫 Teacher: {}\n", t.to_string()));
                s.push(&markdown_format!("🎓 Student: {}\n", st.to_string()));
                s.push(&markdown_format!("📆 Date: {}\n", date.to_string()));
            }

            BookParams::C7(t, st, date, time)
            | BookParams::D0(t, st, date, time)
            | BookParams::DF(t, st, date, time)
            | BookParams::S0(t, st, date, time) => {
                s.push(&markdown_format!("🏫 Teacher: {}\n", t.to_string()));
                s.push(&markdown_format!("🎓 Student: {}\n", st.to_string()));
                s.push(&markdown_format!("📆 Date: {}\n", date.to_string()));
                s.push(&markdown_format!(
                    "⏰ Time: {}\n",
                    time.format("%H:%M").to_string()
                ));
            }

            BookParams::L1(t, st, date, time) => {
                s.push(&markdown_format!("🏫 Teacher: {}\n", t.to_string()));
                s.push(&markdown_format!("🎓 Student: {}\n", st.to_string()));
                s.push(&markdown_format!("📆 Date: {}\n", date.to_string()));
                s.push(&markdown_format!(
                    "⏰ Time: {}\n",
                    time.format("%H:%M").to_string()
                ));
            }

            BookParams::C8(t, st, date, time, dur) | BookParams::CF(t, st, date, time, dur) => {
                s.push(&markdown_format!("🏫 Teacher: {}\n", t.to_string()));
                s.push(&markdown_format!("🎓 Student: {}\n", st.to_string()));
                s.push(&markdown_format!("📆 Date: {}\n", date.to_string()));
                s.push(&markdown_format!(
                    "⏰ Time: {}\n",
                    time.format("%H:%M").to_string()
                ));
                s.push(&markdown_format!("⏱ Duration: {}\n", dur.to_string()));
            }

            BookParams::R1(t, st, od, ot, _, _) => {
                s.push(&markdown_format!("🏫 Teacher: {}\n", t.to_string()));
                s.push(&markdown_format!("🎓 Student: {}\n", st.to_string()));
                s.push(&markdown_format!("📆 Date: {}\n", od.to_string()));
                s.push(&markdown_format!(
                    "⏰ Time: {}\n",
                    ot.format("%H:%M").to_string()
                ));
            }

            BookParams::R2(t, st, od, ot, nd) => {
                s.push(&markdown_format!("🏫 Teacher: {}\n", t.to_string()));
                s.push(&markdown_format!("🎓 Student: {}\n", st.to_string()));
                s.push(&markdown_format!("📆 Old date: {}\n", od.to_string()));
                s.push(&markdown_format!(
                    "⏰ Old time: {}\n",
                    ot.format("%H:%M").to_string()
                ));
                s.push(&markdown_format!("📆 New date: {}\n", nd.to_string()));
            }

            BookParams::R3(t, st, od, ot, nd, nt) | BookParams::RF(t, st, od, ot, nd, nt) => {
                s.push(&markdown_format!("🏫 Teacher: {}\n", t.to_string()));
                s.push(&markdown_format!("🎓 Student: {}\n", st.to_string()));
                s.push(&markdown_format!("📆 Old date: {}\n", od.to_string()));
                s.push(&markdown_format!(
                    "⏰ Old time: {}\n",
                    ot.format("%H:%M").to_string()
                ));
                s.push(&markdown_format!("📆 New date: {}\n", nd.to_string()));
                s.push(&markdown_format!(
                    "⏰ New time: {}\n",
                    nt.format("%H:%M").to_string()
                ));
            }

            BookParams::SF(t, st, date, time, status) => {
                s.push(&markdown_format!("🏫 Teacher: {}\n", t.to_string()));
                s.push(&markdown_format!("🎓 Student: {}\n", st.to_string()));
                s.push(&markdown_format!("📆 Date: {}\n", date.to_string()));
                s.push(&markdown_format!(
                    "⏰ Time: {}\n",
                    time.format("%H:%M").to_string()
                ));
                s.push(&markdown_format!("📊 Status: {}\n", status.to_string()));
            }
        }
        s
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
    let student: TelegramName = p
        .next("student")?
        .parse()
        .map_err(|_| anyhow::anyhow!("expected student TelegramName"))?;
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
