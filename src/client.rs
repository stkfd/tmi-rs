//! Client module, includes websocket connection handling, listener and handler registration

use std::pin::Pin;
use std::sync::Arc;

use derive_builder::Builder;
use futures_core::{FusedFuture, Stream};
use futures_util::future::FutureExt;
use futures_util::{select, StreamExt, TryStreamExt};
use smallvec::SmallVec;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::{delay_for, delay_until, Duration, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

use crate::client_messages::{Capability, ClientMessage};
use crate::event::tags::*;
use crate::event::*;
use crate::event::{Event, TwitchChatStream};
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

    /// Maximum number of retries to reconnect to Twitch
    #[builder(default = "20")]
    pub max_reconnects: u32,

    /// Buffer size
    #[builder(default = "20")]
    pub channel_buffer: usize,
}

impl TwitchClientConfig {
    fn get_capabilities(&self) -> SmallVec<[Capability; 3]> {
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

/// Represents a twitch chat client/connection. Call `connect` to establish a connection.
pub struct TwitchClient<St> {
    /// Channels that this client is supposed to be joining
    channels: Vec<String>,
    /// whether the client is currently (re-)connecting, to avoid losing queued messages during reconnections
    connecting_state: watch::Receiver<bool>,
    /// message sender for client messages
    sender: MessageSender,
    /// stream of chat events
    stream: St,
}

type TimeoutReceiver = oneshot::Receiver<()>;
type MessageSender = mpsc::Sender<ClientMessage<String>>;

const RECONNECT_DELAY: Duration = Duration::from_secs(5);

/// Connects to the Twitch servers, authenticates and listens for messages. Await the returned future
/// to block until the connection is closed.
pub async fn connect(
    cfg: &Arc<TwitchClientConfig>,
    rate_limiter: &Arc<RateLimiter>,
) -> Result<TwitchClient<impl EventStream>, Error> {
    let (sender, sender_stream) = mpsc::channel(cfg.channel_buffer);
    let (connecting_setter, mut connecting_receiver) = watch::channel::<bool>(true);
    let (mut out_sender, out_stream) = mpsc::channel(cfg.channel_buffer);

    let sender_stream = if let Some(ctor) = &cfg.send_middleware {
        ctor(sender_stream)
    } else {
        Box::pin(sender_stream)
    }
    .rate_limited(50, &rate_limiter);

    tokio::spawn({
        let rate_limiter = rate_limiter.clone();
        let cfg = cfg.clone();
        let sender = sender.clone();
        async move {
            let mut sender_stream = sender_stream;
            let mut should_reconnect = true;
            let mut reconnect_counter = 0_u32;

            while should_reconnect {
                if reconnect_counter > 0 {
                    if reconnect_counter < cfg.max_reconnects {
                        info!("Reconnecting in {} seconds...", RECONNECT_DELAY.as_secs());
                        delay_for(RECONNECT_DELAY).await;
                    } else {
                        error!("Maximum number of reconnect attempts reached, quitting.");
                        break;
                    }
                }
                reconnect_counter += 1;

                if connecting_setter.broadcast(true).is_err() {
                    break;
                }
                let (mut connection_future, incoming_stream) = if let Ok(result) =
                    connect_raw(&cfg, sender.clone(), &mut sender_stream).await
                {
                    if connecting_setter.broadcast(false).is_err() {
                        break;
                    }
                    result
                } else {
                    continue;
                };

                let (receiver, timeout_rx) =
                    build_receiver(&rate_limiter, &cfg, &sender, incoming_stream);
                let mut receiver = receiver.fuse();
                let mut timeout_rx_mut = &mut timeout_rx.fuse();

                #[allow(clippy::unnecessary_mut_passed)]
                // prevent clippy warnings from inside select!
                loop {
                    select! {
                        item = receiver.next() => {
                            if let Some(item) = item {
                                if let Err(Error::WebsocketError(tokio_tungstenite::tungstenite::Error::Io(io_err))) = item {
                                    warn!("IO error in websocket, reconnecting: {}", io_err);
                                    break;
                                }

                                if out_sender.send(item).await.is_err() {
                                    info!(
                                        "Chat consumer dropped receiver stream, ending connection"
                                    );
                                    should_reconnect = false;
                                    break;
                                }
                            } else {
                                debug!("Connection closed normally");
                                should_reconnect = false;
                                break;
                            }
                        },
                        timeout = timeout_rx_mut => {
                            warn!("Twitch didn't respond to PING in time, closing connection.");
                            break;
                        },
                        msg_forward = connection_future => {}
                    }
                }
            }

            Ok::<_, Error>(())
        }
    });

    while let Some(false) = connecting_receiver.next().await {}

    Ok(TwitchClient {
        channels: vec![],
        connecting_state: connecting_receiver,
        sender,
        stream: out_stream,
    })
}

fn build_receiver(
    rate_limiter: &Arc<RateLimiter>,
    cfg: &Arc<TwitchClientConfig>,
    sender: &MessageSender,
    chat_receiver: impl EventStream + 'static,
) -> (impl EventStream + 'static, TimeoutReceiver) {
    let (heartbeat_tx, heartbeat_rx) = tokio::sync::watch::channel(Instant::now());
    let rate_limiter = rate_limiter.clone();

    let with_internals = chat_receiver
        .inspect_ok({
            let sender = sender.clone();
            move |event| {
                if let Event::Ping(_) = event {
                    let mut sender = sender.clone();
                    tokio::spawn(async move {
                        if sender.send(ClientMessage::<String>::Pong).await.is_err() {
                            error!("Tried to respond to ping but the send channel was closed");
                        }
                    });
                }
            }
        })
        .inspect_ok({
            move |event| {
                if let Event::UserState(ref event) = event {
                    let badges = event.badges().unwrap();
                    let is_mod = badges
                        .into_iter()
                        .any(|badge| ["moderator", "broadcaster", "vip"].contains(&badge.badge));
                    rate_limiter.update_mod_status(event.channel(), is_mod);
                }
            }
        })
        .inspect_ok(move |event| {
            if let Event::Pong(_) = event {
                if heartbeat_tx.broadcast(Instant::now()).is_err() {
                    error!("heartbeat channel closed!");
                }
            }
        });

    // apply receiver middleware
    let with_middleware: Pin<Box<dyn EventStream>> = if let Some(ctor) = &cfg.recv_middleware {
        ctor(Box::new(with_internals))
    } else {
        Box::pin(with_internals)
    };

    let timeout_rx = spawn_heartbeat(sender, heartbeat_rx);

    (with_middleware, timeout_rx)
}

fn spawn_heartbeat(
    sender: &MessageSender,
    heartbeat_rx: watch::Receiver<Instant>,
) -> TimeoutReceiver {
    const HEARTBEAT_DURATION: Duration = Duration::from_secs(20);
    let (timeout_tx, timeout_rx) = oneshot::channel();
    let mut sender = sender.clone();

    tokio::spawn(async move {
        loop {
            sender.send(ClientMessage::<String>::Ping).await?;
            let sent_at = Instant::now();
            delay_until(sent_at + HEARTBEAT_DURATION).await;
            if *heartbeat_rx.borrow() < sent_at {
                error!(
                    "Connection timed out, waited {} seconds for PONG",
                    HEARTBEAT_DURATION.as_secs()
                );
                if timeout_tx.send(()).is_err() {
                    warn!("Timeout signal was not processed, the connection will silently fail.")
                }
                break;
            }
        }
        Ok::<_, Error>(())
    });
    timeout_rx
}

/// Connects to the Twitch servers, authenticates and listens for messages. Await the returned future
/// to block until the connection is closed.
pub async fn connect_raw<'a>(
    cfg: &Arc<TwitchClientConfig>,
    mut sender: MessageSender,
    sender_stream: &'a mut (impl Stream<Item = ClientMessage<String>> + Unpin),
) -> Result<
    (
        impl FusedFuture<Output = ()> + Unpin + 'a,
        impl EventStream + 'static,
    ),
    Error,
> {
    debug!("Connecting to {}", cfg.url);
    // create the websocket connection
    let (ws, _) = connect_async(cfg.url.clone()).await?;

    // wrap with IRC/Twitch logic
    let (chat_sink, chat_receiver) = TwitchChatStream::new(ws).split::<Message>();

    let task = sender_stream
        .map(|item| Ok(item.into()))
        .forward(chat_sink)
        .map(|_| ());

    // send capability requests on connect
    let capabilities = cfg.get_capabilities();
    sender
        .send(ClientMessage::<String>::CapRequest(capabilities))
        .await?;
    for msg in ClientMessage::login(cfg.username.clone(), cfg.token.clone()).into_iter() {
        sender.send(msg).await?
    }

    Ok((task, chat_receiver))
}

impl<St> TwitchClient<St> {
    /// Get a mutable reference to the message send channel
    pub fn sender_mut(&mut self) -> &mut MessageSender {
        &mut self.sender
    }

    /// Get an owned message sender
    pub fn sender_cloned(&self) -> MessageSender {
        self.sender.clone()
    }

    /// Get a mutable reference to the stream of chat events
    pub fn stream_mut(&mut self) -> &mut St {
        &mut self.stream
    }
}

/*async fn backoff_delay(retry_count: u64) {
    delay_for(Duration::from_secs_f64(
        min(2 << retry_count, 60) as f64 + rand::thread_rng().gen::<f64>(),
    ))
    .await;
}*/
