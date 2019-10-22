#![feature(async_closure)]

#[macro_use]
extern crate log;

use std::env;
use std::error::Error;
use tmi_rs::client::TwitchClient;
use tmi_rs::ClientConfigBuilder;
use tmi_rs::futures::StreamExt;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let config = ClientConfigBuilder::default()
        .username(env::var("TWITCH_USERNAME")?)
        .token(env::var("TWITCH_AUTH")?)
        .cap_membership(true)
        .build()?;
    let client = TwitchClient::new(config);
    let mut connection = client.connect().await?;

    connection.join("forsen").await?;
    connection.stream_mut().by_ref().for_each(async move |event| {
        info!("{:?}", event);
    }).await;
    Ok(())
}
