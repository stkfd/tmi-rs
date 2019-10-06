use tmi_rs::client::TwitchClient;
use tmi_rs::ClientConfigBuilder;
use tokio::prelude::*;
use std::env;

#[tokio::main]
async fn main() {
    env_logger::init();
    let config = ClientConfigBuilder::default()
        .username(env::var("TWITCH_USERNAME").unwrap())
        .token(env::var("TWITCH_AUTH").unwrap())
        .build()
        .unwrap();
    let client = TwitchClient::new(config);

    client.connect().await;
}
