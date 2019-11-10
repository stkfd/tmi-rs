//! Message/command sender helpers

use std::pin::Pin;

use futures_core::task::Context;
use futures_core::Poll;
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

impl<Si, M> Sink<M> for TwitchChatSender<Si>
where
    Si: Sink<ClientMessage<String>> + Unpin + Clone,
    M: Into<ClientMessage<String>>,
{
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut Pin::into_inner(self).sink)
            .poll_ready(cx)
            .map_err(|_| Error::SendError)
    }

    fn start_send(self: Pin<&mut Self>, item: M) -> Result<(), Self::Error> {
        let msg: ClientMessage<String> = item.into();
        debug!("> {}", msg);
        Pin::new(&mut Pin::into_inner(self).sink)
            .start_send(msg)
            .map_err(|_| Error::SendError)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut Pin::into_inner(self).sink)
            .poll_flush(cx)
            .map_err(|_| Error::SendError)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut Pin::into_inner(self).sink)
            .poll_close(cx)
            .map_err(|_| Error::SendError)
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
