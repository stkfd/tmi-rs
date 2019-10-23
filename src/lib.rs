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
//! use tmi_rs::{TwitchClientBuilder, TwitchClient, futures::StreamExt};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn Error>> {
//!     env_logger::init();
//!     let client: TwitchClient = TwitchClientBuilder::default()
//!         .username(env::var("TWITCH_USERNAME")?)
//!         .token(env::var("TWITCH_AUTH")?)
//!         .cap_membership(true)
//!         .build()?;
//!     let mut connection = client.connect().await?;
//!
//!     connection.join("forsen").await?;
//!     connection.stream_mut().by_ref().for_each(async move |event| {
//!         info!("{:?}", event);
//!     }).await;
//!     Ok(())
//! }
//! ```

#![feature(async_closure)]
#![warn(unused_must_use, unused_mut, unused_imports, unused_import_braces)]

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

pub use client::*;
pub use data::*;
pub use errors::*;
pub use futures;
pub use sender::*;

pub mod client;
mod data;
mod errors;
mod irc;
pub mod irc_constants;
mod sender;
mod util;
