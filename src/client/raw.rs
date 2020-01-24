use futures_core::Stream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::tungstenite::{handshake::client::Request, Message};

use crate::event::TwitchChatStream;
use crate::Error;

/// Creates a connection to Twitch chat without any additional handling logic. The
/// websocket is wrapped
pub async fn connect(
    url: impl Into<Request<'static>> + Unpin,
) -> Result<TwitchChatStream<impl Stream<Item = Result<Message, WsError>> + Unpin>, Error> {
    let (ws, _respone) = connect_async(url).await?;
    Ok(TwitchChatStream::new(ws))
}
