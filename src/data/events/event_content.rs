use crate::util::RefToString;
use crate::StringRef;
use fnv::FnvHashMap;
use std::fmt::Debug;

/// Converts events from references into owned versions of themselves
pub trait ToOwnedEvent {
    type Owned;
    /// Convert the event to its owned version
    fn to_owned_event(&self) -> Self::Owned;
}

impl ToOwnedEvent for () {
    type Owned = ();
    fn to_owned_event(&self) -> Self::Owned {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventContent<T, Inner>
where
    T: StringRef,
    Inner: Debug + Clone + Eq,
{
    pub(crate) sender: Option<T>,
    pub(crate) event: Inner,
    pub(crate) tags: Option<FnvHashMap<T, T>>,
}

/// Methods common to all EventContent variants
impl<T, Inner> EventContent<T, Inner>
where
    T: StringRef,
    Inner: Debug + Clone + Eq,
{
    /// Get the sender of the message
    pub fn sender(&self) -> &Option<T> {
        &self.sender
    }

    /// Get the data of the inner event type
    pub fn event(&self) -> &Inner {
        &self.event
    }

    /// Get the map of all IRCv3 tags
    pub fn tags(&self) -> &Option<FnvHashMap<T, T>> {
        &self.tags
    }
}

/// Generic ToOwned implementation for all EventContent variants
impl<T, Inner> ToOwnedEvent for EventContent<T, Inner>
where
    T: StringRef,
    Inner: Debug + Clone + Eq + ToOwnedEvent,
    Inner::Owned: Debug + Clone + Eq,
{
    type Owned = EventContent<String, Inner::Owned>;

    fn to_owned_event(&self) -> Self::Owned {
        EventContent {
            sender: self.sender.as_ref().map(RefToString::ref_to_string),
            event: self.event.to_owned_event(),
            tags: self.tags.as_ref().map(|hash_map| {
                hash_map
                    .iter()
                    .map(|(key, val)| (key.ref_to_string(), val.as_ref().ref_to_string()))
                    .collect::<FnvHashMap<String, String>>()
            }),
        }
    }
}

/// Welcome messages that Twitch sends after connection and logging
/// in successfully
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectMessage<T: Debug + Clone + Eq> {
    pub command: T,
    pub params: Vec<T>,
}

impl<T: StringRef> ToOwnedEvent for ConnectMessage<T> {
    type Owned = ConnectMessage<String>;

    fn to_owned_event(&self) -> Self::Owned {
        ConnectMessage {
            command: self.command.ref_to_string(),
            params: self.params.iter().map(RefToString::ref_to_string).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserEvent<T: StringRef> {
    pub user: T,
}

impl<T: StringRef> ToOwnedEvent for UserEvent<T> {
    type Owned = UserEvent<String>;

    fn to_owned_event(&self) -> Self::Owned {
        UserEvent {
            user: self.user.ref_to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelMessageEvent<T: StringRef> {
    pub channel: T,
    pub message: T,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelEvent<T: StringRef> {
    pub channel: T,
}

impl<T: StringRef> ToOwnedEvent for ChannelEvent<T> {
    type Owned = ChannelEvent<String>;

    fn to_owned_event(&self) -> Self::Owned {
        ChannelEvent {
            channel: self.channel.ref_to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelUserEvent<T: StringRef> {
    pub channel: T,
    pub user: Option<T>,
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

/// NAMES list response data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamesListEvent<T: StringRef> {
    pub user: T,
    pub channel: T,
    pub names: Vec<T>,
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
    pub channel: T,
    pub mode_change: T,
    pub user: T,
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

/// Host event data (hosting channel, target channel, viewers)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostEvent<T: StringRef> {
    pub hosting_channel: T,
    pub target_channel: Option<T>,
    pub viewer_count: Option<usize>,
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
