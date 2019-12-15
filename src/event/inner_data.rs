//! Contains the structs that hold event specific data, and traits to access their contents in
//! EventData<_, _> structs.

use std::fmt::Debug;

use derive_more::{From, Into};

use crate::event::{EventData, ToOwnedEvent};
use crate::util::RefToString;
use crate::StringRef;

/// Welcome messages that Twitch sends after connection and logging
/// in successfully.
///
/// Includes the following IRC reply codes:
///
/// [RPL_WELCOME](crate::irc_constants::RPL_WELCOME), [RPL_YOURHOST](crate::irc_constants::RPL_YOURHOST),
/// [RPL_CREATED](crate::irc_constants::RPL_CREATED), [RPL_MYINFO](crate::irc_constants::RPL_MYINFO),
/// [RPL_MOTDSTART](crate::irc_constants::RPL_MOTDSTART), [RPL_MOTD](crate::irc_constants::RPL_MOTD),
/// [RPL_ENDOFMOTD](crate::irc_constants::RPL_ENDOFMOTD)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectMessageEvent<T: Debug + Clone + Eq> {
    /// IRC command name, typically 3 digit numeric code
    command: T,
    /// IRC command params
    params: Vec<T>,
}

impl<T: StringRef> ConnectMessageEvent<T> {
    /// New connect event
    pub fn new(command: T, params: Vec<T>) -> Self {
        ConnectMessageEvent { command, params }
    }
}

/// Access to data in connect events
pub trait ConnectMessageEventData<T> {
    /// The command name/code
    fn command(&self) -> &T;
    /// The command parameters
    fn params(&self) -> &[T];
}

impl<T: StringRef> ConnectMessageEventData<T> for EventData<T, ConnectMessageEvent<T>> {
    fn command(&self) -> &T {
        &self.event.command
    }

    fn params(&self) -> &[T] {
        self.event.params.as_slice()
    }
}

impl<T: StringRef> ToOwnedEvent for ConnectMessageEvent<T> {
    type Owned = ConnectMessageEvent<String>;

    fn to_owned_event(&self) -> Self::Owned {
        ConnectMessageEvent {
            command: self.command.ref_to_string(),
            params: self.params.iter().map(RefToString::ref_to_string).collect(),
        }
    }
}

/// Event containing just a username
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UserEvent<T: StringRef> {
    user: T,
}

/// User event data accessors
pub trait UserEventData<T> {
    /// Username contained in the message
    fn user(&self) -> &T;
}

impl<T, Inner> UserEventData<T> for EventData<T, Inner>
where
    T: StringRef,
    Inner: AsRef<UserEvent<T>> + Debug + Clone + Eq,
{
    /// Username contained in the message
    fn user(&self) -> &T {
        &self.event.as_ref().user
    }
}

impl<T: StringRef> ToOwnedEvent for UserEvent<T> {
    type Owned = UserEvent<String>;

    fn to_owned_event(&self) -> Self::Owned {
        UserEvent {
            user: self.user.ref_to_string(),
        }
    }
}

/// Events containing a channel and a message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelMessageEvent<T: StringRef> {
    channel: T,
    message: T,
}

impl<T: StringRef> ChannelMessageEvent<T> {
    /// Create a new event from strings
    pub fn new(channel: T, message: T) -> Self {
        ChannelMessageEvent { channel, message }
    }
}

/// Event message access trait
pub trait MessageEventData<T> {
    /// Actual message
    fn message(&self) -> &T;
}

impl<T, U> MessageEventData<T> for EventData<T, U>
where
    T: StringRef,
    U: Debug + Clone + Eq + AsRef<ChannelMessageEvent<T>>,
{
    #[inline]
    fn message(&self) -> &T {
        &self.event.as_ref().message
    }
}

/// Accessors for channel message event data
pub trait ChannelMessageEventData<T> {
    /// Channel name for the message
    fn channel(&self) -> &T;
}

impl<T, U> ChannelMessageEventData<T> for EventData<T, U>
where
    T: StringRef,
    U: Debug + Clone + Eq + AsRef<ChannelMessageEvent<T>>,
{
    /// Get the channel this message was sent from
    #[inline]
    fn channel(&self) -> &T {
        &self.event.as_ref().channel
    }
}

impl<T: StringRef> ToOwnedEvent for ChannelMessageEvent<T> {
    type Owned = ChannelMessageEvent<String>;

    fn to_owned_event(&self) -> Self::Owned {
        ChannelMessageEvent {
            channel: self.channel.ref_to_string(),
            message: self.message.ref_to_string(),
        }
    }
}

/// Event containing only a channel
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelEvent<T: StringRef> {
    channel: T,
}

/// Data accessor for channel events
pub trait ChannelEventData<T> {
    /// Channel name contained in the message
    fn channel(&self) -> &T;
}

impl<T, Inner> ChannelEventData<T> for EventData<T, Inner>
where
    T: StringRef,
    Inner: AsRef<ChannelEvent<T>> + Debug + Clone + Eq,
{
    #[inline]
    fn channel(&self) -> &T {
        &self.event.as_ref().channel
    }
}

impl<T: StringRef> ChannelEvent<T> {
    /// Create a new event from strings
    pub fn new(channel: T) -> Self {
        ChannelEvent { channel }
    }
}

impl<T: StringRef> ToOwnedEvent for ChannelEvent<T> {
    type Owned = ChannelEvent<String>;

    fn to_owned_event(&self) -> Self::Owned {
        ChannelEvent {
            channel: self.channel.ref_to_string(),
        }
    }
}

/// Event containing a channel and a username
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelUserEvent<T: StringRef> {
    channel: T,
    user: Option<T>,
}

impl<T: StringRef> ChannelUserEvent<T> {
    /// Create a new event from strings
    pub fn new(channel: T, user: Option<T>) -> Self {
        ChannelUserEvent { channel, user }
    }
}

/// Channel/user event data access
pub trait ChannelUserEventData<T> {
    /// Channel contained in the message
    fn channel(&self) -> &T;
    /// User contained in the message
    fn user(&self) -> Option<&T>;
}

impl<T, Inner> ChannelUserEventData<T> for EventData<T, Inner>
where
    T: StringRef,
    Inner: AsRef<ChannelUserEvent<T>> + Debug + Clone + Eq,
{
    fn channel(&self) -> &T {
        &self.event.as_ref().channel
    }

    fn user(&self) -> Option<&T> {
        self.event.as_ref().user.as_ref()
    }
}

impl<T: StringRef> ToOwnedEvent for ChannelUserEvent<T> {
    type Owned = ChannelUserEvent<String>;

    fn to_owned_event(&self) -> Self::Owned {
        ChannelUserEvent {
            channel: self.channel.ref_to_string(),
            user: self.user.as_ref().map(RefToString::ref_to_string),
        }
    }
}

macro_rules! impl_inner_to_owned {
    ($type:ident, $inner:ident) => {
        impl<T: StringRef> ToOwnedEvent for $type<T> {
            type Owned = $type<String>;

            fn to_owned_event(&self) -> Self::Owned {
                $type(self.0.to_owned_event())
            }
        }

        impl<'a, T: StringRef> AsRef<$inner<T>> for $type<T> {
            fn as_ref(&self) -> &$inner<T> {
                &self.0
            }
        }
    };
}

/// PRIVMSG event contents
#[derive(Debug, Clone, Eq, PartialEq, From, Into)]
pub struct PrivMsgEvent<T: StringRef>(ChannelMessageEvent<T>);
impl_inner_to_owned!(PrivMsgEvent, ChannelMessageEvent);

/// JOIN event contents
#[derive(Debug, Clone, Eq, PartialEq, From, Into)]
pub struct JoinEvent<T: StringRef>(ChannelEvent<T>);
impl_inner_to_owned!(JoinEvent, ChannelEvent);

/// End of NAMES list event content
#[derive(Debug, Clone, Eq, PartialEq, From, Into)]
pub struct EndOfNamesEvent<T: StringRef>(ChannelEvent<T>);
impl_inner_to_owned!(EndOfNamesEvent, ChannelEvent);

/// PART event content
#[derive(Debug, Clone, Eq, PartialEq, From, Into)]
pub struct PartEvent<T: StringRef>(ChannelEvent<T>);
impl_inner_to_owned!(PartEvent, ChannelEvent);

/// CLEARCHAT event content
#[derive(Debug, Clone, Eq, PartialEq, From, Into)]
pub struct ClearChatEvent<T: StringRef>(ChannelUserEvent<T>);
impl_inner_to_owned!(ClearChatEvent, ChannelUserEvent);

/// CLEARMSG event content
#[derive(Debug, Clone, Eq, PartialEq, From, Into)]
pub struct ClearMsgEvent<T: StringRef>(ChannelMessageEvent<T>);
impl_inner_to_owned!(ClearMsgEvent, ChannelMessageEvent);

/// NOTICE event content
#[derive(Debug, Clone, Eq, PartialEq, From, Into)]
pub struct NoticeEvent<T: StringRef>(ChannelMessageEvent<T>);
impl_inner_to_owned!(NoticeEvent, ChannelMessageEvent);

/// RECONNECT event
#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub struct ReconnectEvent;

/// ROOMSTATE event
#[derive(Debug, Clone, Eq, PartialEq, From, Into)]
pub struct RoomStateEvent<T: StringRef>(ChannelEvent<T>);
impl_inner_to_owned!(RoomStateEvent, ChannelEvent);

/// USERNOTICE event
#[derive(Debug, Clone, Eq, PartialEq, From, Into)]
pub struct UserNoticeEvent<T: StringRef>(ChannelMessageEvent<T>);
impl_inner_to_owned!(UserNoticeEvent, ChannelMessageEvent);

/// USERSTATE event
#[derive(Debug, Clone, Eq, PartialEq, From, Into)]
pub struct UserStateEvent<T: StringRef>(ChannelEvent<T>);
impl_inner_to_owned!(UserStateEvent, ChannelEvent);

/// GLOBALUSERSTATE event
#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub struct GlobalUserStateEvent;

/// Connection close event
#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub struct CloseEvent;

/// IRC PING event
#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub struct PingEvent;

/// IRC PONG event
#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub struct PongEvent;

/// Unknown event that could not be parsed.
#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub struct UnknownEvent;

/// NAMES list response data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamesListEvent<T: StringRef> {
    user: T,
    channel: T,
    names: Vec<T>,
}

impl<T: StringRef> NamesListEvent<T> {
    /// New names event content
    pub fn new(user: T, channel: T, names: Vec<T>) -> Self {
        NamesListEvent {
            user,
            channel,
            names,
        }
    }
}

/// Data accessors for NAMES events
pub trait NamesEventData<T: StringRef> {
    /// Current user
    fn user(&self) -> &T;
    /// Channel that the user list is for
    fn channel(&self) -> &T;
    /// List of user names
    fn names(&self) -> &[T];
}

impl<T, U> NamesEventData<T> for EventData<T, U>
where
    T: StringRef,
    U: Debug + Clone + Eq + AsRef<NamesListEvent<T>>,
{
    fn user(&self) -> &T {
        &self.event.as_ref().user
    }

    fn channel(&self) -> &T {
        &self.event.as_ref().channel
    }

    fn names(&self) -> &[T] {
        &self.event.as_ref().names
    }
}

impl<T: StringRef> ToOwnedEvent for NamesListEvent<T> {
    type Owned = NamesListEvent<String>;

    fn to_owned_event(&self) -> Self::Owned {
        NamesListEvent {
            user: self.user.ref_to_string(),
            channel: self.channel.ref_to_string(),
            names: self.names.iter().map(RefToString::ref_to_string).collect(),
        }
    }
}

/// User mode change event
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeChangeEvent<T: StringRef> {
    channel: T,
    mode_change: T,
    user: T,
}

impl<T: StringRef> ModeChangeEvent<T> {
    /// Create MODE event content
    pub fn new(channel: T, mode_change: T, user: T) -> Self {
        ModeChangeEvent {
            channel,
            mode_change,
            user,
        }
    }
}

/// Data accessors for MODE events
pub trait ModeChangeData<T: StringRef> {
    /// Channel
    fn channel(&self) -> &T;
    /// Mode change, for example +o or -o
    fn mode_change(&self) -> &T;
    /// Affected user
    fn user(&self) -> &T;
}

impl<T> ModeChangeData<T> for EventData<T, ModeChangeEvent<T>>
where
    T: StringRef,
{
    #[inline]
    fn channel(&self) -> &T {
        &self.event.channel
    }

    #[inline]
    fn mode_change(&self) -> &T {
        &self.event.mode_change
    }

    #[inline]
    fn user(&self) -> &T {
        &self.event.user
    }
}

impl<T: StringRef> ToOwnedEvent for ModeChangeEvent<T> {
    type Owned = ModeChangeEvent<String>;

    fn to_owned_event(&self) -> Self::Owned {
        ModeChangeEvent {
            channel: self.channel.ref_to_string(),
            mode_change: self.mode_change.ref_to_string(),
            user: self.user.ref_to_string(),
        }
    }
}

/// Whisper message event data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhisperEvent<T: StringRef> {
    recipient: T,
    message: T,
}

impl<T: StringRef> WhisperEvent<T> {
    /// Create WHISPER event content
    pub fn new(recipient: T, message: T) -> WhisperEvent<T> {
        WhisperEvent { recipient, message }
    }
}

/// Whisper event data accessors
pub trait WhisperEventData<T: StringRef> {
    /// Recipient of the whisper
    fn recipient(&self) -> &T;
}

impl<T: StringRef> WhisperEventData<T> for EventData<T, WhisperEvent<T>> {
    #[inline]
    fn recipient(&self) -> &T {
        &self.event.recipient
    }
}

impl<T: StringRef> MessageEventData<T> for EventData<T, WhisperEvent<T>> {
    #[inline]
    fn message(&self) -> &T {
        &self.event.message
    }
}

impl<T: StringRef> ToOwnedEvent for WhisperEvent<T> {
    type Owned = WhisperEvent<String>;

    fn to_owned_event(&self) -> Self::Owned {
        WhisperEvent {
            recipient: self.recipient.ref_to_string(),
            message: self.message.ref_to_string(),
        }
    }
}

/// HOST event data accessors
pub trait HostEventData<T> {
    /// The hosting channel
    fn hosting_channel(&self) -> &T;
    /// The host target channel. `None` if hosting is ended
    fn target_channel(&self) -> Option<&T>;
    /// Number of viewers included in the host (optional)
    fn viewer_count(&self) -> Option<usize>;
}

/// Host event data (hosting channel, target channel, viewers)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostEvent<T: StringRef> {
    hosting_channel: T,
    target_channel: Option<T>,
    viewer_count: Option<usize>,
}

impl<T: StringRef> HostEvent<T> {
    /// New host event
    pub fn new(hosting_channel: T, target_channel: Option<T>, viewer_count: Option<usize>) -> Self {
        HostEvent {
            hosting_channel,
            target_channel,
            viewer_count,
        }
    }
}

impl<T: StringRef> HostEventData<T> for EventData<T, HostEvent<T>> {
    #[inline]
    fn hosting_channel(&self) -> &T {
        &self.event.hosting_channel
    }

    #[inline]
    fn target_channel(&self) -> Option<&T> {
        self.event.target_channel.as_ref()
    }

    #[inline]
    fn viewer_count(&self) -> Option<usize> {
        self.event.viewer_count
    }
}

impl<T: StringRef> ToOwnedEvent for HostEvent<T> {
    type Owned = HostEvent<String>;

    fn to_owned_event(&self) -> Self::Owned {
        HostEvent {
            hosting_channel: self.hosting_channel.ref_to_string(),
            target_channel: self.target_channel.as_ref().map(RefToString::ref_to_string),
            viewer_count: self.viewer_count,
        }
    }
}

/// IRCv3 CAP response data, sent in response to CAP requests
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityEvent<T: Debug + Clone + Eq> {
    /// Parameters of the CAP response
    pub params: Vec<T>,
}

impl<T: StringRef> ToOwnedEvent for CapabilityEvent<T> {
    type Owned = CapabilityEvent<String>;

    fn to_owned_event(&self) -> Self::Owned {
        CapabilityEvent {
            params: self.params.iter().map(RefToString::ref_to_string).collect(),
        }
    }
}

#[cfg(test)]
mod test {
    use std::convert::TryFrom;

    use crate::event::Event;
    use crate::irc::*;

    use super::*;

    #[test]
    fn test_join() {
        let (remaining, msg) =
            IrcMessage::parse(":ronni!ronni@ronni.tmi.twitch.tv JOIN #dallas").unwrap();
        assert_eq!(remaining, "");
        let event: Event<&str> = Event::try_from(msg).unwrap();
        assert_eq!(
            event,
            Event::Join(EventData {
                sender: Some("ronni"),
                event: ChannelEvent::new("#dallas").into(),
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
            Event::Mode(EventData {
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
                Event::Names(EventData {
                    sender: None,
                    event: NamesListEvent {
                        user: "ronni",
                        channel: "#dallas",
                        names: vec!["ronni", "fred", "wilma"]
                    },
                    tags: None
                }),
                Event::Names(EventData {
                    sender: None,
                    event: NamesListEvent {
                        user: "ronni",
                        channel: "#dallas",
                        names: vec!["barney", "betty"]
                    },
                    tags: None
                }),
                Event::EndOfNames(EventData {
                    sender: None,
                    event: ChannelEvent::new("#dallas").into(),
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
            Event::Part(EventData {
                sender: Some("ronni"),
                event: ChannelEvent::new("#dallas").into(),
                tags: None
            })
        )
    }

    #[test]
    fn test_clearchat() {
        let (remaining, msg1) =
            IrcMessage::parse(":tmi.twitch.tv CLEARCHAT #dallas :ronni").unwrap();
        assert_eq!(remaining, "");
        assert_eq!(
            Event::try_from(msg1).unwrap(),
            Event::ClearChat(EventData {
                sender: None,
                event: ChannelUserEvent::new("#dallas", Some("ronni")).into(),
                tags: None
            })
        );

        let (remaining, msg2) = IrcMessage::parse(":tmi.twitch.tv CLEARCHAT #dallas").unwrap();
        assert_eq!(remaining, "");
        assert_eq!(
            Event::try_from(msg2).unwrap(),
            Event::ClearChat(EventData {
                sender: None,
                event: ChannelUserEvent::new("#dallas", None).into(),
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
            Event::ClearMsg(EventData {
                sender: None,
                event: ChannelMessageEvent::new("#dallas", "HeyGuys").into(),
                tags: Some(FnvHashMap::from_iter(
                    vec![
                        ("login", "ronni".to_string()),
                        ("target-msg-id", "abc-123-def".to_string())
                    ]
                    .into_iter()
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
            Event::Host(EventData {
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
            Event::Host(EventData {
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
            Event::Notice(EventData {
                sender: None,
                event: ChannelMessageEvent::new("#dallas", "This room is no longer in slow mode.")
                    .into(),
                tags: Some(FnvHashMap::from_iter(
                    vec![("msg-id", "slow_off".to_string())].into_iter()
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
            Event::RoomState(EventData {
                sender: None,
                event: ChannelEvent::new("#dallas").into(),
                tags: Some(FnvHashMap::from_iter(
                    vec![
                        ("emote-only", "0".to_string()),
                        ("followers-only", "0".to_string()),
                        ("subs-only", "0".to_string()),
                        ("slow", "0".to_string()),
                        ("r9k", "0".to_string())
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
            Event::UserNotice(EventData {
                sender: None,
                event: ChannelMessageEvent::new("#<channel>", "<message>").into(),
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
            Event::UserState(EventData {
                sender: None,
                event: ChannelEvent::new("#dallas").into(),
                tags: None
            })
        )
    }
}
