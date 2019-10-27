use std::borrow::Borrow;

use futures::{Sink, SinkExt, Stream};
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::tungstenite::Message;

use crate::client_messages::{ClientMessage, Command};
use crate::Error;
use crate::event::TwitchChatStream;

pub struct TwitchChatConnection<Ws> {
    inner_stream: TwitchChatStream<Ws>
}

impl<Ws> TwitchChatConnection<Ws>
    where
        Ws: Stream<Item=Result<Message, WsError>> + Unpin + Sink<Message>
{
    /// Create a new connection using the supplied websocket stream
    pub fn new(ws: Ws) -> TwitchChatConnection<Ws>
    {
        TwitchChatConnection {
            inner_stream: TwitchChatStream::new(ws)
        }
    }

    /// Get an immutable reference to the stream of chat events
    pub fn stream(&self) -> &TwitchChatStream<Ws> {
        &self.inner_stream
    }

    /// Get a mutable reference to the stream of chat events
    pub fn stream_mut(&mut self) -> &mut TwitchChatStream<Ws> {
        &mut self.inner_stream
    }

    /// Send a message to twitch chat
    pub async fn send(&mut self, message: &ClientMessage<&str>) -> Result<(), Error> {
        info!("{}", message);
        self.inner_stream.send(message)
            .await
            .map_err(|_err| Error::SendError)
    }

    /// Send multiple messages from an iterator to the chat. Preferable over repeatedly calling
    /// [`send`](self::TwitchChatConnection::send) if an iterator with multiple messages is available.
    pub async fn send_all<'a>(&mut self, messages: &mut (impl Stream<Item=&'a ClientMessage<&'a str>> + Unpin)) -> Result<(), Error>
    {
        //messages.for_each(async move |msg| info!("{}", msg)).await;
        self.inner_stream.send_all(messages)
            .await
            .map_err(|_err| Error::SendError)
    }

    /// Joins a twitch channel
    pub async fn join(&mut self, channel: &str) -> Result<(), Error> {
        self.send(&ClientMessage::Join(channel)).await
    }

    /// Authenticates the user. Caution: this is normally called automatically when calling
    /// [`TwitchClient::connect`](crate::TwitchClient::connect), only use it if this stream was created in some other
    /// way.
    pub async fn login(&mut self, username: &str, token: &str) -> Result<(), Error> {
        self.send(&ClientMessage::Pass(token)).await?;
        self.send(&ClientMessage::Nick(username)).await
    }

    /// Permanently ban a user
    pub async fn ban(&mut self, channel: &str, username: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: &Command::Ban(username).to_string().borrow(),
        }).await
    }

    /// Unban a user
    pub async fn unban(&mut self, channel: &str, username: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::Unban(username).to_string().borrow(),
        }).await
    }

    /// Clear a chat channel
    pub async fn clear(&mut self, channel: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::<&str>::Clear.to_string().borrow(),
        }).await
    }

    pub async fn color(&mut self, color: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: "jtv",
            message: Command::Color(color).to_string().borrow(),
        }).await
    }

    /// Run a commercial
    pub async fn commercial(&mut self, channel: &str, time: Option<usize>) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::<&str>::Commercial { time }.to_string().borrow(),
        }).await
    }

    /// Delete a specific message by its message ID tag.
    pub async fn delete(&mut self, channel: &str, msg_id: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::Delete { msg_id }.to_string().borrow(),
        }).await
    }

    /// Disconnect from chat
    pub async fn disconnect(&mut self) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: "jtv",
            message: Command::<&str>::Disconnect.to_string().borrow(),
        }).await
    }

    /// Set emote only mode on or off
    pub async fn emote_only(&mut self, channel: &str, status: bool) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::<&str>::EmoteOnly(status).to_string().borrow(),
        }).await
    }

    /// Set followers only mode on or off
    pub async fn followers_only(&mut self, channel: &str, status: bool) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::<&str>::FollowersOnly(status).to_string().borrow(),
        }).await
    }

    /// Host a channel
    pub async fn host(&mut self, channel: &str, host_channel: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::Host(host_channel).to_string().borrow(),
        }).await
    }

    /// Stop hosting
    pub async fn unhost(&mut self, channel: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::<&str>::Unhost.to_string().borrow(),
        }).await
    }

    /// Set a stream marker
    pub async fn marker(&mut self, channel: &str, description: Option<&str>) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::Marker(description).to_string().borrow(),
        }).await
    }

    /// Send a /me message
    pub async fn me(&mut self, message: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: "jtv",
            message: Command::Me(message).to_string().borrow(),
        }).await
    }

    /// Mod a user
    pub async fn make_mod(&mut self, channel: &str, user: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::Mod(user).to_string().borrow(),
        }).await
    }

    /// Unmod a user
    pub async fn unmod(&mut self, channel: &str, user: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::Unmod(user).to_string().borrow(),
        }).await
    }

    /// Enable or disable r9k mode
    pub async fn r9k(&mut self, channel: &str, on: bool) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::<&str>::R9k(on).to_string().borrow(),
        }).await
    }

    /// Start a raid
    pub async fn raid(&mut self, channel: &str, raid_channel: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::Raid(raid_channel).to_string().borrow(),
        }).await
    }

    /// Stop a raid
    pub async fn unraid(&mut self, channel: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::<&str>::Unraid.to_string().borrow(),
        }).await
    }

    /// Enable slow mode with the given amount of seconds
    pub async fn slow(&mut self, channel: &str, seconds: usize) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::<&str>::Slow(seconds).to_string().borrow(),
        }).await
    }

    /// Disable slow mode
    pub async fn slow_off(&mut self, channel: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::<&str>::SlowOff.to_string().borrow(),
        }).await
    }

    /// Enable or disable sub mode
    pub async fn subscribers(&mut self, channel: &str, on: bool) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::<&str>::SubscribersOnly(on).to_string().borrow(),
        }).await
    }

    /// Time a user out for the given amount of seconds or the default timeout if None is given
    pub async fn timeout(&mut self, channel: &str, user: &str, seconds: Option<usize>) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::Timeout {
                time: seconds,
                user,
            }.to_string().borrow(),
        }).await
    }

    /// Make a user VIP in a channel
    pub async fn vip(&mut self, channel: &str, user: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::Vip(user).to_string().borrow(),
        }).await
    }

    /// Remove VIP status from a user
    pub async fn unvip(&mut self, channel: &str, user: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::Vip(user).to_string().borrow(),
        }).await
    }

    /// List the VIPs in a channel
    pub async fn vips(&mut self, channel: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel,
            message: Command::<&str>::Vips.to_string().borrow(),
        }).await
    }

    /// Send a whisper
    pub async fn whisper(&mut self, user: &str, message: &str) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: "jtv",
            message: Command::Whisper { user, message }.to_string().borrow(),
        }).await
    }
}
