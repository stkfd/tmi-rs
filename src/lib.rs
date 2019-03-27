#![allow(dead_code)]

#[macro_use]
extern crate log;

use std::collections::vec_deque::VecDeque;

use ws::{CloseCode, connect, Handler, Handshake, Sender};

const DEFAULT_URL: &'static str = "wss://irc-ws.chat.twitch.tv:443";

#[derive(Clone, Debug, Default)]
pub struct Connection {
    url: Option<String>
}

impl Connection {
    fn url(&self) -> String {
        self.url.as_ref().unwrap_or(&DEFAULT_URL.to_owned()).clone()
    }

    pub fn open(&self) -> Result<(), ws::Error> {
        connect(self.url(), |out| {
            out.send("test");
            Client::new(out)
        })
    }
}

pub struct Client {
    sender: Sender,
    send_queue: VecDeque<Message>,
}

impl Client {
    fn new(sender: Sender) -> Self {
        Client { sender, send_queue: VecDeque::new() }
    }
}

impl Handler for Client {
    fn on_open(&mut self, shake: Handshake) -> Result<(), ws::Error> {
        if let Some(addr) = shake.remote_addr()? {
            debug!("Connection with {} now open", addr);
        }
        Ok(())
    }
}

pub struct Message {
    raw: String
}