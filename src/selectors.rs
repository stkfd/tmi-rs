//! Convenience methods for matching specific event types. The functions in this module select
//! an `Arc<Event<E>>` with a specific `Event` variant. The returned `SelectorResult` implements
//! `Deref` for the inner type contained in the enum variant.
// TODO: add example

use std::borrow::Borrow;
use std::hint::unreachable_unchecked;
use std::marker::PhantomData;
use std::ops::Deref;

use futures_core::Future;
use futures_util::future::ready;

use crate::event::*;

/// Contains a PRIVMSG event
#[derive(Clone, Debug)]
pub struct SelectorResult<E, Inner>(E, PhantomData<Inner>);

impl<E, Inner> SelectorResult<E, Inner> {
    fn new(evt: E) -> Self {
        SelectorResult::<E, Inner>(evt, PhantomData)
    }
}

macro_rules! impl_selector {
    ($fn_name: ident, $enum_variant: ident, $inner_type: ty) => {
        impl<E: Borrow<Event<String>>> Deref for SelectorResult<E, $inner_type> {
            type Target = EventData<String, $inner_type>;

            fn deref(&self) -> &Self::Target {
                match self.0.borrow() {
                    Event::$enum_variant(evt) => evt,
                    _ => unsafe { unreachable_unchecked() },
                }
            }
        }

        #[allow(missing_docs)]
        pub fn $fn_name<E: Borrow<Event<String>>>(
            item: E,
        ) -> impl Future<Output = Option<SelectorResult<E, $inner_type>>> {
            ready(match item.borrow() {
                Event::$enum_variant(_) => Some(SelectorResult::new(item)),
                _ => None,
            })
        }
    };
}

impl_selector!(priv_msg, PrivMsg, PrivMsgEvent<String>);
impl_selector!(whisper, Whisper, WhisperEvent<String>);
impl_selector!(join, Join, JoinEvent<String>);
impl_selector!(mode, Mode, ModeChangeEvent<String>);
impl_selector!(names, Names, NamesListEvent<String>);
impl_selector!(end_of_names, EndOfNames, EndOfNamesEvent<String>);
impl_selector!(part, Part, PartEvent<String>);
impl_selector!(clear_chat, ClearChat, ClearChatEvent<String>);
impl_selector!(clear_msg, ClearMsg, ClearMsgEvent<String>);
impl_selector!(host, Host, HostEvent<String>);
impl_selector!(notice, Notice, NoticeEvent<String>);
impl_selector!(reconnect, Reconnect, ReconnectEvent);
impl_selector!(room_state, RoomState, RoomStateEvent<String>);
impl_selector!(user_notice, UserNotice, UserNoticeEvent<String>);
impl_selector!(user_state, UserState, UserStateEvent<String>);
impl_selector!(capability, Capability, CapabilityEvent<String>);
impl_selector!(connect_message, ConnectMessage, ConnectMessageEvent<String>);
