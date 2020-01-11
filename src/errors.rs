use thiserror::Error;

use std::borrow::Cow;
use std::fmt::Debug;

use crate::event::Event;
use crate::irc::IrcMessage;

/// Error type for tmi-rs methods
#[derive(Debug, Error)]
pub enum Error {
    /// Message sending error
    #[error("Message send error")]
    SendError,
    /// Websocket error
    #[error("Websocket error: {0}")]
    WebsocketError(#[from] tokio_tungstenite::tungstenite::Error),
    /// Missing IRC parameter in one of the received messages
    #[error("Missing IRC command parameter at index {0} in the message {1:?}")]
    MissingIrcCommandParameter(usize, IrcMessage<String>),
    /// Wrong number of IRC parameters in one of the received messages
    #[error("Received unknown IRC command in message {0:?}")]
    WrongIrcParameterCount(usize, IrcMessage<String>),
    /// Unrecognized IRC command was received
    #[error("Received unknown IRC command in message {0:?}")]
    UnknownIrcCommand(IrcMessage<String>),
    /// An IRCv3 tag that is normally expected to be set on a message was missing
    #[error("Message did not contain expected tag \"{tag}\". Message:\n{event:?}")]
    MissingTag {
        /// The tag name that was expected to exist
        tag: Cow<'static, str>,
        /// The event causing the issue
        event: Event<String>,
    },
    /// Send error in an internal message passing channel
    #[error("Internal message channel was unexpectedly closed.")]
    MessageChannelError,
    /// IRC parsing error
    #[error("IRC parse error:\n{0}")]
    IrcParseError(String),
    /// Tag parsing error
    #[error("Tag content parsing error in tag {0}={1}")]
    TagParseError(String, String),
}
