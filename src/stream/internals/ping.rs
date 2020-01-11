use std::pin::Pin;

use futures_core::task::{Context, Poll};
use futures_core::Stream;
use tokio::sync::mpsc::UnboundedSender;

use crate::event::*;
use crate::stream::EventStream;
use crate::{ClientMessage, Error};

pub struct PingHandler<S> {
    inner_stream: S,
    sender: UnboundedSender<ClientMessage<String>>,
    sender_closed: bool,
}

impl<S> PingHandler<S> {
    pub(crate) fn new(inner_stream: S, sender: UnboundedSender<ClientMessage<String>>) -> Self {
        PingHandler {
            inner_stream,
            sender,
            sender_closed: false,
        }
    }
}

impl<S> Stream for PingHandler<S>
where
    S: EventStream,
{
    type Item = Result<Event<String>, Error>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.sender_closed {
            return Poll::Ready(None);
        }

        let poll = Pin::new(&mut (*self).inner_stream).poll_next(cx);
        if let Poll::Ready(Some(item)) = &poll {
            if let Ok(event) = &item {
                if let Event::Ping(_) = event {
                    if self.sender.send(ClientMessage::<String>::Pong).is_err() {
                        self.sender_closed = true;
                        return Poll::Ready(Some(Err(Error::SendError)));
                    }
                }
            };
        }
        poll
    }
}
