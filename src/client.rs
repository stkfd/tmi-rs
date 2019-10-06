use async_trait::async_trait;
use crate::{ClientConfig, TwitchChatStream, Error, ErrorKind};
use crate::events::Event;
use core::fmt;
use fnv::FnvHashMap;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use futures::StreamExt;

#[async_trait]
pub trait EventHandler: fmt::Debug + Sync {
    async fn run(&self);
}

pub trait EventMatcher: fmt::Debug + Sync {
    fn match_event(&self, e: Event);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HandlerGroupId(usize);

#[derive(Debug)]
struct EventHandlerGroup {
    matcher: Box<dyn EventMatcher>,
    handlers: Vec<Box<dyn EventHandler>>,
}

#[derive(Debug)]
pub struct TwitchClient {
    config: ClientConfig,
    event_groups: FnvHashMap<HandlerGroupId, EventHandlerGroup>,
    next_group_id: usize,
}

impl TwitchClient {
    pub fn new(config: ClientConfig) -> TwitchClient {
        TwitchClient {
            config,
            event_groups: FnvHashMap::default(),
            next_group_id: 1,
        }
    }

    pub fn register_matcher(&mut self, matcher: Box<dyn EventMatcher>) -> HandlerGroupId {
        let group_id = HandlerGroupId(self.next_group_id);
        self.next_group_id += 1;
        self.event_groups.insert(group_id, EventHandlerGroup { matcher, handlers: vec![] });
        group_id
    }

    pub fn register_handler(&mut self, group_id: HandlerGroupId, handler: Box<dyn EventHandler>) {
        if let Some(group) = self.event_groups.get_mut(&group_id) {
            group.handlers.push(handler)
        };
    }

    /// Connects to the Twitch servers and listens in a separate thread. Await the returned future
    /// to block until the connection is closed.
    pub async fn connect(&self) -> Result<(), Error> {
        debug!("Connecting to {}", self.config.url);
        let (ws_stream, _) = connect_async(self.config.url.clone())
            .await
            .map_err(|e| {
                error!("Connection to {} failed", self.config.url);
                Error::new(ErrorKind::WebsocketError(e), "Failed to connect to the chat server.")
            })?;

        let mut stream = TwitchChatStream::new(ws_stream);

        stream.login(&self.config.username, &self.config.token).await?;
        while let Some(msg) = stream.ws.next().await {
            if let Message::Text(msg) = msg? {
                info!("{}", msg);
            }
        }

        Ok(())
    }
}

pub trait MatcherBuilder {
    fn match_events(&mut self, matcher: Box<dyn EventMatcher>) -> (HandlerGroupId, &mut TwitchClient);
}

impl MatcherBuilder for TwitchClient {
    fn match_events(&mut self, matcher: Box<dyn EventMatcher>) -> (HandlerGroupId, &mut TwitchClient) {
        (self.register_matcher(matcher), self)
    }
}

impl MatcherBuilder for (HandlerGroupId, &mut TwitchClient) {
    fn match_events(&mut self, matcher: Box<dyn EventMatcher>) -> (HandlerGroupId, &mut TwitchClient) {
        (self.1.register_matcher(matcher), self.1)
    }
}

pub trait HandlerBuilder {
    fn handle_with(&mut self, handler: Box<dyn EventHandler>) -> (HandlerGroupId, &mut TwitchClient);
}

impl HandlerBuilder for (HandlerGroupId, &mut TwitchClient) {
    fn handle_with(&mut self, handler: Box<dyn EventHandler>) -> (HandlerGroupId, &mut TwitchClient) {
        self.1.register_handler(self.0, handler);
        (self.0, self.1)
    }
}
