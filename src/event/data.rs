use std::fmt::Debug;

use crate::event::{EventData, ToOwnedEvent};
use crate::util::RefToString;
use crate::StringRef;

/// Welcome messages that Twitch sends after connection and logging
/// in successfully.
///
/// Includes the following IRC reply codes:
///
/// [RPL_WELCOME](crate::irc_constants::replies::RPL_WELCOME), [RPL_YOURHOST](crate::irc_constants::replies::RPL_YOURHOST),
/// [RPL_CREATED](crate::irc_constants::replies::RPL_CREATED), [RPL_MYINFO](crate::irc_constants::replies::RPL_MYINFO),
/// [RPL_MOTDSTART](crate::irc_constants::replies::RPL_MOTDSTART), [RPL_MOTD](crate::irc_constants::replies::RPL_MOTD),
/// [RPL_ENDOFMOTD](crate::irc_constants::replies::RPL_ENDOFMOTD)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectMessageEvent<T: Debug + Clone + Eq> {
    /// IRC command name, typically 3 digit numeric code
    pub(crate) command: T,

    /// IRC command params
    pub(crate) params: Vec<T>,
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

/// Accessors for channel message event data
pub trait ChannelMessageEventData<T> {
    /// Channel name for the message
    fn channel(&self) -> &T;
    /// Actual message
    fn message(&self) -> &T;
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

    /// Get the message
    #[inline]
    fn message(&self) -> &T {
        &self.event.as_ref().message
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

pub struct Badge<T: StringRef> {
    pub badge: T,
    pub version: T,
}

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
    pub(crate) user: T,
    pub(crate) channel: T,
    pub(crate) names: Vec<T>,
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
    pub(crate) channel: T,
    pub(crate) mode_change: T,
    pub(crate) user: T,
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
    pub recipient: T,
    pub message: T,
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
    pub(crate) hosting_channel: T,
    pub(crate) target_channel: Option<T>,
    pub(crate) viewer_count: Option<usize>,
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
