use std::borrow::Cow;
use std::error::Error as ErrorTrait;

#[derive(Debug)]
pub enum ErrorKind {
    SendError,
    MissingConfigValue,
    WebsocketError,
    EventParseError,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    details: Cow<'static, str>,
    cause: Option<Box<dyn ErrorTrait>>,
}

impl Error {
    pub fn new(kind: impl Into<ErrorKind>, details: impl Into<Cow<'static, str>>) -> Error {
        Error {
            kind: kind.into(),
            details: details.into(),
            cause: None,
        }
    }

    pub fn with_cause(
        kind: impl Into<ErrorKind>,
        details: impl Into<Cow<'static, str>>,
        cause: impl Into<Box<dyn ErrorTrait>>,
    ) -> Error {
        Error {
            kind: kind.into(),
            details: details.into(),
            cause: Some(cause.into()),
        }
    }
}
