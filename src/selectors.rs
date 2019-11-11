//! Convenience methods for matching specific event types. The functions in this module select
//! an `Arc<Event<T>>` with a specific `Event` variant. The returned `SelectorResult` implements
//! `Deref` for the inner type contained in the enum variant.
// TODO: add example

use std::hint::unreachable_unchecked;
use std::ops::Deref;
use std::sync::Arc;

use futures_core::Future;
use futures_util::future::ready;

use crate::event::*;
use crate::StringRef;
use std::marker::PhantomData;

/// Contains a PRIVMSG event
#[derive(Clone, Debug)]
pub struct SelectorResult<T: StringRef, Inner>(Arc<Event<T>>, PhantomData<Inner>);

impl<T: StringRef, Inner> SelectorResult<T, Inner> {
    fn new(evt: Arc<Event<T>>) -> Self {
        SelectorResult::<T, Inner>(evt, PhantomData)
    }
}

macro_rules! impl_selector {
    ($fn_name: ident, $enum_variant: ident, $inner_type: ty) => {
        impl<T: StringRef> Deref for SelectorResult<T, $inner_type> {
            type Target = EventData<T, $inner_type>;

            fn deref(&self) -> &Self::Target {
                match &*self.0 {
                    Event::$enum_variant(evt) => evt,
                    _ => unsafe { unreachable_unchecked() },
                }
            }
        }

        #[allow(missing_docs)]
        pub fn $fn_name<T: StringRef>(
            item: Arc<Event<T>>,
        ) -> impl Future<Output = Option<SelectorResult<T, $inner_type>>> {
            ready(match &*item {
                Event::$enum_variant(_) => Some(SelectorResult::new(item)),
                _ => None,
            })
        }
    };
}

impl_selector!(priv_msg, PrivMsg, PrivMsgEvent<T>);
impl_selector!(whisper, Whisper, WhisperEvent<T>);
impl_selector!(join, Join, JoinEvent<T>);
impl_selector!(mode, Mode, ModeChangeEvent<T>);
impl_selector!(names, Names, NamesListEvent<T>);
impl_selector!(end_of_names, EndOfNames, EndOfNamesEvent<T>);
impl_selector!(part, Part, PartEvent<T>);
impl_selector!(clear_chat, ClearChat, ClearChatEvent<T>);
impl_selector!(clear_msg, ClearMsg, ClearMsgEvent<T>);
impl_selector!(host, Host, HostEvent<T>);
impl_selector!(notice, Notice, NoticeEvent<T>);
impl_selector!(reconnect, Reconnect, ReconnectEvent);
impl_selector!(room_state, RoomState, RoomStateEvent<T>);
impl_selector!(user_notice, UserNotice, UserNoticeEvent<T>);
impl_selector!(user_state, UserState, UserStateEvent<T>);
impl_selector!(capability, Capability, CapabilityEvent<T>);
impl_selector!(connect_message, ConnectMessage, ConnectMessageEvent<T>);
