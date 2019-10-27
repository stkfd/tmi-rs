use std::borrow::Borrow;
use std::str::FromStr;

use fnv::FnvHashMap;
use nom::character::complete::{alpha1, char, digit1};
use nom::multi::{separated_list, separated_nonempty_list};
use nom::sequence::{separated_pair, tuple};
use nom::IResult;

use crate::event::data::{
    Badge, ClearChatEvent, ClearMsgEvent, GlobalUserStateEvent, PrivMsgEvent, RoomStateEvent,
    UserNoticeEvent, UserStateEvent,
};
use crate::event::EventData;
use crate::{Error, StringRef};

pub trait MessageTags<T> {
    /// Get the map of all IRCv3 tags
    fn tags(&self) -> &Option<FnvHashMap<T, T>>;

    /// Get a tag value from the message by its key
    fn tag<Q: Borrow<str>>(&self, key: Q) -> Option<&T>;

    /// Gets a tag value, returns an Error if the value is not set. Intended for use in
    /// internal tag accessor functions where the tag should always be available
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

/// Access to the `emote-sets` tag
pub trait EmoteSetsTag<T: StringRef>: MessageTags<T> {
    /// `emote-sets` tag
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

/// Tags that apply to both PRIVMSG and USERNOTICE.
pub trait UserMessageTags<T: StringRef>: MessageTags<T> {
    /// `emotes` tag
    #[inline]
    fn emotes(&self) -> Result<Vec<EmoteReplacement>, Error> {
        self.tag("emotes").map_or_else(
            || Ok(vec![]),
            |emotes_str| parse_emotes(emotes_str.borrow()),
        )
    }

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

pub struct EmoteReplacement {
    pub emote_id: usize,
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

    /// `followers-only` tag. Set when followers only mode is active.
    #[inline]
    fn followers_only(&self) -> bool {
        match self.tag("followers-only").map(|t| t.borrow()) {
            Some("1") => true,
            _ => false,
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

fn parse_badges<'a>(input: &'a str, tag_name: &str) -> Result<Vec<Badge<&'a str>>, Error> {
    separated_list(char(','), parse_badge)(input)
        .map(|(_, badges)| badges)
        .map_err(|_| Error::TagParseError(tag_name.to_string(), input.to_string()))
}

fn parse_badge(input: &str) -> IResult<&str, Badge<&str>> {
    let (remaining, (badge, _, version)) = tuple((alpha1, char('/'), alpha1))(input)?;
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
