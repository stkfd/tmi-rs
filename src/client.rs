//! Client module, includes websocket connection handling, listener and handler registration

use std::borrow::Cow;

use tokio_tungstenite::{connect_async, WebSocketStream};

use crate::{ClientConfig, Error, TwitchChatConnection};
use crate::client_messages::{Capability, ClientMessage};
use tokio_net::tcp::TcpStream;
use tokio_tls::TlsStream;

/// Holds the configuration for a twitch chat client. Call `connect` to establish a connection using it.
#[derive(Debug)]
pub struct TwitchClient {
    config: ClientConfig,
}

impl TwitchClient {
    /// Create a new chat client
    pub fn new(config: ClientConfig) -> TwitchClient {
        TwitchClient {
            config,
        }
    }

    /// Connects to the Twitch servers and listens in a separate thread. Await the returned future
    /// to block until the connection is closed.
    pub async fn connect(&self) -> Result<TwitchChatConnection<WebSocketStream<tokio_tungstenite::stream::Stream<TcpStream, TlsStream<TcpStream>>>>, Error>
    {
        debug!("Connecting to {}", self.config.url);
        let (ws, _) = connect_async(self.config.url.clone())
            .await
            .map_err(|e| {
                error!("Connection to {} failed", self.config.url);
                Error::WebsocketError { details: Cow::from("Failed to connect to the chat server"), source: e }
            })?;

        let mut connection = TwitchChatConnection::new(ws);

        let mut capabilities = Vec::with_capacity(3);
        if self.config.cap_commands { capabilities.push(Capability::Commands) }
        if self.config.cap_tags { capabilities.push(Capability::Tags) }
        if self.config.cap_membership { capabilities.push(Capability::Membership) }
        connection.send(&ClientMessage::<&str>::CapRequest(capabilities)).await?;
        connection.login(&self.config.username, &self.config.token).await?;

        Ok(connection)
    }
}
