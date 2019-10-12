use fnv::FnvHashMap;

pub struct Event<T> {
    sender: T,
    event: EventContent<T>,
    tags: FnvHashMap<T, T>,
}

impl<T> Event<T> {
    pub fn sender(&self) -> &T {
        &self.sender
    }

    pub fn event(&self) -> &EventContent<T> {
        &self.event
    }

    pub fn tags(&self) -> &FnvHashMap<T, T> {
        &self.tags
    }
}

pub enum EventContent<T> {
    PrivMsg(ChannelMessageEvent<T>),
    Join(UserEvent<T>),
    Mode(ModeChangeEvent<T>),
    Names(NamesListEvent<T>),
    EndOfNames,
    Part(UserEvent<T>),
    ClearChat(ChannelUserEvent<T>),
    ClearMsg(ChannelMessageEvent<T>),
    Host(HostEvent<T>),
    Notice(ChannelMessageEvent<T>),
    Reconnect,
    RoomState(ChannelEvent<T>),
    UserNotice(ChannelMessageEvent<T>),
    UserState(ChannelEvent<T>),
    GlobalUserState,
    Unknown,
}

pub struct NamesListEvent<T> {
    names: Vec<T>,
}
pub struct ModeChangeEvent<T> {
    mode_change: T,
    user: T,
}
pub struct UserEvent<T> {
    user: T,
}
pub struct ChannelMessageEvent<T> {
    channel: T,
    message: T,
}
pub struct ChannelEvent<T> {
    channel: T,
}
pub struct HostEvent<T> {
    hosting_channel: T,
    target_channel: Option<T>,
    viewer_count: usize,
}
pub struct ChannelUserEvent<T> {
    channel: T,
    user: Option<T>,
}
