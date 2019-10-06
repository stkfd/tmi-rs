use crate::client_messages::{ClientMessage, Command};
use crate::Error;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

pub struct TwitchChatStream<S> {
    pub(crate) ws: WebSocketStream<S>,
}

impl<S: AsyncWrite + AsyncRead + Unpin> TwitchChatStream<S> {
    pub fn new(ws_stream: WebSocketStream<S>) -> TwitchChatStream<S> {
        TwitchChatStream {
            ws: ws_stream
        }
    }

    pub async fn send(&mut self, message: ClientMessage<'_>) -> Result<(), Error> {
        let formatted_message = format!("{}", message);
        debug!("{}", formatted_message);
        self.ws.send(Message::from(format!("{}", message)))
            .await
            .map_err(Into::into)
    }

    pub async fn login(&mut self, username: &str, token: &str) -> Result<(), Error> {
        self.send(ClientMessage::Pass(token)).await?;
        self.send(ClientMessage::Nick(username)).await
    }

    pub async fn ban(&mut self, channel: &str, username: &str) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Ban(username),
        }).await
    }

    pub async fn unban(&mut self, channel: &str, username: &str) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Unban(username),
        }).await
    }

    pub async fn clear(&mut self, channel: &str) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Clear,
        }).await
    }

    pub async fn color(&mut self, color: &str) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel: "jtv",
            command: Command::Color(color),
        }).await
    }

    pub async fn commercial(&mut self, channel: &str, time: Option<usize>) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Commercial { time },
        }).await
    }

    pub async fn delete(&mut self, channel: &str, msg_id: &str) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Delete { msg_id },
        }).await
    }

    pub async fn disconnect(&mut self) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel: "jtv",
            command: Command::Disconnect,
        }).await
    }

    pub async fn emote_only(&mut self, channel: &str, status: bool) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::EmoteOnly(status),
        }).await
    }

    pub async fn followers_only(&mut self, channel: &str, status: bool) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::FollowersOnly(status),
        }).await
    }

    pub async fn host(&mut self, channel: &str, host_channel: &str) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Host(host_channel),
        }).await
    }

    pub async fn unhost(&mut self, channel: &str) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Unhost,
        }).await
    }

    pub async fn marker(&mut self, channel: &str, description: Option<&str>) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Marker(description),
        }).await
    }

    pub async fn me(&mut self) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel: "jtv",
            command: Command::Me,
        }).await
    }

    pub async fn make_mod(&mut self, channel: &str, user: &str) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Mod(user),
        }).await
    }

    pub async fn unmod(&mut self, channel: &str, user: &str) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Unmod(user),
        }).await
    }

    pub async fn r9k(&mut self, channel: &str, on: bool) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::R9k(on),
        }).await
    }

    pub async fn raid(&mut self, channel: &str, raid_channel: &str) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Raid(raid_channel),
        }).await
    }

    pub async fn unraid(&mut self, channel: &str) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Unraid,
        }).await
    }

    pub async fn slow(&mut self, channel: &str, seconds: usize) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Slow(seconds),
        }).await
    }

    pub async fn slow_off(&mut self, channel: &str) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::SlowOff,
        }).await
    }

    pub async fn subscribers(&mut self, channel: &str, on: bool) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::SubscribersOnly(on),
        }).await
    }

    pub async fn timeout(&mut self, channel: &str, user: &str, seconds: Option<usize>) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Timeout {
                time: seconds,
                user,
            },
        }).await
    }

    pub async fn vip(&mut self, channel: &str, user: &str) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Vip(user),
        }).await
    }

    pub async fn vips(&mut self, channel: &str) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel,
            command: Command::Vips,
        }).await
    }

    pub async fn whisper(&mut self, user: &str, message: &str) -> Result<(), Error> {
        self.send(ClientMessage::Command {
            channel: "jtv",
            command: Command::Whisper { user, message },
        }).await
    }
}
