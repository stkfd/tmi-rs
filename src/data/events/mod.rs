use std::convert::TryFrom;
use std::fmt::Debug;

use crate::irc::IrcMessage;
use crate::irc_constants::replies::*;
use crate::{Error, StringRef};

mod event_content;
pub use event_content::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<T: StringRef> {
    PrivMsg(EventContent<T, ChannelMessageEvent<T>>),
    Whisper(EventContent<T, WhisperEvent<T>>),
    Join(EventContent<T, ChannelEvent<T>>),
    Mode(EventContent<T, ModeChangeEvent<T>>),
    Names(EventContent<T, NamesListEvent<T>>),
    EndOfNames(EventContent<T, ChannelEvent<T>>),
    Part(EventContent<T, ChannelEvent<T>>),
    ClearChat(EventContent<T, ChannelUserEvent<T>>),
    ClearMsg(EventContent<T, ChannelMessageEvent<T>>),
    Host(EventContent<T, HostEvent<T>>),
    Notice(EventContent<T, ChannelMessageEvent<T>>),
    Reconnect(EventContent<T, ()>),
    RoomState(EventContent<T, ChannelEvent<T>>),
    UserNotice(EventContent<T, ChannelMessageEvent<T>>),
    UserState(EventContent<T, ChannelEvent<T>>),
    Capability(EventContent<T, CapabilityEvent<T>>),
    ConnectMessage(EventContent<T, ConnectMessage<T>>),
    GlobalUserState(EventContent<T, ()>),
    Close,
    Ping,
    Pong,
    Unknown,
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
            Event::Close => Event::Close,
            Event::Ping => Event::Ping,
            Event::Pong => Event::Pong,
            Event::Unknown => Event::Unknown,
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
                Event::PrivMsg(EventContent {
                    sender,
                    event: ChannelMessageEvent {
                        channel: msg.param(0),
                        message: msg.param(1),
                    },
                    tags: msg.tags,
                })
            }
            "WHISPER" => Event::Whisper(EventContent {
                sender,
                event: WhisperEvent {
                    recipient: msg.try_param(0)?,
                    message: msg.try_param(1)?,
                },
                tags: msg.tags,
            }),
            "JOIN" => Event::Join(EventContent {
                sender,
                event: ChannelEvent {
                    channel: msg.try_param(0)?,
                },
                tags: msg.tags,
            }),
            "MODE" => {
                check_parameter_count(3, &msg)?;
                Event::Mode(EventContent {
                    sender,
                    event: ModeChangeEvent {
                        channel: msg.param(0),
                        mode_change: msg.param(1),
                        user: msg.param(2),
                    },
                    tags: msg.tags,
                })
            }
            RPL_NAMREPLY => {
                check_parameter_count(4, &msg)?;
                Event::Names(EventContent {
                    sender,
                    event: NamesListEvent::<&str> {
                        user: msg.param(0),
                        channel: msg.param(2),
                        names: msg.param(3).split(' ').collect(),
                    },
                    tags: msg.tags,
                })
            }
            RPL_ENDOFNAMES => Event::EndOfNames(EventContent {
                sender,
                event: ChannelEvent {
                    channel: msg.try_param(1)?,
                },
                tags: msg.tags,
            }),
            "PART" => Event::Part(EventContent {
                sender,
                event: ChannelEvent {
                    channel: msg.try_param(0)?,
                },
                tags: msg.tags,
            }),
            "CLEARCHAT" => Event::ClearChat(EventContent {
                sender,
                event: ChannelUserEvent {
                    channel: msg.try_param(0)?,
                    user: msg.params().get(1).copied(),
                },
                tags: msg.tags,
            }),
            "CLEARMSG" => {
                check_parameter_count(2, &msg)?;
                Event::ClearMsg(EventContent {
                    sender,
                    event: ChannelMessageEvent {
                        channel: msg.try_param(0)?,
                        message: msg.try_param(1)?,
                    },
                    tags: msg.tags,
                })
            }
            "HOSTTARGET" => {
                let host_parts: Vec<_> = msg.param(1).split(' ').collect();
                Event::Host(EventContent {
                    sender,
                    event: HostEvent {
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
                })
            }
            "NOTICE" => {
                check_parameter_count(2, &msg)?;
                Event::Notice(EventContent {
                    sender,
                    event: ChannelMessageEvent {
                        channel: msg.param(0),
                        message: msg.param(1),
                    },
                    tags: msg.tags,
                })
            }
            "RECONNECT" => Event::Reconnect(EventContent {
                sender,
                event: (),
                tags: msg.tags,
            }),
            "ROOMSTATE" => Event::RoomState(EventContent {
                sender,
                event: ChannelEvent {
                    channel: msg.try_param(0)?,
                },
                tags: msg.tags,
            }),
            "USERNOTICE" => {
                check_parameter_count(2, &msg)?;
                Event::UserNotice(EventContent {
                    sender,
                    event: ChannelMessageEvent {
                        channel: msg.param(0),
                        message: msg.param(1),
                    },
                    tags: msg.tags,
                })
            }
            "USERSTATE" => Event::UserState(EventContent {
                sender,
                event: ChannelEvent {
                    channel: msg.try_param(0)?,
                },
                tags: msg.tags,
            }),
            "CAP" => Event::Capability(EventContent {
                sender,
                event: CapabilityEvent {
                    params: msg.params().to_vec(),
                },
                tags: msg.tags,
            }),
            RPL_WELCOME | RPL_YOURHOST | RPL_CREATED | RPL_MYINFO | RPL_MOTDSTART | RPL_MOTD
            | RPL_ENDOFMOTD => Event::ConnectMessage(EventContent {
                sender,
                event: ConnectMessage {
                    command: msg.command,
                    params: msg.params().to_vec(),
                },
                tags: msg.tags,
            }),
            "GLOBALUSERSTATE" => Event::GlobalUserState(EventContent {
                sender,
                event: (),
                tags: msg.tags,
            }),
            "PING" => Event::Ping,
            "PONG" => Event::Pong,
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
            event: ChannelEvent { channel: "#dallas" },
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
                event: ChannelEvent { channel: "#dallas" },
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
            event: ChannelEvent { channel: "#dallas" },
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
            },
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
            },
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
            },
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
            },
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
            event: ChannelEvent { channel: "#dallas" },
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
            },
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
            event: ChannelEvent { channel: "#dallas" },
            tags: None
        })
    )
}
