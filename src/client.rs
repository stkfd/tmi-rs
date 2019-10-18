use std::sync::Arc;
use std::time::Duration;

use broadcaster::BroadcastChannel;
use futures::{Sink, StreamExt};
use futures_util::future::{Either, poll_fn, select};
use tokio_timer::Interval;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::{ClientConfig, Error, TwitchChatSender};
use crate::data::client_messages::{Capability, ClientMessage};
use crate::events::Event;
use crate::irc::IrcMessage;
use std::convert::TryFrom;
use std::borrow::Cow;

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
    pub async fn connect(&self) -> Result<(TwitchChatSender<impl Sink<Message> + Clone>, BroadcastChannel<Arc<Event<String>>>), Error> {
        debug!("Connecting to {}", self.config.url);
        let (mut ws_stream, _) = connect_async(self.config.url.clone())
            .await
            .map_err(|e| {
                error!("Connection to {} failed", self.config.url);
                Error::WebsocketError { details: Cow::from("Failed to connect to the chat server"), source: e }
            })?;

        // MPSC channel that is used by the consumer program to send messages
        let (channel_sender, mut channel_receiver) = futures::channel::mpsc::unbounded();
        let mut sender = TwitchChatSender::new(channel_sender);

        let mut heartbeat = Interval::new_interval(Duration::from_secs(5));

        // Broadcast
        let evt_channel = BroadcastChannel::new();
        let evt_sender = evt_channel.clone();

        tokio_executor::spawn(async move {
            loop {
                let either1 = select(
                    poll_fn(|cx| heartbeat.poll_next(cx)),
                    select(
                        ws_stream.next(),
                        channel_receiver.next(),
                    ),
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
                                        Message::Text(msg) => {
                                            debug!("< {}", msg);
                                            match IrcMessage::<&str>::parse_many(&msg) {
                                                Ok((_remaining, messages)) => {
                                                    for irc_msg in messages {
                                                        let owned_event: Event<String> = (&Event::try_from(irc_msg).unwrap()).into();
                                                        evt_sender.send(&Arc::new(owned_event)).await.unwrap();
                                                    }
                                                }
                                                Err(err) => {
                                                    error!("IRC parse error: {:?}", err);
                                                }
                                            }
                                        }
                                        Message::Binary(msg) => info!("< Binary<{} bytes>", msg.len()),
                                        Message::Close(close_frame) => {
                                            if let Some(close_frame) = close_frame { info!("Received close frame: {}", close_frame) }
                                            info!("Connection closed by the server.");
                                            evt_sender.send(&Arc::new(Event::close())).await.unwrap();
                                            break;
                                        }
                                        Message::Ping(_payload) => debug!("< WS PING"),
                                        Message::Pong(_payload) => debug!("< WS PONG"),
                                    },
                                    Some(Err(e)) => {
                                        error!("{}", e);
                                        break;
                                    }
                                    None => break
                                }
                            }
                            Either::Right((next_out, _)) => {
                                if let Some(next_out) = next_out {
                                    debug!("> {}", next_out);
                                    ws_stream.send(next_out).await.unwrap();
                                }
                            }
                        }
                    }
                }
            }
            evt_sender.send(&Arc::new(Event::close())).await.unwrap();
        });

        let mut capabilities = Vec::new();
        if self.config.cap_commands { capabilities.push(Capability::Commands) }
        if self.config.cap_tags { capabilities.push(Capability::Tags) }
        if self.config.cap_membership { capabilities.push(Capability::Membership) }
        sender.send(ClientMessage::CapRequest(&capabilities)).await?;
        sender.login(&self.config.username, &self.config.token).await?;

        Ok((sender, evt_channel))
    }
}

/*async fn match_channel_msg<T>(item: &Event<T>) -> bool {
    match item.event() {
        EventContent::PrivMsg(channel_msg) => true,
        _ => false
    }
}*/
