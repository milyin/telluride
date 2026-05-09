use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use telluride::utils::{split_with_screened_spaces, ParamParser};

use crate::models::TelegramName;
use crate::types::{Currency, Timezone};

/// Serialises `Option<Currency>`: `None` → `"-"`, `Some(c)` → ISO code.
/// Parses back: `"-"` → `None`, valid code → `Some(c)`, invalid → error.
fn fmt_opt_currency(opt: &Option<Currency>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match opt {
        None => write!(f, "-"),
        Some(c) => write!(f, "{c}"),
    }
}

fn parse_opt_currency(s: &str) -> Result<Option<Currency>> {
    if s.trim() == "-" {
        Ok(None)
    } else {
        s.parse::<Currency>().map(Some)
    }
}

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
    /// Student submenu with paginated list (page)
    US(i32),
    /// Teacher submenu with paginated list (page)
    UT(i32),
    /// Student add: show instructions + template button
    USA,
    /// Teacher add: show instructions + template button
    UTA,
    /// Student add! forced: execute insertion (@u, tz, currency, name)
    USAF(TelegramName, Timezone, Currency, String),
    /// Teacher add! forced: execute insertion (@u, tz, name)
    UTAF(TelegramName, Timezone, String),
    /// Student delete: show list (page)
    USD(i32),
    /// Teacher delete: show list (page)
    UTD(i32),
    /// Student delete with name: show full info + confirm button
    USDN(TelegramName),
    /// Teacher delete with name: show full info + confirm button
    UTDN(TelegramName),
    /// Student delete! forced: execute deletion
    USDNF(TelegramName),
    /// Teacher delete! forced: execute deletion
    UTDNF(TelegramName),
    /// Student edit: show list (page)
    USE(i32),
    /// Teacher edit: show list (page)
    UTE(i32),
    /// Student edit with name: show full info + pre-filled edit button
    USEN(TelegramName),
    /// Teacher edit with name: show full info + pre-filled edit button
    UTEN(TelegramName),
    /// Student edit! forced: execute edit (@u, tz, currency, name); currency "-" means remove
    USENF(TelegramName, Timezone, Option<Currency>, String),
    /// Teacher edit! forced: execute edit (@u, tz, name)
    UTENF(TelegramName, Timezone, String),
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
            UserParams::US(0) => write!(f, "{student}"),
            UserParams::US(page) => write!(f, "{student} {page}"),
            UserParams::UT(0) => write!(f, "{teacher}"),
            UserParams::UT(page) => write!(f, "{teacher} {page}"),
            UserParams::USA => write!(f, "{student} {add}"),
            UserParams::UTA => write!(f, "{teacher} {add}"),
            UserParams::USAF(name, tz, currency, display_name) => {
                write!(f, "{student} {add_f} {name} {} ", tz.to_param())?;
                fmt::Display::fmt(currency, f)?;
                write!(f, " {display_name}")
            }
            UserParams::UTAF(name, tz, display_name) => {
                write!(f, "{teacher} {add_f} {name} {} {display_name}", tz.to_param())
            }
            UserParams::USD(page) => write!(f, "{student} {del} {page}"),
            UserParams::UTD(page) => write!(f, "{teacher} {del} {page}"),
            UserParams::USDN(name) => write!(f, "{student} {del} {name}"),
            UserParams::UTDN(name) => write!(f, "{teacher} {del} {name}"),
            UserParams::USDNF(name) => write!(f, "{student} {del_f} {name}"),
            UserParams::UTDNF(name) => write!(f, "{teacher} {del_f} {name}"),
            UserParams::USE(page) => write!(f, "{student} {edit} {page}"),
            UserParams::UTE(page) => write!(f, "{teacher} {edit} {page}"),
            UserParams::USEN(name) => write!(f, "{student} {edit} {name}"),
            UserParams::UTEN(name) => write!(f, "{teacher} {edit} {name}"),
            UserParams::USENF(name, tz, currency, display_name) => {
                write!(f, "{student} {edit_f} {name} {} ", tz.to_param())?;
                fmt_opt_currency(currency, f)?;
                write!(f, " {display_name}")
            }
            UserParams::UTENF(name, tz, display_name) => {
                write!(f, "{teacher} {edit_f} {name} {} {display_name}", tz.to_param())
            }
        }
    }
}

impl FromStr for UserParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);
        let mut p = ParamParser::new(&parts, 0);

        let Some(first) = p.next_opt() else {
            return Ok(UserParams::U0);
        };

        let role: UserRole = first.parse()?;

        let Some(second) = p.next_opt() else {
            return Ok(match role {
                UserRole::Student => UserParams::US(0),
                UserRole::Teacher => UserParams::UT(0),
            });
        };

        if let Ok(page) = second.parse::<i32>() {
            return Ok(match role {
                UserRole::Student => UserParams::US(page),
                UserRole::Teacher => UserParams::UT(page),
            });
        }

        let subcmd: UserSubcmd = second.parse()?;

        match (role, subcmd) {
            (UserRole::Student, UserSubcmd::Add(false)) => Ok(UserParams::USA),
            (UserRole::Teacher, UserSubcmd::Add(false)) => Ok(UserParams::UTA),

            (UserRole::Student, UserSubcmd::Add(true)) => {
                let name: TelegramName = p.next("@username")?.parse()?;
                let tz: Timezone = p.next("timezone")?.parse()?;
                let currency: Currency = p.next("currency")?.parse()?;
                let display_name = p.rest().join(" ");
                if display_name.is_empty() {
                    return Err(anyhow::anyhow!("missing name"));
                }
                Ok(UserParams::USAF(name, tz, currency, display_name))
            }

            (UserRole::Teacher, UserSubcmd::Add(true)) => {
                let name: TelegramName = p.next("@username")?.parse()?;
                let tz: Timezone = p.next("timezone")?.parse()?;
                let display_name = p.rest().join(" ");
                if display_name.is_empty() {
                    return Err(anyhow::anyhow!("missing name"));
                }
                Ok(UserParams::UTAF(name, tz, display_name))
            }

            (UserRole::Student, UserSubcmd::Delete(false)) => match p.next_opt() {
                None => Ok(UserParams::USD(0)),
                Some(s) if s.starts_with('@') => Ok(UserParams::USDN(s.parse()?)),
                Some(s) => Ok(UserParams::USD(s.parse().unwrap_or(0))),
            },
            (UserRole::Teacher, UserSubcmd::Delete(false)) => match p.next_opt() {
                None => Ok(UserParams::UTD(0)),
                Some(s) if s.starts_with('@') => Ok(UserParams::UTDN(s.parse()?)),
                Some(s) => Ok(UserParams::UTD(s.parse().unwrap_or(0))),
            },

            (UserRole::Student, UserSubcmd::Delete(true)) => {
                let name: TelegramName = p.next("@username")?.parse()?;
                p.finish()?;
                Ok(UserParams::USDNF(name))
            }
            (UserRole::Teacher, UserSubcmd::Delete(true)) => {
                let name: TelegramName = p.next("@username")?.parse()?;
                p.finish()?;
                Ok(UserParams::UTDNF(name))
            }

            (UserRole::Student, UserSubcmd::Edit(false)) => match p.next_opt() {
                None => Ok(UserParams::USE(0)),
                Some(s) if s.starts_with('@') => Ok(UserParams::USEN(s.parse()?)),
                Some(s) => Ok(UserParams::USE(s.parse().unwrap_or(0))),
            },
            (UserRole::Teacher, UserSubcmd::Edit(false)) => match p.next_opt() {
                None => Ok(UserParams::UTE(0)),
                Some(s) if s.starts_with('@') => Ok(UserParams::UTEN(s.parse()?)),
                Some(s) => Ok(UserParams::UTE(s.parse().unwrap_or(0))),
            },

            (UserRole::Student, UserSubcmd::Edit(true)) => {
                let name: TelegramName = p.next("@username")?.parse()?;
                let tz: Timezone = p.next("timezone")?.parse()?;
                let currency: Option<Currency> = parse_opt_currency(
                    p.next("currency (use '-' to remove)")?,
                )?;
                let display_name = p.rest().join(" ");
                if display_name.is_empty() {
                    return Err(anyhow::anyhow!("missing name"));
                }
                Ok(UserParams::USENF(name, tz, currency, display_name))
            }

            (UserRole::Teacher, UserSubcmd::Edit(true)) => {
                let name: TelegramName = p.next("@username")?.parse()?;
                let tz: Timezone = p.next("timezone")?.parse()?;
                let display_name = p.rest().join(" ");
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
