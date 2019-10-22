use std::convert::TryFrom;
use std::fmt::Debug;

pub use event_types::*;
pub use stream::*;

use crate::irc::IrcMessage;
use crate::irc_constants::replies::*;
use crate::{Error, StringRef};

mod event_types;
mod stream;

#[derive(Debug, Clone, PartialEq, Eq, From)]
pub enum Event<T: StringRef> {
    PrivMsg(EventContent<T, PrivMsgEvent<T>>),
    Whisper(EventContent<T, WhisperEvent<T>>),
    Join(EventContent<T, JoinEvent<T>>),
    Mode(EventContent<T, ModeChangeEvent<T>>),
    Names(EventContent<T, NamesListEvent<T>>),
    EndOfNames(EventContent<T, EndOfNamesEvent<T>>),
    Part(EventContent<T, PartEvent<T>>),
    ClearChat(EventContent<T, ClearChatEvent<T>>),
    ClearMsg(EventContent<T, ClearMsgEvent<T>>),
    Host(EventContent<T, HostEvent<T>>),
    Notice(EventContent<T, NoticeEvent<T>>),
    Reconnect(EventContent<T, ReconnectEvent>),
    RoomState(EventContent<T, RoomStateEvent<T>>),
    UserNotice(EventContent<T, UserNoticeEvent<T>>),
    UserState(EventContent<T, UserStateEvent<T>>),
    Capability(EventContent<T, CapabilityEvent<T>>),
    ConnectMessage(EventContent<T, ConnectMessageEvent<T>>),
    GlobalUserState(EventContent<T, GlobalUserStateEvent>),
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
                EventContent {
                    sender,
                    event: PrivMsgEvent::from(ChannelMessageEvent::<&str> {
                        channel: msg.param(0),
                        message: msg.param(1),
                    }),
                    tags: msg.tags,
                }
                .into()
            }
            "WHISPER" => EventContent {
                sender,
                event: WhisperEvent::<&str> {
                    recipient: msg.try_param(0)?,
                    message: msg.try_param(1)?,
                },
                tags: msg.tags,
            }
            .into(),
            "JOIN" => EventContent {
                sender,
                event: JoinEvent::from(ChannelEvent::<&str> {
                    channel: msg.try_param(0)?,
                }),
                tags: msg.tags,
            }
            .into(),
            "MODE" => {
                check_parameter_count(3, &msg)?;
                EventContent {
                    sender,
                    event: ModeChangeEvent::<&str> {
                        channel: msg.param(0),
                        mode_change: msg.param(1),
                        user: msg.param(2),
                    },
                    tags: msg.tags,
                }
                .into()
            }
            RPL_NAMREPLY => {
                check_parameter_count(4, &msg)?;
                EventContent {
                    sender,
                    event: NamesListEvent::<&str> {
                        user: msg.param(0),
                        channel: msg.param(2),
                        names: msg.param(3).split(' ').collect(),
                    },
                    tags: msg.tags,
                }
                .into()
            }
            RPL_ENDOFNAMES => EventContent {
                sender,
                event: EndOfNamesEvent::from(ChannelEvent::<&str> {
                    channel: msg.try_param(1)?,
                }),
                tags: msg.tags,
            }
            .into(),
            "PART" => EventContent {
                sender,
                event: PartEvent::from(ChannelEvent::<&str> {
                    channel: msg.try_param(0)?,
                }),
                tags: msg.tags,
            }
            .into(),
            "CLEARCHAT" => EventContent {
                sender,
                event: ClearChatEvent::from(ChannelUserEvent::<&str> {
                    channel: msg.try_param(0)?,
                    user: msg.params().get(1).copied(),
                }),
                tags: msg.tags,
            }
            .into(),
            "CLEARMSG" => {
                check_parameter_count(2, &msg)?;
                EventContent {
                    sender,
                    event: ClearMsgEvent::from(ChannelMessageEvent::<&str> {
                        channel: msg.try_param(0)?,
                        message: msg.try_param(1)?,
                    }),
                    tags: msg.tags,
                }
                .into()
            }
            "HOSTTARGET" => {
                let host_parts: Vec<_> = msg.param(1).split(' ').collect();
                EventContent {
                    sender,
                    event: HostEvent::<&str> {
                        hosting_channel: msg.param(0),
                        target_channel: if host_parts[0] == "-" {
                            None
                        } else {
                            host_parts.get(0).copied()
                        },
                        viewer_count: host_parts
                            .get(1)
                            .copied()
                            .and_then(|num| num.parse::<usize>().ok()),
                    },
                    tags: msg.tags,
                }
                .into()
            }
            "NOTICE" => {
                check_parameter_count(2, &msg)?;
                EventContent {
                    sender,
                    event: NoticeEvent::from(ChannelMessageEvent::<&str> {
                        channel: msg.param(0),
                        message: msg.param(1),
                    }),
                    tags: msg.tags,
                }
                .into()
            }
            "RECONNECT" => EventContent {
                sender,
                event: ReconnectEvent,
                tags: msg.tags,
            }
            .into(),
            "ROOMSTATE" => EventContent {
                sender,
                event: RoomStateEvent::from(ChannelEvent::<&str> {
                    channel: msg.try_param(0)?,
                }),
                tags: msg.tags,
            }
            .into(),
            "USERNOTICE" => {
                check_parameter_count(2, &msg)?;
                EventContent {
                    sender,
                    event: UserNoticeEvent::from(ChannelMessageEvent::<&str> {
                        channel: msg.param(0),
                        message: msg.param(1),
                    }),
                    tags: msg.tags,
                }
                .into()
            }
            "USERSTATE" => EventContent {
                sender,
                event: UserStateEvent::from(ChannelEvent::<&str> {
                    channel: msg.try_param(0)?,
                }),
                tags: msg.tags,
            }
            .into(),
            "CAP" => EventContent {
                sender,
                event: CapabilityEvent {
                    params: msg.params().to_vec(),
                },
                tags: msg.tags,
            }
            .into(),
            RPL_WELCOME | RPL_YOURHOST | RPL_CREATED | RPL_MYINFO | RPL_MOTDSTART | RPL_MOTD
            | RPL_ENDOFMOTD => EventContent {
                sender,
                event: ConnectMessageEvent {
                    command: msg.command,
                    params: msg.params().to_vec(),
                },
                tags: msg.tags,
            }
            .into(),
            "GLOBALUSERSTATE" => EventContent {
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

#[test]
fn test_join() {
    let (remaining, msg) =
        IrcMessage::parse(":ronni!ronni@ronni.tmi.twitch.tv JOIN #dallas").unwrap();
    assert_eq!(remaining, "");
    let event: Event<&str> = Event::try_from(msg).unwrap();
    assert_eq!(
        event,
        Event::Join(EventContent {
            sender: Some("ronni"),
            event: ChannelEvent { channel: "#dallas" }.into(),
            tags: None
        })
    )
}

#[test]
fn test_mode() {
    let (remaining, msg) = IrcMessage::parse(":jtv MODE #dallas +o ronni").unwrap();
    assert_eq!(remaining, "");
    let event: Event<&str> = Event::try_from(msg).unwrap();
    assert_eq!(
        event,
        Event::Mode(EventContent {
            sender: Some("jtv"),
            event: ModeChangeEvent {
                channel: "#dallas",
                mode_change: "+o",
                user: "ronni"
            },
            tags: None
        })
    )
}

#[test]
fn test_names() {
    let msg = vec![
        ":ronni.tmi.twitch.tv 353 ronni = #dallas :ronni fred wilma",
        ":ronni.tmi.twitch.tv 353 ronni = #dallas :barney betty",
        ":ronni.tmi.twitch.tv 366 ronni #dallas :End of /NAMES list",
    ]
    .into_iter()
    .map(IrcMessage::parse)
    .map(|result| {
        let (remaining, parsed) = result.unwrap();
        assert_eq!(remaining, "");
        parsed
    })
    .collect::<Vec<_>>();
    info!("{:?}", msg);
    let events: Vec<Event<&str>> = msg
        .into_iter()
        .map(Event::try_from)
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        events,
        vec![
            Event::Names(EventContent {
                sender: None,
                event: NamesListEvent {
                    user: "ronni",
                    channel: "#dallas",
                    names: vec!["ronni", "fred", "wilma"]
                },
                tags: None
            }),
            Event::Names(EventContent {
                sender: None,
                event: NamesListEvent {
                    user: "ronni",
                    channel: "#dallas",
                    names: vec!["barney", "betty"]
                },
                tags: None
            }),
            Event::EndOfNames(EventContent {
                sender: None,
                event: ChannelEvent { channel: "#dallas" }.into(),
                tags: None
            })
        ]
    )
}

#[test]
fn test_part() {
    let (remaining, msg) =
        IrcMessage::parse(":ronni!ronni@ronni.tmi.twitch.tv PART #dallas").unwrap();
    assert_eq!(remaining, "");
    let event: Event<&str> = Event::try_from(msg).unwrap();
    assert_eq!(
        event,
        Event::Part(EventContent {
            sender: Some("ronni"),
            event: ChannelEvent { channel: "#dallas" }.into(),
            tags: None
        })
    )
}

#[test]
fn test_clearchat() {
    let (remaining, msg1) = IrcMessage::parse(":tmi.twitch.tv CLEARCHAT #dallas :ronni").unwrap();
    assert_eq!(remaining, "");
    assert_eq!(
        Event::try_from(msg1).unwrap(),
        Event::ClearChat(EventContent {
            sender: None,
            event: ChannelUserEvent {
                channel: "#dallas",
                user: Some("ronni")
            }
            .into(),
            tags: None
        })
    );

    let (remaining, msg2) = IrcMessage::parse(":tmi.twitch.tv CLEARCHAT #dallas").unwrap();
    assert_eq!(remaining, "");
    assert_eq!(
        Event::try_from(msg2).unwrap(),
        Event::ClearChat(EventContent {
            sender: None,
            event: ChannelUserEvent {
                channel: "#dallas",
                user: None
            }
            .into(),
            tags: None
        })
    );
}

#[test]
fn test_clearmsg() {
    use fnv::FnvHashMap;
    use std::iter::FromIterator;

    let (remaining, msg) = IrcMessage::parse(
        "@login=ronni;target-msg-id=abc-123-def :tmi.twitch.tv CLEARMSG #dallas :HeyGuys",
    )
    .unwrap();
    assert_eq!(remaining, "");
    let event: Event<&str> = Event::try_from(msg).unwrap();
    assert_eq!(
        event,
        Event::ClearMsg(EventContent {
            sender: None,
            event: ChannelMessageEvent {
                channel: "#dallas",
                message: "HeyGuys"
            }
            .into(),
            tags: Some(FnvHashMap::from_iter(
                vec![("login", "ronni"), ("target-msg-id", "abc-123-def")].into_iter()
            ))
        })
    )
}

#[test]
fn test_host() {
    let (remaining, msg) =
        IrcMessage::parse(":tmi.twitch.tv HOSTTARGET #hosting_channel :<channel> 999").unwrap();
    assert_eq!(remaining, "");
    assert_eq!(
        Event::try_from(msg).unwrap(),
        Event::Host(EventContent {
            sender: None,
            event: HostEvent {
                hosting_channel: "#hosting_channel",
                target_channel: Some("<channel>"),
                viewer_count: Some(999)
            },
            tags: None
        })
    )
}

#[test]
fn test_host_end() {
    let (remaining, msg) =
        IrcMessage::parse(":tmi.twitch.tv HOSTTARGET #hosting_channel :-").unwrap();
    assert_eq!(remaining, "");
    assert_eq!(
        Event::try_from(msg).unwrap(),
        Event::Host(EventContent {
            sender: None,
            event: HostEvent {
                hosting_channel: "#hosting_channel",
                target_channel: None,
                viewer_count: None
            },
            tags: None
        })
    )
}

#[test]
fn test_notice() {
    use fnv::FnvHashMap;
    use std::iter::FromIterator;
    let (remaining, msg) = IrcMessage::parse(
        "@msg-id=slow_off :tmi.twitch.tv NOTICE #dallas :This room is no longer in slow mode.",
    )
    .unwrap();
    assert_eq!(remaining, "");
    assert_eq!(
        Event::try_from(msg).unwrap(),
        Event::Notice(EventContent {
            sender: None,
            event: ChannelMessageEvent {
                channel: "#dallas",
                message: "This room is no longer in slow mode."
            }
            .into(),
            tags: Some(FnvHashMap::from_iter(
                vec![("msg-id", "slow_off")].into_iter()
            ))
        })
    )
}

#[test]
fn test_roomstate() {
    use fnv::FnvHashMap;
    use std::iter::FromIterator;
    let (remaining, msg) = IrcMessage::parse(
        "@emote-only=0;followers-only=0;r9k=0;slow=0;subs-only=0 :tmi.twitch.tv ROOMSTATE #dallas",
    )
    .unwrap();
    assert_eq!(remaining, "");
    assert_eq!(
        Event::try_from(msg).unwrap(),
        Event::RoomState(EventContent {
            sender: None,
            event: ChannelEvent { channel: "#dallas" }.into(),
            tags: Some(FnvHashMap::from_iter(
                vec![
                    ("emote-only", "0"),
                    ("followers-only", "0"),
                    ("subs-only", "0"),
                    ("slow", "0"),
                    ("r9k", "0")
                ]
                .into_iter()
            ))
        })
    )
}

#[test]
fn test_usernotice() {
    // this doesn't test the available tags for usernotice
    let (remaining, msg) =
        IrcMessage::parse(":tmi.twitch.tv USERNOTICE #<channel> :<message>").unwrap();
    assert_eq!(remaining, "");
    assert_eq!(
        Event::try_from(msg).unwrap(),
        Event::UserNotice(EventContent {
            sender: None,
            event: ChannelMessageEvent {
                channel: "#<channel>",
                message: "<message>"
            }
            .into(),
            tags: None
        })
    )
}

#[test]
fn test_userstate() {
    // this doesn't test the available tags for userstate
    let (remaining, msg) = IrcMessage::parse(":tmi.twitch.tv USERSTATE #dallas").unwrap();
    assert_eq!(remaining, "");
    assert_eq!(
        Event::try_from(msg).unwrap(),
        Event::UserState(EventContent {
            sender: None,
            event: ChannelEvent { channel: "#dallas" }.into(),
            tags: None
        })
    )
}
