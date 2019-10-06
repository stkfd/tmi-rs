//! Parser for twitch flavored IRC

use fnv::FnvHashMap;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while};
use nom::character::complete::alpha1;
use nom::combinator::{map, map_res, verify};
use nom::multi::many0;
use nom::sequence::{preceded, tuple};
use nom::{AsChar, IResult, InputLength, InputTake, Needed};

pub struct IrcMessage<'a> {
    tags: FnvHashMap<TagKey<'a>, &'a str>,
    prefix: Option<IrcPrefix<'a>>,
    command: &'a str,
    command_params: Vec<&'a str>,
}

pub struct TagKey<'a> {
    is_client_tag: bool,
    vendor: &'a str,
    key_name: &'a str,
}

pub struct IrcPrefix<'a> {
    server_name: &'a str,
    nick: &'a str,
    user: &'a str,
    host: &'a str,
}

fn message(input: &str) -> IResult<&str, IrcMessage<'_>> {
    let (remaining, (tags, prefix, command, command_params)) =
        tuple((irc_tags, irc_prefix, command, command_params))(input)?;
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

fn command(input: &str) -> IResult<&str, &str> {
    alt((alpha1, numeric_command))(input)
}

pub fn numeric_command(input: &str) -> IResult<&str, &str> {
    let (rem, digits) = input.take_split(3);
    if digits.chars().all(|c| c.is_dec_digit()) && digits.input_len() == 3 {
        return Ok((rem, digits));
    }
    return Err(nom::Err::Incomplete(Needed::Size(3)));
}

fn command_params(input: &str) -> IResult<&str, Vec<&str>> {
    many0(preceded(
        tag(" "),
        alt((
            preceded(tag(":"), take_while(is_trailing)),
            verify(take_while(is_middle_param), |s: &str| !s.starts_with(":")),
        )),
    ))(input)
}

#[inline]
fn is_middle_param(chr: char) -> bool {
    chr != '\r' && chr != '\n' && chr != '\0' && chr != ' '
}

#[inline]
fn is_trailing(chr: char) -> bool {
    chr != '\r' && chr != '\n' && chr != '\0'
}

fn irc_prefix(input: &str) -> IResult<&str, Option<IrcPrefix<'_>>> {
    unimplemented!()
}

fn irc_tags(input: &str) -> IResult<&str, FnvHashMap<TagKey<'_>, &str>> {
    unimplemented!()
}

fn irc_tag(input: &str) -> IResult<&str, (&str, &str)> {
    unimplemented!()
}
