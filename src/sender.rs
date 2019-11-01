//! Message/command sender helpers

use std::borrow::{Borrow, Cow};
use std::pin::Pin;

use futures_core::task::Context;
use futures_core::Poll;
use futures_sink::Sink;
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::client_messages::{ClientMessage, Command};
use crate::rate_limits::RateLimitable;
use crate::Error;

/// A wrapper around any `Sink<ClientMessage<Cow<'static, str>>>` that provides methods to send Twitch
/// specific messages or commands to the sink.
#[derive(Clone)]
pub struct TwitchChatSender<Si: Clone + Unpin> {
    sink: Si,
}

impl<Si: Unpin + Clone> Unpin for TwitchChatSender<Si> {}

impl<Si, M> Sink<M> for TwitchChatSender<Si>
where
    Si: Sink<WsMessage> + Unpin + Clone,
    M: Into<WsMessage> + RateLimitable,
{
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut Pin::into_inner(self).sink)
            .poll_ready(cx)
            .map_err(|_| Error::SendError)
    }

    fn start_send(self: Pin<&mut Self>, item: M) -> Result<(), Self::Error> {
        let msg: WsMessage = item.into();
        debug!("> {}", msg);
        Pin::new(&mut Pin::into_inner(self).sink)
            .start_send(msg)
            .map_err(|_| Error::SendError)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut Pin::into_inner(self).sink)
            .poll_flush(cx)
            .map_err(|_| Error::SendError)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut Pin::into_inner(self).sink)
            .poll_close(cx)
            .map_err(|_| Error::SendError)
    }
}

impl<Si> TwitchChatSender<Si>
where
    Si: Sink<WsMessage> + Unpin + Clone,
{
    /// Create a new chat sender using an underlying `Sink<ClientMessage<Cow<'static, str>>>`. This
    pub fn new(sink: Si) -> Self {
        TwitchChatSender { sink }
    }

    /// Send a whisper
    pub async fn message<
        S1: Into<Cow<'static, str>> + Borrow<str>,
        S2: Into<Cow<'static, str>> + Borrow<str>,
    >(
        &mut self,
        channel: S1,
        message: S2,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: message.into(),
        })
        .await
    }

    /// Joins a twitch channel
    pub async fn join<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::Join(channel.into())).await
    }

    /// Authenticates the user. Caution: this is normally called automatically when calling
    /// [`TwitchClient::connect`](crate::TwitchClient::connect), only use it if this stream was created in some other
    /// way.
    pub async fn login<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        username: S,
        token: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::Pass(token.into())).await?;
        self.send(&ClientMessage::Nick(username.into())).await
    }

    /// Permanently ban a user
    pub async fn ban<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        username: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Ban(username.borrow()).to_string().into(),
        })
        .await
    }

    /// Unban a user
    pub async fn unban<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        username: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Unban(username.borrow()).to_string().into(),
        })
        .await
    }

    /// Clear a chat channel
    pub async fn clear<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::Clear.to_string().into(),
        })
        .await
    }

    /// Set the user name color
    pub async fn color<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        color: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: Cow::from("jtv"),
            message: Command::Color(color.borrow()).to_string().into(),
        })
        .await
    }

    /// Run a commercial
    pub async fn commercial<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        time: Option<usize>,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::Commercial { time }.to_string().into(),
        })
        .await
    }

    /// Delete a specific message by its message ID tag.
    pub async fn delete<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        msg_id: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Delete {
                msg_id: msg_id.borrow(),
            }
            .to_string()
            .into(),
        })
        .await
    }

    /// Disconnect from chat
    pub async fn disconnect<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: Cow::from("jtv"),
            message: Command::<&str>::Disconnect.to_string().into(),
        })
        .await
    }

    /// Set emote only mode on or off
    pub async fn emote_only<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        status: bool,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::EmoteOnly(status).to_string().into(),
        })
        .await
    }

    /// Set followers only mode on or off
    pub async fn followers_only<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        status: bool,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::FollowersOnly(status).to_string().into(),
        })
        .await
    }

    /// Host a channel
    pub async fn host<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        host_channel: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Host(host_channel.borrow()).to_string().into(),
        })
        .await
    }

    /// Stop hosting
    pub async fn unhost<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::Unhost.to_string().into(),
        })
        .await
    }

    /// Set a stream marker
    pub async fn marker<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        description: Option<&str>,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Marker(description).to_string().into(),
        })
        .await
    }

    /// Send a /me message
    pub async fn me<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        message: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: Cow::from("jtv"),
            message: Command::Me(message.borrow()).to_string().into(),
        })
        .await
    }

    /// Mod a user
    pub async fn make_mod<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        user: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Mod(user.borrow()).to_string().into(),
        })
        .await
    }

    /// Unmod a user
    pub async fn unmod<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        user: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Unmod(user.borrow()).to_string().into(),
        })
        .await
    }

    /// Enable or disable r9k mode
    pub async fn r9k<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        on: bool,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::R9k(on).to_string().into(),
        })
        .await
    }

    /// Start a raid
    pub async fn raid<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        raid_channel: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Raid(raid_channel.borrow()).to_string().into(),
        })
        .await
    }

    /// Stop a raid
    pub async fn unraid<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::Unraid.to_string().into(),
        })
        .await
    }

    /// Enable slow mode with the given amount of seconds
    pub async fn slow<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        seconds: usize,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::Slow(seconds).to_string().into(),
        })
        .await
    }

    /// Disable slow mode
    pub async fn slow_off<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::SlowOff.to_string().into(),
        })
        .await
    }

    /// Enable or disable sub mode
    pub async fn subscribers<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        on: bool,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::SubscribersOnly(on).to_string().into(),
        })
        .await
    }

    /// Time a user out for the given amount of seconds or the default timeout if None is given
    pub async fn timeout<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        user: S,
        seconds: Option<usize>,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Timeout {
                time: seconds,
                user: user.borrow(),
            }
            .to_string()
            .into(),
        })
        .await
    }

    /// Make a user VIP in a channel
    pub async fn vip<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        user: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Vip(user.borrow()).to_string().into(),
        })
        .await
    }

    /// Remove VIP status from a user
    pub async fn unvip<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
        user: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::Vip(user.borrow()).to_string().into(),
        })
        .await
    }

    /// List the VIPs in a channel
    pub async fn vips<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        channel: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: channel.into(),
            message: Command::<&str>::Vips.to_string().into(),
        })
        .await
    }

    /// Send a whisper
    pub async fn whisper<S: Into<Cow<'static, str>> + Borrow<str>>(
        &mut self,
        user: S,
        message: S,
    ) -> Result<(), Error> {
        self.send(&ClientMessage::PrivMsg {
            channel: Cow::from("jtv"),
            message: Command::Whisper {
                user: user.borrow(),
                message: message.borrow(),
            }
            .to_string()
            .into(),
        })
        .await
    }
}
