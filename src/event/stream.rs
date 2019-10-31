use core::fmt;
use core::pin::Pin;
use std::convert::TryFrom;
use std::iter::FromIterator;
use std::sync::Arc;

use futures_core::{stream::FusedStream, task::Context, Poll, Stream};
use futures_sink::Sink;
use smallvec::SmallVec;
use tokio_tungstenite::tungstenite::error::Error as WsError;
use tokio_tungstenite::tungstenite::Message;

use crate::event::{CloseEvent, Event};
use crate::irc::IrcMessage;
use crate::Error;

type EventBuffer = SmallVec<[Result<Event<String>, Error>; 10]>;

/// A wrapper around the websocket stream that parses incoming IRC messages into event structs
/// and formats Message or Command structs as IRC messages.
#[must_use = "streams do nothing unless polled"]
pub struct TwitchChatStream<St> {
    stream: St,
    buffer: Option<EventBuffer>,
}

impl<St: Unpin> Unpin for TwitchChatStream<St> {}

impl<St> fmt::Debug for TwitchChatStream<St>
where
    St: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Map").field("stream", &self.stream).finish()
    }
}

impl<St> TwitchChatStream<St>
where
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    unsafe_pinned!(stream: St);

    /// Create new `TwitchChatStream` from a stream of websocket messages
    pub fn new(stream: St) -> TwitchChatStream<St> {
        TwitchChatStream {
            stream,
            buffer: None,
        }
    }

    /// Acquires a reference to the underlying stream that this combinator is
    /// pulling from.
    pub fn get_ref(&self) -> &St {
        &self.stream
    }

    /// Acquires a mutable reference to the underlying stream that this
    /// combinator is pulling from.
    ///
    /// Note that care must be taken to avoid tampering with the state of the
    /// stream which may otherwise confuse this combinator.
    pub fn get_mut(&mut self) -> &mut St {
        &mut self.stream
    }

    /// Acquires a pinned mutable reference to the underlying stream that this
    /// combinator is pulling from.
    ///
    /// Note that care must be taken to avoid tampering with the state of the
    /// stream which may otherwise confuse this combinator.
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut St> {
        self.stream()
    }

    /// Consumes this combinator, returning the underlying stream.
    ///
    /// Note that this may discard intermediate state of this combinator, so
    /// care should be taken to avoid losing resources when this is called.
    pub fn into_inner(self) -> St {
        self.stream
    }
}

impl<St> FusedStream for TwitchChatStream<St>
where
    St: FusedStream + Stream<Item = Result<Message, WsError>> + Unpin,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<St> Stream for TwitchChatStream<St>
where
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    type Item = Arc<Result<Event<String>, Error>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let unpin_self = Pin::into_inner(self);
        // if buffer contains any messages, return directly
        if let Some(buffer) = &mut unpin_self.buffer {
            if !buffer.is_empty() {
                return Poll::Ready(Some(Arc::new(buffer.remove(buffer.len() - 1))));
            }
        }

        // otherwise, poll underlying stream
        let parse_result = Pin::new(&mut unpin_self.stream)
            .poll_next(cx)
            .map(|opt| opt.map(parse));

        match parse_result {
            Poll::Ready(result) => {
                if let Some(events) = result {
                    unpin_self.buffer.replace(events);
                    if let Some(buffer) = &mut unpin_self.buffer {
                        Poll::Ready(Some(Arc::new(buffer.remove(buffer.len() - 1))))
                    } else {
                        Poll::Pending
                    }
                } else {
                    Poll::Ready(None)
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

// Forwarding impl of Sink from the underlying stream
impl<St, IntoMessage> Sink<IntoMessage> for TwitchChatStream<St>
where
    St: Stream + Sink<Message> + Unpin,
    Message: From<IntoMessage>,
{
    type Error = St::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut Pin::into_inner(self).stream).poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: IntoMessage) -> Result<(), Self::Error> {
        Pin::new(&mut Pin::into_inner(self).stream).start_send(Message::from(item))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut Pin::into_inner(self).stream).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut Pin::into_inner(self).stream).poll_close(cx)
    }
}

fn parse(msg_result: Result<Message, WsError>) -> EventBuffer {
    match msg_result {
        Ok(msg) => match msg {
            Message::Text(msg) => {
                debug!("< {}", msg);
                match IrcMessage::<&str>::parse_many(&msg) {
                    Ok((_remaining, messages)) => {
                        let event_results = messages.into_iter().map(|irc_msg| {
                            Event::try_from(irc_msg).map(|event| Event::<String>::from(&event))
                        });
                        SmallVec::from_iter(event_results.rev())
                    }
                    Err(err) => {
                        error!("IRC parse error: {:?}", err);
                        smallvec![Err(Error::IrcParseError(format!("{:?}", err)))]
                    }
                }
            }
            Message::Binary(msg) => {
                info!("< Binary<{} bytes>", msg.len());
                SmallVec::new()
            }
            Message::Close(close_frame) => {
                if let Some(close_frame) = close_frame {
                    info!("Received close frame: {}", close_frame)
                }
                info!("Connection closed by the server.");
                smallvec![Ok(CloseEvent.into())]
            }
            Message::Ping(_payload) => {
                debug!("< WS PING");
                SmallVec::new()
            }
            Message::Pong(_payload) => {
                debug!("< WS PONG");
                SmallVec::new()
            }
        },
        Err(e) => smallvec![Err(Error::WebsocketError {
            details: "Error receiving messages from the websocket connection".into(),
            source: e,
        })],
    }
}
