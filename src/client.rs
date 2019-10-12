use crate::{ClientConfig, TwitchChatSender, Error, ErrorKind};
use crate::events::Event;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use std::time::Duration;
use tokio_timer::Interval;
use futures::StreamExt;
use futures::channel::mpsc::{UnboundedSender, UnboundedReceiver};
use futures_util::future::{Either, select, poll_fn};

#[derive(Debug)]
pub struct TwitchClient {
    config: ClientConfig,
    //event_groups: FnvHashMap<HandlerGroupId, EventHandlerGroup>,
    //next_group_id: usize,
}

impl TwitchClient {
    pub fn new(config: ClientConfig) -> TwitchClient {
        TwitchClient {
            config,
            //event_groups: FnvHashMap::default(),
            //next_group_id: 1,
        }
    }

    /*pub fn register_matcher(&mut self, matcher: Box<dyn EventMatcher>) -> HandlerGroupId {
        let group_id = HandlerGroupId(self.next_group_id);
        self.next_group_id += 1;
        self.event_groups.insert(group_id, EventHandlerGroup { matcher, handlers: vec![] });
        group_id
    }

    pub fn register_handler(&mut self, group_id: HandlerGroupId, handler: Box<dyn EventHandler>) {
        if let Some(group) = self.event_groups.get_mut(&group_id) {
            group.handlers.push(handler)
        };
    }*/

    /// Connects to the Twitch servers and listens in a separate thread. Await the returned future
    /// to block until the connection is closed.
    pub async fn connect(&self) -> Result<(TwitchChatSender<UnboundedSender<Message>>, futures::channel::oneshot::Receiver<()>), Error> {
        debug!("Connecting to {}", self.config.url);
        let (mut ws_stream, _) = connect_async(self.config.url.clone())
            .await
            .map_err(|e| {
                error!("Connection to {} failed", self.config.url);
                Error::new(ErrorKind::WebsocketError(e), "Failed to connect to the chat server.")
            })?;

        let (channel_sender, mut channel_receiver): (UnboundedSender<Message>, UnboundedReceiver<Message>) = futures::channel::mpsc::unbounded();

        let mut sender = TwitchChatSender::new(
            channel_sender,
        );

        sender.login(&self.config.username, &self.config.token).await?;

        let mut heartbeat = Interval::new_interval(Duration::from_secs(3));

        let (finish_send, finish_recv) = futures::channel::oneshot::channel();
        tokio_executor::spawn(async move {
            loop {
                let either1 = select(
                    poll_fn(|cx| heartbeat.poll_next(cx)),
                    select(
                        ws_stream.next(),
                        channel_receiver.next()
                    )
                ).await;
                match either1 {
                    Either::Left((_heartbeat, _)) => {
                        trace!("Heartbeat")
                    }
                    Either::Right((either2, _)) => {
                        match either2 {
                            Either::Left((next_in, _)) => {
                                match next_in {
                                    Some(Ok(next_in)) => match next_in {
                                        Message::Text(msg) => info!("< {}", msg),
                                        Message::Binary(msg) => info!("< Binary<{} bytes>", msg.len()),
                                        Message::Close(close_frame) => {
                                            if let Some(close_frame) = close_frame { info!("Received close frame: {}", close_frame) }
                                            info!("Connection closed from the remote side.");
                                            break;
                                        },
                                        Message::Ping(_payload) => debug!("< PING"),
                                        Message::Pong(_payload) => debug!("< PONG"),
                                    },
                                    Some(Err(e)) => {
                                        error!("{}", e);
                                        break;
                                    },
                                    None => break
                                }
                            }
                            Either::Right((next_out, _)) => {
                                if let Some(next_out) = next_out {
                                    info!("> {}", next_out);
                                    ws_stream.send(next_out).await.unwrap();
                                }
                            }
                        }
                    }
                }
            }
            finish_send.send(()).unwrap();
        });

        Ok((sender, finish_recv))
    }
}

pub struct EventStreams {
    events: futures::channel::mpsc::UnboundedReceiver<Event<String>>,
    closed: futures::channel::oneshot::Receiver<()>,
}

/*#[async_trait]
pub trait EventHandler: fmt::Debug + Sync {
    async fn run(&self);
}

pub trait EventMatcher: fmt::Debug + Sync {
    fn match_event(&self, e: Event);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HandlerGroupId(usize);

#[derive(Debug)]
struct EventHandlerGroup {
    matcher: Box<dyn EventMatcher>,
    handlers: Vec<Box<dyn EventHandler>>,
}*/

/*pub trait MatcherBuilder {
    fn match_events(&mut self, matcher: Box<dyn EventMatcher>) -> (HandlerGroupId, &mut TwitchClient);
}

impl MatcherBuilder for TwitchClient {
    fn match_events(&mut self, matcher: Box<dyn EventMatcher>) -> (HandlerGroupId, &mut TwitchClient) {
        (self.register_matcher(matcher), self)
    }
}

impl MatcherBuilder for (HandlerGroupId, &mut TwitchClient) {
    fn match_events(&mut self, matcher: Box<dyn EventMatcher>) -> (HandlerGroupId, &mut TwitchClient) {
        (self.1.register_matcher(matcher), self.1)
    }
}

pub trait HandlerBuilder {
    fn handle_with(&mut self, handler: Box<dyn EventHandler>) -> (HandlerGroupId, &mut TwitchClient);
}

impl HandlerBuilder for (HandlerGroupId, &mut TwitchClient) {
    fn handle_with(&mut self, handler: Box<dyn EventHandler>) -> (HandlerGroupId, &mut TwitchClient) {
        self.1.register_handler(self.0, handler);
        (self.0, self.1)
    }
}
*/