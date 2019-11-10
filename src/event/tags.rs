//! Convenience methods to access the IRCv3 tags twitch sends. It's possible to access the
//! tag values using their names with the [`MessageTags<T>`](self::MessageTags) trait. However, there
//! are accessor methods for all documented tags which are only implemented for the event types where
//! they are expected to be set. Additionally, some of the methods parse the tags and return more
//! easily processed structs instead of the raw string values
//! (see [`EmoteTag::emotes()`](trait.EmoteTag.html#method.emotes)),
//! [`EmoteSetsTag::emote_sets()`](trait.EmoteSetsTag.html#method.emote_sets)).
//!
//! # Examples
//!
//! ```
//! # use tmi_rs::irc::IrcMessage;
//! # use std::convert::TryInto;
//! use tmi_rs::event::{tags::UserDisplayTags, Event};
//! # let msg = "@badge-info=;badges=global_mod/1,turbo/1;color=#0D4200;display-name=ronni;emotes=25:0-4,12-16/1902:6-10;id=b34ccfc7-4977-403a-8a94-33c6bac34fb8;mod=0;room-id=1337;subscriber=0;tmi-sent-ts=1507246572675;turbo=1;user-id=1337;user-type=global_mod :ronni!ronni@ronni.tmi.twitch.tv PRIVMSG #ronni :Kappa Keepo Kappa";
//! # let parsed_message: Event<&str> = IrcMessage::parse(msg).unwrap().1.try_into().unwrap();
//!
//! if let Event::PrivMsg(event) = parsed_message {
//!     println!("{:?}", event.color()); // Some("#0D4200")
//! }
//! ```
//!
//! Using the tag specific methods acts as a compile-time guard against trying to access tags on events
//! where they are never defined:
//!
//! ```compile_fail
//! use tmi_rs::event::Event;
//! use tmi_rs::event::tags::ClearChatTags;
//! # let msg = "@badge-info=;badges=global_mod/1,turbo/1;color=#0D4200;display-name=ronni;emotes=25:0-4,12-16/1902:6-10;id=b34ccfc7-4977-403a-8a94-33c6bac34fb8;mod=0;room-id=1337;subscriber=0;tmi-sent-ts=1507246572675;turbo=1;user-id=1337;user-type=global_mod :ronni!ronni@ronni.tmi.twitch.tv PRIVMSG #ronni :Kappa Keepo Kappa";
//! # let parsed_message: Event<&str> = IrcMessage::parse(msg).unwrap().1.try_into().unwrap();
//!
//! if let Event::PrivMsg(event) = parsed_message {
//!     // only implemented for ClearChat events, won't compile
//!     println!("{:?}", event.ban_duration());
//! }
//! ```

use std::borrow::Borrow;
use std::str::FromStr;

use fnv::FnvHashMap;
use nom::character::complete::{alpha1, char, digit1};
use nom::multi::{separated_list, separated_nonempty_list};
use nom::sequence::{separated_pair, tuple};
use nom::IResult;

use crate::event::inner_data::{
    ClearChatEvent, ClearMsgEvent, GlobalUserStateEvent, PrivMsgEvent, RoomStateEvent,
    UserNoticeEvent, UserStateEvent,
};
use crate::event::{EventData, WhisperEvent};
use crate::{Error, StringRef};

/// Access methods for individual IRC tags by their string tag keys
pub trait MessageTags<T> {
    /// Get the map of all IRCv3 tags.
    fn tags(&self) -> &Option<FnvHashMap<T, T>>;

    /// Get a tag value from the message by its key. `None` for tags that are not present
    /// as well as tags that are set but empty.
    fn tag<Q: Borrow<str>>(&self, key: Q) -> Option<&T>;

    /// Gets a tag value, returns an Error if the value is not set or empty. Intended for use in
    /// cases where the tag should always be available.
    fn required_tag<Q: Borrow<str>>(&self, key: Q) -> Result<&T, Error>;
}

/// Tags specific to CLEARCHAT events
pub trait ClearChatTags<T>: MessageTags<T> {
    /// `ban-duration` tag. Duration of the timeout, in seconds. If omitted, the ban is permanent.
    fn ban_duration(&self) -> Option<&T> {
        self.tag("ban-duration")
    }
}
impl<T: StringRef> ClearChatTags<T> for EventData<T, ClearChatEvent<T>> {}

/// Tags specific to CLEARMSG events
pub trait ClearMsgTags<T>: MessageTags<T> {
    /// `target-msg-id` tag. UUID of the message to be deleted.
    #[inline]
    fn target_msg_id(&self) -> Result<&T, Error> {
        self.required_tag("target-msg-id")
    }

    ///	`login` tag. Name of the user who sent the message
    #[inline]
    fn login(&self) -> Result<&T, Error> {
        self.required_tag("login")
    }
}
impl<T: StringRef> ClearMsgTags<T> for EventData<T, ClearMsgEvent<T>> {}

/// Access to `badges` and `badge-info` tags
pub trait BadgeTags<T: StringRef>: MessageTags<T> {
    /// `badge-info` tag. Metadata related to the chat badges in the `badges` tag.
    fn badge_info<'a>(&'a self) -> Result<Vec<Badge<&'a str>>, Error>
    where
        T: 'a,
    {
        if let Some(badge_str) = self.tag("badge-info") {
            parse_badges(badge_str.borrow(), "badge-info")
        } else {
            Ok(vec![])
        }
    }

    /// `badges` tag. List of chat badges and the version of each badge
    fn badges<'a>(&'a self) -> Result<Vec<Badge<&'a str>>, Error>
    where
        T: 'a,
    {
        if let Some(badge_str) = self.tag("badges") {
            parse_badges(badge_str.borrow(), "badges")
        } else {
            Ok(vec![])
        }
    }
}
impl<T: StringRef> BadgeTags<T> for EventData<T, GlobalUserStateEvent> {}
impl<T: StringRef> BadgeTags<T> for EventData<T, PrivMsgEvent<T>> {}
impl<T: StringRef> BadgeTags<T> for EventData<T, UserNoticeEvent<T>> {}
impl<T: StringRef> BadgeTags<T> for EventData<T, UserStateEvent<T>> {}
impl<T: StringRef> BadgeTags<T> for EventData<T, WhisperEvent<T>> {}

/// Access to `color` and `display-name` tags
pub trait UserDisplayTags<T: StringRef>: MessageTags<T> {
    /// `color` tag
    #[inline]
    fn color(&self) -> Option<&T> {
        self.tag("color")
    }

    /// `display-name` tag
    #[inline]
    fn display_name(&self) -> Option<&T> {
        self.tag("display-name")
    }
}
impl<T: StringRef> UserDisplayTags<T> for EventData<T, GlobalUserStateEvent> {}
impl<T: StringRef> UserDisplayTags<T> for EventData<T, PrivMsgEvent<T>> {}
impl<T: StringRef> UserDisplayTags<T> for EventData<T, UserNoticeEvent<T>> {}
impl<T: StringRef> UserDisplayTags<T> for EventData<T, UserStateEvent<T>> {}
impl<T: StringRef> UserDisplayTags<T> for EventData<T, WhisperEvent<T>> {}

/// Access to the `emote-sets` tag
pub trait EmoteSetsTag<T: StringRef>: MessageTags<T> {
    /// `emote-sets` tag, returned as a Vec of integers. If the tag is missing, an empty Vec
    /// is returned.
    fn emote_sets(&self) -> Result<Vec<usize>, Error> {
        if let Some(tag_content) = self.tag("emote-sets") {
            tag_content
                .borrow()
                .split(',')
                .map(|emote_set| usize::from_str(emote_set))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| {
                    Error::TagParseError("emote-sets".to_string(), tag_content.borrow().to_string())
                })
        } else {
            Ok(vec![])
        }
    }
}
impl<T: StringRef> EmoteSetsTag<T> for EventData<T, GlobalUserStateEvent> {}
impl<T: StringRef> EmoteSetsTag<T> for EventData<T, UserStateEvent<T>> {}

/// Access to the `user-id` tag
pub trait UserIdTag<T: StringRef>: MessageTags<T> {
    /// `user-id` tag
    #[inline]
    fn user_id(&self) -> Result<&T, Error> {
        self.required_tag("user-id")
    }
}
impl<T: StringRef> UserIdTag<T> for EventData<T, GlobalUserStateEvent> {}
impl<T: StringRef> UserIdTag<T> for EventData<T, UserNoticeEvent<T>> {}
impl<T: StringRef> UserIdTag<T> for EventData<T, PrivMsgEvent<T>> {}
impl<T: StringRef> UserIdTag<T> for EventData<T, WhisperEvent<T>> {}

/// Access to `mod` tag
pub trait ModTag<T: StringRef>: MessageTags<T> {
    /// `mod` tag
    #[inline]
    fn is_mod(&self) -> bool {
        match self.tag("mod").map(|t| t.borrow()) {
            Some("1") => true,
            _ => false,
        }
    }
}
impl<T: StringRef> ModTag<T> for EventData<T, UserStateEvent<T>> {}
impl<T: StringRef> ModTag<T> for EventData<T, PrivMsgEvent<T>> {}
impl<T: StringRef> ModTag<T> for EventData<T, UserNoticeEvent<T>> {}

/// Access to `bits` tag
pub trait BitsTag<T: StringRef>: MessageTags<T> {
    /// `bits` tag
    #[inline]
    fn bits(&self) -> Option<&T> {
        self.tag("bits")
    }
}
impl<T: StringRef> BitsTag<T> for EventData<T, PrivMsgEvent<T>> {}

/// Emote tag accessor
pub trait EmotesTag<T: StringRef>: MessageTags<T> {
    /// `emotes` tag
    ///
    /// # Examples
    /// ```
    /// # use tmi_rs::irc::IrcMessage;
    /// # use std::convert::TryInto;
    /// use tmi_rs::event::{Event, tags::{EmotesTag, EmoteReplacement}};
    /// # let msg = "@display-name=ronni;emotes=25:0-4,12-16/1902:6-10 :ronni!ronni@ronni.tmi.twitch.tv PRIVMSG #ronni :Kappa Keepo Kappa";
    /// # let parsed: Event<&str> = IrcMessage::parse(msg).unwrap().1.try_into().unwrap();
    /// // `parsed` is some `Event<&str>` message with the emote tag emotes=25:0-4,12-16/1902:6-10
    /// if let Event::PrivMsg(event) = &parsed {
    ///     assert_eq!(event.emotes().unwrap(), vec![
    ///         EmoteReplacement { emote_id: 25, indices: vec![(0, 4), (12, 16)] },
    ///         EmoteReplacement { emote_id: 1902, indices: vec![(6, 10)] }
    ///     ])
    /// }
    /// # assert!(match &parsed {
    /// #     Event::PrivMsg(event) => true,
    /// #     _ => false
    /// # });
    /// ```
    #[inline]
    fn emotes(&self) -> Result<Vec<EmoteReplacement>, Error> {
        self.tag("emotes").map_or_else(
            || Ok(vec![]),
            |emotes_str| parse_emotes(emotes_str.borrow()),
        )
    }
}
impl<T: StringRef> EmotesTag<T> for EventData<T, PrivMsgEvent<T>> {}
impl<T: StringRef> EmotesTag<T> for EventData<T, UserNoticeEvent<T>> {}
impl<T: StringRef> EmotesTag<T> for EventData<T, WhisperEvent<T>> {}

/// Tags that apply to both PRIVMSG and USERNOTICE.
pub trait UserMessageTags<T: StringRef>: MessageTags<T> {
    /// `id` tag
    #[inline]
    fn id(&self) -> Result<&T, Error> {
        self.required_tag("id")
    }

    /// `room-id` tag
    #[inline]
    fn room_id(&self) -> Result<&T, Error> {
        self.required_tag("room-id")
    }

    /// `tmi-sent-ts` tag
    fn sent_timestamp(&self) -> Result<usize, Error> {
        let tag = self.required_tag("tmi-sent-ts")?;
        Ok(usize::from_str(tag.borrow())
            .map_err(|_| Error::TagParseError("".to_string(), tag.to_string()))?)
    }
}
impl<T: StringRef> UserMessageTags<T> for EventData<T, PrivMsgEvent<T>> {}
impl<T: StringRef> UserMessageTags<T> for EventData<T, UserNoticeEvent<T>> {}

/// Replacement instruction for an emote
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmoteReplacement {
    /// Emote ID
    pub emote_id: usize,
    /// Start and end index of the emote in the message
    pub indices: Vec<(usize, usize)>,
}

/// Tags specific to USERNOTICE messages
pub trait UserNoticeTags<T: StringRef>: MessageTags<T> {
    /// `login` tag. Name of the user who sent the notice.
    #[inline]
    fn login(&self) -> Result<&T, Error> {
        self.required_tag("login")
    }

    /// `msg-id` tag.
    ///
    /// The type of notice (not the ID). Valid values: sub, resub, subgift, anonsubgift,
    /// submysterygift, giftpaidupgrade, rewardgift, anongiftpaidupgrade, raid, unraid,
    /// ritual, bitsbadgetier.
    #[inline]
    fn msg_id(&self) -> Result<&T, Error> {
        self.required_tag("msg-id")
    }

    /// `system-msg` tag.
    ///
    /// The message printed in chat along with this notice.
    #[inline]
    fn system_msg(&self) -> Result<&T, Error> {
        self.required_tag("system-msg")
    }
}

/// Tags specific to ROOMSTATE events
pub trait RoomStateTags<T: StringRef>: MessageTags<T> {
    /// `emote-only` tag. Set when emote only mode is active.
    #[inline]
    fn emote_only(&self) -> bool {
        match self.tag("emote-only").map(|t| t.borrow()) {
            Some("1") => true,
            _ => false,
        }
    }

    /// `followers-only` tag. Some(amount of minutes followed) when followers only mode is active,
    /// None when inactive.
    #[inline]
    fn followers_only(&self) -> Option<isize> {
        match self
            .tag("followers-only")
            .and_then(|t| isize::from_str(t.borrow()).ok())
        {
            Some(v) if v >= 0 => Some(v),
            _ => None,
        }
    }

    /// `r9k` tag. Set when r9k mode is active.
    #[inline]
    fn r9k(&self) -> bool {
        match self.tag("r9k").map(|t| t.borrow()) {
            Some("1") => true,
            _ => false,
        }
    }

    /// `slow` tag. Set to the number of seconds set for slow mode if active.
    #[inline]
    fn slow(&self) -> Option<usize> {
        match self.tag("subs").map(|t| t.borrow()) {
            Some(v) => usize::from_str(v).ok(),
            _ => None,
        }
    }

    /// `subs-only` tag. Set when emote only mode is active.
    #[inline]
    fn subs_only(&self) -> bool {
        match self.tag("subs-only").map(|t| t.borrow()) {
            Some("1") => true,
            _ => false,
        }
    }
}
impl<T: StringRef> RoomStateTags<T> for EventData<T, RoomStateEvent<T>> {}

/// Acessors for tags on whisper messages
pub trait WhisperTags<T: StringRef>: MessageTags<T> {
    /// `message-id` tag for whispers, identifies messages within conversations.
    fn message_id(&self) -> Result<usize, Error> {
        self.required_tag("message-id").and_then(|tag| {
            usize::from_str(tag.borrow()).map_err(|_| {
                Error::TagParseError("message-id".to_string(), tag.borrow().to_string())
            })
        })
    }

    /// `thread-id` tag for whispers, to identify conversations.
    fn thread_id(&self) -> Result<&T, Error> {
        self.required_tag("thread-id")
    }
}
impl<T: StringRef> WhisperTags<T> for EventData<T, WhisperEvent<T>> {}

/// Badges from the `badges` and `badges-info` tags
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Badge<T: StringRef> {
    /// Badge name
    pub badge: T,
    /// Badge "version", meaning depends on the badge
    pub version: T,
}

fn parse_badges<'a>(input: &'a str, tag_name: &str) -> Result<Vec<Badge<&'a str>>, Error> {
    separated_list(char(','), parse_badge)(input)
        .map(|(_, badges)| badges)
        .map_err(|_| Error::TagParseError(tag_name.to_string(), input.to_string()))
}

#[test]
fn test_badge_parsing() {
    assert_eq!(
        parse_badges("broadcaster/1", "badges").unwrap(),
        vec![Badge {
            badge: "broadcaster",
            version: "1"
        }]
    );
}

#[test]
fn test_badge_parsing_2() {
    assert_eq!(
        parse_badges("broadcaster/1,subscriber/1", "badges").unwrap(),
        vec![
            Badge {
                badge: "broadcaster",
                version: "1"
            },
            Badge {
                badge: "subscriber",
                version: "1"
            }
        ]
    );
}

fn parse_badge(input: &str) -> IResult<&str, Badge<&str>> {
    let (remaining, (badge, _, version)) = tuple((alpha1, char('/'), digit1))(input)?;
    Ok((remaining, Badge { badge, version }))
}

fn take_usize(input: &str) -> IResult<&str, usize> {
    ::nom::combinator::map(digit1, |s| usize::from_str(s).unwrap())(input)
}

fn parse_emote(input: &str) -> IResult<&str, EmoteReplacement> {
    let (rem, (emote_id, indices)) = separated_pair(
        take_usize,
        char(':'),
        separated_nonempty_list(char(','), separated_pair(take_usize, char('-'), take_usize)),
    )(input)?;
    Ok((rem, EmoteReplacement { emote_id, indices }))
}

fn parse_emotes(input: &str) -> Result<Vec<EmoteReplacement>, Error> {
    let (_rem, replacements) = separated_list(char('/'), parse_emote)(input)
        .map_err(|_| Error::TagParseError("emotes".to_string(), input.to_string()))?;
    Ok(replacements)
}
