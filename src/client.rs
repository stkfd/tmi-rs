use crate::{ClientConfig, TwitchChatSender, Error, ErrorKind};
use crate::events::Event;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use std::time::Duration;
use tokio_timer::Interval;
use futures::{StreamExt, SinkExt, Stream};
use futures_util::future::{Either, select, poll_fn};
use std::sync::Arc;
use futures::channel::mpsc;
use futures::channel::mpsc::UnboundedSender;
use crate::irc::IrcMessage;
use crate::data::client_messages::{ClientMessage, Capability};
use std::convert::TryFrom;

#[derive(Debug)]
pub struct TwitchClient {
    config: ClientConfig,
}

impl TwitchClient {
    pub fn new(config: ClientConfig) -> TwitchClient {
        TwitchClient {
            config,
        }
    }

    /// Connects to the Twitch servers and listens in a separate thread. Await the returned future
    /// to block until the connection is closed.
    pub async fn connect(&self) -> Result<(TwitchChatSender<mpsc::UnboundedSender<Message>>, impl Stream<Item=Arc<Event<String>>>), Error> {
        debug!("Connecting to {}", self.config.url);
        let (mut ws_stream, _) = connect_async(self.config.url.clone())
            .await
            .map_err(|e| {
                error!("Connection to {} failed", self.config.url);
                Error::with_cause(ErrorKind::WebsocketError, "Failed to connect to the chat server.", Box::new(e))
            })?;

        let (channel_sender, mut channel_receiver) = futures::channel::mpsc::unbounded();

        let mut sender = TwitchChatSender::new(
            channel_sender,
        );

        let mut heartbeat = Interval::new_interval(Duration::from_secs(5));

        let (mut event_sink, event_stream): (UnboundedSender<Arc<Event<String>>>, _) = futures::channel::mpsc::unbounded();

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
                        debug!("Heartbeat")
                    }
                    Either::Right((either2, _)) => {
                        match either2 {
                            Either::Left((next_in, _)) => {
                                match next_in {
                                    Some(Ok(next_in)) => match next_in {
                                        Message::Text(msg) => {
                                            match IrcMessage::<&str>::parse_many(&msg) {
                                                Ok((_remaining, messages)) => {
                                                    for irc_msg in messages {
                                                        debug!("{:?}", irc_msg);
                                                        let owned_event: Event<String> = Event::try_from(irc_msg).unwrap().into();
                                                        event_sink.send(Arc::new(owned_event)).await.unwrap();
                                                    }
                                                },
                                                Err(err) => {
                                                    error!("IRC parse error: {:?}", err);
                                                }
                                            }
                                        },
                                        Message::Binary(msg) => info!("< Binary<{} bytes>", msg.len()),
                                        Message::Close(close_frame) => {
                                            if let Some(close_frame) = close_frame { info!("Received close frame: {}", close_frame) }
                                            info!("Connection closed by the server.");
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
            event_sink.send(Arc::new(Event::close())).await.unwrap();
        });

        let mut capabilities = Vec::new();
        if self.config.cap_commands { capabilities.push(Capability::Commands) }
        if self.config.cap_tags { capabilities.push(Capability::Tags) }
        if self.config.cap_membership { capabilities.push(Capability::Membership) }
        sender.send(ClientMessage::CapRequest(&capabilities)).await?;
        sender.login(&self.config.username, &self.config.token).await?;

        Ok((sender, event_stream))
    }
}

/*async fn match_channel_msg<T>(item: &Event<T>) -> bool {
    match item.event() {
        EventContent::PrivMsg(channel_msg) => true,
        _ => false
    }
}*/
