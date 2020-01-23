#[macro_use]
extern crate log;

use std::env;
use std::error::Error;
use std::sync::Arc;

use futures::stream::StreamExt;

use pin_utils::pin_mut;
use tmi_rs::client_messages::ClientMessage;
use tmi_rs::event::*;
use tmi_rs::selectors::priv_msg;
use tmi_rs::{connect, TwitchClientConfig, TwitchClientConfigBuilder};

/// To run this example, the TWITCH_CHANNEL, TWITCH_USERNAME and TWITCH_AUTH environment variables
/// need to be set.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let channel = env::var("TWITCH_CHANNEL")?;
    let config: Arc<TwitchClientConfig> = Arc::new(
        TwitchClientConfigBuilder::default()
            .username(env::var("TWITCH_USERNAME")?)
            .token(env::var("TWITCH_AUTH")?)
            .build()?,
    );
    let mut client = connect(&config).await?;
    let mut sender = client.sender_cloned();
    let receiver = client.stream_mut();

    // join a channel
    sender.send(ClientMessage::join(channel.clone())).await?;
    info!("Joined channel");

    // process messages and do stuff with the data
    let privmsg_stream = receiver
        .filter_map(|event| {
            async move {
                match event {
                    Ok(event) => Some(event),
                    Err(error) => {
                        error!("Connection error: {}", error);
                        None
                    }
                }
            }
        })
        .filter_map(priv_msg); // filter only privmsg events

    pin_mut!(privmsg_stream);

    while let Some(event_data) = privmsg_stream.next().await {
        info!("{:?}", event_data);
        if event_data.message().starts_with("!hello") {
            // return response message to the stream
            sender
                .send(ClientMessage::message(
                    event_data.channel().to_owned(),
                    "Hello World!",
                ))
                .await?;
        }
    }
    Ok(())
}
