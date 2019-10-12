use std::borrow::Cow;
use tokio_tungstenite::tungstenite::Error as TungsteniteError;

#[derive(Debug)]
pub enum ErrorKind {
    SendError,
    MissingConfigValue,
    WebsocketError(TungsteniteError),
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    details: Cow<'static, str>,
}

impl Error {
    pub fn new(kind: impl Into<ErrorKind>, details: impl Into<Cow<'static, str>>) -> Error {
        Error {
            kind: kind.into(),
            details: details.into(),
        }
    }
}

impl From<TungsteniteError> for Error {
    fn from(err: TungsteniteError) -> Self {
        Error::new(ErrorKind::WebsocketError(err), "Websocket error")
    }
}
