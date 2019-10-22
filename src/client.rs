//! Client module, includes websocket connection handling, listener and handler registration

use std::borrow::Cow;

use tokio_tungstenite::{connect_async, WebSocketStream};

use crate::{Error, TwitchChatConnection};
use crate::client_messages::{Capability, ClientMessage};
use tokio_net::tcp::TcpStream;
use tokio_tls::TlsStream;
use url::Url;

/// Holds the configuration for a twitch chat client. Call `connect` to establish a connection using it.
#[derive(Debug, Clone, Builder)]
pub struct TwitchClient {
    /// The chat server, by default wss://irc-ws.chat.twitch.tv:443
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
}

impl TwitchClient {
    /// Connects to the Twitch servers and listens in a separate thread. Await the returned future
    /// to block until the connection is closed.
    pub async fn connect(&self) -> Result<TwitchChatConnection<WebSocketStream<tokio_tungstenite::stream::Stream<TcpStream, TlsStream<TcpStream>>>>, Error>
    {
        debug!("Connecting to {}", self.url);
        let (ws, _) = connect_async(self.url.clone())
            .await
            .map_err(|e| {
                error!("Connection to {} failed", self.url);
                Error::WebsocketError { details: Cow::from("Failed to connect to the chat server"), source: e }
            })?;

        let mut connection = TwitchChatConnection::new(ws);

        let mut capabilities = Vec::with_capacity(3);
        if self.cap_commands { capabilities.push(Capability::Commands) }
        if self.cap_tags { capabilities.push(Capability::Tags) }
        if self.cap_membership { capabilities.push(Capability::Membership) }
        connection.send(&ClientMessage::<&str>::CapRequest(capabilities)).await?;
        connection.login(&self.username, &self.token).await?;

        Ok(connection)
    }
}
