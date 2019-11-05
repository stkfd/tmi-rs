//! Helpers for rate limiting

use core::pin::Pin;
use std::collections::vec_deque::VecDeque;
use std::time::{Duration, Instant};

use fnv::FnvHashMap;
use futures_core::stream::{FusedStream, Stream};
use futures_core::task::{Context, Poll};
use futures_util::FutureExt;
use pin_utils::{unsafe_pinned, unsafe_unpinned};
use tokio_timer::clock::now;
use tokio_timer::timer::Handle;
use tokio_timer::Delay;

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
    channel_slowmode: FnvHashMap<String, (SlowModeLimit, Option<Delay>)>,
    default_slow: SlowModeLimit,
}

impl<St: Stream<Item = Item> + Unpin, Item: RateLimitable> Unpin for BufferedRateLimiter<St, Item> {}

impl<St: Stream<Item = Item> + Unpin, Item: RateLimitable> BufferedRateLimiter<St, Item> {
    unsafe_pinned!(stream: St);
    unsafe_unpinned!(buf: VecDeque<Item>);
    unsafe_unpinned!(capacity: usize);
    unsafe_unpinned!(channel_slowmode: FnvHashMap<String, (SlowModeLimit, Option<Delay>)>);

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
    pub fn set_channel_slowmode(&mut self, channel: String, limit: SlowModeLimit) {
        self.channel_slowmode.insert(channel, (limit, None));
    }

    fn ensure_interval(mut self: Pin<&mut Self>, channel: &str) -> bool {
        // if the channel was never queried before, insert the default setting
        if !self.channel_slowmode.contains_key(channel) {
            let default_slow = self.default_slow;
            self.as_mut()
                .channel_slowmode()
                .insert(channel.to_owned(), (default_slow, None));
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
    ) -> Poll<()> {
        self.as_mut().ensure_interval(channel);

        self.as_mut()
            .channel_slowmode()
            .get_mut(channel)
            .and_then(|(_, delay)| delay.as_mut())
            .map(|delay| delay.poll_unpin(cx))
            .unwrap_or_else(|| Poll::Ready(()))
    }

    fn pop_first_ready_item(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Option<Item> {
        let Self {
            buf,
            channel_slowmode,
            ..
        } = Pin::into_inner(self);
        let ready_item_idx = buf.iter().enumerate().find_map(|(i, item)| {
            if let Some(channel) = item.channel_slow_mode() {
                let ready = channel_slowmode
                    .get_mut(channel)
                    .map(|(mode, delay)| {
                        if let Some(delay) = delay {
                            let poll = delay.poll_unpin(cx);
                            if let Poll::Ready(_) = poll {
                                if let Some(deadline) = mode.next_deadline() {
                                    delay.reset(deadline);
                                }
                            }
                            poll
                        } else {
                            Poll::Ready(())
                        }
                    })
                    .unwrap_or_else(|| Poll::Ready(()));
                if let Poll::Ready(_) = ready {
                    return Some(i);
                }
            }
            None
        });

        ready_item_idx.and_then(|i| buf.swap_remove_back(i))
    }

    fn reset_channel_delay(mut self: Pin<&mut Self>, channel: &str) {
        if let Some((mode, opt_delay)) = self.as_mut().channel_slowmode.get_mut(channel) {
            if let Some(delay) = mode.new_delay() {
                opt_delay.replace(delay);
            } else {
                opt_delay.take();
            }
        }
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
            let ready_item = self.as_mut().pop_first_ready_item(cx);
            if ready_item.is_some() {
                return Poll::Ready(ready_item);
            }
        }
        match self.as_mut().stream().poll_next(cx) {
            Poll::Ready(Some(item)) => {
                if let Some(channel) = item.channel_slow_mode() {
                    let poll_channel = self.as_mut().poll_channel_slowmode(channel, cx);
                    if poll_channel.is_ready() {
                        self.as_mut().reset_channel_delay(channel);
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
                if self.buf.is_empty() {
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
    pub fn new_delay(&self) -> Option<Delay> {
        self.next_deadline()
            .map(|deadline| Handle::current().delay(deadline))
    }

    /// Get the next time when a message can be posted within the limits
    pub fn next_deadline(&self) -> Option<Instant> {
        match self {
            SlowModeLimit::Channel(secs) => Some(now() + Duration::from_secs(*secs as u64)),
            SlowModeLimit::Global => Some(now() + Duration::from_secs(1)),
            SlowModeLimit::Unlimited => None,
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use futures_util::stream::iter;
    use futures_util::StreamExt;
    use tokio_test::{assert_pending, assert_ready, assert_ready_eq, task};

    use crate::rate_limits::{RateLimitExt, RateLimitable, SlowModeLimit};

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Limitable(Option<String>);
    impl RateLimitable for Limitable {
        fn channel_slow_mode(&self) -> Option<&str> {
            self.0.as_ref().map(AsRef::as_ref)
        }
    }

    #[test]
    fn test_default_slowmode() {
        task::mock(|cx| {
            tokio_test::clock::mock(|handle| {
                let a = Limitable(Some("a".to_string()));
                let mut stream = iter(vec![a.clone(), a.clone(), a.clone()])
                    .rate_limited(10, SlowModeLimit::Global);
                assert_ready!(stream.poll_next_unpin(cx));
                assert_pending!(stream.poll_next_unpin(cx));

                handle.advance(Duration::from_millis(1100));

                assert_ready_eq!(stream.poll_next_unpin(cx), Some(a.clone()));
                assert_pending!(stream.poll_next_unpin(cx));

                handle.advance(Duration::from_millis(1000));

                assert_ready_eq!(stream.poll_next_unpin(cx), Some(a.clone()));
                assert_ready_eq!(stream.poll_next_unpin(cx), None);
            });
        });
    }

    #[test]
    fn test_custom_slowmode() {
        task::mock(|cx| {
            tokio_test::clock::mock(|handle| {
                let a = Limitable(Some("a".to_string()));
                let mut stream =
                    iter(vec![a.clone(), a.clone()]).rate_limited(10, SlowModeLimit::Global);
                stream.set_channel_slowmode("a".to_string(), SlowModeLimit::Channel(10));
                assert_ready!(stream.poll_next_unpin(cx));
                handle.advance(Duration::from_secs(5));
                assert_pending!(stream.poll_next_unpin(cx));
                handle.advance(Duration::from_secs(5));
                assert_ready_eq!(stream.poll_next_unpin(cx), Some(a.clone()));
                assert_ready_eq!(stream.poll_next_unpin(cx), None);
            });
        });
    }
}
