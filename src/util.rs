use std::borrow::Borrow;
use std::task::Poll;

use futures_sink::Sink;
use tokio::sync::{broadcast, mpsc, mpsc::error::TrySendError};

use crate::EventChannelError;

pub(crate) trait RefToString {
    fn ref_to_string(&self) -> String;
}

impl<T: Borrow<str>> RefToString for T {
    #[inline]
    fn ref_to_string(&self) -> String {
        self.borrow().into()
    }
}

pub(crate) struct InternalSender<T>(pub(crate) T);

impl<T> Sink<T> for InternalSender<mpsc::Sender<T>> {
    type Error = EventChannelError;

    fn poll_ready(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx).map_err(|_| EventChannelError::Closed)
    }

    fn start_send(mut self: std::pin::Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        match self.0.try_send(item) {
            Ok(_) => Ok(()),
            Err(TrySendError::Full(_item)) => Err(EventChannelError::Full),
            Err(TrySendError::Closed(_item)) => Err(EventChannelError::Closed),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl<T> Sink<T> for InternalSender<broadcast::Sender<T>> {
    type Error = EventChannelError;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        self.0.send(item).map_err(|_| EventChannelError::Closed)?;
        Ok(())
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
