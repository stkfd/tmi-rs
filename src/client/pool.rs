use std::sync::{Arc, Weak};

use fnv::FnvHashMap;
use futures_core::Stream;
use tokio::stream;
use tokio::sync::broadcast::RecvError;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{interval_at, Duration, Instant};

use crate::client::single::{connect_internal, ConnectionContext};
use crate::client::MessageSender;
use crate::event::Event;
use crate::stream::rate_limits::RateLimiter;
use crate::stream::{MessageResponder, RespondWithErrors, SentClientMessage};
use crate::util::InternalSender;
use crate::EventChannelError;
use crate::{ClientMessage, Error, TwitchClientConfig};
use crate::{MessageResponse, MessageSendError};

/// Create a connection pool
pub async fn connect(
    cfg: &Arc<TwitchClientConfig>,
    pool_cfg: PoolConfig,
) -> Result<ConnectionPoolHandle, Error> {
    let (message_sender, mut message_receiver) =
        mpsc::channel::<SentClientMessage>(cfg.channel_buffer);
    let rate_limiter = Arc::new(RateLimiter::from(&cfg.rate_limiter));
    let (event_sender, event_receiver) = broadcast::channel(cfg.channel_buffer);

    let mut default_connections = vec![];

    // create whisper connection
    let whisper_connection = new_connection(&ConnectionConfig {
        cfg: &cfg,
        rate_limiter: &rate_limiter,
        event_sender: &event_sender,
        handle_whispers: true,
    })
    .await?;
    default_connections.push(Arc::new(whisper_connection));

    // create remaining "default" connections
    for _ in 0..(pool_cfg.init_connections - 1) {
        let conn = new_connection(&ConnectionConfig {
            cfg: &cfg,
            rate_limiter: &rate_limiter,
            event_sender: &event_sender,
            handle_whispers: false,
        })
        .await?;
        default_connections.push(Arc::new(conn));
    }

    let pool = ConnectionPool {
        whisper_connection: default_connections[0].clone(),
        channel_connections_map: Default::default(),
        event_sender: event_sender.clone(),
        event_receiver,
        connections: RwLock::new(default_connections),
    };

    {
        // capture variables for spawned task
        let cfg = cfg.clone();
        let event_sender = event_sender.clone();
        tokio::spawn(async move {
            let mut pool = pool;

            let connection_cfg = ConnectionConfig {
                cfg: &cfg,
                rate_limiter: &rate_limiter,
                event_sender: &event_sender,
                handle_whispers: false,
            };

            let mut connection_cleanup_interval = interval_at(
                Instant::now() + Duration::from_secs(30),
                Duration::from_secs(30),
            );

            loop {
                tokio::select! {
                    next_msg = message_receiver.recv() => {
                        if let Some(SentClientMessage {
                            message: client_message,
                            responder,
                        }) = next_msg {
                            // process messages sent to the server and route them to their appropriate connections
                            send_message(
                                client_message,
                                &mut pool,
                                &pool_cfg,
                                responder,
                                &connection_cfg,
                            ).await;
                        } else {
                            break;
                        }
                    },
                    _ = connection_cleanup_interval.tick() => {
                        pool.close_stale_connections().await;
                    }
                }
            }
        });
    }

    let pool_handle = ConnectionPoolHandle {
        event_sender,
        message_sender: MessageSender::from(message_sender),
    };

    Ok(pool_handle)
}

/// Route a message to the proper connection and actually send it
async fn send_message(
    client_message: ClientMessage,
    pool: &mut ConnectionPool,
    pool_cfg: &PoolConfig,
    responder: MessageResponder,
    connection_cfg: &ConnectionConfig<'_>,
) {
    use futures_util::stream::StreamExt;

    match &client_message {
        ClientMessage::Whisper { .. } => {
            pool.whisper_connection
                .send(client_message)
                .await
                .respond_with_errors(responder);
        }
        ClientMessage::Ping | ClientMessage::Pong => {
            pool.whisper_connection
                .send(client_message)
                .await
                .respond_with_errors(responder);
        }
        ClientMessage::PrivMsg { channel, .. } | ClientMessage::Part(channel) => {
            if let Some(handle) = pool.get_channel_connection(channel) {
                handle
                    .send(client_message)
                    .await
                    .respond_with_errors(responder);
            } else {
                responder
                    .send(Err(MessageSendError::ChannelNotJoined(client_message)))
                    .ok();
            }
        }
        ClientMessage::Join(channel) => {
            // already joined this channel
            if let Some(connection) = pool.get_channel_connection(channel) {
                connection
                    .send(client_message)
                    .await
                    .respond_with_errors(responder);
            } else {
                // get connection with the lowest amount of joined channels
                let connections = pool.connections.read().await.clone();
                let handle = stream::iter(&connections)
                    .filter_map(|handle| {
                        let threshold = pool_cfg.threshold;
                        async move {
                            let count = handle.context.joined_channels.read().await.len();
                            if count <= threshold as usize {
                                Some((handle, count))
                            } else {
                                None
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .await
                    .into_iter()
                    .min_by_key(|(_handle, joined_count)| *joined_count)
                    .map(|(handle, _)| handle);

                if let Some(channel_handle) = handle {
                    debug!("Joining channel on existing connection.");
                    pool.channel_connections_map
                        .insert(channel.clone(), Arc::downgrade(channel_handle));
                    channel_handle
                        .send(client_message)
                        .await
                        .respond_with_errors(responder);
                } else {
                    debug!("Adding new connection to the pool.");
                    let conn_result = new_connection(connection_cfg)
                        .await
                        .map_err(|e| MessageSendError::NewConnectionFailed(format!("{}", e)));
                    match conn_result {
                        Ok(conn) => {
                            let channel = channel.clone();
                            conn.send(client_message)
                                .await
                                .respond_with_errors(responder);
                            let arc = Arc::new(conn);
                            let weak = Arc::downgrade(&arc);
                            pool.connections.write().await.push(arc);
                            pool.channel_connections_map.insert(channel.clone(), weak);
                        }
                        Err(error) => {
                            responder.send(Err(error)).ok();
                        }
                    }
                }
            }
        }
        ClientMessage::Nick(_) => {
            responder
                .send(Err(MessageSendError::UnsupportedMessage(
                    "NICK is sent automatically in managed connection pools.",
                )))
                .ok();
        }
        ClientMessage::Pass(_) => {
            responder
                .send(Err(MessageSendError::UnsupportedMessage(
                    "PASS is sent automatically in managed connection pools.",
                )))
                .ok();
        }
        ClientMessage::CapRequest(_) => {
            responder
                .send(Err(MessageSendError::UnsupportedMessage(
                    "CAP REQs are sent automatically in managed connection pools.",
                )))
                .ok();
        }
        ClientMessage::Close => {
            for connection in pool.connections.write().await.drain(..) {
                if let Err(e) = connection.send(ClientMessage::Close).await {
                    responder.send(Err(e)).ok();
                    return;
                }
            }
            responder.send(Ok(MessageResponse::Ok)).ok();
        }
    }
}

struct ConnectionConfig<'a> {
    cfg: &'a Arc<TwitchClientConfig>,
    rate_limiter: &'a Arc<RateLimiter>,
    event_sender: &'a broadcast::Sender<Result<Event, Error>>,
    handle_whispers: bool,
}

async fn new_connection(connection_cfg: &ConnectionConfig<'_>) -> Result<ConnectionHandle, Error> {
    let (sender, context) = connect_internal(
        connection_cfg.cfg,
        connection_cfg.rate_limiter.clone(),
        InternalSender(connection_cfg.event_sender.clone()),
        connection_cfg.handle_whispers,
    )
    .await?;

    Ok(ConnectionHandle { sender, context })
}

/// Connection pool settings
#[derive(Clone, Debug)]
pub struct PoolConfig {
    /// Number of initially created connections
    pub init_connections: u32,
    /// Maximum number of connections
    pub connection_limit: u32,
    /// When all connections reach this number of joined channels, a new connection
    /// will be created
    pub threshold: u32,
}

struct ConnectionHandle {
    sender: MessageSender,
    context: Arc<ConnectionContext>,
}

impl ConnectionHandle {
    async fn send(&self, msg: ClientMessage) -> Result<MessageResponse, MessageSendError> {
        self.sender.clone().send(msg).await
    }

    async fn close(&self) {
        self.sender.clone().send(ClientMessage::Close).await.ok();
    }

    /// Returns true when a connection is no longer required in a pool because it doesn't handle
    /// whispers and has no joined channels.
    pub async fn is_stale(&self) -> bool {
        !self.context.whisper_enabled && self.context.joined_channels.read().await.len() == 0
    }
}

/// Handle to a connection pool
#[derive(Clone, Debug)]
pub struct ConnectionPoolHandle {
    event_sender: broadcast::Sender<Result<Event, Error>>,
    message_sender: MessageSender,
}

impl ConnectionPoolHandle {
    /// Subscribe to a receiver for messages
    pub fn subscribe_events(&self) -> impl Stream<Item = Result<Event, Error>> {
        use tokio::stream::StreamExt;
        self.event_sender.subscribe().map(|result| match result {
            Ok(event) => event,
            Err(recv_err) => Err(match recv_err {
                RecvError::Closed => EventChannelError::Closed,
                RecvError::Lagged(_) => EventChannelError::Overflow,
            }
            .into()),
        })
    }

    /// Get an owned sender for messages
    pub fn clone_sender(&self) -> MessageSender {
        self.message_sender.clone()
    }

    /// Get a reference to a message sender
    pub fn sender(&self) -> &MessageSender {
        &self.message_sender
    }
}

struct ConnectionPool {
    /// sender for the event broadcast channel
    event_sender: broadcast::Sender<Result<Event<String>, Error>>,
    /// receiver for the event broadcast channel
    event_receiver: broadcast::Receiver<Result<Event<String>, Error>>,
    /// default connections as specified in `init_connections`
    connections: RwLock<Vec<Arc<ConnectionHandle>>>,
    /// connection for whispers
    whisper_connection: Arc<ConnectionHandle>,
    /// weak connection handles for individual channels
    channel_connections_map: FnvHashMap<String, Weak<ConnectionHandle>>,
}

impl ConnectionPool {
    fn get_channel_connection(&self, channel: &str) -> Option<Arc<ConnectionHandle>> {
        self.channel_connections_map
            .get(channel)
            .and_then(|weak| weak.upgrade())
    }

    async fn close_stale_connections(&self) {
        let mut lock = self.connections.write().await;
        let mut live_connections = Vec::new();
        let mut stale_connections = Vec::new();
        for connection in lock.iter().cloned() {
            if connection.is_stale().await {
                stale_connections.push(connection);
            } else {
                live_connections.push(connection);
            }
        }

        debug!("Closing {} stale connections", stale_connections.len());
        tokio::spawn(async move {
            for stale_connection in stale_connections {
                stale_connection.close().await
            }
        });

        std::mem::swap(lock.as_mut(), &mut live_connections);
    }
}
