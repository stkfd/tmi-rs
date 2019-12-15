//! Client module, includes websocket connection handling, listener and handler registration

use std::borrow::Cow;
use std::sync::Arc;

use derive_builder::Builder;
use future_bus::BusSubscriber;
use futures_channel::mpsc;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

use crate::client_messages::{Capability, ClientMessage};
use crate::event::tags::BadgeTags;
use crate::event::{ChannelEventData, Event, SharedEvent, TwitchChatStream};
use crate::stream::rate_limits::{RateLimiter, RateLimiterConfig};
use crate::stream::{RecvMiddlewareConstructor, SendMiddlewareConstructor, SendStreamExt};
use crate::{Error, TwitchChatSender};

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
    #[builder(default = "None")]
    pub recv_middleware: Option<RecvMiddlewareConstructor>,

    /// Send middlewares
    #[builder(default = "None")]
    pub send_middleware: Option<SendMiddlewareConstructor>,

    /// Rate limiting configuration
    #[builder(default = "RateLimiterConfig::default()")]
    pub rate_limiter: RateLimiterConfig,
}

/// Represents a twitch chat client/connection. Call `connect` to establish a connection.
pub struct TwitchClient {
    url: Url,
    username: String,
    token: String,
    cap_membership: bool,
    cap_commands: bool,
    cap_tags: bool,
    rate_limiter: Arc<RateLimiter>,
    recv_middleware: Option<RecvMiddlewareConstructor>,
    send_middleware: Option<SendMiddlewareConstructor>,
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
            rate_limiter: Arc::new(cfg.rate_limiter.into()),
            recv_middleware: cfg.recv_middleware,
            send_middleware: cfg.send_middleware,
        }
    }
}

impl TwitchClient {
    /// Connects to the Twitch servers, authenticates and listens for messages. Await the returned future
    /// to block until the connection is closed.
    pub async fn connect(&self) -> Result<TwitchChatConnection, Error> {
        debug!("Connecting to {}", self.url);

        // create the underlying websocket connection
        let (ws, _) = connect_async(self.url.clone()).await.map_err(|e| {
            error!("Connection to {} failed", self.url);
            Error::WebsocketError {
                details: Cow::from("Failed to connect to the chat server"),
                source: e,
            }
        })?;

        // wrap websocket to convert websocket messages into and from Twitch events
        let (mut ws_sink, ws_recv) = TwitchChatStream::new(ws).split::<Message>();

        let mut message_bus = ::future_bus::bounded::<SharedEvent>(200);
        let mut error_bus = ::future_bus::bounded::<Arc<Error>>(200);
        let message_receiver = message_bus.subscribe();
        let error_receiver = error_bus.subscribe();
        let (client_sender, client_recv) = mpsc::unbounded::<ClientMessage<String>>();
        let sender = TwitchChatSender::new(client_sender);

        // receive messages from websocket and forward to broadcast channel
        let recv_middleware_ctor = self.recv_middleware.clone();
        tokio::spawn(async move {
            // apply receive middleware
            let mut mapped_ws_recv = if let Some(ctor) = recv_middleware_ctor {
                ctor(Box::new(ws_recv))
            } else {
                Box::pin(ws_recv)
            };

            while let Some(result) = mapped_ws_recv.next().await {
                let send_result = match result {
                    Ok(event) => message_bus.send(Arc::new(event)).await,
                    Err(err) => error_bus.send(Arc::new(err)).await,
                };
                if let Err(err) = send_result {
                    error!("Internal channel error. Couldn't pass message. {}", err);
                }
            }
        });

        // apply rate limiting to sent messages and forward them to the websocket sink
        let rate_limiter = self.rate_limiter.clone();
        let send_middleware_ctor = self.send_middleware.clone();
        tokio::spawn(async move {
            // apply send middleware
            let mapped_client_recv = if let Some(ctor) = send_middleware_ctor {
                ctor(client_recv)
            } else {
                Box::pin(client_recv)
            };

            let messages = mapped_client_recv
                .rate_limited(200, &rate_limiter)
                .map(|msg| Message::from(msg.to_string()));
            messages.map(Ok).forward(&mut ws_sink).await.unwrap();
        });

        // do any internal message handling like rate limit detection and ping pong
        let internal_receiver = message_receiver.try_clone().expect("Get internal receiver");
        let internal_sender = sender.clone();
        let rate_limiter = self.rate_limiter.clone();
        tokio::spawn(async move {
            let responses = internal_receiver.filter_map(|e: SharedEvent| {
                futures_util::future::ready(match *e {
                    Event::Ping(_) => Some(ClientMessage::<String>::Pong),
                    Event::UserState(ref event) => {
                        let badges = event.badges().unwrap();
                        let is_mod = badges.into_iter().any(|badge| {
                            ["moderator", "broadcaster", "vip"].contains(&badge.badge)
                        });
                        rate_limiter.update_mod_status(event.channel(), is_mod);
                        None
                    }
                    _ => None,
                })
            });

            responses.map(Ok).forward(&internal_sender).await.unwrap();
        });

        // send capability requests on connect
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
        (&sender)
            .send(ClientMessage::<String>::CapRequest(capabilities))
            .await?;
        (&sender)
            .send_all(&mut ClientMessage::login(self.username.clone(), self.token.clone()).map(Ok))
            .await?;

        Ok(TwitchChatConnection {
            receiver: message_receiver,
            error_receiver,
            sender: sender.clone(),
        })
    }

    /// Get the rate limiter used for this client, can be used to change the rate limiting configuration
    /// at runtime.
    pub fn rate_limiter(&self) -> &Arc<RateLimiter> {
        &self.rate_limiter
    }
}

/// Receiver channel for all chat events
pub type ChatReceiver =
    BusSubscriber<SharedEvent, mpsc::Sender<SharedEvent>, mpsc::Receiver<SharedEvent>>;

/// Receiver channel for connection and parsing errors
pub type ErrorReceiver =
    BusSubscriber<Arc<Error>, mpsc::Sender<Arc<Error>>, mpsc::Receiver<Arc<Error>>>;

/// Sender to push messages to Twitch chat
pub type ChatSender = TwitchChatSender<mpsc::UnboundedSender<ClientMessage<String>>>;

/// Contains the Streams and Sinks associated with an underlying websocket connection. They
/// can be cloned freely to be shared across different tasks and threads.
pub struct TwitchChatConnection {
    /// Receiver for chat messages
    pub receiver: ChatReceiver,
    /// Receiver for connection errors
    pub error_receiver: ErrorReceiver,
    /// Sender for commands/messages
    pub sender: ChatSender,
}
