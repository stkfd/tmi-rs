//! A WebSocket based, asynchronous Twitch chat client.
//!
//! # Example
//! ```no_run
//! #![feature(async_closure)]
//! #[macro_use]
//! extern crate log;
//!
//! use std::env;
//! use std::error::Error;
//!
//! use tmi_rs::event::{ChannelMessageEventData, Event};
//! use tmi_rs::rate_limits::RateLimiterConfig;
//! use tmi_rs::{futures_util::stream::StreamExt, TwitchClient, TwitchClientConfigBuilder};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn Error>> {
//!     env_logger::init();
//!     let channel = env::var("TWITCH_CHANNEL")?;
//!     let client: TwitchClient = TwitchClientConfigBuilder::default()
//!         .username(env::var("TWITCH_USERNAME")?)
//!         .token(env::var("TWITCH_AUTH")?)
//!         .cap_membership(true)
//!         .rate_limiter(RateLimiterConfig::default())
//!         .build()?
//!         .into();
//!     let (mut sender, receiver) = client.connect().await?;
//!     sender.join(channel.clone()).await?;
//!     receiver
//!         .for_each_concurrent(None, |event| {
//!             let mut sender = sender.clone();
//!             async move {
//!               match &*event {
//!                   Ok(event) => {
//!                       info!("{:?}", &event);
//!                       match event {
//!                           Event::PrivMsg(event_data) => {
//!                               if event_data.message().starts_with("!hello") {
//!                                   sender
//!                                       .message(event_data.channel().to_owned(), "Hello World!")
//!                                       .await
//!                                       .unwrap();
//!                               }
//!                           }
//!                           _ => {}
//!                       }
//!                   }
//!                   Err(e) => error!("Connection error: {}", e),
//!               }
//!           }
//!         })
//!         .await;
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
extern crate derive_builder;
#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate log;
#[macro_use]
extern crate pin_utils;
#[macro_use]
extern crate smallvec;

use std::borrow::Borrow;
use std::fmt::{Debug, Display};
use std::hash::Hash;

pub use futures_channel;
pub use futures_core;
pub use futures_sink;
pub use futures_util;

pub use client::*;
pub use errors::*;
pub use sender::*;

mod client;
pub mod client_messages;
mod errors;
pub mod event;
pub mod irc;
pub mod irc_constants;
pub mod rate_limits;
pub mod selectors;
mod sender;
pub(crate) mod util;

/// Trait that is used when generically referring to a &str, String, or other type that can be
/// used like a borrowed string
pub trait StringRef: Borrow<str> + Debug + Clone + Hash + Eq + Display {}
impl<T> StringRef for T where T: Borrow<str> + Debug + Clone + Hash + Eq + Display {}
