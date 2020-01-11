#[macro_use]
extern crate log;

use std::env;
use std::error::Error;

use futures::stream::StreamExt;

use std::sync::Arc;
use tmi_rs::client_messages::ClientMessage;
use tmi_rs::event::*;
use tmi_rs::selectors::priv_msg;
use tmi_rs::stream::rate_limits::RateLimiter;
use tmi_rs::{TwitchClient, TwitchClientConfig, TwitchClientConfigBuilder};

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
    let client = TwitchClient::new(&config, &Arc::new(RateLimiter::from(&config.rate_limiter)));

    let (sender, receiver) = client.connect().await?;

    // join a channel
    sender.send(ClientMessage::join(channel.clone()))?;

    // process messages and do stuff with the data
    receiver
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
        .filter_map(priv_msg)
        .for_each(|event_data| {
            let sender = &sender;
            async move {
                info!("{:?}", event_data);
                if event_data.message().starts_with("!hello") {
                    // return response message to the stream
                    sender
                        .send(ClientMessage::message(
                            event_data.channel().to_owned(),
                            "Hello World!",
                        ))
                        .unwrap();
                }
            }
        })
        .await;
    Ok(())
}
