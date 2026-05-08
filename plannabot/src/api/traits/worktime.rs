use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate, NaiveTime, Weekday};
use telluride::utils::{format_screen_spaces, split_with_screened_spaces};

fn weekday_short(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

fn parse_weekday(s: &str) -> Result<Weekday> {
    match s.to_lowercase().as_str() {
        "mon" | "monday" | "1" => Ok(Weekday::Mon),
        "tue" | "tuesday" | "2" => Ok(Weekday::Tue),
        "wed" | "wednesday" | "3" => Ok(Weekday::Wed),
        "thu" | "thursday" | "4" => Ok(Weekday::Thu),
        "fri" | "friday" | "5" => Ok(Weekday::Fri),
        "sat" | "saturday" | "6" => Ok(Weekday::Sat),
        "sun" | "sunday" | "7" | "0" => Ok(Weekday::Sun),
        other => Err(anyhow::anyhow!("unknown weekday: {}", other)),
    }
}

pub enum WorktimeSubcmd {
    Weekday,
    WeekdayAdd(bool),
    WeekdayRemove(bool),
    Exception,
    ExceptionAdd(bool),
    ExceptionRemove(bool),
}

impl fmt::Display for WorktimeSubcmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorktimeSubcmd::Weekday => write!(f, "weekday"),
            WorktimeSubcmd::WeekdayAdd(false) => write!(f, "weekday add"),
            WorktimeSubcmd::WeekdayAdd(true) => write!(f, "weekday add!"),
            WorktimeSubcmd::WeekdayRemove(false) => write!(f, "weekday remove"),
            WorktimeSubcmd::WeekdayRemove(true) => write!(f, "weekday remove!"),
            WorktimeSubcmd::Exception => write!(f, "exception"),
            WorktimeSubcmd::ExceptionAdd(false) => write!(f, "exception add"),
            WorktimeSubcmd::ExceptionAdd(true) => write!(f, "exception add!"),
            WorktimeSubcmd::ExceptionRemove(false) => write!(f, "exception remove"),
            WorktimeSubcmd::ExceptionRemove(true) => write!(f, "exception remove!"),
        }
    }
}

#[allow(non_camel_case_types)]
pub enum WorktimeParams {
    /// Main menu: Week days / Exceptions
    M0,
    /// Weekday list view
    WL,
    /// Weekday add step 0: select weekday
    WA,
    /// Weekday add step 1: weekday selected, select start hour
    WAW(Weekday),
    /// Weekday add step 2: start selected, select end hour
    WAWH(Weekday, NaiveTime),
    /// Weekday add step 3: both times selected, show inline-query button
    WAWHH(Weekday, NaiveTime, NaiveTime),
    /// Weekday add forced: execute insert
    WAWHHF(Weekday, NaiveTime, NaiveTime),
    /// Weekday remove: show list of entries
    WR,
    /// Weekday remove confirmation: weekday + start identified
    WRWH(Weekday, NaiveTime),
    /// Weekday remove forced: execute delete
    WRWHF(Weekday, NaiveTime),
    /// Exception list for year/month
    EYM(i32, u32),
    /// Exception add: show calendar for year/month
    EAYM(i32, u32),
    /// Exception date selected for add: select start hour
    ED(NaiveDate),
    /// Exception add step 2: start selected, select end hour
    EDH(NaiveDate, NaiveTime),
    /// Exception add step 3: both times, show inline-query button
    EDHH(NaiveDate, NaiveTime, NaiveTime),
    /// Exception add forced: execute insert
    EDHHF(NaiveDate, NaiveTime, NaiveTime),
    /// Exception remove list for year/month
    ERYM(i32, u32),
    /// Exception remove confirmation for a date
    ERD(NaiveDate),
    /// Exception remove forced: execute delete
    ERDF(NaiveDate),
}

impl fmt::Display for WorktimeParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let wd = WorktimeSubcmd::Weekday;
        let wa = WorktimeSubcmd::WeekdayAdd(false);
        let waf = WorktimeSubcmd::WeekdayAdd(true);
        let wr = WorktimeSubcmd::WeekdayRemove(false);
        let wrf = WorktimeSubcmd::WeekdayRemove(true);
        let ex = WorktimeSubcmd::Exception;
        let ea = WorktimeSubcmd::ExceptionAdd(false);
        let eaf = WorktimeSubcmd::ExceptionAdd(true);
        let er = WorktimeSubcmd::ExceptionRemove(false);
        let erf = WorktimeSubcmd::ExceptionRemove(true);
        match self {
            WorktimeParams::M0 => String::new(),
            WorktimeParams::WL => format_screen_spaces!(wd),
            WorktimeParams::WA => format_screen_spaces!(wa),
            WorktimeParams::WAW(w) => format_screen_spaces!(wa, weekday_short(*w)),
            WorktimeParams::WAWH(w, s) => {
                format_screen_spaces!(wa, weekday_short(*w), s.format("%H:%M"))
            }
            WorktimeParams::WAWHH(w, s, e) => {
                format_screen_spaces!(wa, weekday_short(*w), s.format("%H:%M"), e.format("%H:%M"))
            }
            WorktimeParams::WAWHHF(w, s, e) => {
                format_screen_spaces!(waf, weekday_short(*w), s.format("%H:%M"), e.format("%H:%M"))
            }
            WorktimeParams::WR => format_screen_spaces!(wr),
            WorktimeParams::WRWH(w, s) => {
                format_screen_spaces!(wr, weekday_short(*w), s.format("%H:%M"))
            }
            WorktimeParams::WRWHF(w, s) => {
                format_screen_spaces!(wrf, weekday_short(*w), s.format("%H:%M"))
            }
            WorktimeParams::EYM(y, m) => format_screen_spaces!(ex, y, m),
            WorktimeParams::EAYM(y, m) => format_screen_spaces!(ea, y, m),
            WorktimeParams::ED(d) => format_screen_spaces!(ex, d.format("%Y-%m-%d")),
            WorktimeParams::EDH(d, s) => {
                format_screen_spaces!(ex, d.format("%Y-%m-%d"), s.format("%H:%M"))
            }
            WorktimeParams::EDHH(d, s, e) => {
                format_screen_spaces!(
                    ex,
                    d.format("%Y-%m-%d"),
                    s.format("%H:%M"),
                    e.format("%H:%M")
                )
            }
            WorktimeParams::EDHHF(d, s, e) => {
                format_screen_spaces!(
                    eaf,
                    d.format("%Y-%m-%d"),
                    s.format("%H:%M"),
                    e.format("%H:%M")
                )
            }
            WorktimeParams::ERYM(y, m) => format_screen_spaces!(er, y, m),
            WorktimeParams::ERD(d) => format_screen_spaces!(er, d.format("%Y-%m-%d")),
            WorktimeParams::ERDF(d) => format_screen_spaces!(erf, d.format("%Y-%m-%d")),
        }
        .fmt(f)
    }
}

struct P<'a> {
    parts: &'a [String],
    pos: usize,
}

impl<'a> P<'a> {
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
    fn peek(&self) -> Option<&str> {
        self.parts.get(self.pos).map(|s| s.as_str())
    }
}

impl FromStr for WorktimeParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);
        let now = Local::now();
        let cur_year = now.year();
        let cur_month = now.month();

        let Some(branch) = parts.first() else {
            return Ok(WorktimeParams::M0);
        };

        match branch.as_str() {
            "weekday" => {
                let mut p = P::new(&parts, 1);
                match p.peek() {
                    None => Ok(WorktimeParams::WL),
                    Some("add" | "add!") => {
                        let forced = p.next("subcmd")? == "add!";
                        match p.peek() {
                            None => {
                                if forced {
                                    Err(anyhow::anyhow!("add! requires weekday start end"))
                                } else {
                                    Ok(WorktimeParams::WA)
                                }
                            }
                            _ => {
                                let w = parse_weekday(p.next("weekday")?)?;
                                match p.peek() {
                                    None => Ok(WorktimeParams::WAW(w)),
                                    _ => {
                                        let s: NaiveTime = p.next("start")?.parse()?;
                                        match p.peek() {
                                            None => Ok(WorktimeParams::WAWH(w, s)),
                                            _ => {
                                                let e: NaiveTime = p.next("end")?.parse()?;
                                                if forced {
                                                    Ok(WorktimeParams::WAWHHF(w, s, e))
                                                } else {
                                                    Ok(WorktimeParams::WAWHH(w, s, e))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Some("remove" | "remove!") => {
                        let forced = p.next("subcmd")? == "remove!";
                        match p.peek() {
                            None => {
                                if forced {
                                    Err(anyhow::anyhow!("remove! requires weekday start"))
                                } else {
                                    Ok(WorktimeParams::WR)
                                }
                            }
                            _ => {
                                let w = parse_weekday(p.next("weekday")?)?;
                                let s: NaiveTime = p.next("start")?.parse()?;
                                if forced {
                                    Ok(WorktimeParams::WRWHF(w, s))
                                } else {
                                    Ok(WorktimeParams::WRWH(w, s))
                                }
                            }
                        }
                    }
                    Some(other) => Err(anyhow::anyhow!("unknown weekday subcommand: {}", other)),
                }
            }
            "exception" => {
                let mut p = P::new(&parts, 1);
                match p.peek() {
                    None => Ok(WorktimeParams::EYM(cur_year, cur_month)),
                    Some("add" | "add!") => {
                        let forced = p.next("subcmd")? == "add!";
                        if forced {
                            let d: NaiveDate = p.next("date")?.parse()?;
                            let s: NaiveTime = p.next("start")?.parse()?;
                            let e: NaiveTime = p.next("end")?.parse()?;
                            Ok(WorktimeParams::EDHHF(d, s, e))
                        } else {
                            let y: i32 = p
                                .next("year")
                                .and_then(|s| s.parse().map_err(|_| anyhow::anyhow!("invalid year")))?;
                            let m: u32 = p
                                .next("month")
                                .and_then(|s| s.parse().map_err(|_| anyhow::anyhow!("invalid month")))?;
                            Ok(WorktimeParams::EAYM(y, m))
                        }
                    }
                    Some("remove" | "remove!") => {
                        let forced = p.next("subcmd")? == "remove!";
                        // Peek at next token: could be YYYY-MM-DD or integer year
                        match p.peek() {
                            None => {
                                if forced {
                                    Err(anyhow::anyhow!("remove! requires date"))
                                } else {
                                    Ok(WorktimeParams::ERYM(cur_year, cur_month))
                                }
                            }
                            Some(tok) if tok.contains('-') => {
                                let d: NaiveDate = p.next("date")?.parse()?;
                                if forced {
                                    Ok(WorktimeParams::ERDF(d))
                                } else {
                                    Ok(WorktimeParams::ERD(d))
                                }
                            }
                            _ => {
                                let y: i32 = p
                                    .next("year")
                                    .and_then(|s| s.parse().map_err(|_| anyhow::anyhow!("invalid year")))?;
                                let m: u32 = p
                                    .next("month")
                                    .and_then(|s| s.parse().map_err(|_| anyhow::anyhow!("invalid month")))?;
                                if forced {
                                    Err(anyhow::anyhow!("remove! requires a date, not year/month"))
                                } else {
                                    Ok(WorktimeParams::ERYM(y, m))
                                }
                            }
                        }
                    }
                    Some(tok) if tok.contains('-') => {
                        // YYYY-MM-DD date
                        let d: NaiveDate = p.next("date")?.parse()?;
                        match p.peek() {
                            None => Ok(WorktimeParams::ED(d)),
                            _ => {
                                let s: NaiveTime = p.next("start")?.parse()?;
                                match p.peek() {
                                    None => Ok(WorktimeParams::EDH(d, s)),
                                    _ => {
                                        let e: NaiveTime = p.next("end")?.parse()?;
                                        Ok(WorktimeParams::EDHH(d, s, e))
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        // Integer year + month
                        let y: i32 = p
                            .next("year")
                            .and_then(|s| s.parse().map_err(|_| anyhow::anyhow!("invalid year")))?;
                        let m: u32 = p
                            .next("month")
                            .and_then(|s| s.parse().map_err(|_| anyhow::anyhow!("invalid month")))?;
                        Ok(WorktimeParams::EYM(y, m))
                    }
                }
            }
            other => Err(anyhow::anyhow!("unknown worktime branch: {}", other)),
        }
    }
}
