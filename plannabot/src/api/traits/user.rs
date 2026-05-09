use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use telluride::utils::split_with_screened_spaces;

use crate::models::TelegramName;

pub enum UserRole {
    Student,
    Teacher,
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserRole::Student => write!(f, "student"),
            UserRole::Teacher => write!(f, "teacher"),
        }
    }
}

impl FromStr for UserRole {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "student" => Ok(UserRole::Student),
            "teacher" => Ok(UserRole::Teacher),
            _ => Err(anyhow::anyhow!("unknown role: {}", s)),
        }
    }
}

pub enum UserSubcmd {
    Add(bool),
    Delete(bool),
    Edit(bool),
}

impl fmt::Display for UserSubcmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserSubcmd::Add(false) => write!(f, "add"),
            UserSubcmd::Add(true) => write!(f, "add!"),
            UserSubcmd::Delete(false) => write!(f, "delete"),
            UserSubcmd::Delete(true) => write!(f, "delete!"),
            UserSubcmd::Edit(false) => write!(f, "edit"),
            UserSubcmd::Edit(true) => write!(f, "edit!"),
        }
    }
}

impl FromStr for UserSubcmd {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "add" => Ok(UserSubcmd::Add(false)),
            "add!" => Ok(UserSubcmd::Add(true)),
            "delete" => Ok(UserSubcmd::Delete(false)),
            "delete!" => Ok(UserSubcmd::Delete(true)),
            "edit" => Ok(UserSubcmd::Edit(false)),
            "edit!" => Ok(UserSubcmd::Edit(true)),
            _ => Err(anyhow::anyhow!("unknown user subcommand: {}", s)),
        }
    }
}

#[allow(non_camel_case_types)]
pub enum UserParams {
    /// No args: show student/teacher role selection
    U0,
    /// Student submenu: Add / Edit / Delete
    US,
    /// Teacher submenu: Add / Edit / Delete
    UT,
    /// Student add: show instructions + template button
    USA,
    /// Teacher add: show instructions + template button
    UTA,
    /// Student add! forced: execute insertion (@u, tz, currency, name)
    USAF(TelegramName, String, String, String),
    /// Teacher add! forced: execute insertion (@u, tz, name)
    UTAF(TelegramName, String, String),
    /// Student delete: show list
    USD,
    /// Teacher delete: show list
    UTD,
    /// Student delete with name: show full info + confirm button
    USDN(TelegramName),
    /// Teacher delete with name: show full info + confirm button
    UTDN(TelegramName),
    /// Student delete! forced: execute deletion
    USDNF(TelegramName),
    /// Teacher delete! forced: execute deletion
    UTDNF(TelegramName),
    /// Student edit: show list
    USE,
    /// Teacher edit: show list
    UTE,
    /// Student edit with name: show full info + pre-filled edit button
    USEN(TelegramName),
    /// Teacher edit with name: show full info + pre-filled edit button
    UTEN(TelegramName),
    /// Student edit! forced: execute edit (@u, tz, currency, name)
    USENF(TelegramName, String, String, String),
    /// Teacher edit! forced: execute edit (@u, tz, name)
    UTENF(TelegramName, String, String),
}

impl fmt::Display for UserParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let student = UserRole::Student;
        let teacher = UserRole::Teacher;
        let add = UserSubcmd::Add(false);
        let add_f = UserSubcmd::Add(true);
        let del = UserSubcmd::Delete(false);
        let del_f = UserSubcmd::Delete(true);
        let edit = UserSubcmd::Edit(false);
        let edit_f = UserSubcmd::Edit(true);

        match self {
            UserParams::U0 => write!(f, ""),
            UserParams::US => write!(f, "{student}"),
            UserParams::UT => write!(f, "{teacher}"),
            UserParams::USA => write!(f, "{student} {add}"),
            UserParams::UTA => write!(f, "{teacher} {add}"),
            UserParams::USAF(name, tz, currency, display_name) => {
                write!(f, "{student} {add_f} {name} {tz} {currency} {display_name}")
            }
            UserParams::UTAF(name, tz, display_name) => {
                write!(f, "{teacher} {add_f} {name} {tz} {display_name}")
            }
            UserParams::USD => write!(f, "{student} {del}"),
            UserParams::UTD => write!(f, "{teacher} {del}"),
            UserParams::USDN(name) => write!(f, "{student} {del} {name}"),
            UserParams::UTDN(name) => write!(f, "{teacher} {del} {name}"),
            UserParams::USDNF(name) => write!(f, "{student} {del_f} {name}"),
            UserParams::UTDNF(name) => write!(f, "{teacher} {del_f} {name}"),
            UserParams::USE => write!(f, "{student} {edit}"),
            UserParams::UTE => write!(f, "{teacher} {edit}"),
            UserParams::USEN(name) => write!(f, "{student} {edit} {name}"),
            UserParams::UTEN(name) => write!(f, "{teacher} {edit} {name}"),
            UserParams::USENF(name, tz, currency, display_name) => {
                write!(f, "{student} {edit_f} {name} {tz} {currency} {display_name}")
            }
            UserParams::UTENF(name, tz, display_name) => {
                write!(f, "{teacher} {edit_f} {name} {tz} {display_name}")
            }
        }
    }
}

impl FromStr for UserParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);

        let Some(first) = parts.first() else {
            return Ok(UserParams::U0);
        };

        let role: UserRole = first.parse()?;

        let Some(second) = parts.get(1) else {
            return Ok(match role {
                UserRole::Student => UserParams::US,
                UserRole::Teacher => UserParams::UT,
            });
        };

        let subcmd: UserSubcmd = second.parse()?;

        match (role, subcmd) {
            (UserRole::Student, UserSubcmd::Add(false)) => Ok(UserParams::USA),
            (UserRole::Teacher, UserSubcmd::Add(false)) => Ok(UserParams::UTA),

            (UserRole::Student, UserSubcmd::Add(true)) => {
                let name: TelegramName = parts
                    .get(2)
                    .ok_or_else(|| anyhow::anyhow!("missing @username"))?
                    .parse()?;
                let tz = parts
                    .get(3)
                    .ok_or_else(|| anyhow::anyhow!("missing timezone"))?
                    .clone();
                let currency = parts
                    .get(4)
                    .ok_or_else(|| anyhow::anyhow!("missing currency"))?
                    .clone();
                let display_name = parts[5..].join(" ");
                if display_name.is_empty() {
                    return Err(anyhow::anyhow!("missing name"));
                }
                Ok(UserParams::USAF(name, tz, currency, display_name))
            }

            (UserRole::Teacher, UserSubcmd::Add(true)) => {
                let name: TelegramName = parts
                    .get(2)
                    .ok_or_else(|| anyhow::anyhow!("missing @username"))?
                    .parse()?;
                let tz = parts
                    .get(3)
                    .ok_or_else(|| anyhow::anyhow!("missing timezone"))?
                    .clone();
                let display_name = parts[4..].join(" ");
                if display_name.is_empty() {
                    return Err(anyhow::anyhow!("missing name"));
                }
                Ok(UserParams::UTAF(name, tz, display_name))
            }

            (UserRole::Student, UserSubcmd::Delete(false)) => {
                match parts.get(2) {
                    None => Ok(UserParams::USD),
                    Some(n) => Ok(UserParams::USDN(n.parse()?)),
                }
            }
            (UserRole::Teacher, UserSubcmd::Delete(false)) => {
                match parts.get(2) {
                    None => Ok(UserParams::UTD),
                    Some(n) => Ok(UserParams::UTDN(n.parse()?)),
                }
            }

            (UserRole::Student, UserSubcmd::Delete(true)) => {
                let name: TelegramName = parts
                    .get(2)
                    .ok_or_else(|| anyhow::anyhow!("missing @username"))?
                    .parse()?;
                Ok(UserParams::USDNF(name))
            }
            (UserRole::Teacher, UserSubcmd::Delete(true)) => {
                let name: TelegramName = parts
                    .get(2)
                    .ok_or_else(|| anyhow::anyhow!("missing @username"))?
                    .parse()?;
                Ok(UserParams::UTDNF(name))
            }

            (UserRole::Student, UserSubcmd::Edit(false)) => {
                match parts.get(2) {
                    None => Ok(UserParams::USE),
                    Some(n) => Ok(UserParams::USEN(n.parse()?)),
                }
            }
            (UserRole::Teacher, UserSubcmd::Edit(false)) => {
                match parts.get(2) {
                    None => Ok(UserParams::UTE),
                    Some(n) => Ok(UserParams::UTEN(n.parse()?)),
                }
            }

            (UserRole::Student, UserSubcmd::Edit(true)) => {
                let name: TelegramName = parts
                    .get(2)
                    .ok_or_else(|| anyhow::anyhow!("missing @username"))?
                    .parse()?;
                let tz = parts
                    .get(3)
                    .ok_or_else(|| anyhow::anyhow!("missing timezone"))?
                    .clone();
                let currency = parts
                    .get(4)
                    .ok_or_else(|| anyhow::anyhow!("missing currency"))?
                    .clone();
                let display_name = parts[5..].join(" ");
                if display_name.is_empty() {
                    return Err(anyhow::anyhow!("missing name"));
                }
                Ok(UserParams::USENF(name, tz, currency, display_name))
            }

            (UserRole::Teacher, UserSubcmd::Edit(true)) => {
                let name: TelegramName = parts
                    .get(2)
                    .ok_or_else(|| anyhow::anyhow!("missing @username"))?
                    .parse()?;
                let tz = parts
                    .get(3)
                    .ok_or_else(|| anyhow::anyhow!("missing timezone"))?
                    .clone();
                let display_name = parts[4..].join(" ");
                if display_name.is_empty() {
                    return Err(anyhow::anyhow!("missing name"));
                }
                Ok(UserParams::UTENF(name, tz, display_name))
            }
        }
    }
}

pub trait UserCommand: Sized + Clone {
    fn user(params: UserParams) -> Self;
}
