use crate::Command;

use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_until, take_while},
    character::complete::{char, line_ending},
    error::{Error as nomError, ErrorKind},
    IResult, Parser,
};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ParseCommandError {
    #[error("parsing")]
    Nom(String, ErrorKind),
    #[error("{0}")]
    ParseSetCommandError(ParseSetCommandError),
    #[error("{0}")]
    ParseGetCommandError(ParseGetCommandError),
}

#[derive(Debug, Error, PartialEq)]
pub enum ParseSetCommandError {}

#[derive(Debug, Error, PartialEq)]
pub enum ParseGetCommandError {}

pub fn command_parse(input: &str) -> Result<Command, ParseCommandError> {
    let mut parser = command_parser();
    match parser.parse(input) {
        Ok((_tail, cmd)) => Ok(cmd),
        Err(nom::Err::Error(e)) => Err(ParseCommandError::Nom(input.to_string(), e.code)),
        _ => unimplemented!(),
    }
}

fn command_parser<'input>() -> impl Parser<&'input str, Command, nomError<&'input str>> {
    move |inp: &'input str| alt((get_command_parser, set_command_parser))(inp)
}

pub fn get_command_parser<'input>(
    inp: &'input str,
) -> IResult<&str, Command, nomError<&'input str>> {
    let (tail, _get) = tag("get")(inp)?;
    let (tail, _whitespace) = char(' ')(tail)?;
    let (tail, keys) = take_until("\r")(tail)?;
    let keys: Vec<String> = if let Ok(keys) = keys
        .split(' ')
        .map(|key| {
            if key.chars().any(|c| !c.is_control()) {
                Ok(key.to_string())
            } else {
                Err(key.to_string())
            }
        })
        .collect::<Result<Vec<_>, _>>()
    {
        keys
    } else {
        return Err(nom::Err::Error(nom::error::Error {
            input: keys,
            code: ErrorKind::Fail,
        }));
    };
    let (tail, _newline) = line_ending(tail)?;
    Ok((tail, Command::Get(keys)))
}

pub fn set_command_parser<'input>(
    inp: &'input str,
) -> IResult<&str, Command, nomError<&'input str>> {
    let (tail, _get) = tag("set")(inp)?;
    let (tail, _whitespace) = char(' ')(tail)?;
    let (tail, key) = take_while(|c| c != ' ')(tail)?;
    let (tail, _whitespace) = char(' ')(tail)?;
    let (tail, _flags) = take_while(char::is_numeric)(tail)?;
    let (tail, _whitespace) = char(' ')(tail)?;
    let (tail, _flags2) = take_while(char::is_numeric)(tail)?;
    let (tail, _whitespace) = char(' ')(tail)?;
    let (tail, size) = take_while(char::is_numeric)(tail)?;

    let size = if let Ok(size) = size.parse::<u8>() {
        size
    } else {
        return Err(nom::Err::Error(nom::error::Error {
            input: size,
            code: ErrorKind::Fail,
        }));
    };

    let (tail, _newline) = tag("\r\n")(tail)?;
    let (tail, value) = take(size)(tail)?;
    let (tail, _newline) = tag("\r\n")(tail)?;
    Ok((tail, Command::Set(key.to_string(), value.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    use nom::error::{Error, ErrorKind};
    use pretty_assertions::assert_eq;

    #[test]
    fn it_parses_one_key() {
        let ex = "get key\r\n";
        assert_eq!(
            get_command_parser(ex),
            Ok(("", Command::Get(vec!["key".to_string()])))
        );
    }

    #[test]
    fn it_parses_two_keys() {
        let ex = "get key1 key2\r\n";
        assert_eq!(
            get_command_parser(ex),
            Ok((
                "",
                Command::Get(vec!["key1".to_string(), "key2".to_string()])
            ))
        );
    }

    #[test]
    fn it_parses_simple_set() {
        let ex = "set xyzkey 0 0 6\r\nabcdef\r\n";
        assert_eq!(
            set_command_parser(ex),
            Ok(("", Command::Set("xyzkey".to_string(), "abcdef".to_string())))
        );
    }

    #[test]
    fn it_fails_to_parse_when_size_is_smaller() {
        let ex = "set xyzkey 0 0 4\r\nabcdef\r\n";
        assert_eq!(
            set_command_parser(ex),
            Err(nom::Err::Error(Error::new("ef\r\n", ErrorKind::Tag)))
        );
    }

    #[test]
    fn it_fails_to_parse_when_size_is_bigger() {
        let ex = "set xyzkey 0 0 10\r\nabcdef\r\n";
        let parser = set_command_parser(ex);
        assert_eq!(
            parser,
            Err(nom::Err::Error(Error::new("abcdef\r\n", ErrorKind::Eof)))
        );
    }

    #[test]
    fn it_tests_the_parser() {
        let ex = "set xyzkey 0 0 6\r\nabcdef\r\n";
        let parser = command_parse(ex);
        assert_eq!(
            parser,
            Ok(Command::Set("xyzkey".to_string(), "abcdef".to_string())),
        );
    }
}
