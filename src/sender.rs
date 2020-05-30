//! Message/command sender helpers

use std::pin::Pin;

use futures_core::task::{Context, Poll};
use futures_sink::Sink;

use crate::client_messages::ClientMessage;
use crate::Error;

/// A wrapper around any `Sink<ClientMessage>` that provides methods to send Twitch
/// specific messages or commands to the sink.
#[derive(Clone)]
pub struct TwitchChatSender<Si: Clone + Unpin> {
    sink: Si,
}

impl<Si: Unpin + Clone> Unpin for TwitchChatSender<Si> {}

impl<'a, Si, M, E> Sink<M> for &'a TwitchChatSender<Si>
where
    Si: Unpin + Clone,
    Error: From<E>,
    &'a Si: Sink<ClientMessage, Error = E>,
    M: Into<ClientMessage>,
{
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <&Si>::poll_ready(Pin::new(&mut &self.sink), cx).map_err(Into::into)
    }

    fn start_send(self: Pin<&mut Self>, item: M) -> Result<(), Self::Error> {
        let msg: ClientMessage = item.into();
        debug!("> {:?}", msg);
        <&Si>::start_send(Pin::new(&mut &self.sink), msg).map_err(Into::into)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <&Si>::poll_flush(Pin::new(&mut &self.sink), cx).map_err(Into::into)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <&Si>::poll_close(Pin::new(&mut &self.sink), cx).map_err(Into::into)
    }
}

impl<Si> TwitchChatSender<Si>
where
    Si: Sink<ClientMessage> + Unpin + Clone,
{
    /// Create a new chat sender using an underlying channel.
    pub fn new(sink: Si) -> Self {
        TwitchChatSender { sink }
    }
}
