#![feature(async_closure)]
#[macro_use]
extern crate log;

use std::env;
use std::error::Error;

use futures::future::{join, ready};
use futures_util::SinkExt;

use tmi_rs::{futures_util::stream::StreamExt, TwitchChatConnection, TwitchClient, TwitchClientConfigBuilder};
use tmi_rs::client_messages::ClientMessage;
use tmi_rs::event::{ChannelMessageEventData, Event};
use tmi_rs::rate_limits::RateLimiterConfig;

/// To run this example, the TWITCH_CHANNEL, TWITCH_USERNAME and TWITCH_AUTH environment variables
/// need to be set.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let channel = env::var("TWITCH_CHANNEL")?;
    let client: TwitchClient = TwitchClientConfigBuilder::default()
        .username(env::var("TWITCH_USERNAME")?)
        .token(env::var("TWITCH_AUTH")?)
        .rate_limiter(RateLimiterConfig::default())
        .build()?
        .into();

    let TwitchChatConnection { mut sender, receiver, error_receiver } = client.connect().await?;

    // join a channel
    sender.send(ClientMessage::join(channel.clone())).await?;

    // process messages and do stuff with the data
    let mut process_messages = receiver.filter_map(
        |event| {
            info!("{:?}", &event);
            ready(match &*event {
                Event::PrivMsg(event_data) if event_data.message().starts_with("!hello") => {
                    // return response message to the stream
                    Some(ClientMessage::message(event_data.channel().to_owned(), "Hello World!"))
                }
                _ => None
            })
        });

    // send responses from above
    let send_responses = async {
        let result = sender.send_all(&mut process_messages).await;
        if let Err(e) = result {
            // log errors from sending the messages
            error!("{}", e);
        }
    };

    // log any connection errors
    let process_errors = error_receiver.for_each(async move |error| {
        error!("Connection error: {}", error);
    });
    join(send_responses, process_errors).await;
    Ok(())
}
