//! Client module, includes websocket connection handling, listener and handler registration

use std::pin::Pin;
use std::sync::Arc;

use derive_builder::Builder;
use futures_core::Stream;
use futures_util::{SinkExt, StreamExt};
use smallvec::SmallVec;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

use crate::client_messages::{Capability, ClientMessage};
use crate::event::{Event, TwitchChatStream};
use crate::stream::internals::InternalStreamExt;
use crate::stream::rate_limits::{RateLimiter, RateLimiterConfig};
use crate::stream::{
    EventStream, RecvMiddlewareConstructor, SendMiddlewareConstructor, SendStreamExt,
};
use crate::Error;

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
}

/// Represents a twitch chat client/connection. Call `connect` to establish a connection.
pub struct TwitchClient {
    cfg: Arc<TwitchClientConfig>,
    rate_limiter: Arc<RateLimiter>,
}

impl TwitchClient {
    /// Initialize a Twitch chat client
    pub fn new(cfg: &Arc<TwitchClientConfig>, rate_limiter: &Arc<RateLimiter>) -> TwitchClient {
        TwitchClient {
            cfg: cfg.clone(),
            rate_limiter: rate_limiter.clone(),
        }
    }

    /// Connects to the Twitch servers, authenticates and listens for messages. Await the returned future
    /// to block until the connection is closed.
    pub async fn connect(
        &self,
    ) -> Result<
        (
            mpsc::UnboundedSender<ClientMessage<String>>,
            impl Stream<Item = Result<Event<String>, Error>>,
        ),
        Error,
    > {
        debug!("Connecting to {}", self.cfg.url);

        // create the websocket connection
        let (ws, _) = connect_async(self.cfg.url.clone()).await?;

        // wrap with IRC/Twitch logic
        let (mut chat_sink, chat_recv) = TwitchChatStream::new(ws).split::<Message>();
        let (sender, sender_stream) = mpsc::unbounded_channel();

        let chat_recv = chat_recv
            .handle_pings(sender.clone())
            .auto_rate_limit(self.rate_limiter.clone());

        let mut sender_stream = if let Some(ctor) = &self.cfg.send_middleware {
            ctor(sender_stream)
        } else {
            Box::pin(sender_stream)
        }
        .rate_limited(200, &self.rate_limiter)
        .map(|msg| -> Message { msg.into() });

        // apply receiver middleware
        let chat_recv: Pin<Box<dyn EventStream>> = if let Some(ctor) = &self.cfg.recv_middleware {
            ctor(Box::new(chat_recv))
        } else {
            Box::pin(chat_recv)
        };

        tokio::spawn(async move {
            while let Some(message) = sender_stream.next().await {
                chat_sink.send(message).await?;
            }
            Ok::<(), Error>(())
        });

        // send capability requests on connect
        let capabilities = self.get_capabilities();
        sender
            .send(ClientMessage::<String>::CapRequest(capabilities))
            .map_err(|_| Error::MessageChannelError)?;
        for msg in
            ClientMessage::login(self.cfg.username.clone(), self.cfg.token.clone()).into_iter()
        {
            sender.send(msg).map_err(|_| Error::MessageChannelError)?
        }

        Ok((sender, chat_recv))
    }

    /// Connects to the Twitch servers, authenticates and listens for messages. Await the returned future
    /// to block until the connection is closed.
    /*pub async fn connect(&self) -> Result<TwitchChatConnection, Error> {
        debug!("Connecting to {}", self.cfg.url);

        // create the underlying websocket connection
        let (ws, _) = connect_async(self.cfg.url.clone()).await.map_err(|e| {
            error!("Connection to {} failed", self.cfg.url);
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
        let recv_middleware_ctor = self.cfg.recv_middleware.clone();
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
        let send_middleware_ctor = self.cfg.send_middleware.clone();
        tokio::spawn(async move {
            // apply send middleware
            let mapped_client_recv = if let Some(ctor) = send_middleware_ctor {
                ctor(client_recv)
            } else {
                Box::pin(client_recv)
            };

            let messages = mapped_client_recv
                .rate_limited(200, &rate_limiter)
                .map(|msg| -> Message { msg.into() });
            messages.map(Ok).forward(&mut ws_sink).await.unwrap();
        });

        // do any internal message handling like rate limit detection and ping pong
        let internal_receiver = message_receiver.try_clone().expect("Get internal receiver");
        let internal_sender = sender.clone();
        let rate_limiter = self.rate_limiter.clone();
        tokio::spawn(async move {
            let responses = internal_receiver.filter_map(|e: SharedEvent| {
                ready(match *e {
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
        let capabilities = self.get_capabilities();
        (&sender)
            .send(ClientMessage::<String>::CapRequest(capabilities))
            .await?;
        (&sender)
            .send_all(
                &mut ClientMessage::login(self.cfg.username.clone(), self.cfg.token.clone())
                    .map(Ok),
            )
            .await?;

        Ok(TwitchChatConnection {
            receiver: message_receiver,
            error_receiver,
            sender: sender.clone(),
        })
    }*/

    /// Get the rate limiter used for this client, can be used to change the rate limiting configuration
    /// at runtime.
    pub fn rate_limiter(&self) -> &Arc<RateLimiter> {
        &self.rate_limiter
    }

    fn get_capabilities(&self) -> SmallVec<[Capability; 3]> {
        let mut capabilities = SmallVec::new();
        if self.cfg.cap_commands {
            capabilities.push(Capability::Commands)
        }
        if self.cfg.cap_tags {
            capabilities.push(Capability::Tags)
        }
        if self.cfg.cap_membership {
            capabilities.push(Capability::Membership)
        }
        capabilities
    }
}
