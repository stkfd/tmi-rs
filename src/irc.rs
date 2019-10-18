//! Parser for twitch flavored IRC

use fnv::FnvHashMap;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while, take_while1, take_while_m_n};
use nom::character::complete::{alpha1, char};
use nom::combinator::{opt, recognize, verify};
use nom::multi::{many0, separated_list};
use nom::sequence::{delimited, preceded, terminated, tuple};
use nom::{AsChar, IResult};
use std::convert::identity;
use std::hash::Hash;
use std::iter::FromIterator;

#[derive(Debug, Eq, PartialEq)]
pub struct IrcMessage<T: PartialEq + Eq + Hash> {
    pub tags: FnvHashMap<T, T>,
    pub prefix: Option<IrcPrefix<T>>,
    pub command: T,
    pub command_params: Vec<T>,
}

impl IrcMessage<&str> {
    pub fn parse_many(input: &str) -> IResult<&str, Vec<IrcMessage<&str>>> {
        separated_list(tag("\r\n"), opt(Self::parse))(input)
            .map(|(rem, messages)| (rem, messages.into_iter().filter_map(identity).collect()))
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

impl From<&IrcMessage<&str>> for IrcMessage<String> {
    fn from(_: &IrcMessage<&str>) -> Self {
        unimplemented!()
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct IrcTagKey<T> {
    is_client_tag: bool,
    vendor: Option<T>,
    key_name: T,
}

impl<'a, T: AsRef<str> + 'a> From<&'a T> for IrcTagKey<&'a str> {
    fn from(source: &'a T) -> Self {
        irc_tag_key(source.as_ref()).unwrap().1
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct IrcPrefix<T> {
    pub host: T,
    pub nick: Option<T>,
    pub user: Option<T>,
}

impl<T: Eq + PartialEq + Hash> IrcPrefix<T> {
    pub fn user_or_nick(&self) -> Option<&T> {
        self.user.as_ref().or(self.nick.as_ref())
    }
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

/// Matches characters allowed in a normal (not-trailing) command parameter
fn middle_param(input: &str) -> IResult<&str, &str> {
    verify(take_while1(|c: char| !"\r\n\0 ".contains(c)), |s: &str| {
        !s.starts_with(':')
    })(input)
}

/// Matches characters allowed in a trailing command parameter
fn trailing_param(input: &str) -> IResult<&str, &str> {
    preceded(tag(":"), take_while(|c: char| !"\r\n\0".contains(c)))(input)
}

/// Parse an IRC message prefix
fn irc_prefix(input: &str) -> IResult<&str, IrcPrefix<&str>> {
    let (remaining, (nick_or_server, user, host)) = delimited(
        char(':'),
        tuple((
            take_while1(|chr| !"! ".contains(chr)),
            opt(preceded(tag("!"), take_while1(|chr| !"@ ".contains(chr)))),
            opt(preceded(tag("@"), not_spaces1)),
        )),
        char(' '),
    )(input)?;

    Ok((
        remaining,
        if host.is_some() {
            IrcPrefix {
                host: host.unwrap(),
                user,
                nick: Some(nick_or_server),
            }
        } else {
            IrcPrefix {
                host: nick_or_server,
                user,
                nick: None,
            }
        },
    ))
}

/// Parse IRCv3 tags into a HashMap
fn irc_tags(input: &str) -> IResult<&str, FnvHashMap<&str, &str>> {
    let (remaining, list_opt) = opt(delimited(
        char('@'),
        separated_list(char(';'), irc_tag),
        spaces0,
    ))(input)?;
    Ok((
        remaining,
        match list_opt {
            Some(list) => {
                FnvHashMap::from_iter(list.into_iter().filter_map(|(k, v)| match (k, v) {
                    (k, Some(v)) => Some((k, v)),
                    (_, None) => None,
                }))
            }
            None => FnvHashMap::default(),
        },
    ))
}

/// Parse a single IRCv3 tag
fn irc_tag(input: &str) -> IResult<&str, (&str, Option<&str>)> {
    let (remaining, (key, val)) = tuple((
        irc_tag_key_str,
        opt(preceded(
            char('='),
            opt(take_while1(|c: char| !" ;".contains(c))),
        )),
    ))(input)?;
    Ok((remaining, (key, val.and_then(identity))))
}

fn irc_tag_key_str(input: &str) -> IResult<&str, &str> {
    recognize(tuple((
        opt(char('+')),
        opt(terminated(take_while1(|c| !"=/".contains(c)), char('/'))),
        take_while1(|c: char| c.is_alphanumeric() || c == '-'),
    )))(input)
}

/// Parse the key of an IRC tag
fn irc_tag_key(input: &str) -> IResult<&str, IrcTagKey<&str>> {
    let (remaining, (client_prefix, vendor, key_name)) = tuple((
        opt(char('+')),
        opt(terminated(take_while1(|c| !"=/".contains(c)), char('/'))),
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

/// Take 1 or more non-space characters
fn not_spaces1(input: &str) -> IResult<&str, &str> {
    take_while1(|c| c != ' ')(input)
}

/// Take 1 or more spaces
fn spaces1(input: &str) -> IResult<&str, &str> {
    take_while1(|c| c == ' ')(input)
}

/// Take 0 or more spaces
fn spaces0(input: &str) -> IResult<&str, &str> {
    take_while(|c| c == ' ')(input)
}

// ------------------------------ TESTS ------------------------------
// -------------------------------------------------------------------

#[test]
fn test_irc_tags() {
    let tag_str = "@tag-name-1=<tag-value-1>;tag-name-2=<tag-value-2>;empty-tag=";
    let (remaining, map) = irc_tags(tag_str).unwrap();
    assert_eq!(remaining.len(), 0);
    assert_eq!(map[&IrcTagKey::from(&"tag-name-1")], Some("<tag-value-1>"));
    assert_eq!(map[&IrcTagKey::from(&"tag-name-2")], Some("<tag-value-2>"));
    assert_eq!(map[&IrcTagKey::from(&"empty-tag")], None);
}

#[test]
fn test_command_params() {
    let result = command_params("  middle1 middle2  middle3 :trailing");
    assert_eq!(
        result,
        Ok(("", vec!["middle1", "middle2", "middle3", "trailing",],),)
    );
}

#[test]
fn test_irc_tag_key() {
    assert_eq!(
        IrcTagKey::from(&"badge-info"),
        IrcTagKey {
            is_client_tag: false,
            vendor: None,
            key_name: "badge-info"
        }
    );
    assert_eq!(
        IrcTagKey::from(&"+vend/badge-info"),
        IrcTagKey {
            is_client_tag: true,
            vendor: Some("vend"),
            key_name: "badge-info"
        }
    );
}

#[test]
fn test_welcome_message() {
    let msg = ":tmi.twitch.tv 001 zapbeeblebrox123 :Welcome, GLHF!\r\n";
    println!("{:#?}", IrcMessage::parse(msg).unwrap());
}

#[test]
fn test_message() {
    let msg = "@badge-info=;badges=;color=#5F9EA0;display-name=SomeUser;emotes=;flags=;id=7be7b0d9-ba18-4f7c-acb5-439dad989d41;mod=0;room-id=22484632;subscriber=0;tmi-sent-ts=1570895688837;turbo=0;user-id=427147774;user-type= :someusername!someusername@someusername.tmi.twitch.tv PRIVMSG #forsen :FeelsDankMan";
    println!("{:#?}", IrcMessage::parse(msg).unwrap());
}
