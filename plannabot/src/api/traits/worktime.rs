use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate, NaiveTime, Weekday};
use telluride::utils::{format_screen_spaces, split_with_screened_spaces, ParamParser};

pub enum WorktimeBranch {
    Weekday,
    Exception,
}

impl fmt::Display for WorktimeBranch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorktimeBranch::Weekday => write!(f, "weekday"),
            WorktimeBranch::Exception => write!(f, "exception"),
        }
    }
}

impl FromStr for WorktimeBranch {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "weekday" => Ok(WorktimeBranch::Weekday),
            "exception" => Ok(WorktimeBranch::Exception),
            _ => Err(anyhow::anyhow!("unknown worktime branch: {}", s)),
        }
    }
}

pub enum WorktimeSubcmd {
    Add(bool),
    Remove(bool),
}

impl fmt::Display for WorktimeSubcmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorktimeSubcmd::Add(false) => write!(f, "add"),
            WorktimeSubcmd::Add(true) => write!(f, "add!"),
            WorktimeSubcmd::Remove(false) => write!(f, "remove"),
            WorktimeSubcmd::Remove(true) => write!(f, "remove!"),
        }
    }
}

impl FromStr for WorktimeSubcmd {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "add" => Ok(WorktimeSubcmd::Add(false)),
            "add!" => Ok(WorktimeSubcmd::Add(true)),
            "remove" => Ok(WorktimeSubcmd::Remove(false)),
            "remove!" => Ok(WorktimeSubcmd::Remove(true)),
            _ => Err(anyhow::anyhow!("unknown worktime subcommand: {}", s)),
        }
    }
}

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
    /// Weekday remove confirmation: weekday + start (unique ID; full period looked up at render)
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
    /// Exception remove confirmation: date + start (unique ID; full period looked up at render)
    ERDH(NaiveDate, NaiveTime),
    /// Exception remove forced: execute delete
    ERDHF(NaiveDate, NaiveTime),
}

impl fmt::Display for WorktimeParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let wd = WorktimeBranch::Weekday;
        let ex = WorktimeBranch::Exception;
        let add = WorktimeSubcmd::Add(false);
        let add_f = WorktimeSubcmd::Add(true);
        let rm = WorktimeSubcmd::Remove(false);
        let rm_f = WorktimeSubcmd::Remove(true);
        match self {
            WorktimeParams::M0 => String::new(),
            WorktimeParams::WL => format_screen_spaces!(wd),
            WorktimeParams::WA => format_screen_spaces!(wd, add),
            WorktimeParams::WAW(w) => format_screen_spaces!(wd, add, weekday_short(*w)),
            WorktimeParams::WAWH(w, s) => {
                format_screen_spaces!(wd, add, weekday_short(*w), s.format("%H:%M"))
            }
            WorktimeParams::WAWHH(w, s, e) => format_screen_spaces!(
                wd, add, weekday_short(*w), s.format("%H:%M"), e.format("%H:%M")
            ),
            WorktimeParams::WAWHHF(w, s, e) => format_screen_spaces!(
                wd, add_f, weekday_short(*w), s.format("%H:%M"), e.format("%H:%M")
            ),
            WorktimeParams::WR => format_screen_spaces!(wd, rm),
            WorktimeParams::WRWH(w, s) => {
                format_screen_spaces!(wd, rm, weekday_short(*w), s.format("%H:%M"))
            }
            WorktimeParams::WRWHF(w, s) => {
                format_screen_spaces!(wd, rm_f, weekday_short(*w), s.format("%H:%M"))
            }
            WorktimeParams::EYM(y, m) => format_screen_spaces!(ex, y, m),
            WorktimeParams::EAYM(y, m) => format_screen_spaces!(ex, add, y, m),
            WorktimeParams::ED(d) => format_screen_spaces!(ex, d.format("%Y-%m-%d")),
            WorktimeParams::EDH(d, s) => {
                format_screen_spaces!(ex, d.format("%Y-%m-%d"), s.format("%H:%M"))
            }
            WorktimeParams::EDHH(d, s, e) => format_screen_spaces!(
                ex, d.format("%Y-%m-%d"), s.format("%H:%M"), e.format("%H:%M")
            ),
            WorktimeParams::EDHHF(d, s, e) => format_screen_spaces!(
                ex, add_f, d.format("%Y-%m-%d"), s.format("%H:%M"), e.format("%H:%M")
            ),
            WorktimeParams::ERYM(y, m) => format_screen_spaces!(ex, rm, y, m),
            WorktimeParams::ERDH(d, s) => {
                format_screen_spaces!(ex, rm, d.format("%Y-%m-%d"), s.format("%H:%M"))
            }
            WorktimeParams::ERDHF(d, s) => {
                format_screen_spaces!(ex, rm_f, d.format("%Y-%m-%d"), s.format("%H:%M"))
            }
        }
        .fmt(f)
    }
}


impl FromStr for WorktimeParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);
        let now = Local::now();
        let cur_year = now.year();
        let cur_month = now.month();

        let Some(first) = parts.first() else {
            return Ok(WorktimeParams::M0);
        };

        let branch: WorktimeBranch = first.parse()?;
        let mut p = ParamParser::new(&parts, 1);

        match branch {
            WorktimeBranch::Weekday => {
                let Some(tok) = p.peek() else {
                    return Ok(WorktimeParams::WL);
                };
                let subcmd: WorktimeSubcmd = tok.parse()?;
                p.next("subcmd")?;
                match subcmd {
                    WorktimeSubcmd::Add(forced) => match p.peek() {
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
                    },
                    WorktimeSubcmd::Remove(forced) => match p.peek() {
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
                    },
                }
            }
            WorktimeBranch::Exception => {
                let Some(tok) = p.peek() else {
                    return Ok(WorktimeParams::EYM(cur_year, cur_month));
                };
                // Peek: could be a subcmd, a YYYY-MM-DD date, or an integer year
                if let Ok(subcmd) = tok.parse::<WorktimeSubcmd>() {
                    p.next("subcmd")?;
                    match subcmd {
                        WorktimeSubcmd::Add(forced) => {
                            if forced {
                                let d: NaiveDate = p.next("date")?.parse()?;
                                let s: NaiveTime = p.next("start")?.parse()?;
                                let e: NaiveTime = p.next("end")?.parse()?;
                                Ok(WorktimeParams::EDHHF(d, s, e))
                            } else {
                                let y: i32 = p.next("year")?.parse().map_err(|_| anyhow::anyhow!("invalid year"))?;
                                let m: u32 = p.next("month")?.parse().map_err(|_| anyhow::anyhow!("invalid month"))?;
                                Ok(WorktimeParams::EAYM(y, m))
                            }
                        }
                        WorktimeSubcmd::Remove(forced) => match p.peek() {
                            None => {
                                if forced {
                                    Err(anyhow::anyhow!("remove! requires date"))
                                } else {
                                    Ok(WorktimeParams::ERYM(cur_year, cur_month))
                                }
                            }
                            Some(tok) if tok.contains('-') => {
                                let d: NaiveDate = p.next("date")?.parse()?;
                                let s: NaiveTime = p.next("start")?.parse()?;
                                if forced {
                                    Ok(WorktimeParams::ERDHF(d, s))
                                } else {
                                    Ok(WorktimeParams::ERDH(d, s))
                                }
                            }
                            _ => {
                                let y: i32 = p.next("year")?.parse().map_err(|_| anyhow::anyhow!("invalid year"))?;
                                let m: u32 = p.next("month")?.parse().map_err(|_| anyhow::anyhow!("invalid month"))?;
                                if forced {
                                    Err(anyhow::anyhow!("remove! requires a date, not year/month"))
                                } else {
                                    Ok(WorktimeParams::ERYM(y, m))
                                }
                            }
                        },
                    }
                } else if tok.contains('-') {
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
                } else {
                    // Integer year + month
                    let y: i32 = p.next("year")?.parse().map_err(|_| anyhow::anyhow!("invalid year"))?;
                    let m: u32 = p.next("month")?.parse().map_err(|_| anyhow::anyhow!("invalid month"))?;
                    Ok(WorktimeParams::EYM(y, m))
                }
            }
        }
    }
}
