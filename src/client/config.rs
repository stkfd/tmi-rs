use derive_builder::Builder;
use smallvec::SmallVec;

use crate::stream::rate_limits::RateLimiterConfig;
use crate::stream::{RecvMiddlewareConstructor, SendMiddlewareConstructor};
use crate::Capability;
use url::Url;

/// Holds the configuration for a twitch chat client. Convert it to a `TwitchClient` and call
/// `connect` to establish a connection using it.
#[derive(Clone, Builder)]
pub struct TwitchClientConfig {
    /// The chat server, by default `wss://irc-ws.chat.twitch.tv:443`
    #[builder(default = r#"Url::parse("wss://irc-ws.chat.twitch.tv:443").unwrap()"#)]
    pub url: Url,

    /// Twitch username to use
    pub username: String,

    /// OAuth token to use
    pub token: String,

    /// Whether to enable membership capability (default: false)
    #[builder(default = "false")]
    pub cap_membership: bool,

    /// Whether to enable commands capability (default: true)
    #[builder(default = "true")]
    pub cap_commands: bool,

    /// Whether to enable tags capability (default: true)
    #[builder(default = "true")]
    pub cap_tags: bool,

    /// Receiver middlewares
    #[builder(default = "None", setter(strip_option))]
    pub recv_middleware: Option<RecvMiddlewareConstructor>,

    /// Send middlewares
    #[builder(default = "None", setter(strip_option))]
    pub send_middleware: Option<SendMiddlewareConstructor>,

    /// Rate limiting configuration
    #[builder(default = "RateLimiterConfig::default()")]
    pub rate_limiter: RateLimiterConfig,

    /// Maximum number of retries to reconnect to Twitch
    #[builder(default = "20")]
    pub max_reconnects: u32,

    /// Buffer size
    #[builder(default = "20")]
    pub channel_buffer: usize,
}

impl TwitchClientConfig {
    pub(crate) fn get_capabilities(&self) -> SmallVec<[Capability; 3]> {
        let mut capabilities = SmallVec::new();
        if self.cap_commands {
            capabilities.push(Capability::Commands)
        }
        if self.cap_tags {
            capabilities.push(Capability::Tags)
        }
        if self.cap_membership {
            capabilities.push(Capability::Membership)
        }
        capabilities
    }
}
