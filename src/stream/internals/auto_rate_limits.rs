use std::pin::Pin;
use std::sync::Arc;

use futures_core::task::{Context, Poll};
use futures_core::Stream;

use crate::event::tags::*;
use crate::event::*;
use crate::stream::rate_limits::RateLimiter;
use crate::stream::EventStream;
use crate::Error;

pub struct AutoRateLimits<S> {
    rate_limiter: Arc<RateLimiter>,
    inner_stream: S,
}

impl<S> AutoRateLimits<S> {
    pub(crate) fn new(inner_stream: S, rate_limiter: Arc<RateLimiter>) -> Self {
        AutoRateLimits {
            rate_limiter,
            inner_stream,
        }
    }
}

impl<S> Stream for AutoRateLimits<S>
where
    S: EventStream,
{
    type Item = Result<Event<String>, Error>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let poll = Pin::new(&mut (*self).inner_stream).poll_next(cx);
        if let Poll::Ready(Some(item)) = &poll {
            if let Ok(event) = &item {
                if let Event::UserState(ref event) = event {
                    let badges = event.badges().unwrap();
                    let is_mod = badges
                        .into_iter()
                        .any(|badge| ["moderator", "broadcaster", "vip"].contains(&badge.badge));
                    self.rate_limiter.update_mod_status(event.channel(), is_mod);
                }
            };
        }
        poll
    }
}
