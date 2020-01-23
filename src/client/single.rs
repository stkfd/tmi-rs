use std::pin::Pin;
use std::sync::Arc;

use futures_util::future::FutureExt;
use futures_util::{pin_mut, select, SinkExt, StreamExt, TryStreamExt};
use tokio::sync::{mpsc, oneshot, watch, RwLock};
use tokio::time::{delay_for, delay_until, Duration, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::client_messages::ClientMessage;
use crate::event::tags::*;
use crate::event::*;
use crate::event::{Event, TwitchChatStream};
use crate::irc_constants::RPL_ENDOFMOTD;
use crate::stream::rate_limits::RateLimiter;
use crate::stream::{ClientMessageStream, EventStream, SendStreamExt};
use crate::{Error, TwitchClientConfig};

use super::RECONNECT_DELAY;

/// Represents a twitch chat client/connection. Call `connect` to establish a connection.
pub struct TwitchClient<St> {
    /// message sender for client messages
    sender: MessageSender,
    /// stream of chat events
    stream: St,
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

type TimeoutReceiver = oneshot::Receiver<()>;
type MessageSender = mpsc::Sender<ClientMessage<String>>;

/// Connects to the Twitch servers, authenticates and listens for messages. Await the returned future
/// to block until the connection is closed.
pub async fn connect(
    cfg: &Arc<TwitchClientConfig>,
) -> Result<TwitchClient<impl EventStream>, Error> {
    let (connected_setter, connected_state) = watch::channel(ConnectedState::Disconnected);
    let state = Arc::new(ConnectionContext {
        connected_state,
        connected_setter,
        connecting_lock: RwLock::new(()),
        joined_channels: RwLock::new(vec![]),
        rate_limiter: Arc::new(RateLimiter::from(&cfg.rate_limiter)),
    });

    let (sender, sender_stream) = mpsc::channel(cfg.channel_buffer);
    let (mut out_sender, out_stream) = mpsc::channel(cfg.channel_buffer);

    let sender_stream = if let Some(ctor) = &cfg.send_middleware {
        ctor(sender_stream)
    } else {
        Box::pin(sender_stream)
    }
    .rate_limited(50, state.rate_limiter.clone());

    tokio::spawn({
        let cfg = cfg.clone();
        let mut sender = sender.clone();
        let state = state.clone();
        async move {
            let mut sender_stream = sender_stream;
            let mut reconnect_counter = 0_u32;

            loop {
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

                match run(
                    &state,
                    &cfg,
                    &mut out_sender,
                    &mut sender,
                    &mut sender_stream,
                )
                .await?
                {
                    DisconnectReason::Closed => break,
                    DisconnectReason::Canceled
                    | DisconnectReason::Timeout
                    | DisconnectReason::IoError
                    | DisconnectReason::ConnectFailed => {}
                }
            }

            Ok::<_, Error>(())
        }
    });

    let mut connected_state = state.connected_state.clone();
    while connected_state.next().await != Some(ConnectedState::Active) {}

    Ok(TwitchClient {
        sender,
        stream: out_stream,
    })
}

enum DisconnectReason {
    Closed,
    Canceled,
    Timeout,
    IoError,
    ConnectFailed,
}

/// Everything stateful relating to a chat connection, usually passed around in an `Arc`
pub struct ConnectionContext {
    /// whether the client is currently (re-)connecting, to avoid losing queued messages during reconnections.
    /// Locked as read access for sending messages, as write access while reconnecting
    pub connecting_lock: RwLock<()>,
    /// Channels that this client is supposed to be joining on reconnect
    pub joined_channels: RwLock<Vec<String>>,
    /// Rate limiter used for the connection. Since this can be shared between multiple connections,
    /// it is wrapped in an Arc
    pub rate_limiter: Arc<RateLimiter>,
    /// whether the connection is currently active
    pub connected_state: watch::Receiver<ConnectedState>,
    connected_setter: watch::Sender<ConnectedState>,
}

/// State of the connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectedState {
    /// Not connected
    Disconnected,
    /// WebSocket connection established, but no IRC welcome message yet
    Established,
    /// Fully active connection
    Active,
}

async fn run(
    state: &Arc<ConnectionContext>,
    cfg: &TwitchClientConfig,
    out_sender: &mut mpsc::Sender<Result<Event<String>, Error>>,
    sender: &mut MessageSender,
    sender_stream: &mut (impl ClientMessageStream + 'static),
) -> Result<DisconnectReason, Error> {
    let (connection_future, incoming_stream) = {
        state
            .connected_setter
            .broadcast(ConnectedState::Disconnected)
            .expect("set connecting state");
        let _connecting_guard = state.connecting_lock.write();

        debug!("Connecting to {}", cfg.url);
        // create the websocket connection
        let (ws, _) = match connect_async(cfg.url.clone()).await {
            Ok(conn) => conn,
            Err(_) => return Ok(DisconnectReason::ConnectFailed),
        };

        state
            .connected_setter
            .broadcast(ConnectedState::Established)
            .expect("set connecting state");

        // wrap with IRC/Twitch logic
        let (mut chat_sink, incoming_stream) = TwitchChatStream::new(ws).split::<Message>();

        let mut decorated_sender_stream = sender_stream.map(|item| {
            match &item {
                ClientMessage::Join(channel) => {
                    let conn_ctx = state.clone();
                    let channel = channel.clone();
                    tokio::spawn(async move {
                        conn_ctx.joined_channels.write().await.push(channel);
                    });
                }
                ClientMessage::Part(channel) => {
                    let conn_ctx = state.clone();
                    let channel = channel.clone();
                    tokio::spawn(async move {
                        conn_ctx
                            .joined_channels
                            .write()
                            .await
                            .retain(|ch| ch != &channel);
                    });
                }
                _ => {}
            }
            item
        });
        let connection_future = async move {
            while let Some(msg) = decorated_sender_stream.next().await {
                let _connecting_guard = state.connecting_lock.read().await;
                dbg!(&msg);
                chat_sink.send(msg.into()).await?;
            }
            Ok::<(), Error>(())
        }
        .fuse();

        (connection_future, incoming_stream)
    };

    // send capability requests on connect
    let capabilities = cfg.get_capabilities();
    sender
        .send(ClientMessage::<String>::CapRequest(capabilities))
        .await?;
    for msg in ClientMessage::login(cfg.username.clone(), cfg.token.clone()).into_iter() {
        sender.send(msg).await?
    }

    for channel in state.joined_channels.read().await.clone() {
        sender.send(ClientMessage::Join(channel)).await?;
    }

    let (receiver, timeout_rx) = decorate_receiver(state, cfg, sender, incoming_stream);
    let mut receiver = receiver.fuse();
    let mut timeout_rx_mut = &mut timeout_rx.fuse();

    pin_mut!(connection_future);

    #[allow(clippy::unnecessary_mut_passed)]
    // prevent clippy warnings from inside select!
    loop {
        select! {
            item = receiver.next() => {
                if let Some(item) = item {
                    if let Err(Error::WebsocketError(tokio_tungstenite::tungstenite::Error::Io(io_err))) = item {
                        warn!("IO error in websocket, reconnecting: {}", io_err);
                        return Ok(DisconnectReason::IoError);
                    }

                    if out_sender.send(item).await.is_err() {
                        info!(
                            "Chat consumer dropped receiver stream, ending connection"
                        );
                        return Ok(DisconnectReason::Canceled);
                    }
                } else {
                    debug!("Connection closed normally");
                    return Ok(DisconnectReason::Closed);
                }
            },
            timeout = timeout_rx_mut => {
                warn!("Twitch didn't respond to PING in time, closing connection.");
                return Ok(DisconnectReason::Timeout);
            },
            msg_forward = connection_future => {}
        }
    }
}

/// Wraps the chat receiver with additional commonly needed logic. Currently includes these features:
///
/// * Configure a provided rate limiter for each joined channel (e.g. set VIP, Mod, Broadcaster status)
/// * Responds to PING messages from the server with PONG
/// * Sends a heartbeat PONG signal and returns a channel that is notified when the server does not
///   respond
/// * Keeps track of joined channels
fn decorate_receiver(
    conn_ctx: &Arc<ConnectionContext>,
    cfg: &TwitchClientConfig,
    sender: &MessageSender,
    chat_receiver: impl EventStream + 'static,
) -> (impl EventStream + 'static, TimeoutReceiver) {
    let (heartbeat_tx, heartbeat_rx) = tokio::sync::watch::channel(Instant::now());
    let conn_ctx = conn_ctx.clone();

    let with_internals = chat_receiver.inspect_ok({
        let sender = sender.clone();
        move |event| match event {
            Event::Ping(_) => {
                let mut sender = sender.clone();
                tokio::spawn(async move {
                    if sender.send(ClientMessage::<String>::Pong).await.is_err() {
                        error!("Tried to respond to ping but the send channel was closed");
                    }
                });
            }
            Event::UserState(ref event) => {
                let badges = event.badges().unwrap();
                let is_mod = badges
                    .into_iter()
                    .any(|badge| ["moderator", "broadcaster", "vip"].contains(&badge.badge));
                conn_ctx
                    .rate_limiter
                    .update_mod_status(event.channel(), is_mod);
            }
            Event::Pong(_) => {
                if heartbeat_tx.broadcast(Instant::now()).is_err() {
                    error!("heartbeat channel closed!");
                }
            }
            Event::ConnectMessage(msg) if msg.command() == RPL_ENDOFMOTD => {
                conn_ctx
                    .connected_setter
                    .broadcast(ConnectedState::Active)
                    .ok();
            }
            _ => {}
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
                timeout_tx.send(()).ok(); // if this fails it just means a reconnect happened in the meantime
                break;
            }
        }
        Ok::<_, Error>(())
    });
    timeout_rx
}
