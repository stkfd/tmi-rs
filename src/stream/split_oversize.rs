//! Splits oversize messsages

use std::collections::VecDeque;
use std::pin::Pin;

use futures_core::task::{Context, Poll};
use futures_core::Stream;
use futures_util::stream::FuturesUnordered;

use crate::stream::{message_responder_channel, MessageResponder, SentClientMessage};
use crate::ClientMessage;

/// Splits oversize messages into multiple messages. See [`split_oversize`](../trait.SendStreamExt.html#method.split_oversize)
pub struct SplitOversize<St>
where
    St: Stream<Item = SentClientMessage> + Unpin,
{
    stream: St,
    pending_messages: VecDeque<SentClientMessage>,
    max_len: usize,
}

impl<St> Stream for SplitOversize<St>
where
    St: Stream<Item = SentClientMessage> + Unpin,
{
    type Item = SentClientMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use futures_util::stream::StreamExt;

        if !self.pending_messages.is_empty() {
            return Poll::Ready(self.pending_messages.pop_front());
        }

        let max_len = self.max_len;

        match (&mut self.stream).poll_next_unpin(cx) {
            Poll::Ready(Some(sent_msg)) => {
                let SentClientMessage {
                    message: msg,
                    responder,
                } = sent_msg;
                match msg {
                    ClientMessage::PrivMsg {
                        ref message,
                        ref channel,
                    } => {
                        if message.len() > self.max_len {
                            self.queue_split_message(message, max_len, responder, |chunk| {
                                ClientMessage::message(channel.to_string(), chunk)
                            });

                            Poll::Ready(self.pop_queue())
                        } else {
                            Poll::Ready(Some(SentClientMessage {
                                message: msg,
                                responder,
                            }))
                        }
                    }
                    ClientMessage::Whisper {
                        ref recipient,
                        ref message,
                    } => {
                        // subtract the length of "/w username " from the length limit
                        let whisper_max_len = max_len - 4 - recipient.len();
                        self.queue_split_message(message, whisper_max_len, responder, |chunk| {
                            ClientMessage::whisper(recipient.to_string(), chunk)
                        });

                        Poll::Ready(self.pop_queue())
                    }
                    // not a channel message, just forward
                    _ => Poll::Ready(Some(SentClientMessage {
                        message: msg,
                        responder,
                    })),
                }
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<St> SplitOversize<St>
where
    St: Stream<Item = SentClientMessage> + Unpin,
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
        responder: MessageResponder,
        map_to_message: impl Fn(String) -> ClientMessage,
    ) {
        use tokio::stream::StreamExt;

        // receivers for send results from the individual chunks of the message
        let chunk_results = FuturesUnordered::new();

        self.pending_messages.extend(
            string_chunks(message, max_len)
                .into_iter()
                .map(map_to_message)
                .map(|message| {
                    let (tx, rx) = message_responder_channel();
                    chunk_results.push(rx);
                    SentClientMessage {
                        message,
                        responder: tx,
                    }
                }),
        );

        tokio::spawn(async {
            if let Some(Ok(result)) = chunk_results
                .take_while(|res| res.is_ok())
                .fold(None, |_, res| Some(res))
                .await
            {
                responder.send(result).ok();
            }
        });
    }

    fn pop_queue(&mut self) -> Option<SentClientMessage> {
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

    use crate::stream::{message_responder_channel, SendStreamExt, SentClientMessage};
    use crate::ClientMessage;

    #[tokio::test]
    async fn test_splitting() {
        let message = SentClientMessage {
            message: ClientMessage::message("#channel", repeat('a').take(550).collect::<String>()),
            responder: message_responder_channel().0,
        };
        let mut stream = stream::iter(vec![message]).split_oversize(500);
        for len in &[500, 50] {
            match stream.next().await.unwrap().message {
                ClientMessage::PrivMsg { message, .. } => {
                    assert_eq!(message.len(), *len);
                }
                _ => unreachable!(),
            }
        }
    }

    #[tokio::test]
    async fn test_whisper_splitting() {
        let message = SentClientMessage {
            message: ClientMessage::whisper("joey", repeat('a').take(550).collect::<String>()),
            responder: message_responder_channel().0,
        };
        let mut stream = stream::iter(vec![message]).split_oversize(500);
        for len in &[500, 66] {
            match stream.next().await.unwrap().message {
                ClientMessage::Whisper { message, .. } => {
                    assert_eq!(format!("/w joey {}", message).len(), *len);
                }
                _ => unreachable!(),
            }
        }
    }
}
