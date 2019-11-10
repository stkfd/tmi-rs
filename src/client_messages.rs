//! Types to represent messages sent by the client

use std::borrow::Borrow;
use std::fmt;
use tokio_tungstenite::tungstenite::Message;

use crate::rate_limits::RateLimitable;
use crate::StringRef;
use futures_core::Stream;

/// Messages to be sent from the client to twitch servers
#[allow(missing_docs)]
#[derive(Clone)]
pub enum ClientMessage<T: Borrow<str>> {
    PrivMsg { channel: T, message: T },
    Join(T),
    Part(T),
    Nick(T),
    Pass(T),
    CapRequest(Vec<Capability>),
    Ping,
    Pong,
}

impl ClientMessage<String> {
    /// Send a whisper
    pub fn message<S1: Into<String> + Borrow<str>, S2: Into<String> + Borrow<str>>(
        channel: S1,
        message: S2,
    ) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: message.into(),
        }
    }

    /// Joins a twitch channel
    pub fn join<S: Into<String> + Borrow<str>>(channel: S) -> Self {
        ClientMessage::Join(channel.into())
    }

    /// Authenticates the user. Caution: this is normally called automatically when calling
    /// [`TwitchClient::connect`](crate::TwitchClient::connect), only use it if this stream was created in some other
    /// way.
    pub fn login<S: Into<String> + Borrow<str>>(username: S, token: S) -> impl Stream<Item = Self> {
        futures_util::stream::iter(vec![
            ClientMessage::Pass(token.into()),
            ClientMessage::Nick(username.into()),
        ])
    }

    /// Permanently ban a user
    pub fn ban<S: Into<String> + Borrow<str>>(channel: S, username: S) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Ban(username.borrow()).to_string(),
        }
    }

    /// Unban a user
    pub fn unban<S: Into<String> + Borrow<str>>(channel: S, username: S) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Unban(username.borrow()).to_string(),
        }
    }

    /// Clear a chat channel
    pub fn clear<S: Into<String> + Borrow<str>>(channel: S) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::Clear.to_string(),
        }
    }

    /// Set the user name color
    pub fn color<S: Into<String> + Borrow<str>>(color: S) -> Self {
        ClientMessage::PrivMsg {
            channel: String::from("jtv"),
            message: Command::Color(color.borrow()).to_string(),
        }
    }

    /// Run a commercial
    pub fn commercial<S: Into<String> + Borrow<str>>(channel: S, time: Option<usize>) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::Commercial { time }.to_string(),
        }
    }

    /// Delete a specific message by its message ID tag.
    pub fn delete<S: Into<String> + Borrow<str>>(channel: S, msg_id: S) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Delete {
                msg_id: msg_id.borrow(),
            }
            .to_string(),
        }
    }

    /// Disconnect from chat
    pub fn disconnect<S: Into<String> + Borrow<str>>(&mut self) -> Self {
        ClientMessage::PrivMsg {
            channel: String::from("jtv"),
            message: Command::<&str>::Disconnect.to_string(),
        }
    }

    /// Set emote only mode on or off
    pub fn emote_only<S: Into<String> + Borrow<str>>(channel: S, status: bool) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::EmoteOnly(status).to_string(),
        }
    }

    /// Set followers only mode on or off
    pub fn followers_only<S: Into<String> + Borrow<str>>(channel: S, status: bool) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::FollowersOnly(status).to_string(),
        }
    }

    /// Host a channel
    pub fn host<S: Into<String> + Borrow<str>>(channel: S, host_channel: S) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Host(host_channel.borrow()).to_string(),
        }
    }

    /// Stop hosting
    pub fn unhost<S: Into<String> + Borrow<str>>(channel: S) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::Unhost.to_string(),
        }
    }

    /// Set a stream marker
    pub fn marker<S: Into<String> + Borrow<str>>(channel: S, description: Option<&str>) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Marker(description).to_string(),
        }
    }

    /// Send a /me message
    pub fn me<S: Into<String> + Borrow<str>>(message: S) -> Self {
        ClientMessage::PrivMsg {
            channel: String::from("jtv"),
            message: Command::Me(message.borrow()).to_string(),
        }
    }

    /// Mod a user
    pub fn make_mod<S: Into<String> + Borrow<str>>(channel: S, user: S) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Mod(user.borrow()).to_string(),
        }
    }

    /// Unmod a user
    pub fn unmod<S: Into<String> + Borrow<str>>(channel: S, user: S) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Unmod(user.borrow()).to_string(),
        }
    }

    /// Enable or disable r9k mode
    pub fn r9k<S: Into<String> + Borrow<str>>(channel: S, on: bool) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::R9k(on).to_string(),
        }
    }

    /// Start a raid
    pub fn raid<S: Into<String> + Borrow<str>>(channel: S, raid_channel: S) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Raid(raid_channel.borrow()).to_string(),
        }
    }

    /// Stop a raid
    pub fn unraid<S: Into<String> + Borrow<str>>(channel: S) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::Unraid.to_string(),
        }
    }

    /// Enable slow mode with the given amount of seconds
    pub fn slow<S: Into<String> + Borrow<str>>(channel: S, seconds: usize) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::Slow(seconds).to_string(),
        }
    }

    /// Disable slow mode
    pub fn slow_off<S: Into<String> + Borrow<str>>(channel: S) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::SlowOff.to_string(),
        }
    }

    /// Enable or disable sub mode
    pub fn subscribers<S: Into<String> + Borrow<str>>(channel: S, on: bool) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::SubscribersOnly(on).to_string(),
        }
    }

    /// Time a user out for the given amount of seconds or the default timeout if None is given
    pub fn timeout<S: Into<String> + Borrow<str>>(
        channel: S,
        user: S,
        seconds: Option<usize>,
    ) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Timeout {
                time: seconds,
                user: user.borrow(),
            }
            .to_string(),
        }
    }

    /// Make a user VIP in a channel
    pub fn vip<S: Into<String> + Borrow<str>>(channel: S, user: S) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Vip(user.borrow()).to_string(),
        }
    }

    /// Remove VIP status from a user
    pub fn unvip<S: Into<String> + Borrow<str>>(channel: S, user: S) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Vip(user.borrow()).to_string(),
        }
    }

    /// List the VIPs in a channel
    pub fn vips<S: Into<String> + Borrow<str>>(channel: S) -> Self {
        ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::Vips.to_string(),
        }
    }

    /// Send a whisper
    pub fn whisper<S: Into<String> + Borrow<str>>(user: S, message: S) -> Self {
        ClientMessage::PrivMsg {
            channel: String::from("jtv"),
            message: Command::Whisper {
                user: user.borrow(),
                message: message.borrow(),
            }
            .to_string(),
        }
    }
}

impl<T: StringRef> fmt::Display for ClientMessage<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            ClientMessage::PrivMsg { channel, message } => {
                write!(f, "PRIVMSG {} :{}", channel, message)
            }
            ClientMessage::Join(channel) => write!(f, "JOIN {}", channel),
            ClientMessage::Part(channel) => write!(f, "PART {}", channel),
            ClientMessage::CapRequest(caps) => write!(
                f,
                "CAP REQ :{}",
                caps.iter()
                    .map(|cap| -> &str { cap.into() })
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            ClientMessage::Nick(nick) => write!(f, "NICK {}", nick),
            ClientMessage::Pass(pass) => write!(f, "PASS {}", pass),
            ClientMessage::Ping => write!(f, "PING"),
            ClientMessage::Pong => write!(f, "PONG"),
        }
    }
}

impl<T: StringRef> Into<Message> for &ClientMessage<T> {
    fn into(self) -> Message {
        Message::Text(self.to_string())
    }
}

impl<T: StringRef> Into<Message> for ClientMessage<T> {
    fn into(self) -> Message {
        Message::Text(self.to_string())
    }
}

impl<T: StringRef> RateLimitable for &ClientMessage<T> {
    fn channel_limits(&self) -> Option<&str> {
        match self {
            ClientMessage::PrivMsg { channel, .. } => Some(channel.borrow()),
            _ => None,
        }
    }
}

impl<T: StringRef> RateLimitable for ClientMessage<T> {
    fn channel_limits(&self) -> Option<&str> {
        match self {
            ClientMessage::PrivMsg { channel, .. } => Some(channel.borrow()),
            _ => None,
        }
    }
}

/// Available twitch chat commands (/timeout etc)
#[allow(missing_docs)]
pub enum Command<T: Borrow<str>> {
    Ban(T),
    Unban(T),
    Clear,
    Color(T),
    Commercial { time: Option<usize> },
    Delete { msg_id: T },
    Disconnect,
    EmoteOnly(bool),
    FollowersOnly(bool),
    Host(T),
    Unhost,
    Marker(Option<T>),
    Me(T),
    Mod(T),
    Unmod(T),
    Mods,
    R9k(bool),
    Raid(T),
    Unraid,
    Slow(usize),
    SlowOff,
    SubscribersOnly(bool),
    Timeout { user: T, time: Option<usize> },
    Vip(T),
    Vips,
    Whisper { user: T, message: T },
}

impl<T: StringRef> fmt::Display for Command<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Command::Ban(user) => write!(f, "/ban {}", user),
            Command::Unban(user) => write!(f, "/unban {}", user),
            Command::Clear => write!(f, "/clear"),
            Command::Color(color) => write!(f, "/color {}", color),
            Command::Commercial { time: time_opt } => {
                if let Some(time) = time_opt {
                    write!(f, "/commercial {}", time)
                } else {
                    write!(f, "/commercial")
                }
            }
            Command::Delete { msg_id } => write!(f, "/delete {}", msg_id),
            Command::Disconnect => write!(f, "/disconnect"),
            Command::EmoteOnly(on) => {
                if *on {
                    write!(f, "/emoteonly")
                } else {
                    write!(f, "/emoteonlyoff")
                }
            }
            Command::FollowersOnly(on) => {
                if *on {
                    write!(f, "/followers")
                } else {
                    write!(f, "/followersoff")
                }
            }
            Command::Host(host_channel) => write!(f, "/host {}", host_channel),
            Command::Unhost => write!(f, "/unhost"),
            Command::Marker(opt_description) => {
                if let Some(description) = opt_description {
                    write!(f, "/marker {}", description)
                } else {
                    write!(f, "/marker")
                }
            }
            Command::Me(msg) => write!(f, "/me {}", msg),
            Command::Mod(user) => write!(f, "/mod {}", user),
            Command::Unmod(user) => write!(f, "/unmod {}", user),
            Command::Mods => write!(f, "/mods"),
            Command::R9k(on) => {
                if *on {
                    write!(f, "/r9kbeta")
                } else {
                    write!(f, "/r9kbetaoff")
                }
            }
            Command::Raid(target) => write!(f, "/raid {}", target),
            Command::Unraid => write!(f, "/unraid"),
            Command::Slow(seconds) => write!(f, "/slow {}", seconds),
            Command::SlowOff => write!(f, "/slowoff"),
            Command::SubscribersOnly(on) => {
                if *on {
                    write!(f, "/subscribers")
                } else {
                    write!(f, "/subscribersoff")
                }
            }
            Command::Timeout { user, time } => {
                if let Some(time) = time {
                    write!(f, "/timeout {} {}", user, time)
                } else {
                    write!(f, "/timeout {}", user)
                }
            }
            Command::Vip(user) => write!(f, "/vip {}", user),
            Command::Vips => write!(f, "/vips"),
            Command::Whisper { user, message } => write!(f, "/w {} {}", user, message),
        }
    }
}

/// Twitch client capabilities
#[derive(Clone, Copy)]
pub enum Capability {
    /// twitch.tv/membership capability
    Membership,
    /// twitch.tv/tags tags capability
    Tags,
    /// twitch.tv/commands capability
    Commands,
}

impl Into<&'static str> for &Capability {
    fn into(self) -> &'static str {
        match self {
            Capability::Membership => "twitch.tv/membership",
            Capability::Tags => "twitch.tv/tags",
            Capability::Commands => "twitch.tv/commands",
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let cap_as_str: &'static str = self.into();
        write!(f, "{}", cap_as_str)
    }
}
