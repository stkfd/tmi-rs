use std::borrow::Cow;
use std::error::Error as ErrorTrait;
use std::fmt::{Debug, Formatter};

use crate::event::Event;
use crate::irc::IrcMessage;

/// Error type for tmi-rs methods
#[derive(Debug)]
pub enum Error {
    /// Message sending error
    SendError,
    /// Websocket error
    WebsocketError {
        /// Error details
        details: Cow<'static, str>,
        /// Underlying Websocket error that was the cause
        source: tokio_tungstenite::tungstenite::Error,
    },
    /// Missing IRC parameter in one of the received messages
    MissingIrcCommandParameter(usize, IrcMessage<String>),
    /// Wrong number of IRC parameters in one of the received messages
    WrongIrcParameterCount(usize, IrcMessage<String>),
    /// Unrecognized IRC command was received
    UnknownIrcCommand(IrcMessage<String>),
    /// An IRCv3 tag that is normally expected to be set on a message was missing
    MissingTag {
        /// The tag name that was expected to exist
        tag: Cow<'static, str>,
        /// The event causing the issue
        event: Event<String>,
    },
    /// Send error in an internal message passing channel
    MessageChannelError(futures_channel::mpsc::SendError),
    /// IRC parsing error
    IrcParseError(String),
    /// Tag parsing error
    TagParseError(String, String),
}

impl ErrorTrait for Error {
    fn cause(&self) -> Option<&dyn ErrorTrait> {
        match self {
            Error::WebsocketError { source, .. } => Some(source),
            Error::MessageChannelError(source) => Some(source),
            _ => None,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Error::SendError => write!(f, "Message send error"),
            Error::WebsocketError { details, source } => write!(
                f,
                "Websocket error: {}\nUnderlying error: {}",
                details, source
            ),
            Error::MissingIrcCommandParameter(index, message) => write!(
                f,
                "Missing IRC command parameter at index {} in the message {:?}",
                index, message
            ),
            Error::WrongIrcParameterCount(index, msg) => write!(
                f,
                "Expected {} IRC parameters in message, got {} {:?}",
                index,
                msg.params().len(),
                msg
            ),
            Error::UnknownIrcCommand(msg) => {
                write!(f, "Received unknown IRC command in message {:?}", msg)
            }
            Error::MissingTag { tag, event } => write!(
                f,
                "Message did not contain expected tag \"{}\". Message:\n{:?}",
                tag, event
            ),
            Error::MessageChannelError(source) => write!(
                f,
                "The internal message channel returned an error: {}",
                source
            ),
            Error::IrcParseError(source) => write!(f, "IRC parse error:\n{}", source),
            Error::TagParseError(tag, content) => {
                write!(f, "Tag content parsing error in tag {}={}", tag, content)
            }
        }
    }
}
