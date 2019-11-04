//! Helpers for rate limiting

use core::pin::Pin;
use std::collections::vec_deque::VecDeque;
use std::time::{Duration, Instant};

use fnv::FnvHashMap;
use futures_core::stream::{FusedStream, Stream};
use futures_core::task::{Context, Poll};
use futures_util::StreamExt;
use pin_utils::{unsafe_pinned, unsafe_unpinned};
use tokio_timer::Interval;

/// Rate limiting sink extension methods
pub trait RateLimitExt<Item>: Stream {
    /// Composes a fixed size buffered rate limiter in front of this sink. Messages passed into
    /// the sink are treated according to their implementations of [`RateLimitable`](self::RateLimitable).
    /// Messages that report rate limiting applies are put into a buffer which is sent  
    fn rate_limited(
        self,
        capacity: usize,
        default_slow: SlowModeLimit,
    ) -> BufferedRateLimiter<Self, Item>
    where
        Self: Sized + Stream<Item = Item> + Unpin,
        Item: RateLimitable,
    {
        BufferedRateLimiter::new(self, capacity, default_slow)
    }
}
impl<Si, Item> RateLimitExt<Item> for Si where Si: Stream<Item = Item> {}

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
pub struct BufferedRateLimiter<St: Stream<Item = Item>, Item: RateLimitable> {
    stream: St,
    buf: VecDeque<Item>,
    // Track capacity separately from the `VecDeque`, which may be rounded up
    capacity: usize,
    channel_slowmode: FnvHashMap<String, (SlowModeLimit, Option<Interval>)>,
    default_slow: SlowModeLimit,
}

impl<St: Stream<Item = Item> + Unpin, Item: RateLimitable> Unpin for BufferedRateLimiter<St, Item> {}

impl<St: Stream<Item = Item> + Unpin, Item: RateLimitable> BufferedRateLimiter<St, Item> {
    unsafe_pinned!(stream: St);
    unsafe_unpinned!(buf: VecDeque<Item>);
    unsafe_unpinned!(capacity: usize);
    unsafe_unpinned!(channel_slowmode: FnvHashMap<String, (SlowModeLimit, Option<Interval>)>);

    pub(super) fn new(stream: St, capacity: usize, default_slow: SlowModeLimit) -> Self {
        BufferedRateLimiter {
            stream,
            buf: VecDeque::with_capacity(capacity),
            capacity,
            channel_slowmode: Default::default(),
            default_slow,
        }
    }

    /// Configure slow mode rate limiting for a channel.
    pub fn set_channel_slowmode(mut self: Pin<&mut Self>, channel: String, limit: SlowModeLimit) {
        let interval = limit.new_interval();
        self.as_mut()
            .channel_slowmode()
            .insert(channel, (limit, interval));
    }

    fn ensure_interval(mut self: Pin<&mut Self>, channel: &str) -> bool {
        // if the channel was never queried before, insert the default setting
        if !self.channel_slowmode.contains_key(channel) {
            let default_slow = self.default_slow;
            self.as_mut().channel_slowmode().insert(
                channel.to_owned(),
                (default_slow, default_slow.new_interval()),
            );
        }

        // check if a slow mode interval is set for the channel
        self.channel_slowmode
            .get(channel)
            .filter(|(_, interval)| interval.is_some())
            .is_some()
    }

    fn poll_channel_slowmode(
        mut self: Pin<&mut Self>,
        channel: &str,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Instant>> {
        self.as_mut().ensure_interval(channel);

        self.as_mut()
            .channel_slowmode()
            .get_mut(channel)
            .and_then(|(_, interval)| interval.as_mut())
            .map(|interval| interval.poll_next(cx))
            .unwrap_or_else(|| Poll::Ready(Some(Instant::now())))
    }

    fn ready_buf_item(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Option<Item> {
        let Self {
            buf,
            channel_slowmode,
            ..
        } = Pin::into_inner(self);
        let ready_item_idx = buf.iter().enumerate().find_map(|(i, item)| {
            if let Some(channel) = item.channel_slow_mode() {
                let ready = channel_slowmode
                    .get_mut(channel)
                    .and_then(|(_, interval)| interval.as_mut())
                    .map(|interval| interval.poll_next_unpin(cx))
                    .unwrap_or_else(|| Poll::Ready(Some(Instant::now())));
                if let Poll::Ready(_) = ready {
                    return Some(i);
                }
                return None;
            }
            None
        });

        ready_item_idx.and_then(|i| buf.swap_remove_back(i))
    }

    /// Get a shared reference to the inner sink.
    pub fn get_ref(&self) -> &St {
        &self.stream
    }

    /// Get a mutable reference to the inner sink.
    pub fn get_mut(&mut self) -> &mut St {
        &mut self.stream
    }

    /// Get a pinned mutable reference to the inner sink.
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut St> {
        self.stream()
    }

    /// Consumes this combinator, returning the underlying sink.
    ///
    /// Note that this may discard intermediate state of this combinator, so
    /// care should be taken to avoid losing resources when this is called.
    pub fn into_inner(self) -> St {
        self.stream
    }
}

impl<S, Item> Stream for BufferedRateLimiter<S, Item>
where
    S: Stream<Item = Item> + Unpin,
    Item: RateLimitable,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        // return any ready items in the buffer if available
        if !self.as_ref().buf.is_empty() {
            let ready_item = self.as_mut().ready_buf_item(cx);
            if ready_item.is_some() {
                return Poll::Ready(ready_item);
            }
        }
        match self.as_mut().stream().poll_next(cx) {
            Poll::Ready(Some(item)) => {
                if let Some(channel) = item.channel_slow_mode() {
                    let poll_channel = self.as_mut().poll_channel_slowmode(channel, cx);
                    if poll_channel.is_ready() {
                        Poll::Ready(Some(item))
                    } else {
                        self.as_mut().buf().push_back(item);
                        Poll::Pending
                    }
                } else {
                    Poll::Ready(Some(item))
                }
            }
            Poll::Ready(None) => {
                if self.buf.len() == 0 {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

impl<S, Item> FusedStream for BufferedRateLimiter<S, Item>
where
    S: Stream<Item = Item> + FusedStream + Unpin,
    Item: RateLimitable,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

/// Slow mode configuration for a channel
#[derive(Copy, Clone, Debug)]
pub enum SlowModeLimit {
    /// limit to an amount of seconds per message
    Channel(usize),
    /// no channel specific setting, but respect global 1 second slow mode
    Global,
    /// no limit, also ignoring global 1 second slow
    Unlimited,
}

impl SlowModeLimit {
    /// Create an `Interval` that matches this limit
    pub fn new_interval(&self) -> Option<Interval> {
        match self {
            SlowModeLimit::Channel(secs) => Some(Duration::from_secs(*secs as u64)),
            SlowModeLimit::Global => Some(Duration::from_secs(1)),
            SlowModeLimit::Unlimited => None,
        }
        .map(Interval::new_interval)
    }
}

#[cfg(test)]
mod test {
    use futures_util::stream::iter;

    struct Limitable {
        channel: Option<String>,
    }

    #[test]
    fn test_throttle() {
        //iter(vec![1,2,3,4,5])
    }
}
