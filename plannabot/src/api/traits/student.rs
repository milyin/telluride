use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use telluride::utils::{ParamParser, split_with_screened_spaces};

use crate::models::TelegramName;
use crate::types::Duration;

pub enum PairingSubcmd {
    Add(bool),
    Edit(bool),
    Remove(bool),
}

impl fmt::Display for PairingSubcmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PairingSubcmd::Add(false) => write!(f, "add"),
            PairingSubcmd::Add(true) => write!(f, "add!"),
            PairingSubcmd::Edit(false) => write!(f, "edit"),
            PairingSubcmd::Edit(true) => write!(f, "edit!"),
            PairingSubcmd::Remove(false) => write!(f, "remove"),
            PairingSubcmd::Remove(true) => write!(f, "remove!"),
        }
    }
}

impl FromStr for PairingSubcmd {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "add" => Ok(PairingSubcmd::Add(false)),
            "add!" => Ok(PairingSubcmd::Add(true)),
            "edit" => Ok(PairingSubcmd::Edit(false)),
            "edit!" => Ok(PairingSubcmd::Edit(true)),
            "remove" => Ok(PairingSubcmd::Remove(false)),
            "remove!" => Ok(PairingSubcmd::Remove(true)),
            _ => Err(anyhow::anyhow!("unknown student subcommand: {}", s)),
        }
    }
}

#[allow(non_camel_case_types)]
pub enum PairingParams {
    /// No args: show teacher's student list with Add/Edit/Remove buttons
    P0,
    /// Add flow: show all students paginated (page)
    PA(i32),
    /// Add flow: show cost/duration form for this student
    PAN(TelegramName),
    /// Add! forced: execute add pairing (name, duration, cost)
    PANF(TelegramName, Duration, i64),
    /// Edit flow: show paired students paginated (page)
    PE(i32),
    /// Edit flow: show 3-button choice (lesson/zoom/board) for this student
    PEN(TelegramName),
    /// Edit → Lesson: show prefilled duration/cost form
    PENL(TelegramName),
    /// Edit → Lesson! forced: execute lesson update (name, duration, cost)
    PENLF(TelegramName, Duration, i64),
    /// Edit → Zoom: show prefilled zoom URL form
    PENZ(TelegramName),
    /// Edit → Zoom! forced: execute zoom URL update (name, url)
    PENZF(TelegramName, String),
    /// Edit → Board: show prefilled board URL form
    PENB(TelegramName),
    /// Edit → Board! forced: execute board URL update (name, url)
    PENBF(TelegramName, String),
    /// Remove flow: show paired students paginated (page)
    PR(i32),
    /// Remove flow: show confirmation for this student
    PRN(TelegramName),
    /// Remove! forced: execute remove pairing
    PRNF(TelegramName),
}

impl fmt::Display for PairingParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let add = PairingSubcmd::Add(false);
        let add_f = PairingSubcmd::Add(true);
        let edit = PairingSubcmd::Edit(false);
        let remove = PairingSubcmd::Remove(false);
        let remove_f = PairingSubcmd::Remove(true);

        match self {
            PairingParams::P0 => write!(f, ""),
            PairingParams::PA(0) => write!(f, "{add}"),
            PairingParams::PA(page) => write!(f, "{add} {page}"),
            PairingParams::PAN(name) => write!(f, "{add} {name}"),
            PairingParams::PANF(name, dur, cost) => write!(f, "{add_f} {name} {dur} {cost}"),
            PairingParams::PE(0) => write!(f, "{edit}"),
            PairingParams::PE(page) => write!(f, "{edit} {page}"),
            PairingParams::PEN(name) => write!(f, "{edit} {name}"),
            PairingParams::PENL(name) => write!(f, "{edit} lesson {name}"),
            PairingParams::PENLF(name, dur, cost) => {
                write!(f, "{edit} lesson! {name} {dur} {cost}")
            }
            PairingParams::PENZ(name) => write!(f, "{edit} zoom {name}"),
            PairingParams::PENZF(name, url) => write!(f, "{edit} zoom! {name} {url}"),
            PairingParams::PENB(name) => write!(f, "{edit} board {name}"),
            PairingParams::PENBF(name, url) => write!(f, "{edit} board! {name} {url}"),
            PairingParams::PR(0) => write!(f, "{remove}"),
            PairingParams::PR(page) => write!(f, "{remove} {page}"),
            PairingParams::PRN(name) => write!(f, "{remove} {name}"),
            PairingParams::PRNF(name) => write!(f, "{remove_f} {name}"),
        }
    }
}

impl FromStr for PairingParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);
        let mut p = ParamParser::new(&parts, 0);

        let Some(first) = p.next_opt() else {
            return Ok(PairingParams::P0);
        };

        let subcmd: PairingSubcmd = first.parse()?;

        match subcmd {
            PairingSubcmd::Add(false) => match p.next_opt() {
                None => Ok(PairingParams::PA(0)),
                Some(s) if s.starts_with('@') => Ok(PairingParams::PAN(s.parse()?)),
                Some(s) => Ok(PairingParams::PA(s.parse().unwrap_or(0))),
            },
            PairingSubcmd::Add(true) => {
                let name: TelegramName = p.next("@username")?.parse()?;
                let dur: Duration = p.next("duration")?.parse()?;
                let cost: i64 = p.next("cost")?.parse()?;
                p.finish()?;
                Ok(PairingParams::PANF(name, dur, cost))
            }
            PairingSubcmd::Edit(false) => match p.next_opt() {
                None => Ok(PairingParams::PE(0)),
                Some(s) if s.starts_with('@') => Ok(PairingParams::PEN(s.parse()?)),
                Some(s) if s == "lesson" => {
                    let name: TelegramName = p.next("@username")?.parse()?;
                    Ok(PairingParams::PENL(name))
                }
                Some(s) if s == "lesson!" => {
                    let name: TelegramName = p.next("@username")?.parse()?;
                    let dur: Duration = p.next("duration")?.parse()?;
                    let cost: i64 = p.next("cost")?.parse()?;
                    p.finish()?;
                    Ok(PairingParams::PENLF(name, dur, cost))
                }
                Some(s) if s == "zoom" => {
                    let name: TelegramName = p.next("@username")?.parse()?;
                    Ok(PairingParams::PENZ(name))
                }
                Some(s) if s == "zoom!" => {
                    let name: TelegramName = p.next("@username")?.parse()?;
                    let url = p.next("url")?.to_string();
                    p.finish()?;
                    Ok(PairingParams::PENZF(name, url))
                }
                Some(s) if s == "board" => {
                    let name: TelegramName = p.next("@username")?.parse()?;
                    Ok(PairingParams::PENB(name))
                }
                Some(s) if s == "board!" => {
                    let name: TelegramName = p.next("@username")?.parse()?;
                    let url = p.next("url")?.to_string();
                    p.finish()?;
                    Ok(PairingParams::PENBF(name, url))
                }
                Some(s) => Ok(PairingParams::PE(s.parse().unwrap_or(0))),
            },
            PairingSubcmd::Edit(true) => Err(anyhow::anyhow!(
                "use 'edit lesson!' / 'edit zoom!' / 'edit board!' instead of 'edit!'"
            )),
            PairingSubcmd::Remove(false) => match p.next_opt() {
                None => Ok(PairingParams::PR(0)),
                Some(s) if s.starts_with('@') => Ok(PairingParams::PRN(s.parse()?)),
                Some(s) => Ok(PairingParams::PR(s.parse().unwrap_or(0))),
            },
            PairingSubcmd::Remove(true) => {
                let name: TelegramName = p.next("@username")?.parse()?;
                p.finish()?;
                Ok(PairingParams::PRNF(name))
            }
        }
    }
}

pub trait StudentCommand: Sized + Clone {
    fn student(params: PairingParams) -> Self;
}
