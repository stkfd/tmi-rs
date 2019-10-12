//! Parser for twitch flavored IRC

use fnv::FnvHashMap;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while, take_while1, take_while_m_n};
use nom::character::complete::{alpha1, char};
use nom::combinator::{opt, verify};
use nom::multi::{many0, separated_nonempty_list};
use nom::sequence::{delimited, preceded, terminated, tuple};
use nom::{AsChar, IResult};
use std::convert::identity;
use std::iter::FromIterator;

pub struct IrcMessage<T> {
    tags: FnvHashMap<IrcTagKey<T>, Option<T>>,
    prefix: Option<IrcPrefix<T>>,
    command: T,
    command_params: Vec<T>,
}

impl<T> IrcMessage<T> {
    pub fn parse_many(input: &str) -> IResult<&str, Vec<IrcMessage<&str>>> {
        many0(Self::parse)(input)
    }

    pub fn parse(input: &str) -> IResult<&str, IrcMessage<&str>> {
        let (remaining, (tags, prefix, command, command_params)) =
            tuple((irc_tags, opt(irc_prefix), command, command_params))(input)?;
        Ok((
            remaining,
            IrcMessage {
                tags,
                prefix,
                command,
                command_params,
            },
        ))
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct IrcTagKey<T> {
    is_client_tag: bool,
    vendor: Option<T>,
    key_name: T,
}

pub struct IrcPrefix<T> {
    nick: T,
    user: Option<T>,
    host: Option<T>,
}

/// Parse an IRC command name
fn command(input: &str) -> IResult<&str, &str> {
    alt((alpha1, numeric_command))(input)
}

/// Parse numeric three digit IRC command name
fn numeric_command(input: &str) -> IResult<&str, &str> {
    take_while_m_n(3, 3, |c: char| c.is_dec_digit())(input)
}

/// Parse IRC command parameters
fn command_params(input: &str) -> IResult<&str, Vec<&str>> {
    many0(preceded(spaces1, alt((trailing_param, middle_param))))(input)
}

fn spaces1(input: &str) -> IResult<&str, &str> {
    take_while1(|c| c == ' ')(input)
}
fn not_spaces1(input: &str) -> IResult<&str, &str> {
    take_while1(|c| c != ' ')(input)
}

/// Matches characters allowed in a normal (not-trailing) command parameter
fn middle_param(input: &str) -> IResult<&str, &str> {
    verify(take_while1(|c: char| "\r\n\0 ".contains(c)), |s: &str| {
        !s.starts_with(':')
    })(input)
}

/// Matches characters allowed in a trailing command parameter
fn trailing_param(input: &str) -> IResult<&str, &str> {
    preceded(tag(":"), take_while(|c: char| "\r\n\0".contains(c)))(input)
}

/// Parse an IRC message prefix
fn irc_prefix(input: &str) -> IResult<&str, IrcPrefix<&str>> {
    let (remaining, (nick, user, host)) = delimited(
        char(':'),
        tuple((
            take_while1(|chr| chr != '!' && chr != ' '),
            opt(preceded(
                tag("!"),
                take_while1(|chr| chr != '@' && chr != ' '),
            )),
            opt(preceded(tag("@"), not_spaces1)),
        )),
        char(' '),
    )(input)?;
    Ok((remaining, IrcPrefix { nick, user, host }))
}

/// Parse IRCv3 tags into a HashMap
fn irc_tags(input: &str) -> IResult<&str, FnvHashMap<IrcTagKey<&str>, Option<&str>>> {
    let (remaining, list): (&str, Vec<(IrcTagKey<&str>, Option<&str>)>) =
        separated_nonempty_list(char(';'), irc_tag)(input)?;
    Ok((remaining, FnvHashMap::from_iter(list)))
}

/// Parse a single IRCv3 tag
fn irc_tag(input: &str) -> IResult<&str, (IrcTagKey<&str>, Option<&str>)> {
    let (remaining, (key, val)) = tuple((
        irc_tag_key,
        opt(preceded(char('='), opt(take_while1(|c: char| c != ';')))),
    ))(input)?;
    Ok((remaining, (key, val.and_then(identity))))
}

#[test]
fn test_irc_tag() {
    assert_eq!(
        irc_tag("key=value"),
        Ok((
            "",
            (
                IrcTagKey {
                    is_client_tag: false,
                    vendor: None,
                    key_name: "key"
                },
                Some("value")
            )
        ))
    )
}

#[test]
fn test_irc_tag_empty() {
    let empty_tag = Ok((
        "",
        (
            IrcTagKey {
                is_client_tag: false,
                vendor: None,
                key_name: "key",
            },
            None,
        ),
    ));
    assert_eq!(irc_tag("key="), empty_tag);
    assert_eq!(irc_tag("key"), empty_tag);
}

fn irc_tag_key(input: &str) -> IResult<&str, IrcTagKey<&str>> {
    let (remaining, (client_prefix, vendor, key_name)) = tuple((
        opt(char('+')),
        opt(terminated(take_while1(|c| c != '/'), char('/'))),
        take_while1(|c: char| c.is_alphanumeric() || c == '-'),
    ))(input)?;
    Ok((
        remaining,
        IrcTagKey {
            vendor,
            key_name,
            is_client_tag: client_prefix.is_some(),
        },
    ))
}
