use crate::irc::{IrcMessage, IrcTagKey};
use crate::irc_constants::replies::*;
use crate::{Error, ErrorKind};
use fnv::FnvHashMap;
use std::convert::{identity, TryFrom};
use std::fmt::Debug;
use std::hash::Hash;

#[derive(Debug, Clone)]
pub struct Event<T: Debug + Clone + Hash + Eq> {
    sender: Option<T>,
    event: EventContent<T>,
    tags: Option<FnvHashMap<IrcTagKey<T>, Option<T>>>,
}

impl<T: Debug + Clone + Hash + Eq> Event<T> {
    pub fn close() -> Event<T> {
        Event {
            sender: None,
            event: EventContent::Close,
            tags: None,
        }
    }
}

impl From<Event<&str>> for Event<String> {
    fn from(evt: Event<&str>) -> Self {
        Event {
            sender: evt.sender.map(|f| f.to_owned()),
            event: EventContent::EndOfNames,
            tags: None,
        }
    }
}

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
                "PRIVMSG" => EventContent::PrivMsg(ChannelMessageEvent {
                    channel: &msg.command_params[0],
                    message: &msg.command_params[1],
                }),
                "JOIN" => EventContent::Join(UserEvent {
                    user: msg
                        .prefix
                        .and_then(|p| p.user_or_nick().map(|&x| x))
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::EventParseError,
                                "Prefix didn't contain a nick or username",
                            )
                        })?,
                }),
                "PING" => EventContent::Unknown,
                "PONG" => EventContent::Unknown,
                RPL_NAMREPLY => EventContent::Unknown,
                RPL_ENDOFNAMES => EventContent::Unknown,
                _ => EventContent::Unknown,
            },
            tags: Some(msg.tags),
        };
        warn!("{} {:?}", msg.command, msg.command_params);
        warn!("{:?}", evt);
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

    pub fn tags(&self) -> &Option<FnvHashMap<IrcTagKey<T>, Option<T>>> {
        &self.tags
    }
}

#[derive(Debug, Clone)]
pub enum EventContent<T: Debug + Clone> {
    PrivMsg(ChannelMessageEvent<T>),
    Join(UserEvent<T>),
    Mode(ModeChangeEvent<T>),
    Names(NamesListEvent<T>),
    EndOfNames,
    Part(UserEvent<T>),
    ClearChat(ChannelUserEvent<T>),
    ClearMsg(ChannelMessageEvent<T>),
    Host(HostEvent<T>),
    Notice(ChannelMessageEvent<T>),
    Reconnect,
    RoomState(ChannelEvent<T>),
    UserNotice(ChannelMessageEvent<T>),
    UserState(ChannelEvent<T>),
    GlobalUserState,
    Close,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct NamesListEvent<T: Debug + Clone> {
    names: Vec<T>,
}

#[derive(Debug, Clone)]
pub struct ModeChangeEvent<T: Debug + Clone> {
    mode_change: T,
    user: T,
}

#[derive(Debug, Clone)]
pub struct UserEvent<T: Debug + Clone> {
    user: T,
}

#[derive(Debug, Clone)]
pub struct ChannelMessageEvent<T: Debug + Clone> {
    channel: T,
    message: T,
}

#[derive(Debug, Clone)]
pub struct ChannelEvent<T: Debug + Clone> {
    channel: T,
}

#[derive(Debug, Clone)]
pub struct HostEvent<T: Debug + Clone> {
    hosting_channel: T,
    target_channel: Option<T>,
    viewer_count: usize,
}

#[derive(Debug, Clone)]
pub struct ChannelUserEvent<T: Debug + Clone> {
    channel: T,
    user: Option<T>,
}
