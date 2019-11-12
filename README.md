
# tmi-rs: Rust Twitch chat interface

This is an asynchronous websocket based interface to Twitch chat intended as
a base for chat bots and other programs that interact with Twitch chat.

This library is currently still highly experimental since it is based on the `futures-preview` alpha and makes
use of several dependencies which themselves are not yet stable on the new async/await system.

All official chat events and tags are supported, but some more advanced features are still
missing, including:

* Rate limiting
* Automatic detection of user status (mod/vip/etc.) and adjustment of rate limits based on that status
* Ability to directly `await` the results of commands that have a response from Twitch, like `/mods`

### Example usage

```rust
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
    let process_messages = async {
        let send_result = receiver.filter_map(
            |event| {
                info!("{:?}", &event);
                ready(match &*event {
                    Event::PrivMsg(event_data) if event_data.message().starts_with("!hello") => {
                        // return response message to the stream
                        Some(ClientMessage::message(event_data.channel().to_owned(), "Hello World!"))
                    }
                    _ => None
                })
            })
            .map(Ok)
            .forward(&mut sender).await;

        if let Err(e) = send_result { error!("{}", e); }
    };

    // log any connection errors
    let process_errors = error_receiver.for_each(async move |error| {
        error!("Connection error: {}", error);
    });
    join(process_messages, process_errors).await;
    Ok(())
}
```
