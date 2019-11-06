//! Helpers for rate limiting

use core::pin::Pin;
use std::collections::vec_deque::VecDeque;
use std::iter::FromIterator;
use std::sync::Arc;
use std::time::{Duration, Instant};

use fnv::FnvHashMap;
use futures_core::Future;
use futures_core::stream::{FusedStream, Stream};
use futures_core::task::{Context, Poll};
use futures_util::FutureExt;
use pin_utils::{unsafe_pinned, unsafe_unpinned};
use smallvec::SmallVec;
use tokio_sync::semaphore::{AcquireError, Permit, Semaphore};
use tokio_timer::clock::now;
use tokio_timer::Delay;
use tokio_timer::timer::Handle;

/// Rate limiting sink extension methods
pub trait RateLimitExt<Item>: Stream {
    /// Composes a fixed size buffered rate limiter in front of this sink. Messages passed into
    /// the sink are treated according to their implementations of [`RateLimitable`](self::RateLimitable).
    /// Messages that report rate limiting applies are put into a buffer which is sent  
    fn rate_limited(
        self,
        capacity: usize,
        default_slow: SlowModeLimit,
        default_buckets: impl IntoIterator<Item=Arc<RateLimitBucket>>
    ) -> BufferedRateLimiter<Self, Item>
    where
        Self: Sized + Stream<Item = Item> + Unpin,
        Item: RateLimitable,
    {
        BufferedRateLimiter::new(self, capacity, default_slow, default_buckets)
    }
}
impl<Si, Item> RateLimitExt<Item> for Si where Si: Stream<Item = Item> {}

/// Trait to apply to messages that contains information about which rate limits apply
/// to the message
pub trait RateLimitable {
    /// Determines whether the global 1 second slow mode or a set channel slow mode applies
    /// to the message
    fn slow_mode_channel(&self) -> Option<&str>;
}

/// Rate limiting buffered sink
#[derive(Debug)]
#[must_use = "sinks do nothing unless polled"]
pub struct BufferedRateLimiter<St: Stream<Item = Item>, Item: RateLimitable> {
    stream: St,
    buf: VecDeque<Item>,
    // Track capacity separately from the `VecDeque`, which may be rounded up
    capacity: usize,
    limits_map: FnvHashMap<String, ChannelLimits>,
    default_slow: SlowModeLimit,
    default_buckets: SmallVec<[Arc<RateLimitBucket>; 5]>
}

impl<St: Stream<Item = Item> + Unpin, Item: RateLimitable> Unpin for BufferedRateLimiter<St, Item> {}

impl<St: Stream<Item = Item> + Unpin, Item: RateLimitable> BufferedRateLimiter<St, Item> {
    unsafe_pinned!(stream: St);
    unsafe_unpinned!(buf: VecDeque<Item>);
    unsafe_unpinned!(capacity: usize);
    unsafe_unpinned!(limits_map: FnvHashMap<String, ChannelLimits>);

    pub(super) fn new(stream: St, capacity: usize, default_slow: SlowModeLimit, default_buckets: impl IntoIterator<Item=Arc<RateLimitBucket>>) -> Self {
        BufferedRateLimiter {
            stream,
            buf: VecDeque::with_capacity(capacity),
            capacity,
            limits_map: Default::default(),
            default_slow,
            default_buckets: SmallVec::from_iter(default_buckets)
        }
    }

    /// Configure slow mode rate limiting for a channel.
    pub fn set_slow_mode(&mut self, channel: &str, limit: SlowModeLimit) {
        if self.limits_map.contains_key(channel) {
            self.limits_map.get_mut(channel).unwrap().set_slow_mode(limit);
        } else {
            self.limits_map.insert(channel.to_owned(), ChannelLimits::new(limit, self.default_buckets.iter().cloned()));
        }
    }

    /// Configure bucket based rate limiting for a channel.
    pub fn set_limit_buckets(&mut self, channel: &str, buckets: impl IntoIterator<Item=Arc<RateLimitBucket>>) {
        if self.limits_map.contains_key(channel) {
            self.limits_map.get_mut(channel).unwrap().set_buckets(buckets);
        } else {
            self.limits_map.insert(channel.to_owned(), ChannelLimits::new(self.default_slow, buckets));
        }
    }

    fn init_channel(self: Pin<&mut Self>, channel: &str) -> bool {
        // if the channel was never queried before, insert the default setting
        let BufferedRateLimiter {
            ref default_slow,
            ref default_buckets,
            limits_map,
            ..
        } = Pin::into_inner(self);
        if !limits_map.contains_key(channel) {
            limits_map
                .insert(channel.to_owned(), ChannelLimits::new(*default_slow, default_buckets.iter().cloned()));
        }

        // check if a slow mode interval is set for the channel
        limits_map
            .get(channel)
            .into_iter()
            .any(|limits| limits.slow_mode_delay.is_some())
    }

    fn poll_channel_slowmode(
        mut self: Pin<&mut Self>,
        channel: &str,
        cx: &mut Context<'_>,
    ) -> Poll<()> {
        self.as_mut().init_channel(channel);
        self.as_mut()
            .limits_map()
            .get_mut(channel)
            .and_then(|limits| limits.slow_mode_delay.as_mut())
            .map(|delay| delay.poll_unpin(cx))
            .unwrap_or_else(|| Poll::Ready(()))
    }

    fn pop_first_ready_item(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Option<Item> {
        let Self {
            buf,
            limits_map,
            ..
        } = Pin::into_inner(self);
        let ready_item_idx = buf.iter().enumerate().find_map(|(i, item)| {
            if let Some(channel) = item.slow_mode_channel() {
                let ready = limits_map
                    .get_mut(channel)
                    .map(|limits| {

                        let slow_ready = limits.poll_slow_mode(cx).is_ready();

                        if slow_ready { Poll::Ready(()) }
                        else { Poll::Pending }
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
        if let Some(limits) = self.as_mut().limits_map.get_mut(channel) {
            if let Some(delay) = limits.slow_mode.new_delay() {
                limits.slow_mode_delay.replace(delay);
            } else {
                limits.slow_mode_delay.take();
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
                if let Some(channel) = item.slow_mode_channel() {
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

#[derive(Debug)]
pub struct ChannelLimits {
    slow_mode: SlowModeLimit,
    slow_mode_delay: Option<Delay>,
    limit_buckets: SmallVec<[Arc<RateLimitBucket>; 5]>,
}

impl ChannelLimits {
    pub fn new(slow: SlowModeLimit, limit_buckets: impl IntoIterator<Item=Arc<RateLimitBucket>>) -> Self {
        ChannelLimits { slow_mode: slow, slow_mode_delay: None, limit_buckets: SmallVec::from_iter(limit_buckets) }
    }

    pub fn set_slow_mode(&mut self, slow: SlowModeLimit) {
        self.slow_mode = slow;
        self.slow_mode_delay = None;
    }

    pub fn set_buckets(&mut self, buckets: impl IntoIterator<Item=Arc<RateLimitBucket>>) {
        self.limit_buckets.truncate(0);
        self.limit_buckets.extend(buckets)
    }

    fn reset_slow_mode(&mut self) {
        if let Some(delay) = self.slow_mode.new_delay() {
            self.slow_mode_delay.replace(delay);
        }
    }

    fn poll_slow_mode(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(delay) = &mut self.slow_mode_delay {
            match delay.poll_unpin(cx) {
                Poll::Ready(_) => {
                    if let Some(deadline) = self.slow_mode.next_deadline() { delay.reset(deadline) }
                    Poll::Ready(())
                },
                Poll::Pending => Poll::Pending,
            }
        } else {
            self.reset_slow_mode();
            Poll::Ready(())
        }
    }

    fn poll_buckets(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if self.limit_buckets.iter().all(|bucket| bucket.is_ready()) {
            for bucket in &self.limit_buckets {
                if (&**bucket).poll_unpin(cx).is_pending() {
                    return Poll::Pending
                }
            }
            Poll::Ready(())
        } else {
            Poll::Pending
        }
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

/// Semaphore based rate limit bucket with configurable refill delay and capacity
#[derive(Clone, Debug)]
pub struct RateLimitBucket {
    refill_delay: Duration,
    capacity: usize,
    semaphore: Arc<Semaphore>,
}

impl RateLimitBucket {
    pub fn new(capacity: usize, refill_delay: Duration) -> Self {
        RateLimitBucket {
            refill_delay,
            capacity,
            semaphore: Arc::new(Semaphore::new(capacity)),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.semaphore.available_permits() > 0
    }
}

impl Future for &RateLimitBucket {
    type Output = Result<(), AcquireError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut permit = Permit::new();
        match permit.poll_acquire(cx, &*self.semaphore) {
            Poll::Ready(Ok(())) => {
                let semaphore = self.semaphore.clone();
                let deadline = now() + self.refill_delay;
                tokio_executor::spawn(async move {
                    Handle::current().delay(deadline).await;
                    permit.release(&*semaphore)
                });
                Poll::Ready(Ok(()))
            },
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use futures_util::stream::iter;
    use futures_util::StreamExt;
    use tokio_test::{assert_pending, assert_ready, assert_ready_eq, task};

    use crate::rate_limits::{RateLimitable, RateLimitExt, SlowModeLimit};

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Limitable(Option<String>);
    impl RateLimitable for Limitable {
        fn slow_mode_channel(&self) -> Option<&str> {
            self.0.as_ref().map(AsRef::as_ref)
        }
    }

    #[test]
    fn test_default_slowmode() {
        task::mock(|cx| {
            tokio_test::clock::mock(|handle| {
                let a = Limitable(Some("a".to_string()));
                let mut stream = iter(vec![a.clone(), a.clone(), a.clone()])
                    .rate_limited(10, SlowModeLimit::Global, vec![]);
                assert_ready!(stream.poll_next_unpin(cx));
                assert_pending!(stream.poll_next_unpin(cx));

                handle.advance(Duration::from_millis(1100));

                assert_ready_eq!(stream.poll_next_unpin(cx), Some(a.clone()));

                handle.advance(Duration::from_millis(800));

                assert_pending!(stream.poll_next_unpin(cx));

                handle.advance(Duration::from_millis(1000));

                assert_ready_eq!(stream.poll_next_unpin(cx), Some(a));
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
                    iter(vec![a.clone(), a.clone()]).rate_limited(10, SlowModeLimit::Global, vec![]);
                stream.set_slow_mode("a", SlowModeLimit::Channel(10));
                assert_ready!(stream.poll_next_unpin(cx));
                handle.advance(Duration::from_secs(5));
                assert_pending!(stream.poll_next_unpin(cx));
                handle.advance(Duration::from_secs(5));
                assert_ready_eq!(stream.poll_next_unpin(cx), Some(a));
                assert_ready_eq!(stream.poll_next_unpin(cx), None);
            });
        });
    }
}
