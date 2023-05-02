use crate::Event;
use crate::futures::futures::{
    Future, Sink, StreamExt,
    channel::mpsc,
    select,
    task::{Context, Poll},
};
use crate::graphics::shell;
use crate::runtime::Action;
use crate::runtime::window;
use std::hash::DefaultHasher;
use std::pin::Pin;

/// An event loop proxy with backpressure that implements `Sink`.
pub struct Proxy<T: 'static> {
    pub(crate) raw: winit::event_loop::EventLoopProxy,
    sender: mpsc::Sender<Action<T>>,
    event_sender: mpsc::UnboundedSender<Event<T>>,
    notifier: mpsc::Sender<usize>,
}

impl<T: 'static> Clone for Proxy<T> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            sender: self.sender.clone(),
            notifier: self.notifier.clone(),
            event_sender: self.event_sender.clone(),
        }
    }
}

impl<T: 'static> Proxy<T> {
    const MAX_SIZE: usize = 100;

    /// Creates a new [`Proxy`] from an `EventLoopProxy`.
    pub fn new(
        raw: winit::event_loop::EventLoopProxy,
        event_sender: mpsc::UnboundedSender<Event<T>>,
    ) -> (Self, impl Future<Output = ()>) {
        let (notifier, mut processed) = mpsc::channel(Self::MAX_SIZE);
        let (sender, mut receiver): (mpsc::Sender<Action<T>>, _) =
            mpsc::channel(Self::MAX_SIZE);
        let proxy = raw.clone();
        let event_sender_clone = event_sender.clone();

        let worker = async move {
            let mut count = 0;

            loop {
                if count < Self::MAX_SIZE {
                    select! {
                        message = receiver.select_next_some() => {
                            let _ = event_sender_clone.unbounded_send(Event::UserEvent(message));
                            let _ = proxy.wake_up();
                            count += 1;

                        }
                        amount = processed.select_next_some() => {
                            count = count.saturating_sub(amount);
                        }
                        complete => break,
                    }
                } else {
                    select! {
                        amount = processed.select_next_some() => {
                            count = count.saturating_sub(amount);
                        }
                        complete => break,
                    }
                }
            }
        };

        (
            Self {
                raw,
                sender,
                notifier,
                event_sender,
            },
            worker,
        )
    }

    /// Sends a value to the event loop.
    ///
    /// Note: This skips the backpressure mechanism with an unbounded
    /// channel. Use sparingly!
    pub fn send(&self, value: T) {
        self.send_action(Action::Output(value));
    }

    /// Sends an action to the event loop.
    ///
    /// Note: This skips the backpressure mechanism with an unbounded
    /// channel. Use sparingly!
    pub fn send_action(&self, action: Action<T>) {
        self.event_sender
            .unbounded_send(Event::UserEvent(action))
            .expect("Send message to event loop");
    }

    /// Frees an amount of slots for additional messages to be queued in
    /// this [`Proxy`].
    pub fn free_slots(&mut self, amount: usize) {
        let _ = self.notifier.start_send(amount);
    }
}

impl<T: 'static> Sink<Action<T>> for Proxy<T> {
    type Error = mpsc::SendError;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.sender.poll_ready(cx)
    }

    fn start_send(
        mut self: Pin<&mut Self>,
        action: Action<T>,
    ) -> Result<(), Self::Error> {
        self.sender.start_send(action)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        match self.sender.poll_ready(cx) {
            Poll::Ready(Err(ref e)) if e.is_disconnected() => {
                // If the receiver disconnected, we consider the sink to be flushed.
                Poll::Ready(Ok(()))
            }
            x => x,
        }
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.sender.disconnect();
        Poll::Ready(Ok(()))
    }
}

impl<T> shell::Notifier for Proxy<T>
where
    T: Send,
{
    fn request_redraw(&self) {
        self.send_action(Action::Window(window::Action::RedrawAll));
    }

    fn invalidate_layout(&self) {
        self.send_action(Action::Window(window::Action::RelayoutAll));
    }
}
