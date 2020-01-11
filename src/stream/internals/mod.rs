use crate::stream::internals::auto_rate_limits::AutoRateLimits;
use crate::stream::internals::ping::PingHandler;
use crate::stream::rate_limits::RateLimiter;
use crate::stream::EventStream;
use crate::ClientMessage;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

pub(crate) mod auto_rate_limits;
pub(crate) mod ping;

pub(crate) trait InternalStreamExt: EventStream {
    fn handle_pings(self, sender: UnboundedSender<ClientMessage<String>>) -> PingHandler<Self>
    where
        Self: Sized,
    {
        PingHandler::new(self, sender)
    }

    fn auto_rate_limit(self, rate_limiter: Arc<RateLimiter>) -> AutoRateLimits<Self>
    where
        Self: Sized,
    {
        AutoRateLimits::new(self, rate_limiter)
    }
}
impl<T: EventStream> InternalStreamExt for T {}
