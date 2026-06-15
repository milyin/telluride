use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use telluride::utils::{ParamParser, format_screen_spaces, split_with_screened_spaces};

pub enum ScheduleParams {
    L0,
    L1(i32),
}

impl fmt::Display for ScheduleParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScheduleParams::L0 => format_screen_spaces!(),
            ScheduleParams::L1(w) => format_screen_spaces!(w),
        }
        .fmt(f)
    }
}

impl FromStr for ScheduleParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = split_with_screened_spaces(s);
        let mut p = ParamParser::new(&parts, 0);
        let Some(first) = p.next_opt() else {
            return Ok(ScheduleParams::L0);
        };
        let w: i32 = first.parse()?;
        p.finish()?;
        Ok(ScheduleParams::L1(w))
    }
}
