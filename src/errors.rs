use thiserror::Error;
use tokio::sync::{broadcast, mpsc};

use std::borrow::Cow;
use std::fmt::Debug;

use crate::event::Event;
use crate::irc::IrcMessage;
use crate::stream::SentClientMessage;
use crate::ClientMessage;
use std::sync::Arc;

/// Error type for tmi-rs methods
#[derive(Debug, Error, Clone)]
pub enum Error {
    /// Websocket error
    #[error("Websocket error: {0}")]
    WebsocketError(#[from] Arc<tokio_tungstenite::tungstenite::Error>),
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
    #[error("Message sending failed")]
    MessageSendError(#[from] MessageSendError),
    /// Event channel errors
    #[error("Event channel error")]
    EventChannelError(#[from] EventChannelError),
    /// IRC parsing error
    #[error("IRC parse error:\n{0}")]
    IrcParseError(String),
    /// Tag parsing error
    #[error("Tag content parsing error in tag {0}={1}")]
    TagParseError(String, String),
}

/// Errors from the internal event channels sharing events between tasks
#[derive(Debug, Error, Clone)]
pub enum EventChannelError {
    /// Channel was closed
    #[error("Channel was closed")]
    Closed,
    /// Channel overflowed unexpectedly
    #[error("Channel overflowed unexpectedly")]
    Overflow,
}

/// Errors that happen while trying to send a message
#[derive(Debug, Error, Clone)]
pub enum MessageSendError {
    /// Sending the message failed because the connection has already been closed
    #[error("Sending the message failed because the connection has already been closed")]
    Closed(ClientMessage),
    /// No connection in the pool has the twitch channel specified in the message
    #[error("No connection in the pool has the twitch channel specified in the message")]
    ChannelNotJoined(ClientMessage),
    /// Unsupported message type
    #[error("Unsupported message type: {0}")]
    UnsupportedMessage(&'static str),
    /// The connection pool had to create a new connection before joining a channel, but an error
    /// happened while connecting
    #[error("Error while trying to create a new connection: {0}")]
    NewConnectionFailed(String),
}

impl From<mpsc::error::SendError<ClientMessage>> for MessageSendError {
    fn from(source: mpsc::error::SendError<ClientMessage>) -> Self {
        MessageSendError::Closed(source.0)
    }
}

impl From<mpsc::error::SendError<SentClientMessage>> for MessageSendError {
    fn from(source: mpsc::error::SendError<SentClientMessage>) -> Self {
        MessageSendError::Closed((source.0).message)
    }
}

impl From<broadcast::SendError<ClientMessage>> for MessageSendError {
    fn from(source: broadcast::SendError<ClientMessage>) -> Self {
        MessageSendError::Closed(source.0)
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Error::WebsocketError(Arc::new(err))
    }
}
