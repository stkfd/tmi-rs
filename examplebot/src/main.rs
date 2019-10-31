#![feature(async_closure)]

#[macro_use]
extern crate log;

use std::env;
use std::error::Error;
use tmi_rs::{TwitchClientBuilder, TwitchClient, futures::StreamExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let client: TwitchClient = TwitchClientBuilder::default()
        .username(env::var("TWITCH_USERNAME")?)
        .token(env::var("TWITCH_AUTH")?)
        .cap_membership(true)
        .build()?;
    let (mut sender, receiver) = client.connect().await?;

    sender.join("forsen").await?;
    receiver.for_each(async move |event| {
        info!("{:?}", event);
    }).await;
    Ok(())
}
