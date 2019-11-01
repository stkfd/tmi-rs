//! Types to represent messages sent by the client

use std::borrow::Borrow;
use std::fmt;
use tokio_tungstenite::tungstenite::Message;

use crate::rate_limits::RateLimitable;
use crate::StringRef;

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

impl<T: StringRef> fmt::Display for ClientMessage<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            ClientMessage::PrivMsg { channel, message } => {
                write!(f, "PRIVMSG #{} :{}", channel, message)
            }
            ClientMessage::Join(channel) => write!(f, "JOIN #{}", channel),
            ClientMessage::Part(channel) => write!(f, "PART #{}", channel),
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
    fn channel_slow_mode(&self) -> Option<&str> {
        match self {
            ClientMessage::PrivMsg { channel, .. } => Some(channel.borrow()),
            _ => None,
        }
    }
}

impl<T: StringRef> RateLimitable for ClientMessage<T> {
    fn channel_slow_mode(&self) -> Option<&str> {
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
