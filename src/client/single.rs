use std::pin::Pin;
use std::sync::Arc;

use futures_sink::Sink;
use futures_util::future::FutureExt;
use futures_util::{pin_mut, select, SinkExt, StreamExt, TryStreamExt};
use tokio::pin;
use tokio::sync::{mpsc, oneshot, watch, RwLock};
use tokio::time::{delay_for, delay_until, Duration, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::client::{send_capabilities, MessageSender, TimeoutReceiver, TwitchClient};
use crate::client_messages::ClientMessage;
use crate::event::tags::*;
use crate::event::*;
use crate::event::{Event, TwitchChatStream};
use crate::irc_constants::RPL_ENDOFMOTD;
use crate::stream::rate_limits::RateLimiter;
use crate::stream::{ClientMessageStream, EventStream, SendStreamExt, SentClientMessage};
use crate::util::InternalSender;
use crate::{Error, TwitchClientConfig};

/// Connects to the Twitch servers, authenticates and listens for messages. Await the returned future
/// to block until the connection is closed.
pub async fn connect(
    cfg: &Arc<TwitchClientConfig>,
) -> Result<TwitchClient<impl EventStream>, Error> {
    let (event_sender, event_stream) = mpsc::channel(cfg.channel_buffer);

    let (sender, _) = connect_internal(
        cfg,
        Arc::new(RateLimiter::from(&cfg.rate_limiter)),
        InternalSender(event_sender),
    )
    .await?;
    Ok(TwitchClient {
        sender,
        stream: event_stream,
    })
}

pub(crate) async fn connect_internal(
    cfg: &Arc<TwitchClientConfig>,
    rate_limiter: Arc<RateLimiter>,
    event_sender: impl Sink<Result<Event, Error>> + Send + Sync + 'static,
) -> Result<(MessageSender, Arc<ConnectionContext>), Error> {
    let (connected_setter, connected_state) = watch::channel(ConnectedState::Disconnected);
    let state = Arc::new(ConnectionContext {
        connected_state,
        connected_setter,
        connecting_lock: RwLock::new(()),
        joined_channels: RwLock::new(vec![]),
        rate_limiter,
    });

    let (message_sender, message_stream) = mpsc::channel::<SentClientMessage>(cfg.channel_buffer);

    let mut message_stream =
        message_stream.rate_limited(cfg.channel_buffer, state.rate_limiter.clone());

    tokio::spawn({
        let cfg = cfg.clone();
        let mut message_sender = MessageSender::from(message_sender.clone());
        let context = state.clone();

        async move {
            pin!(event_sender);
            let mut reconnect_counter = 0_u32;
            loop {
                if reconnect_counter > 0 {
                    if reconnect_counter < cfg.max_reconnects {
                        info!(
                            "Reconnecting in {} seconds...",
                            cfg.reconnect_delay.as_secs()
                        );
                        delay_for(cfg.reconnect_delay).await;
                    } else {
                        error!("Maximum number of reconnect attempts reached, quitting.");
                        break;
                    }
                }
                reconnect_counter += 1;

                match inner_connect_task(
                    &context,
                    &cfg,
                    Pin::new(&mut event_sender),
                    &mut message_sender,
                    &mut message_stream,
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

    Ok((MessageSender::from(message_sender), state))
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

async fn inner_connect_task(
    context: &Arc<ConnectionContext>,
    cfg: &TwitchClientConfig,
    mut event_sender: Pin<&mut impl Sink<Result<Event, Error>>>,
    message_sender: &mut MessageSender,
    message_stream: &mut (impl ClientMessageStream + 'static),
) -> Result<DisconnectReason, Error> {
    let (connection_future, incoming_stream) = {
        context
            .connected_setter
            .broadcast(ConnectedState::Disconnected)
            .expect("set connecting state");
        let _connecting_guard = context.connecting_lock.write();

        debug!("Connecting to {}", cfg.url);
        // create the websocket connection
        let (ws, _) = match connect_async(cfg.url.clone()).await {
            Ok(conn) => conn,
            Err(_) => return Ok(DisconnectReason::ConnectFailed),
        };

        context
            .connected_setter
            .broadcast(ConnectedState::Established)
            .expect("set connecting state");

        // wrap with IRC/Twitch logic
        let (mut chat_sink, incoming_stream) = TwitchChatStream::new(ws).split::<Message>();

        let mut decorated_sender_stream = message_stream.map(|item| {
            match &item.message {
                ClientMessage::Join(channel) => {
                    let conn_ctx = context.clone();
                    let channel = channel.clone();
                    tokio::spawn(async move {
                        conn_ctx.joined_channels.write().await.push(channel);
                    });
                }
                ClientMessage::Part(channel) => {
                    let conn_ctx = context.clone();
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
                let _connecting_guard = context.connecting_lock.read().await;
                chat_sink.send(msg.message.into()).await?;
            }
            Ok::<(), Error>(())
        }
        .fuse();

        (connection_future, incoming_stream)
    };

    send_capabilities(cfg, message_sender).await?;

    for channel in context.joined_channels.read().await.clone() {
        message_sender.send(ClientMessage::Join(channel)).await?;
    }

    let (event_receiver, timeout_receiver) =
        decorate_receiver_with_internals(context, cfg, message_sender, incoming_stream);
    let mut event_receiver = event_receiver.fuse();

    pin_mut!(connection_future);

    #[inline]
    async fn handle_event(
        item: Option<Result<Event, Error>>,
        event_sender: &mut (impl Sink<Result<Event, Error>> + Unpin),
    ) -> Option<Result<DisconnectReason, Error>> {
        if let Some(item) = item {
            if let Err(Error::WebsocketError(tokio_tungstenite::tungstenite::Error::Io(io_err))) =
                item
            {
                warn!("IO error in websocket, reconnecting: {}", io_err);
                return Some(Ok(DisconnectReason::IoError));
            }

            if event_sender.send(item).await.is_err() {
                info!("Chat consumer dropped receiver stream, ending connection");
                return Some(Ok(DisconnectReason::Canceled));
            }
        } else {
            debug!("Connection closed normally");
            return Some(Ok(DisconnectReason::Closed));
        }
        None
    }

    if let Some(timeout_receiver) = timeout_receiver {
        let mut timeout_receiver = timeout_receiver.fuse();

        #[allow(clippy::unnecessary_mut_passed)]
        // prevent clippy warnings from inside select!
        loop {
            select! {
                item = event_receiver.next() => {
                    if let Some(handle_event_result) = handle_event(item, &mut event_sender).await {
                        return handle_event_result;
                    }
                },
                timeout = timeout_receiver => {
                    warn!("Twitch didn't respond to PING in time, closing connection.");
                    return Ok(DisconnectReason::Timeout);
                },
                msg_forward = connection_future => {}
            }
        }
    } else {
        #[allow(clippy::unnecessary_mut_passed)]
        // prevent clippy warnings from inside select!
        loop {
            select! {
                item = event_receiver.next() => {
                    if let Some(handle_event_result) = handle_event(item, &mut event_sender).await {
                        return handle_event_result;
                    }
                },
                msg_forward = connection_future => {}
            }
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
fn decorate_receiver_with_internals(
    conn_ctx: &Arc<ConnectionContext>,
    cfg: &TwitchClientConfig,
    sender: &MessageSender,
    chat_receiver: impl EventStream + 'static,
) -> (impl EventStream + 'static, Option<TimeoutReceiver>) {
    let (heartbeat_tx, timeout_rx) = if cfg.heartbeat {
        let (heartbeat_tx, heartbeat_rx) = watch::channel(Instant::now());
        let timeout_rx = spawn_heartbeat(sender, heartbeat_rx);
        (Some(heartbeat_tx), Some(timeout_rx))
    } else {
        (None, None)
    };

    let conn_ctx = conn_ctx.clone();

    let with_internals = chat_receiver.inspect_ok({
        let sender = sender.clone();
        move |event| match event {
            Event::Ping(_) => {
                let mut sender = sender.clone();
                tokio::spawn(async move {
                    if sender.send(ClientMessage::Pong).await.is_err() {
                        error!("Tried to respond to ping but the send channel was closed");
                    }
                });
            }
            Event::UserState(ref event) => {
                let is_mod = event
                    .badges()
                    .unwrap()
                    .into_iter()
                    .any(|badge| ["moderator", "broadcaster", "vip"].contains(&badge.badge));
                conn_ctx
                    .rate_limiter
                    .update_mod_status(event.channel(), is_mod);
            }
            Event::Pong(_) => {
                if let Some(ref heartbeat_tx) = heartbeat_tx {
                    if heartbeat_tx.broadcast(Instant::now()).is_err() {
                        error!("heartbeat channel closed!");
                    }
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

    (with_internals, timeout_rx)
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
