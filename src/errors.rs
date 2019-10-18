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
    EventParseError {
        message: IrcMessage<String>,
        details: Cow<'static, str>,
    },
}

impl ErrorTrait for Error {
    fn description(&self) -> &str {
        match self {
            Error::SendError => "Error in the send channel",
            Error::WebsocketError {
                details: message, ..
            } => message.borrow(),
            Error::EventParseError { details, .. } => details.borrow(),
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
        unimplemented!()
    }
}
