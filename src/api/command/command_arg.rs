use std::{error::Error, fmt::Display, str::FromStr};

use teloxide::utils::command::ParseError;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct EmptyArg;

pub trait ParseCommandArg {
    fn parse_command_arg(arg: &str) -> Result<Self, ParseError>
    where
        Self: Sized;
}

impl Display for EmptyArg {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl ParseCommandArg for EmptyArg {
    fn parse_command_arg(arg: &str) -> Result<Self, ParseError>
    where
        Self: Sized,
    {
        if arg.is_empty() {
            Ok(EmptyArg)
        } else {
            Err(ParseError::Custom(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Expected no argument for EmptyArg",
            ))))
        }
    }
}

impl<T> ParseCommandArg for T
where
    T: FromStr,
    T::Err: Error + Send + Sync + 'static,
{
    fn parse_command_arg(arg: &str) -> Result<Self, ParseError>
    where
        Self: Sized,
    {
        arg.parse::<T>()
            .map_err(|e| ParseError::Custom(Box::new(e)))
    }
}
