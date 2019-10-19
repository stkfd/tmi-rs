use crate::events::Event;
use crate::StringRef;

pub fn channel_message<T: StringRef>(evt: &Event<T>) -> bool {
    match evt {
        Event::PrivMsg(_) => true,
        _ => false,
    }
}
