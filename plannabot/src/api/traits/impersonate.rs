use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use telluride::utils::{format_screen_spaces, split_with_screened_spaces, ParamParser};

use crate::models::TelegramName;

pub enum ImpersonateRole {
    Student,
    Teacher,
}

impl fmt::Display for ImpersonateRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImpersonateRole::Student => write!(f, "student"),
            ImpersonateRole::Teacher => write!(f, "teacher"),
        }
    }
}

impl FromStr for ImpersonateRole {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "student" => Ok(ImpersonateRole::Student),
            "teacher" => Ok(ImpersonateRole::Teacher),
            _ => Err(anyhow::anyhow!("unknown role: {}", s)),
        }
    }
}

pub enum ImpersonateParams {
    /// No args: show role selection
    I0,
    /// Role=student, optional username
    Student(Option<TelegramName>),
    /// Role=teacher, optional username (not implemented)
    Teacher(Option<TelegramName>),
}

impl fmt::Display for ImpersonateParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let student = ImpersonateRole::Student;
        let teacher = ImpersonateRole::Teacher;
        match self {
            ImpersonateParams::I0 => format_screen_spaces!(),
            ImpersonateParams::Student(None) => format_screen_spaces!(student),
            ImpersonateParams::Student(Some(name)) => format_screen_spaces!(student, name),
            ImpersonateParams::Teacher(None) => format_screen_spaces!(teacher),
            ImpersonateParams::Teacher(Some(name)) => format_screen_spaces!(teacher, name),
        }
        .fmt(f)
    }
}

impl FromStr for ImpersonateParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);
        let mut p = ParamParser::new(&parts, 0);

        let Some(first) = p.next_opt() else {
            return Ok(ImpersonateParams::I0);
        };

        let role: ImpersonateRole = first.parse()?;
        let name = p.next_opt().map(|n| n.parse::<TelegramName>()).transpose()?;
        p.finish()?;

        match role {
            ImpersonateRole::Student => Ok(ImpersonateParams::Student(name)),
            ImpersonateRole::Teacher => Ok(ImpersonateParams::Teacher(name)),
        }
    }
}
