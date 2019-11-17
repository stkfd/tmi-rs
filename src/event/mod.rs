//! Data types and parsing logic for events that can be received
//! from the twitch servers.

use std::borrow::Borrow;
use std::convert::{Into, TryFrom};
use std::fmt::Debug;

use fnv::FnvHashMap;

pub use inner_data::*;
pub use stream::*;

use crate::event::tags::MessageTags;
use crate::irc::IrcMessage;
use crate::irc_constants::*;
use crate::util::RefToString;
use crate::{Error, StringRef};

mod inner_data;
mod stream;
pub mod tags;

/// Enum containing all event types that can be received from Twitch
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, From)]
pub enum Event<T: StringRef> {
    PrivMsg(EventData<T, PrivMsgEvent<T>>),
    Whisper(EventData<T, WhisperEvent<T>>),
    Join(EventData<T, JoinEvent<T>>),
    Mode(EventData<T, ModeChangeEvent<T>>),
    Names(EventData<T, NamesListEvent<T>>),
    EndOfNames(EventData<T, EndOfNamesEvent<T>>),
    Part(EventData<T, PartEvent<T>>),
    ClearChat(EventData<T, ClearChatEvent<T>>),
    ClearMsg(EventData<T, ClearMsgEvent<T>>),
    Host(EventData<T, HostEvent<T>>),
    Notice(EventData<T, NoticeEvent<T>>),
    Reconnect(EventData<T, ReconnectEvent>),
    RoomState(EventData<T, RoomStateEvent<T>>),
    UserNotice(EventData<T, UserNoticeEvent<T>>),
    UserState(EventData<T, UserStateEvent<T>>),
    Capability(EventData<T, CapabilityEvent<T>>),
    ConnectMessage(EventData<T, ConnectMessageEvent<T>>),
    GlobalUserState(EventData<T, GlobalUserStateEvent>),
    Close(CloseEvent),
    Ping(PingEvent),
    Pong(PongEvent),
    Unknown(UnknownEvent),
}

impl<T> From<&Event<T>> for Event<String>
where
    T: StringRef,
{
    fn from(e: &Event<T>) -> Self {
        match e {
            Event::ConnectMessage(inner) => Event::ConnectMessage(inner.to_owned_event()),
            Event::PrivMsg(inner) => Event::PrivMsg(inner.to_owned_event()),
            Event::Whisper(inner) => Event::Whisper(inner.to_owned_event()),
            Event::Join(inner) => Event::Join(inner.to_owned_event()),
            Event::Mode(inner) => Event::Mode(inner.to_owned_event()),
            Event::Names(inner) => Event::Names(inner.to_owned_event()),
            Event::EndOfNames(inner) => Event::EndOfNames(inner.to_owned_event()),
            Event::Part(inner) => Event::Part(inner.to_owned_event()),
            Event::ClearChat(inner) => Event::ClearChat(inner.to_owned_event()),
            Event::ClearMsg(inner) => Event::ClearMsg(inner.to_owned_event()),
            Event::Host(inner) => Event::Host(inner.to_owned_event()),
            Event::Notice(inner) => Event::Notice(inner.to_owned_event()),
            Event::RoomState(inner) => Event::RoomState(inner.to_owned_event()),
            Event::UserNotice(inner) => Event::UserNotice(inner.to_owned_event()),
            Event::UserState(inner) => Event::UserState(inner.to_owned_event()),
            Event::Capability(inner) => Event::Capability(inner.to_owned_event()),
            Event::GlobalUserState(inner) => Event::GlobalUserState(inner.to_owned_event()),
            Event::Reconnect(inner) => Event::Reconnect(inner.to_owned_event()),
            Event::Close(e) => Event::Close(*e),
            Event::Ping(e) => Event::Ping(*e),
            Event::Pong(e) => Event::Pong(*e),
            Event::Unknown(e) => Event::Unknown(*e),
        }
    }
}

#[inline]
fn check_parameter_count<T: StringRef>(count: usize, msg: &IrcMessage<T>) -> Result<(), Error> {
    if msg.params().len() != count {
        return Err(Error::WrongIrcParameterCount(count, msg.into()));
    }
    Ok(())
}

impl<'a> TryFrom<IrcMessage<&'a str>> for Event<&'a str> {
    type Error = Error;

    /// Attempts to convert any parsed IRC message into a twitch specific
    /// event struct. This doesn't copy any strings or the hash map of tags,
    /// just references.
    fn try_from(msg: IrcMessage<&'a str>) -> Result<Self, Error> {
        let sender = msg.sender().copied();
        Ok(match msg.command {
            "PRIVMSG" => {
                check_parameter_count(2, &msg)?;
                EventData {
                    sender,
                    event: PrivMsgEvent::from(ChannelMessageEvent::new(
                        *msg.param(0),
                        *msg.param(1),
                    )),
                    tags: msg.tags,
                }
                .into()
            }
            "WHISPER" => EventData {
                sender,
                event: WhisperEvent::<&str>::new(msg.try_param(0)?, msg.try_param(1)?),
                tags: msg.tags,
            }
            .into(),
            "JOIN" => EventData {
                sender,
                event: JoinEvent::from(ChannelEvent::<&str>::new(msg.try_param(0)?)),
                tags: msg.tags,
            }
            .into(),
            "MODE" => {
                check_parameter_count(3, &msg)?;
                EventData {
                    sender,
                    event: ModeChangeEvent::<&str>::new(msg.param(0), msg.param(1), msg.param(2)),
                    tags: msg.tags,
                }
                .into()
            }
            RPL_NAMREPLY => {
                check_parameter_count(4, &msg)?;
                EventData {
                    sender,
                    event: NamesListEvent::<&str>::new(
                        msg.param(0),
                        msg.param(2),
                        msg.param(3).split(' ').collect(),
                    ),
                    tags: msg.tags,
                }
                .into()
            }
            RPL_ENDOFNAMES => EventData {
                sender,
                event: EndOfNamesEvent::from(ChannelEvent::<&str>::new(msg.try_param(1)?)),
                tags: msg.tags,
            }
            .into(),
            "PART" => EventData {
                sender,
                event: PartEvent::from(ChannelEvent::<&str>::new(msg.try_param(0)?)),
                tags: msg.tags,
            }
            .into(),
            "CLEARCHAT" => EventData {
                sender,
                event: ClearChatEvent::from(ChannelUserEvent::<&str>::new(
                    msg.try_param(0)?,
                    msg.params().get(1).copied(),
                )),
                tags: msg.tags,
            }
            .into(),
            "CLEARMSG" => {
                check_parameter_count(2, &msg)?;
                EventData {
                    sender,
                    event: ClearMsgEvent::from(ChannelMessageEvent::new(
                        *msg.try_param(0)?,
                        *msg.try_param(1)?,
                    )),
                    tags: msg.tags,
                }
                .into()
            }
            "HOSTTARGET" => {
                let host_parts: Vec<_> = msg.param(1).split(' ').collect();
                let hosting_channel = msg.param(0);
                let target_channel = if host_parts[0] == "-" {
                    None
                } else {
                    host_parts.get(0).copied()
                };
                let viewer_count = host_parts
                    .get(1)
                    .copied()
                    .and_then(|num| num.parse::<usize>().ok());
                EventData {
                    sender,
                    event: HostEvent::<&str>::new(hosting_channel, target_channel, viewer_count),
                    tags: msg.tags,
                }
                .into()
            }
            "NOTICE" => {
                check_parameter_count(2, &msg)?;
                EventData {
                    sender,
                    event: NoticeEvent::from(ChannelMessageEvent::<&str>::new(
                        msg.param(0),
                        msg.param(1),
                    )),
                    tags: msg.tags,
                }
                .into()
            }
            "RECONNECT" => EventData {
                sender,
                event: ReconnectEvent,
                tags: msg.tags,
            }
            .into(),
            "ROOMSTATE" => EventData {
                sender,
                event: RoomStateEvent::from(ChannelEvent::<&str>::new(msg.try_param(0)?)),
                tags: msg.tags,
            }
            .into(),
            "USERNOTICE" => {
                check_parameter_count(2, &msg)?;
                EventData {
                    sender,
                    event: UserNoticeEvent::from(ChannelMessageEvent::<&str>::new(
                        msg.param(0),
                        msg.param(1),
                    )),
                    tags: msg.tags,
                }
                .into()
            }
            "USERSTATE" => EventData {
                sender,
                event: UserStateEvent::from(ChannelEvent::<&str>::new(msg.try_param(0)?)),
                tags: msg.tags,
            }
            .into(),
            "CAP" => EventData {
                sender,
                event: CapabilityEvent {
                    params: msg.params().to_vec(),
                },
                tags: msg.tags,
            }
            .into(),
            RPL_WELCOME | RPL_YOURHOST | RPL_CREATED | RPL_MYINFO | RPL_MOTDSTART | RPL_MOTD
            | RPL_ENDOFMOTD => EventData {
                sender,
                event: ConnectMessageEvent::new(msg.command, msg.params().to_vec()),
                tags: msg.tags,
            }
            .into(),
            "GLOBALUSERSTATE" => EventData {
                sender,
                event: GlobalUserStateEvent,
                tags: msg.tags,
            }
            .into(),
            "PING" => PingEvent.into(),
            "PONG" => PongEvent.into(),
            _ => return Err(Error::UnknownIrcCommand((&msg).into())),
        })
    }
}

/// Converts from events with inner reference types into owned versions
pub trait ToOwnedEvent {
    /// Owned version of the event type
    type Owned;
    /// Convert the event to its owned version
    fn to_owned_event(&self) -> Self::Owned;
}

impl<T: Copy> ToOwnedEvent for T {
    type Owned = T;
    fn to_owned_event(&self) -> Self::Owned {
        *self
    }
}

/// Content of a received message. Contains the sender, tags and and a generic `Inner` which
/// contains the data specific to each event type.
///
/// Helper functions to access the contained data (message, target, specific tags) are implemented
/// for for the EventContent<T, Inner> types they apply to.  
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventData<T, Inner>
where
    T: StringRef,
    Inner: Debug + Clone + Eq,
{
    /// Sender of the message, if applicable
    pub(crate) sender: Option<T>,
    /// Inner type specific event data
    pub(crate) event: Inner,
    /// Map of IRCv3 tags
    pub(crate) tags: Option<FnvHashMap<T, String>>,
}

/// Methods common to all EventContent variants
impl<T, Inner> EventData<T, Inner>
where
    T: StringRef,
    Inner: Debug + Clone + Eq,
    Event<T>: From<EventData<T, Inner>>,
{
    /// Get the sender of the message
    pub fn sender(&self) -> &Option<T> {
        &self.sender
    }

    /// Get the data of the inner event type
    pub fn event(&self) -> &Inner {
        &self.event
    }
}

impl<T, Inner> MessageTags<T> for EventData<T, Inner>
where
    T: StringRef,
    Inner: Clone + Debug + Eq,
    Event<T>: From<EventData<T, Inner>>,
{
    /// Get the map of all IRCv3 tags
    fn tags(&self) -> &Option<FnvHashMap<T, String>> {
        &self.tags
    }

    /// Get a tag value from the message by its key
    fn tag<Q: Borrow<str>>(&self, key: Q) -> Option<&str> {
        self.tags
            .as_ref()
            .and_then(|tags| tags.get(key.borrow()).map(|s| s.as_str()))
    }

    /// Gets a tag value, returns an Error if the value is not set. Intended for use in
    /// internal tag accessor functions where the tag should always be available
    fn required_tag<Q: Borrow<str>>(&self, key: Q) -> Result<&str, Error> {
        self.tag(key.borrow()).ok_or_else(|| Error::MissingTag {
            tag: key.borrow().to_string().into(),
            event: (&Event::<T>::from(self.clone())).into(),
        })
    }
}

/// Generic ToOwned implementation for all EventContent variants
impl<T, Inner> ToOwnedEvent for EventData<T, Inner>
where
    T: StringRef,
    Inner: Debug + Clone + Eq + ToOwnedEvent,
    Inner::Owned: Debug + Clone + Eq,
{
    type Owned = EventData<String, Inner::Owned>;

    fn to_owned_event(&self) -> Self::Owned {
        EventData {
            sender: self.sender.as_ref().map(RefToString::ref_to_string),
            event: self.event.to_owned_event(),
            tags: self.tags.as_ref().map(|hash_map| {
                hash_map
                    .iter()
                    .map(|(key, val)| (key.ref_to_string(), val.ref_to_string()))
                    .collect::<FnvHashMap<String, String>>()
            }),
        }
    }
}
