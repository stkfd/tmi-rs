#![feature(async_closure)]

use tmi_rs::client::TwitchClient;
use tmi_rs::ClientConfigBuilder;
use tmi_rs::futures::StreamExt;
use std::env;
use std::error::Error;

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let config = ClientConfigBuilder::default()
        .username(env::var("TWITCH_USERNAME")?)
        .token(env::var("TWITCH_AUTH")?)
        .cap_membership(true)
        .build()?;
    let client = TwitchClient::new(config);
    let (mut sender, events) = client.connect().await?;

    sender.join("forsen").await?;
    events.clone().for_each(async move |event| {
        info!("{:?}", event);
    }).await;
    Ok(())
}
