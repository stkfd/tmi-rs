#![allow(dead_code)]
#![feature(async_closure)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate derive_builder;

pub mod client;
mod config;
mod data;
mod errors;
mod irc;
mod irc_constants;
mod sender;
mod util;

pub use futures;

pub use config::*;
pub use config::*;
pub use data::*;
pub use errors::*;
pub use sender::*;
