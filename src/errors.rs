use crate::events::Event;
use crate::irc::IrcMessage;
use std::borrow::{Borrow, Cow};
use std::error::Error as ErrorTrait;
use std::fmt::{Debug, Formatter};

#[derive(Debug)]
pub enum Error {
    SendError,
    WebsocketError {
        details: Cow<'static, str>,
        source: tokio_tungstenite::tungstenite::Error,
    },
    MissingIrcCommandParameter(usize, IrcMessage<String>),
    WrongIrcParameterCount(usize, IrcMessage<String>),
    UnknownIrcCommand(IrcMessage<String>),
    MissingTag {
        tag: Cow<'static, str>,
        event: Event<String>,
    },
}

impl ErrorTrait for Error {
    fn description(&self) -> &str {
        match self {
            Error::SendError => "Error in the send channel",
            Error::WebsocketError { details, .. } => details.borrow(),
            Error::MissingIrcCommandParameter(_, _) => "Missing IRC command parameter",
            Error::WrongIrcParameterCount(_, _) => "Incorrect number of IRC command parameters",
            Error::UnknownIrcCommand(_) => "Unknown IRC command",
            Error::MissingTag { .. } => "Missing message tag",
        }
    }

    fn cause(&self) -> Option<&dyn ErrorTrait> {
        match self {
            Error::WebsocketError { source, .. } => Some(source),
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
        }
    }
}
