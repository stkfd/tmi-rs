#![feature(async_closure)]

use tmi_rs::client::TwitchClient;
use tmi_rs::ClientConfigBuilder;
use tmi_rs::futures::StreamExt;
use std::env;

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() {
    env_logger::init();
    let config = ClientConfigBuilder::default()
        .username(env::var("TWITCH_USERNAME").unwrap())
        .token(env::var("TWITCH_AUTH").unwrap())
        .cap_membership(true)
        .build()
        .unwrap();
    let client = TwitchClient::new(config);
    let (mut sender, events) = client.connect().await.unwrap();

    sender.join("forsen").await.unwrap();
    events.clone().for_each(async move |event| {
        info!("{:?}", event);
    }).await;
}
