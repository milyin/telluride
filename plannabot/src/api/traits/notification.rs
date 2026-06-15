use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use telluride::utils::{ParamParser, format_screen_spaces, split_with_screened_spaces};

use crate::types::Duration;

pub enum NotificationParams {
    /// Show the closest upcoming lesson + [Lesson] [Setup] buttons.
    N0,
    /// Show the notification-setup explanation + [Set...] button.
    S0,
    /// Force-set (execute) the notification delay.
    SF(Duration),
}

pub enum NotificationSubcmd {
    Show,
    Setup(bool),
}

impl fmt::Display for NotificationSubcmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotificationSubcmd::Show => write!(f, "show"),
            NotificationSubcmd::Setup(false) => write!(f, "setup"),
            NotificationSubcmd::Setup(true) => write!(f, "setup!"),
        }
    }
}

impl FromStr for NotificationSubcmd {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "show" => Ok(NotificationSubcmd::Show),
            "setup" => Ok(NotificationSubcmd::Setup(false)),
            "setup!" => Ok(NotificationSubcmd::Setup(true)),
            _ => Err(anyhow::anyhow!("unknown notification subcommand: {}", s)),
        }
    }
}

impl fmt::Display for NotificationParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let show = NotificationSubcmd::Show;
        let setup = NotificationSubcmd::Setup(false);
        let setup_f = NotificationSubcmd::Setup(true);
        match self {
            NotificationParams::N0 => format_screen_spaces!(show),
            NotificationParams::S0 => format_screen_spaces!(setup),
            NotificationParams::SF(dur) => format_screen_spaces!(setup_f, dur),
        }
        .fmt(f)
    }
}

impl FromStr for NotificationParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);

        let Some(first) = parts.first() else {
            return Ok(NotificationParams::N0);
        };

        match first.parse::<NotificationSubcmd>()? {
            NotificationSubcmd::Show => Ok(NotificationParams::N0),
            NotificationSubcmd::Setup(false) => Ok(NotificationParams::S0),
            NotificationSubcmd::Setup(true) => {
                let mut p = ParamParser::new(&parts, 1);
                let dur: Duration = p.next("duration")?.parse()?;
                p.finish()?;
                Ok(NotificationParams::SF(dur))
            }
        }
    }
}

pub trait NotificationCommand: Sized + Clone {
    fn notification(params: NotificationParams) -> Self;
}
