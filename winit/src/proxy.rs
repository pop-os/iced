use dnd::{DndEvent, DndSurface};

use crate::{
    application::UserEventWrapper,
    futures::futures::{
        channel::mpsc,
        task::{Context, Poll},
        Sink,
    },
};
use std::pin::Pin;

/// An event loop proxy that implements `Sink`.
#[derive(Debug)]
pub struct Proxy<Message: 'static> {
    pub(crate) raw: winit::event_loop::EventLoopProxy<Message>,
}

impl<Message: 'static> Clone for Proxy<Message> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
        }
    }
}

impl<Message: 'static> Proxy<Message> {
    /// Creates a new [`Proxy`] from an `EventLoopProxy`.
    pub fn new(raw: winit::event_loop::EventLoopProxy<Message>) -> Self {
        Self { raw }
    }
}

impl<Message: 'static> Sink<Message> for Proxy<Message> {
    type Error = mpsc::SendError;

    fn poll_ready(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(
        self: Pin<&mut Self>,
        message: Message,
    ) -> Result<(), Self::Error> {
        let _ = self.raw.send_event(message);

        Ok(())
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl<M> dnd::Sender<DndSurface> for Proxy<UserEventWrapper<M>> {
    fn send(
        &self,
        event: DndEvent<DndSurface>,
    ) -> Result<(), std::sync::mpsc::SendError<DndEvent<DndSurface>>> {
        self.raw
            .send_event(UserEventWrapper::Dnd(event))
            .map_err(|_err| {
                std::sync::mpsc::SendError(DndEvent::Offer(
                    None,
                    dnd::OfferEvent::Leave,
                ))
            })
    }
}
