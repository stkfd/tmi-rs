//! Splits oversize messsages

use std::collections::VecDeque;
use std::pin::Pin;

use futures_core::task::{Context, Poll};
use futures_core::Stream;
use futures_util::StreamExt;

use crate::ClientMessage;

/// Splits oversize messages into multiple messages. See [`split_oversize`](../trait.SendStreamExt.html#method.split_oversize)
pub struct SplitOversize<St>
where
    St: Stream<Item = ClientMessage<String>> + Unpin,
{
    stream: St,
    pending_messages: VecDeque<ClientMessage<String>>,
    max_len: usize,
}

impl<St> Stream for SplitOversize<St>
where
    St: Stream<Item = ClientMessage<String>> + Unpin,
{
    type Item = ClientMessage<String>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if !self.pending_messages.is_empty() {
            return Poll::Ready(self.pending_messages.pop_front());
        }

        let max_len = self.max_len;

        match (&mut self.stream).poll_next_unpin(cx) {
            Poll::Ready(Some(msg)) => match msg {
                ClientMessage::PrivMsg {
                    ref message,
                    ref channel,
                } => {
                    if message.len() > self.max_len {
                        self.queue_split_message(message, max_len, |chunk| {
                            ClientMessage::message(channel.to_string(), chunk)
                        });

                        Poll::Ready(self.pop_queue())
                    } else {
                        Poll::Ready(Some(msg))
                    }
                }
                ClientMessage::Whisper {
                    ref recipient,
                    ref message,
                } => {
                    // subtract the length of "/w username " from the length limit
                    let whisper_max_len = max_len - 4 - recipient.len();
                    self.queue_split_message(message, whisper_max_len, |chunk| {
                        ClientMessage::whisper(recipient.to_string(), chunk)
                    });

                    Poll::Ready(self.pop_queue())
                }
                // not a channel message, just forward
                _ => Poll::Ready(Some(msg)),
            },
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<St> SplitOversize<St>
where
    St: Stream<Item = ClientMessage<String>> + Unpin,
{
    pub(crate) fn new(stream: St, max_len: usize) -> Self {
        SplitOversize {
            stream,
            pending_messages: VecDeque::new(),
            max_len,
        }
    }

    fn queue_split_message(
        &mut self,
        message: &str,
        max_len: usize,
        map_to_message: impl Fn(String) -> ClientMessage<String>,
    ) {
        self.pending_messages.extend(
            string_chunks(message, max_len)
                .into_iter()
                .map(map_to_message),
        );
    }

    fn pop_queue(&mut self) -> Option<ClientMessage<String>> {
        self.pending_messages.pop_front()
    }
}

fn string_chunks(string: &str, sub_len: usize) -> Vec<String> {
    let mut subs = Vec::with_capacity(string.len() / sub_len);
    let mut iter = string.chars();
    let mut pos = 0;

    while pos < string.len() {
        let mut len = 0;
        for ch in iter.by_ref().take(sub_len) {
            len += ch.len_utf8();
        }
        subs.push(string[pos..pos + len].to_string());
        pos += len;
    }
    subs
}

#[cfg(test)]
mod test {
    use std::iter::repeat;

    use futures::{stream, StreamExt};

    use crate::stream::SendStreamExt;
    use crate::ClientMessage;

    #[tokio::test]
    async fn test_splitting() {
        let message = ClientMessage::message("#channel", repeat('a').take(550).collect::<String>());
        let mut stream = stream::iter(vec![message]).split_oversize(500);
        for len in &[500, 50] {
            match stream.next().await.unwrap() {
                ClientMessage::PrivMsg { message, .. } => {
                    assert_eq!(message.len(), *len);
                }
                _ => unreachable!(),
            }
        }
    }

    #[tokio::test]
    async fn test_whisper_splitting() {
        let message = ClientMessage::whisper("joey", repeat('a').take(550).collect::<String>());
        let mut stream = stream::iter(vec![message]).split_oversize(500);
        for len in &[500, 66] {
            match stream.next().await.unwrap() {
                ClientMessage::Whisper { message, .. } => {
                    assert_eq!(format!("/w joey {}", message).len(), *len);
                }
                _ => unreachable!(),
            }
        }
    }
}
