use std::convert::{identity, TryFrom};
use std::fmt::Debug;
use std::hash::Hash;

use fnv::FnvHashMap;

use crate::irc::IrcMessage;
use crate::irc_constants::replies::*;
use crate::Error;

#[derive(Debug, Clone)]
pub struct Event<T: Debug + Clone + Hash + Eq> {
    sender: Option<T>,
    event: EventContent<T>,
    tags: Option<FnvHashMap<T, T>>,
}

impl<T: Debug + Clone + Hash + Eq> Event<T> {
    /// Create a connection closed event
    pub fn close() -> Event<T> {
        Event {
            sender: None,
            event: EventContent::Close,
            tags: None,
        }
    }
}

trait RefToString {
    fn ref_to_string(&self) -> String;
}

impl<T: AsRef<str>> RefToString for T {
    #[inline]
    fn ref_to_string(&self) -> String {
        AsRef::<str>::as_ref(self).into()
    }
}

/// Turn a borrowed version of the event into one with
/// owned strings as the content
impl<T> From<&Event<T>> for Event<String>
where
    T: AsRef<str> + Debug + Clone + Hash + Eq,
{
    fn from(e: &Event<T>) -> Self {
        Event {
            sender: e.sender.as_ref().map(RefToString::ref_to_string),
            event: (&e.event).into(),
            tags: e.tags.as_ref().map(|hash_map| {
                hash_map
                    .iter()
                    .map(|(key, val)| (key.ref_to_string(), val.as_ref().ref_to_string()))
                    .collect::<FnvHashMap<String, String>>()
            }),
        }
    }
}

#[inline]
fn parse_error(message: &IrcMessage<&str>, details: &'static str) -> Error {
    Error::EventParseError {
        message: message.into(),
        details: details.into(),
    }
}

/// Attempts to convert any parsed IRC message into a twitch specific
/// event struct
impl<'a> TryFrom<IrcMessage<&'a str>> for Event<&'a str> {
    type Error = Error;

    fn try_from(msg: IrcMessage<&'a str>) -> Result<Self, Error> {
        let evt: Event<&str> = Event {
            sender: msg
                .prefix
                .as_ref()
                .map(|p| p.user_or_nick())
                .and_then(identity)
                .map(|&x| x),
            event: match msg.command {
                "PRIVMSG" => {
                    if msg.command_params.len() != 2 {
                        return Err(parse_error(&msg, "Wrong PRIVMSG parameter count"));
                    }
                    EventContent::PrivMsg(ChannelMessageEvent {
                        channel: &msg.command_params[0],
                        message: &msg.command_params[1],
                    })
                }
                "WHISPER" => {
                    if msg.command_params.len() != 2 {
                        return Err(parse_error(&msg, "Wrong WHISPER parameter count"));
                    }
                    EventContent::Whisper(WhisperEvent {
                        recipient: &msg.command_params[0],
                        message: &msg.command_params[1],
                    })
                }
                "JOIN" => EventContent::Join(ChannelEvent {
                    channel: msg
                        .command_params
                        .get(0)
                        .ok_or_else(|| parse_error(&msg, "Missing channel name"))?,
                }),
                "MODE" => {
                    if msg.command_params.len() != 3 {
                        return Err(parse_error(&msg, "Wrong parameter count"));
                    }
                    EventContent::Mode(ModeChangeEvent {
                        channel: msg.command_params[0],
                        mode_change: msg.command_params[1],
                        user: msg.command_params[2],
                    })
                }
                RPL_NAMREPLY => {
                    if msg.command_params.len() < 3 {
                        return Err(parse_error(&msg, "Wrong parameter count"));
                    }
                    EventContent::Names(NamesListEvent {
                        user: msg.command_params[0],
                        channel: msg.command_params[2],
                        names: msg.command_params.iter().skip(3).map(|&s| s).collect(),
                    })
                }
                RPL_ENDOFNAMES => EventContent::EndOfNames,
                "PART" => EventContent::Part(ChannelEvent {
                    channel: msg
                        .command_params
                        .get(0)
                        .ok_or_else(|| parse_error(&msg, "Missing user name"))?,
                }),
                "CLEARCHAT" => EventContent::ClearChat(ChannelUserEvent {
                    channel: msg
                        .command_params
                        .get(0)
                        .ok_or_else(|| parse_error(&msg, "Missing channel name"))?,
                    user: msg.command_params.get(1).map(|&s| s),
                }),
                "CLEARMSG" => unimplemented!(),
                "HOSTTARGET" => unimplemented!(),
                "NOTICE" => unimplemented!(),
                "RECONNECT" => EventContent::Reconnect,
                "ROOMSTATE" => EventContent::RoomState(ChannelEvent {
                    channel: msg
                        .command_params
                        .get(0)
                        .ok_or_else(|| parse_error(&msg, "Missing channel name"))?,
                }),
                "USERNOTICE" => {
                    if msg.command_params.len() != 2 {
                        return Err(parse_error(&msg, "Wrong USERNOTICE parameter count"));
                    }
                    EventContent::UserNotice(ChannelMessageEvent {
                        channel: msg.command_params[0],
                        message: msg.command_params[1],
                    })
                }
                "USERSTATE" => EventContent::UserState(ChannelEvent {
                    channel: msg
                        .command_params
                        .get(0)
                        .ok_or_else(|| parse_error(&msg, "Missing channel name"))?,
                }),
                "CAP" => EventContent::Capability(CapabilityEvent {
                    params: msg.command_params.clone(),
                }),
                RPL_WELCOME | RPL_YOURHOST | RPL_CREATED | RPL_MYINFO | RPL_MOTDSTART
                | RPL_MOTD | RPL_ENDOFMOTD => EventContent::ConnectMessage(ConnectMessage {
                    command: msg.command,
                    params: msg.command_params.clone(),
                }),
                "GLOBALUSERSTATE" => EventContent::GlobalUserState,
                "PING" => EventContent::Ping,
                "PONG" => EventContent::Pong,
                _ => return Err(parse_error(&msg, "Unexpected IRC command type")),
            },
            tags: Some(msg.tags),
        };
        Ok(evt)
    }
}

impl<T: Clone + Debug + Hash + Eq> Event<T> {
    pub fn sender(&self) -> &Option<T> {
        &self.sender
    }

    pub fn event(&self) -> &EventContent<T> {
        &self.event
    }

    pub fn tags(&self) -> &Option<FnvHashMap<T, T>> {
        &self.tags
    }
}

#[derive(Debug, Clone)]
pub enum EventContent<T: Debug + Clone> {
    PrivMsg(ChannelMessageEvent<T>),
    Whisper(WhisperEvent<T>),
    Join(ChannelEvent<T>),
    Mode(ModeChangeEvent<T>),
    Names(NamesListEvent<T>),
    EndOfNames,
    Part(ChannelEvent<T>),
    ClearChat(ChannelUserEvent<T>),
    ClearMsg(ChannelMessageEvent<T>),
    Host(HostEvent<T>),
    Notice(ChannelMessageEvent<T>),
    Reconnect,
    RoomState(ChannelEvent<T>),
    UserNotice(ChannelMessageEvent<T>),
    UserState(ChannelEvent<T>),
    Capability(CapabilityEvent<T>),
    ConnectMessage(ConnectMessage<T>),
    GlobalUserState,
    Close,
    Ping,
    Pong,
    Unknown,
}

impl<T> From<&EventContent<T>> for EventContent<String>
where
    T: AsRef<str> + Clone + Debug,
{
    fn from(e: &EventContent<T>) -> Self {
        match e {
            EventContent::ConnectMessage(inner) => EventContent::ConnectMessage(inner.into()),
            EventContent::PrivMsg(inner) => EventContent::PrivMsg(inner.into()),
            EventContent::Whisper(inner) => EventContent::Whisper(inner.into()),
            EventContent::Join(inner) => EventContent::Join(inner.into()),
            EventContent::Mode(inner) => EventContent::Mode(inner.into()),
            EventContent::Names(inner) => EventContent::Names(inner.into()),
            EventContent::EndOfNames => EventContent::EndOfNames,
            EventContent::Part(inner) => EventContent::Part(inner.into()),
            EventContent::ClearChat(inner) => EventContent::ClearChat(inner.into()),
            EventContent::ClearMsg(inner) => EventContent::ClearMsg(inner.into()),
            EventContent::Host(inner) => EventContent::Host(inner.into()),
            EventContent::Notice(inner) => EventContent::Notice(inner.into()),
            EventContent::RoomState(inner) => EventContent::RoomState(inner.into()),
            EventContent::UserNotice(inner) => EventContent::UserNotice(inner.into()),
            EventContent::UserState(inner) => EventContent::UserState(inner.into()),
            EventContent::Capability(inner) => EventContent::Capability(inner.into()),
            EventContent::GlobalUserState => EventContent::GlobalUserState,
            EventContent::Reconnect => EventContent::Reconnect,
            EventContent::Close => EventContent::Close,
            EventContent::Ping => EventContent::Ping,
            EventContent::Pong => EventContent::Pong,
            EventContent::Unknown => EventContent::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectMessage<T: Debug + Clone> {
    command: T,
    params: Vec<T>,
}

impl<T> From<&ConnectMessage<T>> for ConnectMessage<String>
where
    T: AsRef<str> + Clone + Debug,
{
    fn from(from: &ConnectMessage<T>) -> Self {
        ConnectMessage {
            command: from.command.ref_to_string(),
            params: from.params.iter().map(RefToString::ref_to_string).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NamesListEvent<T: Debug + Clone> {
    user: T,
    channel: T,
    names: Vec<T>,
}

impl<T> From<&NamesListEvent<T>> for NamesListEvent<String>
where
    T: AsRef<str> + Clone + Debug,
{
    fn from(from: &NamesListEvent<T>) -> Self {
        NamesListEvent {
            user: from.user.ref_to_string(),
            channel: from.channel.ref_to_string(),
            names: from.names.iter().map(RefToString::ref_to_string).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModeChangeEvent<T: Debug + Clone> {
    channel: T,
    mode_change: T,
    user: T,
}

impl<T> From<&ModeChangeEvent<T>> for ModeChangeEvent<String>
where
    T: AsRef<str> + Clone + Debug,
{
    fn from(from: &ModeChangeEvent<T>) -> Self {
        ModeChangeEvent {
            channel: from.channel.ref_to_string(),
            mode_change: from.mode_change.ref_to_string(),
            user: from.user.ref_to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserEvent<T: Debug + Clone> {
    user: T,
}

impl<T> From<&UserEvent<T>> for UserEvent<String>
where
    T: AsRef<str> + Clone + Debug,
{
    fn from(from: &UserEvent<T>) -> Self {
        UserEvent {
            user: from.user.ref_to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelMessageEvent<T: Debug + Clone> {
    channel: T,
    message: T,
}

impl<T> From<&ChannelMessageEvent<T>> for ChannelMessageEvent<String>
where
    T: AsRef<str> + Clone + Debug,
{
    fn from(from: &ChannelMessageEvent<T>) -> Self {
        ChannelMessageEvent {
            channel: from.channel.ref_to_string(),
            message: from.message.ref_to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WhisperEvent<T: Debug + Clone> {
    recipient: T,
    message: T,
}

impl<T> From<&WhisperEvent<T>> for WhisperEvent<String>
where
    T: AsRef<str> + Clone + Debug,
{
    fn from(from: &WhisperEvent<T>) -> Self {
        WhisperEvent {
            recipient: from.recipient.ref_to_string(),
            message: from.message.ref_to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelEvent<T: Debug + Clone> {
    channel: T,
}

impl<T> From<&ChannelEvent<T>> for ChannelEvent<String>
where
    T: AsRef<str> + Clone + Debug,
{
    fn from(from: &ChannelEvent<T>) -> Self {
        ChannelEvent {
            channel: from.channel.ref_to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HostEvent<T: Debug + Clone> {
    hosting_channel: T,
    target_channel: Option<T>,
    viewer_count: usize,
}

impl<T> From<&HostEvent<T>> for HostEvent<String>
where
    T: AsRef<str> + Clone + Debug,
{
    fn from(from: &HostEvent<T>) -> Self {
        HostEvent {
            hosting_channel: from.hosting_channel.ref_to_string(),
            target_channel: from.target_channel.as_ref().map(RefToString::ref_to_string),
            viewer_count: from.viewer_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelUserEvent<T: Debug + Clone> {
    channel: T,
    user: Option<T>,
}

impl<T> From<&ChannelUserEvent<T>> for ChannelUserEvent<String>
where
    T: AsRef<str> + Clone + Debug,
{
    fn from(from: &ChannelUserEvent<T>) -> Self {
        ChannelUserEvent {
            channel: from.channel.ref_to_string(),
            user: from.user.as_ref().map(RefToString::ref_to_string),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CapabilityEvent<T: Debug + Clone> {
    params: Vec<T>,
}

impl<T> From<&CapabilityEvent<T>> for CapabilityEvent<String>
where
    T: AsRef<str> + Clone + Debug,
{
    fn from(from: &CapabilityEvent<T>) -> Self {
        CapabilityEvent {
            params: from.params.iter().map(RefToString::ref_to_string).collect(),
        }
    }
}
