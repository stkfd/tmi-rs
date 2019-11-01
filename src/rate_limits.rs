//! Helpers for rate limiting

use core::pin::Pin;
use std::collections::VecDeque;
use std::time::Duration;

use fnv::FnvHashMap;
use futures_core::stream::{FusedStream, Stream};
use futures_core::task::{Context, Poll};
use futures_sink::Sink;
use pin_utils::{unsafe_pinned, unsafe_unpinned};
use tokio_timer::Interval;

/// Trait to apply to messages that contains information about which rate limits apply
/// to the message
pub trait RateLimitable {
    /// Determines whether the global 1 second slow mode or a set channel slow mode applies
    /// to the message
    fn channel_slow_mode(&self) -> Option<&str>;
}

/// Rate limiting buffered sink
#[derive(Debug)]
#[must_use = "sinks do nothing unless polled"]
pub struct BufferedRateLimiter<Si: Sink<Item>, Item: RateLimitable> {
    sink: Si,
    buf: VecDeque<Item>,
    // Track capacity separately from the `VecDeque`, which may be rounded up
    capacity: usize,
    channel_slowmode: FnvHashMap<String, Interval>,
}

impl<Si: Sink<Item> + Unpin, Item: RateLimitable> Unpin for BufferedRateLimiter<Si, Item> {}

impl<Si: Sink<Item>, Item: RateLimitable> BufferedRateLimiter<Si, Item> {
    unsafe_pinned!(sink: Si);
    unsafe_unpinned!(buf: VecDeque<Item>);
    unsafe_unpinned!(capacity: usize);

    pub(super) fn new(sink: Si, capacity: usize) -> Self {
        BufferedRateLimiter {
            sink,
            buf: VecDeque::with_capacity(capacity),
            capacity,
            channel_slowmode: Default::default(),
        }
    }

    pub fn set_channel_slowmode(&mut self, channel: String, slow: Option<u64>) {
        if let Some(secs) = slow {
            self.channel_slowmode
                .insert(channel, Interval::new_interval(Duration::from_secs(secs)));
        } else {
            self.channel_slowmode.remove(&channel);
        }
    }

    fn get_channel_interval(&mut self, channel: &str) -> Option<&Interval> {
        match self.channel_slowmode.get(channel) {
            Some(interval) => Some(interval),
            None => self
                .channel_slowmode
                .insert(
                    channel.to_owned(),
                    Interval::new_interval(Duration::from_secs(1)),
                )
                .as_ref(),
        }
    }

    /// Get a shared reference to the inner sink.
    pub fn get_ref(&self) -> &Si {
        &self.sink
    }

    /// Get a mutable reference to the inner sink.
    pub fn get_mut(&mut self) -> &mut Si {
        &mut self.sink
    }

    /// Get a pinned mutable reference to the inner sink.
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut Si> {
        self.sink()
    }

    /// Consumes this combinator, returning the underlying sink.
    ///
    /// Note that this may discard intermediate state of this combinator, so
    /// care should be taken to avoid losing resources when this is called.
    pub fn into_inner(self) -> Si {
        self.sink
    }

    fn try_empty_buffer(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Si::Error>> {
        ready!(self.as_mut().sink().poll_ready(cx))?;
        while let Some(item) = self.as_mut().buf().pop_front() {
            self.as_mut().sink().start_send(item)?;
            if !self.buf.is_empty() {
                ready!(self.as_mut().sink().poll_ready(cx))?;
            }
        }
        Poll::Ready(Ok(()))
    }
}

// Forwarding impl of Stream from the underlying sink
impl<S, Item> Stream for BufferedRateLimiter<S, Item>
where
    S: Sink<Item> + Stream,
    Item: RateLimitable,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        self.sink().poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.sink.size_hint()
    }
}

impl<S, Item> FusedStream for BufferedRateLimiter<S, Item>
where
    S: Sink<Item> + FusedStream,
    Item: RateLimitable,
{
    fn is_terminated(&self) -> bool {
        self.sink.is_terminated()
    }
}

impl<Si: Sink<Item>, Item: RateLimitable> Sink<Item> for BufferedRateLimiter<Si, Item> {
    type Error = Si::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.capacity == 0 {
            return self.as_mut().sink().poll_ready(cx);
        }

        let _ = self.as_mut().try_empty_buffer(cx)?;

        if self.buf.len() >= self.capacity {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        if self.capacity == 0 {
            self.as_mut().sink().start_send(item)
        } else {
            self.as_mut().buf().push_back(item);
            Ok(())
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().try_empty_buffer(cx))?;
        debug_assert!(self.as_mut().buf().is_empty());
        self.as_mut().sink().poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().try_empty_buffer(cx))?;
        debug_assert!(self.as_mut().buf().is_empty());
        self.as_mut().sink().poll_close(cx)
    }
}
