//! Client module, includes websocket connection handling, listener and handler registration

use std::borrow::Cow;
use std::sync::Arc;

use future_bus::BusSubscriber;
use futures_channel::mpsc;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

use crate::client_messages::{Capability, ClientMessage};
use crate::event::{Event, TwitchChatStream};
use crate::{Error, TwitchChatSender};
use crate::rate_limits::{RateLimiter, RateLimiterConfig, RateLimitExt};

/// Holds the configuration for a twitch chat client. Convert it to a `TwitchClient` and call
/// `connect` to establish a connection using it.
#[derive(Debug, Clone, Builder)]
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

    /// Rate limiting configuration
    pub rate_limiter: RateLimiterConfig,
}

/// Represents a twitch chat client/connection. Call `connect` to establish a connection.
#[derive(Debug)]
pub struct TwitchClient {
    url: Url,
    username: String,
    token: String,
    cap_membership: bool,
    cap_commands: bool,
    cap_tags: bool,
    rate_limiter: Arc<RateLimiter>,
}

impl From<TwitchClientConfig> for TwitchClient {
    fn from(cfg: TwitchClientConfig) -> Self {
        TwitchClient {
            url: cfg.url,
            username: cfg.username,
            token: cfg.token,
            cap_membership: cfg.cap_membership,
            cap_commands: cfg.cap_commands,
            cap_tags: cfg.cap_tags,
            rate_limiter: Arc::new(cfg.rate_limiter.into())
        }
    }
}

type ChatReceiver = BusSubscriber<
    Arc<Result<Event<String>, Error>>,
    mpsc::Sender<Arc<Result<Event<String>, Error>>>,
    mpsc::Receiver<Arc<Result<Event<String>, Error>>>,
>;

impl TwitchClient {
    /// Connects to the Twitch servers, authenticates and listens for messages. Await the returned future
    /// to block until the connection is closed.
    pub async fn connect(
        &self,
    ) -> Result<(TwitchChatSender<mpsc::Sender<ClientMessage<String>>>, ChatReceiver), Error> {
        debug!("Connecting to {}", self.url);
        let (ws, _) = connect_async(self.url.clone()).await.map_err(|e| {
            error!("Connection to {} failed", self.url);
            Error::WebsocketError {
                details: Cow::from("Failed to connect to the chat server"),
                source: e,
            }
        })?;

        let (mut ws_sink, mut ws_recv) = TwitchChatStream::new(ws).split::<Message>();
        let mut event_bus = ::future_bus::bounded(100);
        let event_receiver = event_bus.subscribe();
        let (client_sender, client_recv) = mpsc::channel::<ClientMessage<String>>(100);
        let mut sender = TwitchChatSender::new(client_sender);

        tokio_executor::spawn(async move {
            event_bus.send_all(&mut ws_recv).await.unwrap();
        });

        let rate_limiter = self.rate_limiter.clone();
        tokio_executor::spawn(async move {
            let mut messages = client_recv
                .rate_limited(200, &rate_limiter)
                .map(|msg| {
                    debug!("msg: {}", msg.to_string());
                    Message::from(msg.to_string())
                });
            ws_sink.send_all(&mut messages).await.unwrap();
        });

        // do any internal message handling like rate limit detection and ping pong
        let internal_receiver = event_receiver.try_clone().expect("Get internal receiver");
        let internal_sender = sender.clone();
        tokio_executor::spawn(async {
            let mut responses =
                internal_receiver.filter_map(|e: Arc<Result<Event<String>, Error>>| {
                    futures_util::future::ready(match *e {
                        Ok(Event::Ping(_)) => Some(ClientMessage::<String>::Pong),
                        _ => None,
                    })
                });

            let mut snd = internal_sender;
            snd.send_all(&mut responses).await.unwrap();
        });

        let mut capabilities = Vec::with_capacity(3);
        if self.cap_commands {
            capabilities.push(Capability::Commands)
        }
        if self.cap_tags {
            capabilities.push(Capability::Tags)
        }
        if self.cap_membership {
            capabilities.push(Capability::Membership)
        }
        sender
            .send(ClientMessage::<String>::CapRequest(capabilities))
            .await?;
        sender
            .login(self.username.clone(), self.token.clone())
            .await?;

        Ok((sender, event_receiver))
    }

    /// Get the rate limiter used for this client, can be used to change the rate limiting configuration
    /// at runtime.
    pub fn rate_limiter(&self) -> &Arc<RateLimiter> {
        &self.rate_limiter
    }
}
