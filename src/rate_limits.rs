//! Helpers for rate limiting

use core::pin::Pin;
use std::collections::vec_deque::VecDeque;
use std::iter::FromIterator;
use std::sync::Arc;
use std::time::{Duration, Instant};

use fnv::FnvHashMap;
use futures_core::stream::{FusedStream, Stream};
use futures_core::task::{Context, Poll};
use futures_core::Future;
use futures_util::FutureExt;
use parking_lot::RwLock;
use pin_utils::{unsafe_pinned, unsafe_unpinned};
use tokio_sync::semaphore::{Permit, Semaphore};
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
        rate_limiter: &Arc<RateLimiter>,
    ) -> BufferedRateLimiter<Self, Item>
    where
        Self: Sized + Stream<Item = Item> + Unpin,
        Item: RateLimitable,
    {
        BufferedRateLimiter::new(self, capacity, rate_limiter)
    }
}
impl<Si, Item> RateLimitExt<Item> for Si where Si: Stream<Item = Item> {}

/// Trait to apply to messages that contains information about which rate limits apply
/// to the message
pub trait RateLimitable {
    /// Should return a channel name, if available. If a channel is returned, applies the slow mode
    /// and rate limit buckets configured for that channel
    fn channel_limits(&self) -> Option<&str>;

    /// Poll for sending the item using the given rate limiter instance
    fn poll(&self, rate_limiter: &RateLimiter, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(channel) = self.channel_limits() {
            rate_limiter.init_channel(channel);

            let limits = rate_limiter.limits_map.read();
            let buckets = rate_limiter.buckets.read();

            let channel_limits = limits.get(channel).expect("Get channel rate limits");
            let slow_ready = channel_limits.read().poll_slow_mode(cx).is_ready();
            if !slow_ready {
                return Poll::Pending;
            }

            let buckets_ready = channel_limits
                .read()
                .limit_buckets
                .iter()
                .filter_map(|&bucket_name| buckets.get(bucket_name))
                .all(|mut bucket| (&mut bucket).poll_unpin(cx).is_ready());
            if !buckets_ready {
                return Poll::Pending;
            }

            Poll::Ready(())
        } else {
            // no limits apply, always return ready
            Poll::Ready(())
        }
    }
}

/// Rate limiting buffered sink
#[derive(Debug)]
#[must_use = "sinks do nothing unless polled"]
pub struct BufferedRateLimiter<St: Stream<Item = Item>, Item: RateLimitable> {
    stream: St,
    buf: VecDeque<Item>,
    capacity: usize,
    rate_limiter: Arc<RateLimiter>,
}

impl<St: Stream<Item = Item> + Unpin, Item: RateLimitable> Unpin for BufferedRateLimiter<St, Item> {}

impl<St: Stream<Item = Item> + Unpin, Item: RateLimitable> BufferedRateLimiter<St, Item> {
    unsafe_pinned!(stream: St);
    unsafe_unpinned!(buf: VecDeque<Item>);
    unsafe_unpinned!(capacity: usize);

    pub(super) fn new(stream: St, capacity: usize, rate_limiter: &Arc<RateLimiter>) -> Self {
        BufferedRateLimiter {
            stream,
            buf: VecDeque::with_capacity(capacity),
            capacity,
            rate_limiter: rate_limiter.clone(),
        }
    }

    fn pop_ready_item(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Option<Item> {
        let Self {
            buf,
            ref rate_limiter,
            ..
        } = Pin::into_inner(self);
        let ready_item_idx = buf.iter().enumerate().find_map(|(i, item)| {
            if item.poll(&**rate_limiter, cx).is_ready() {
                Some(i)
            } else {
                None
            }
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
            let ready_item = self.as_mut().pop_ready_item(cx);
            if ready_item.is_some() {
                return Poll::Ready(ready_item);
            }
        }
        match self.as_mut().stream().poll_next(cx) {
            Poll::Ready(Some(item)) => {
                if item.poll(&*self.rate_limiter, cx).is_ready() {
                    Poll::Ready(Some(item))
                } else {
                    self.as_mut().buf().push_back(item);
                    Poll::Pending
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
        self.stream.is_terminated() && self.buf.is_empty()
    }
}

/// Configuration for a rate limiter
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// The buckets used for global rate limiting
    pub buckets: FnvHashMap<&'static str, RateLimitBucketConfig>,
    /// Default configuration for slow mode, defaults to the global 1 second slow mode
    pub default_slow: SlowModeLimit,
    /// Default bucket names that apply to a message
    pub default_buckets: Vec<&'static str>,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        RateLimiterConfig {
            buckets: {
                let mut map = FnvHashMap::default();
                map.insert(
                    "privmsg-moderator",
                    RateLimitBucketConfig::new(100, Duration::from_secs(30)),
                );
                map.insert(
                    "privmsg",
                    RateLimitBucketConfig::new(20, Duration::from_secs(30)),
                );
                map
            },
            default_slow: SlowModeLimit::Global,
            default_buckets: vec!["privmsg", "privmsg-moderator"],
        }
    }
}

impl RateLimiterConfig {
    /// Initialize a rate limiter with the rate limit buckets set for an account that is a known bot
    pub fn known_bot() -> Self {
        RateLimiterConfig {
            buckets: {
                let mut map = FnvHashMap::default();
                map.insert(
                    "privmsg-moderator",
                    RateLimitBucketConfig::new(100, Duration::from_secs(30)),
                );
                map.insert(
                    "privmsg",
                    RateLimitBucketConfig::new(50, Duration::from_secs(30)),
                );
                map
            },
            default_slow: SlowModeLimit::Global,
            default_buckets: vec!["privmsg", "privmsg-moderator"],
        }
    }

    /// Initialize a rate limiter with the rate limit buckets set for an account that is a verified bot
    pub fn verified_bot() -> Self {
        RateLimiterConfig {
            buckets: {
                let mut map = FnvHashMap::default();
                map.insert(
                    "privmsg-moderator",
                    RateLimitBucketConfig::new(7500, Duration::from_secs(30)),
                );
                map.insert(
                    "privmsg",
                    RateLimitBucketConfig::new(7500, Duration::from_secs(30)),
                );
                map
            },
            default_slow: SlowModeLimit::Global,
            default_buckets: vec!["privmsg", "privmsg-moderator"],
        }
    }
}

/// A reusable, thread-safe rate limiter that allows "slow mode" and bucket based rate limiting. It
/// can be reconfigured at runtime, although this requires locking/mutexes so it shouldn't be done
/// constantly to avoid performance issues.
///
/// Constructed using `From<RateLimiterConfig>`
#[derive(Debug)]
pub struct RateLimiter {
    buckets: RwLock<FnvHashMap<&'static str, RateLimitBucket>>,
    limits_map: RwLock<FnvHashMap<String, RwLock<ChannelLimits>>>,
    default_slow: SlowModeLimit,
    default_buckets: Vec<&'static str>,
}

impl From<RateLimiterConfig> for RateLimiter {
    fn from(cfg: RateLimiterConfig) -> Self {
        RateLimiter {
            buckets: RwLock::new(
                cfg.buckets
                    .into_iter()
                    .map(|(key, cfg)| (key, RateLimitBucket::from(cfg)))
                    .collect(),
            ),
            limits_map: Default::default(),
            default_slow: cfg.default_slow,
            default_buckets: cfg.default_buckets,
        }
    }
}

impl RateLimiter {
    /// Configure slow mode rate limiting for a channel.
    pub fn set_slow_mode(&self, channel: &str, limit: SlowModeLimit) {
        let mut map = self.limits_map.write();
        if map.contains_key(channel) {
            map.get_mut(channel).unwrap().write().set_slow_mode(limit);
        } else {
            map.insert(
                channel.to_owned(),
                ChannelLimits::new(limit, self.default_buckets.iter().cloned()).into(),
            );
        }
    }

    /// Update the rate limiting buckets if necessary, when the user gains or loses mod status in
    /// a channel
    pub fn update_mod_status(&self, channel: &str, is_mod: bool) {
        self.init_channel(channel);

        let limits_map = self.limits_map.read();
        let limits = limits_map.get(channel).unwrap();
        let non_privileged_bucket = limits
            .read()
            .limit_buckets
            .iter()
            .enumerate()
            .find_map(|(idx, b)| if *b == "privmsg" { Some(idx) } else { None });
        if is_mod && non_privileged_bucket.is_some() {
            info!("Applying moderator rate limits in channel {}.", channel);
            limits
                .write()
                .limit_buckets
                .swap_remove(non_privileged_bucket.unwrap());
        } else if !is_mod && non_privileged_bucket.is_none() {
            info!("Applying non-moderator rate limits in channel {}.", channel);
            limits.write().limit_buckets.push("privmsg");
        }
    }

    fn init_channel(&self, channel: &str) {
        // if the channel was never queried before, insert the default setting
        if !self.limits_map.read().contains_key(channel) {
            self.limits_map.write().insert(
                channel.to_owned(),
                ChannelLimits::new(self.default_slow, self.default_buckets.iter().cloned()).into(),
            );
        }
    }
}

/// Container for all the rate limits that apply to a channel
#[derive(Debug)]
pub struct ChannelLimits {
    slow_mode: SlowModeLimit,
    slow_mode_delay: RwLock<Option<Delay>>,
    limit_buckets: Vec<&'static str>,
}

impl ChannelLimits {
    /// New channel limit instance
    pub fn new(slow: SlowModeLimit, limit_buckets: impl IntoIterator<Item = &'static str>) -> Self {
        ChannelLimits {
            slow_mode: slow,
            slow_mode_delay: RwLock::new(None),
            limit_buckets: Vec::from_iter(limit_buckets),
        }
    }

    /// Set the slow mode interval for this channel
    pub fn set_slow_mode(&mut self, slow: SlowModeLimit) {
        let mut delay = self.slow_mode_delay.write();
        self.slow_mode = slow;
        delay.take();
    }

    /// Set the rate limit buckets applied to this channel
    pub fn set_buckets(&mut self, buckets: impl IntoIterator<Item = &'static str>) {
        self.limit_buckets.truncate(0);
        self.limit_buckets.extend(buckets)
    }

    #[inline]
    fn reset_slow_mode(&self) {
        if let Some(delay) = self.slow_mode.new_delay() {
            self.slow_mode_delay.write().replace(delay);
        } else {
            self.slow_mode_delay.write().take();
        }
    }

    fn poll_slow_mode(&self, cx: &mut Context<'_>) -> Poll<()> {
        if self.slow_mode_delay.read().is_some() {
            if let Some(delay) = self.slow_mode_delay.write().as_mut() {
                match delay.poll_unpin(cx) {
                    Poll::Ready(_) => {
                        if let Some(deadline) = self.slow_mode.next_deadline() {
                            delay.reset(deadline)
                        }
                        Poll::Ready(())
                    }
                    Poll::Pending => Poll::Pending,
                }
            } else {
                unreachable!()
            }
        } else {
            self.reset_slow_mode();
            Poll::Ready(())
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

/// Semaphore based rate limit bucket with configurable refill delay and capacity. The semaphore is
/// initialized with the given capacity. Before a message is sent, the bucket is polled and if
/// capacity is available, a permit is taken from the semaphore. The permit is released when the refill
/// delay has elapsed. The refill times are kept in an internal ring buffer and the semaphore is
/// refilled before every poll.
///
/// Constructed using `From<RateLimitBucketConfig>`.
#[derive(Debug)]
pub struct RateLimitBucket {
    cfg: RateLimitBucketConfig,
    refill_queue: RwLock<VecDeque<(Instant, Permit)>>,
    semaphore: Semaphore,
}

/// Configuration for a rate limiting bucket.
#[derive(Debug, Clone)]
pub struct RateLimitBucketConfig {
    refill_delay: Duration,
    capacity: usize,
}

impl RateLimitBucketConfig {
    /// Create a new rate limiting bucket configuration
    pub fn new(capacity: usize, refill_delay: Duration) -> Self {
        RateLimitBucketConfig {
            refill_delay,
            capacity,
        }
    }
}

impl From<RateLimitBucketConfig> for RateLimitBucket {
    fn from(cfg: RateLimitBucketConfig) -> Self {
        let cap = cfg.capacity;
        RateLimitBucket {
            cfg,
            refill_queue: RwLock::new(VecDeque::with_capacity(cap)),
            semaphore: Semaphore::new(cap),
        }
    }
}

impl RateLimitBucket {
    /// Returns whether there are currently any messages left in the contingent
    pub fn is_ready(&self) -> bool {
        self.semaphore.available_permits() > 0
    }

    fn refill(&self) {
        let time = now();
        let mut queue = self.refill_queue.write();
        while queue.len() > 0 && queue.front().expect("Peek next queue item").0 <= time {
            let (_, mut permit) = queue.pop_front().expect("Pop instant");
            permit.release(&self.semaphore);
        }
    }
}

impl Future for &RateLimitBucket {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.refill();
        let mut permit = Permit::new();
        match permit.try_acquire(&self.semaphore) {
            Ok(_) => {
                self.refill_queue
                    .write()
                    .push_back((now() + self.cfg.refill_delay, permit));
                Poll::Ready(())
            }
            Err(_) => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use futures_util::stream::iter;
    use futures_util::{FutureExt, StreamExt};
    use tokio_test::{assert_pending, assert_ready, assert_ready_eq, task};

    use crate::rate_limits::{
        RateLimitBucket, RateLimitBucketConfig, RateLimitExt, RateLimitable, RateLimiterConfig,
        SlowModeLimit,
    };
    use std::sync::Arc;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Limitable(Option<String>);
    impl RateLimitable for Limitable {
        fn channel_limits(&self) -> Option<&str> {
            self.0.as_ref().map(AsRef::as_ref)
        }
    }

    #[test]
    fn test_default_slowmode() {
        task::mock(|cx| {
            tokio_test::clock::mock(|handle| {
                let rate_limiter = Arc::new(RateLimiterConfig::default().into());
                let a = Limitable(Some("a".to_string()));
                let mut stream =
                    iter(vec![a.clone(), a.clone(), a.clone()]).rate_limited(10, &rate_limiter);
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
    fn test_change_limits() {
        task::mock(|cx| {
            tokio_test::clock::mock(|handle| {
                let rate_limiter = Arc::new(RateLimiterConfig::default().into());
                let a = Limitable(Some("a".to_string()));
                let mut stream =
                    iter(vec![a.clone(), a.clone(), a.clone()]).rate_limited(10, &rate_limiter);
                rate_limiter.set_slow_mode("a", SlowModeLimit::Channel(10));
                assert_ready!(stream.poll_next_unpin(cx));
                assert_pending!(stream.poll_next_unpin(cx));

                handle.advance(Duration::from_millis(1100));
                rate_limiter.set_slow_mode("a", SlowModeLimit::Channel(5));

                assert_ready_eq!(stream.poll_next_unpin(cx), Some(a));
            });
        });
    }

    #[test]
    fn test_custom_slowmode() {
        task::mock(|cx| {
            tokio_test::clock::mock(|handle| {
                let a = Limitable(Some("a".to_string()));
                let rate_limiter = Arc::new(RateLimiterConfig::default().into());
                let mut stream = iter(vec![a.clone(), a.clone()]).rate_limited(10, &rate_limiter);
                rate_limiter.set_slow_mode("a", SlowModeLimit::Channel(10));
                assert_ready!(stream.poll_next_unpin(cx));
                handle.advance(Duration::from_secs(5));
                assert_pending!(stream.poll_next_unpin(cx));
                handle.advance(Duration::from_secs(5));
                assert_ready_eq!(stream.poll_next_unpin(cx), Some(a));
                assert_ready_eq!(stream.poll_next_unpin(cx), None);
            });
        });
    }

    #[test]
    fn test_bucket1() {
        task::mock(|cx| {
            tokio_test::clock::mock(|handle| {
                let mut b: &RateLimitBucket =
                    &RateLimitBucketConfig::new(10, Duration::from_secs(10)).into();
                for _ in 0..20 {
                    assert_ready!(b.poll_unpin(cx));
                    handle.advance(Duration::from_secs(2));
                }
            });
        });
    }

    #[test]
    fn test_bucket2() {
        task::mock(|cx| {
            tokio_test::clock::mock(|handle| {
                let mut b: &RateLimitBucket =
                    &RateLimitBucketConfig::new(2, Duration::from_secs(10)).into();
                assert_ready!(b.poll_unpin(cx));

                handle.advance(Duration::from_secs(3));
                assert_ready!(b.poll_unpin(cx));
                assert_pending!(b.poll_unpin(cx));

                handle.advance(Duration::from_secs(7));
                assert_ready!(b.poll_unpin(cx));
                assert_pending!(b.poll_unpin(cx));

                handle.advance(Duration::from_secs(3));
                assert_ready!(b.poll_unpin(cx));
                assert_pending!(b.poll_unpin(cx));
            });
        });
    }
}
