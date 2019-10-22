#![allow(dead_code)]
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

pub use futures;

pub use config::*;
pub use config::*;
pub use data::*;
pub use errors::*;
pub use sender::*;

pub mod client;
mod config;
mod data;
mod errors;
mod irc;
mod irc_constants;
mod sender;
mod util;
