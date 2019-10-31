#![feature(async_closure)]
#[macro_use]
extern crate log;

use std::env;
use std::error::Error;
use tmi_rs::{TwitchClientBuilder, TwitchClient, futures_util::StreamExt};
use tmi_rs::event::{Event, ChannelMessageEventData};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let channel = env::var("TWITCH_CHANNEL")?;
    let client: TwitchClient = TwitchClientBuilder::default()
        .username(env::var("TWITCH_USERNAME")?)
        .token(env::var("TWITCH_AUTH")?)
        .cap_membership(true)
        .build()?;
    let (mut sender, mut receiver) = client.connect().await?;

    sender.join(channel.clone()).await?;

    while let Some(event) = receiver.next().await {
        match &*event {
            Ok(event) => {
                match event {
                    Event::PrivMsg(event_data) => {
                        if event_data.message().starts_with("!hello") {
                            sender.message(event_data.channel().to_owned(), "Hello World!").await?;
                        }
                    }
                    _ => {
                        info!("Event received: {:?}", event)
                    }
                }
            }
            Err(e) => error!("Connection error: {}", e)
        }
    }
    receiver.for_each(async move |event| {
        info!("{:?}", event);
    }).await;
    Ok(())
}
