use fnv::FnvHashMap;

pub struct Event<'a> {
    sender: &'a str,
    event: EventContent<'a>,
    tags: FnvHashMap<&'a str, &'a str>,
}

impl<'a> Event<'a> {
    pub fn sender(&self) -> &str {
        self.sender
    }

    pub fn event(&self) -> &EventContent {
        &self.event
    }

    pub fn tags(&self) -> &FnvHashMap<&'a str, &'a str> {
        &self.tags
    }
}

pub enum EventContent<'a> {
    PrivMsg(ChannelMessageEvent<'a>),
    Join(UserEvent<'a>),
    Mode(ModeChangeEvent<'a>),
    Names(NamesListEvent<'a>),
    EndOfNames,
    Part(UserEvent<'a>),
    ClearChat(ChannelUserEvent<'a>),
    ClearMsg(ChannelMessageEvent<'a>),
    Host(HostEvent<'a>),
    Notice(ChannelMessageEvent<'a>),
    Reconnect,
    RoomState(ChannelEvent<'a>),
    UserNotice(ChannelMessageEvent<'a>),
    UserState(ChannelEvent<'a>),
    GlobalUserState,
    Unknown,
}

pub struct NamesListEvent<'a> {
    names: Vec<&'a str>,
}
pub struct ModeChangeEvent<'a> {
    mode_change: &'a str,
    user: &'a str,
}
pub struct UserEvent<'a> {
    user: &'a str,
}
pub struct ChannelMessageEvent<'a> {
    channel: &'a str,
    message: &'a str,
}
pub struct ChannelEvent<'a> {
    channel: &'a str,
}
pub struct HostEvent<'a> {
    hosting_channel: &'a str,
    target_channel: Option<&'a str>,
    viewer_count: usize,
}
pub struct ChannelUserEvent<'a> {
    channel: &'a str,
    user: Option<&'a str>,
}
