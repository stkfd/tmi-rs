//! A WebSocket based, asynchronous Twitch chat client.
//!
//! # Example
//! ```no_run
//! #[macro_use]
//! extern crate log;
//!
//! use std::env;
//! use std::error::Error;
//! use std::sync::Arc;
//!
//! use futures::stream::{StreamExt, TryStreamExt};
//!
//! use tmi_rs::client_messages::ClientMessage;
//! use tmi_rs::event::*;
//! use tmi_rs::selectors::priv_msg;
//! use tmi_rs::{TwitchClient, TwitchClientConfig, TwitchClientConfigBuilder};
//!
//! /// To run this example, the TWITCH_CHANNEL, TWITCH_USERNAME and TWITCH_AUTH environment variables
//! /// need to be set.
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn Error>> {
//!     env_logger::init();
//!     let channel = env::var("TWITCH_CHANNEL")?;
//!     let config: Arc<TwitchClientConfig> = Arc::new(
//!         TwitchClientConfigBuilder::default()
//!             .username(env::var("TWITCH_USERNAME")?)
//!             .token(env::var("TWITCH_AUTH")?)
//!             .build()?,
//!     );
//!     let client = TwitchClient::new(&config);
//!
//!     let (sender, receiver) = client.connect().await?;
//!
//!     // join a channel
//!     sender.send(ClientMessage::join(channel.clone()))?;
//!
//!     // process messages and do stuff with the data
//!     receiver
//!         .filter_map(|event| {
//!             async move {
//!                 match event {
//!                     Ok(event) => Some(event),
//!                     Err(error) => {
//!                         error!("Connection error: {}", error);
//!                         None
//!                     }
//!                 }
//!             }
//!         })
//!         .filter_map(priv_msg) // filter only privmsg events
//!         .map(Ok)
//!         .try_for_each(|event_data| {
//!             // explicit binding of the reference is needed so the `async move` closure does not
//!             // move the sender itself
//!             let sender = &sender;
//!             async move {
//!                 info!("{:?}", event_data);
//!                 if event_data.message().starts_with("!hello") {
//!                     // return response message to the stream
//!                     sender.send(ClientMessage::message(
//!                         event_data.channel().to_owned(),
//!                         "Hello World!",
//!                     ))?;
//!                 }
//!                 // type hint is required because there is no other way to specify the type of the error
//!                 // returned by the async block
//!                 Ok::<_, Box<dyn Error>>(())
//!             }
//!         })
//!         .await?;
//!     Ok(())
//! }
//! ```

#![deny(
    unused_must_use,
    unused_mut,
    unused_imports,
    unused_import_braces,
    missing_docs
)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate smallvec;

use std::borrow::Borrow;
use std::fmt::{Debug, Display};
use std::hash::Hash;

pub use client::*;
pub use client_messages::*;
pub use errors::*;
pub use sender::*;

mod client;
pub mod client_messages;
mod errors;
pub mod event;
pub mod irc;
pub mod irc_constants;
pub mod selectors;
mod sender;
pub mod stream;
pub(crate) mod util;

/// Trait that is used when generically referring to a &str, String, or other type that can be
/// used like a borrowed string
pub trait StringRef: Borrow<str> + Debug + Clone + Hash + Eq + Display {}
impl<T> StringRef for T where T: Borrow<str> + Debug + Clone + Hash + Eq + Display {}
