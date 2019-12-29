//! Deduplicator to bypass Twitch's 30 second duplicate message detection

use std::pin::Pin;
use std::time::Duration;

use fnv::FnvHashMap;
use futures_core::task::{Context, Poll};
use futures_core::Stream;
use futures_util::StreamExt;
use tokio::time::Instant;

use crate::ClientMessage;

// invisible character to add to duplicate messages
const INVIS_CHAR: char = '\u{0}';

struct MessageRecord {
    sent_at: Instant,
    message: String,
}

/// Deduplicator to bypass Twitch's 30 second duplicate message detection. See [`dedup`](../trait.SendStreamExt.html#method.dedup).
pub struct DedupMessages<St>
where
    St: Stream<Item = ClientMessage<String>> + Unpin,
{
    // map that holds the last message for each channel
    sent_messages: FnvHashMap<String, MessageRecord>,
    // source stream
    stream: St,
}

impl<St> Stream for DedupMessages<St>
where
    St: Stream<Item = ClientMessage<String>> + Unpin,
{
    type Item = ClientMessage<String>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<ClientMessage<String>>> {
        match (&mut self.stream).poll_next_unpin(cx) {
            Poll::Ready(Some(mut msg)) => match msg {
                ClientMessage::PrivMsg {
                    ref channel,
                    ref mut message,
                } => {
                    (&mut self).dedup_message(channel, message);
                    Poll::Ready(Some(msg))
                }
                // not a channel message, just forward
                _ => Poll::Ready(Some(msg)),
            },
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

static DEDUP_DURATION: Duration = Duration::from_secs(30);

impl<St> DedupMessages<St>
where
    St: Stream<Item = ClientMessage<String>> + Unpin,
{
    pub(crate) fn new(stream: St) -> Self {
        Self {
            sent_messages: Default::default(),
            stream,
        }
    }

    fn dedup_message(&mut self, channel: &str, msg: &mut String) {
        let instant = Instant::now();

        let matching_message = self.sent_messages.get(channel).filter(|message_record| {
            let MessageRecord {
                sent_at,
                message: past_message,
            } = message_record;
            *sent_at >= instant - DEDUP_DURATION && past_message == msg.as_str()
        });
        if matching_message.is_some() {
            msg.push(INVIS_CHAR);
        }

        self.sent_messages.insert(
            channel.to_string(),
            MessageRecord {
                sent_at: instant,
                message: (*msg).to_string(),
            },
        );
    }
}

#[cfg(test)]
mod test {
    use std::iter::repeat;
    use std::time::Duration;

    use futures::channel::mpsc::unbounded;
    use futures::{stream, SinkExt, StreamExt};
    use tokio::time::{advance, delay_for, pause};

    use crate::stream::SendStreamExt;
    use crate::ClientMessage;

    #[tokio::test]
    async fn test_dedup() {
        let test_message = ClientMessage::message("#channel", "test");
        let input_stream = stream::iter(repeat(test_message).take(3));

        let received = input_stream.dedup().collect::<Vec<_>>().await;
        assert_eq!(
            received,
            vec![
                ClientMessage::message("#channel", "test"),
                ClientMessage::message("#channel", "test\u{0}"),
                ClientMessage::message("#channel", "test"),
            ]
        );
    }

    #[tokio::test]
    async fn test_dedup_timed() {
        pause();
        let (mut snd, recv) = unbounded();

        tokio::spawn(async move {
            let test_message = ClientMessage::message("#channel", "test");
            loop {
                snd.send(test_message.clone()).await.unwrap();
                delay_for(Duration::from_secs(30)).await;
            }
        });

        let mut recv = recv.dedup();

        assert_eq!(
            recv.next().await.unwrap(),
            ClientMessage::message("#channel", "test"),
        );
        advance(Duration::from_secs_f64(30.01)).await;
        assert_eq!(
            recv.next().await.unwrap(),
            ClientMessage::message("#channel", "test"),
        );
    }
}
