use iced::event::listen_raw;
use iced::Task;
use iced::{
    event::wayland::{Event as WaylandEvent, OutputEvent, SessionLockEvent},
    platform_specific::shell::commands::session_lock,
    widget::text,
    window, Element, Subscription,
};

fn main() -> iced::Result {
    iced::daemon(Locker::title, Locker::update, Locker::view)
        .subscription(Locker::subscription)
        .run_with(Locker::new)
}

#[derive(Debug, Clone, Default)]
struct Locker {
    _exit: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    WaylandEvent(WaylandEvent),
    TimeUp,
    Ignore,
}

impl Locker {
    fn new() -> (Locker, Task<Message>) {
        (
            Locker {
                ..Locker::default()
            },
            session_lock::lock(),
        )
    }

    fn title(&self, _id: window::Id) -> String {
        String::from("Locker")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WaylandEvent(evt) => match evt {
                WaylandEvent::Output(evt, output) => match evt {
                    OutputEvent::Created(_) => {
                        return session_lock::get_lock_surface(
                            window::Id::unique(),
                            output,
                        );
                    }
                    OutputEvent::Removed => {}
                    _ => {}
                },
                WaylandEvent::SessionLock(evt) => match evt {
                    SessionLockEvent::Locked => {
                        return iced::Task::perform(
                            async_std::task::sleep(
                                std::time::Duration::from_secs(5),
                            ),
                            |_| Message::TimeUp,
                        );
                    }
                    SessionLockEvent::Unlocked => {
                        // Server has processed unlock, so it's safe to exit
                        std::process::exit(0);
                    }
                    _ => {}
                },
                _ => {}
            },
            Message::TimeUp => {
                return session_lock::unlock();
            }
            Message::Ignore => {}
        }
        Task::none()
    }

    fn view(&self, id: window::Id) -> Element<Message> {
        text(format!("Lock Surface {:?}", id)).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        listen_raw(|evt, _, _| {
            if let iced::Event::PlatformSpecific(
                iced::event::PlatformSpecific::Wayland(evt),
            ) = evt
            {
                Some(Message::WaylandEvent(evt))
            } else {
                None
            }
        })
    }
}
