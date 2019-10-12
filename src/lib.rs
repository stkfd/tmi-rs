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
mod futures;
mod irc;
mod sender;

pub use config::*;
pub use config::*;
pub use data::*;
pub use errors::*;
pub use sender::*;
