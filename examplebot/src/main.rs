#![feature(async_closure)]

#[macro_use]
extern crate log;

use std::env;
use std::error::Error;
use tmi_rs::{TwitchClientBuilder, TwitchClient, futures::StreamExt};
use tmi_rs::dispatch::{EventDispatch, MatcherBuilder, HandlerBuilder, EventHandlerResult};
use tmi_rs::futures::future::ok;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let client: TwitchClient = TwitchClientBuilder::default()
        .username(env::var("TWITCH_USERNAME")?)
        .token(env::var("TWITCH_AUTH")?)
        .cap_membership(true)
        .build()?;
    let mut connection = client.connect().await?;

    let mut dispatch = EventDispatch::default();
    dispatch.match_events(async move |e| true)
        .handle(Box::new(async move |event| {
            EventHandlerResult::Ok
        }));

    connection.join("zapbeeblebrox123").await?;
    connection.stream_mut().by_ref().for_each(async move |event| {
        info!("{:?}", event);
    }).await;
    Ok(())
}
