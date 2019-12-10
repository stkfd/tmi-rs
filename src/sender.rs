//! Message/command sender helpers

use std::pin::Pin;

use futures_core::task::{Context, Poll};
use futures_sink::Sink;

use crate::client_messages::ClientMessage;
use crate::Error;

/// A wrapper around any `Sink<ClientMessage<String>>` that provides methods to send Twitch
/// specific messages or commands to the sink.
#[derive(Clone)]
pub struct TwitchChatSender<Si: Clone + Unpin> {
    sink: Si,
}

impl<Si: Unpin + Clone> Unpin for TwitchChatSender<Si> {}

impl<'a, Si, M> Sink<M> for &'a TwitchChatSender<Si>
where
    Si: Unpin + Clone,
    &'a Si: Sink<ClientMessage<String>>,
    M: Into<ClientMessage<String>>,
{
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <&Si>::poll_ready(Pin::new(&mut &self.sink), cx).map_err(|_| Error::SendError)
    }

    fn start_send(self: Pin<&mut Self>, item: M) -> Result<(), Self::Error> {
        let msg: ClientMessage<String> = item.into();
        debug!("> {}", msg);
        <&Si>::start_send(Pin::new(&mut &self.sink), msg).map_err(|_| Error::SendError)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <&Si>::poll_flush(Pin::new(&mut &self.sink), cx).map_err(|_| Error::SendError)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <&Si>::poll_close(Pin::new(&mut &self.sink), cx).map_err(|_| Error::SendError)
    }
}

impl<Si> TwitchChatSender<Si>
where
    Si: Sink<ClientMessage<String>> + Unpin + Clone,
{
    /// Create a new chat sender using an underlying channel.
    pub fn new(sink: Si) -> Self {
        TwitchChatSender { sink }
    }
}
