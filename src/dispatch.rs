use std::fmt::Debug;
use std::pin::Pin;

use fnv::FnvHashMap;
use futures::Future;

use crate::event::Event;

#[derive(Debug, Clone, Copy)]
pub enum EventHandlerResult {
    /// Ran successfully
    Ok,
    /// Ran successfully, deregister handler after this run
    Remove,
}

/// # Examples
///
/// ```rust
/// #![feature(async_closure)]
/// use tmi_rs::dispatch::*;
/// use tmi_rs::event::Event;
///
/// let mut dispatch = EventDispatch::default();
///    dispatch.match_events(async move |e: &Event<&str>| true)
///        .handle(Box::new(async move |event: &Event<&str>| {
///            EventHandlerResult::Ok
///        }));
/// ```
#[derive(Default)]
pub struct EventDispatch {
    next_id: usize,
    event_groups: FnvHashMap<HandlerGroupId, EventHandlerGroup>,
}

impl EventDispatch {
    fn next_id(&mut self) -> HandlerGroupId {
        let id = HandlerGroupId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("Can't get next selector id due to int overflow");
        id
    }

    pub fn register_matcher(&mut self, matcher: Box<dyn EventMatcher>) -> HandlerGroupId {
        let group_id = self.next_id();
        self.event_groups.insert(
            group_id,
            EventHandlerGroup {
                matcher,
                handlers: vec![],
            },
        );
        group_id
    }

    pub fn register_handler(&mut self, group_id: HandlerGroupId, handler: Box<dyn EventHandler>) {
        if let Some(group) = self.event_groups.get_mut(&group_id) {
            group.handlers.push(handler)
        };
    }
}

pub trait EventHandler {
    fn run(&mut self, event: &Event<&str>) -> Pin<Box<dyn Future<Output = EventHandlerResult>>>;
}
impl<T, Fut> EventHandler for T
where
    T: FnMut(&Event<&str>) -> Fut,
    Fut: Future<Output = EventHandlerResult> + 'static,
{
    fn run(&mut self, e: &Event<&str>) -> Pin<Box<dyn Future<Output = EventHandlerResult>>> {
        Pin::from(Box::new(self(e)))
    }
}

pub trait EventMatcher {
    fn match_event(&mut self, e: &Event<&str>) -> Pin<Box<dyn Future<Output = bool>>>;
}
impl<T, Fut> EventMatcher for T
where
    for<'x> T: FnMut(&'x Event<&'x str>) -> Fut,
    Fut: Future<Output = bool> + 'static,
{
    fn match_event(&mut self, e: &Event<&str>) -> Pin<Box<dyn Future<Output = bool>>> {
        Pin::from(Box::new(self(e)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HandlerGroupId(usize);

struct EventHandlerGroup {
    matcher: Box<dyn EventMatcher>,
    handlers: Vec<Box<dyn EventHandler>>,
}

pub trait MatcherBuilder {
    fn match_events(
        &mut self,
        matcher: impl EventMatcher + 'static,
    ) -> (HandlerGroupId, &mut EventDispatch);
}

impl MatcherBuilder for EventDispatch {
    fn match_events(
        &mut self,
        matcher: impl EventMatcher + 'static,
    ) -> (HandlerGroupId, &mut EventDispatch) {
        (self.register_matcher(Box::new(matcher)), self)
    }
}

impl MatcherBuilder for (HandlerGroupId, &mut EventDispatch) {
    fn match_events(
        &mut self,
        matcher: impl EventMatcher + 'static,
    ) -> (HandlerGroupId, &mut EventDispatch) {
        (self.1.register_matcher(Box::new(matcher)), self.1)
    }
}

pub trait HandlerBuilder {
    fn handle(&mut self, handler: Box<dyn EventHandler>) -> (HandlerGroupId, &mut EventDispatch);
}

impl HandlerBuilder for (HandlerGroupId, &mut EventDispatch) {
    fn handle(&mut self, handler: Box<dyn EventHandler>) -> (HandlerGroupId, &mut EventDispatch) {
        self.1.register_handler(self.0, handler);
        (self.0, self.1)
    }
}
