//! Client module, includes websocket connection handling, listener and handler registration

use tokio::sync::{mpsc, oneshot};

use crate::MessageSendError;
use crate::{ClientMessage, Error};

use crate::stream::{message_responder_channel, SentClientMessage};
pub use config::*;

mod config;

/// Managed connection pools
pub mod pool;
/// Create a raw connection with no extras
pub mod raw;
/// Single, "batteries included" connections
pub mod single;

type TimeoutReceiver = oneshot::Receiver<()>;
type InnerMessageSender = mpsc::Sender<SentClientMessage>;

// TODO: expand on this to handle actual responses from Twitch to messages
/// Message response container
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MessageResponse {
    /// Message sent successfully with no response data
    Ok,
}

/// Handle to a connection that can be used to send messages
#[derive(Debug, Clone)]
pub struct MessageSender {
    sender: InnerMessageSender,
}

impl From<InnerMessageSender> for MessageSender {
    fn from(sender: InnerMessageSender) -> Self {
        MessageSender { sender }
    }
}

impl MessageSender {
    /// Send a message
    pub async fn send(&mut self, msg: ClientMessage) -> Result<MessageResponse, MessageSendError> {
        let (tx, _rx) = message_responder_channel();
        self.sender
            .send(SentClientMessage {
                message: msg,
                responder: tx,
            })
            .await
            .map_err(|e| MessageSendError::Closed(e.0.message))?;
        Ok(MessageResponse::Ok)
    }
}

/// Represents a twitch chat client/connection. Call `connect` to establish a connection.
pub struct TwitchClient<St> {
    /// message sender for client messages
    sender: MessageSender,
    /// stream of chat events
    stream: St,
}

impl<St> TwitchClient<St> {
    /// Get a mutable reference to the message send channel
    pub fn sender_mut(&mut self) -> &mut MessageSender {
        &mut self.sender
    }

    /// Get an owned message sender
    pub fn sender_cloned(&self) -> MessageSender {
        self.sender.clone()
    }

    /// Get a mutable reference to the stream of chat events
    pub fn stream_mut(&mut self) -> &mut St {
        &mut self.stream
    }
}

async fn send_capabilities(
    cfg: &TwitchClientConfig,
    sender: &mut MessageSender,
) -> Result<(), Error> {
    // send capability requests on connect
    let capabilities = cfg.get_capabilities();
    sender.send(ClientMessage::CapRequest(capabilities)).await?;
    for msg in ClientMessage::login(cfg.username.clone(), cfg.token.clone()).into_iter() {
        sender.send(msg).await?;
    }
    Ok(())
}
