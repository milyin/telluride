use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use telluride::utils::{split_with_screened_spaces, ParamParser};

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
    /// Edit flow: show prefilled cost/duration form for this student
    PEN(TelegramName),
    /// Edit! forced: execute update pairing (name, duration, cost)
    PENF(TelegramName, Duration, i64),
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
        let edit_f = PairingSubcmd::Edit(true);
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
            PairingParams::PENF(name, dur, cost) => write!(f, "{edit_f} {name} {dur} {cost}"),
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
                Some(s) => Ok(PairingParams::PE(s.parse().unwrap_or(0))),
            },
            PairingSubcmd::Edit(true) => {
                let name: TelegramName = p.next("@username")?.parse()?;
                let dur: Duration = p.next("duration")?.parse()?;
                let cost: i64 = p.next("cost")?.parse()?;
                p.finish()?;
                Ok(PairingParams::PENF(name, dur, cost))
            }
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
