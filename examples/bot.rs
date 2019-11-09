#![feature(async_closure)]
#[macro_use]
extern crate log;

use std::env;
use std::error::Error;

use tmi_rs::{futures_util::stream::StreamExt, TwitchClient, TwitchClientConfigBuilder};
use tmi_rs::event::{ChannelMessageEventData, Event};
use tmi_rs::rate_limits::RateLimiterConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let channel = env::var("TWITCH_CHANNEL")?;
    let client: TwitchClient = TwitchClientConfigBuilder::default()
        .username(env::var("TWITCH_USERNAME")?)
        .token(env::var("TWITCH_AUTH")?)
        .cap_membership(true)
        .rate_limiter(RateLimiterConfig::default())
        .build()?
        .into();
    let (mut sender, receiver) = client.connect().await?;
    sender.join(channel.clone()).await?;
    receiver.for_each_concurrent(None, |event| {
        let mut sender = sender.clone();
        async move {
            match &*event {
                Ok(event) => {
                    info!("{:?}", &event);
                    match event {
                        Event::PrivMsg(event_data) => {
                            if event_data.message().starts_with("!hello") {
                                sender.message(event_data.channel().to_owned(), "Hello World!").await.unwrap();
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => error!("Connection error: {}", e)
            }
        }
    }).await;
    Ok(())
}