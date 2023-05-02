use iced::event::listen_raw;
use iced::wayland::application::session_lock;
use iced::{
    event::wayland::{Event as WaylandEvent, OutputEvent, SessionLockEvent},
    wayland::InitialSurface,
    widget::text,
    window, Application, Command, Element, Subscription, Theme,
};
use iced_runtime::window::Id as SurfaceId;

fn main() {
    let mut settings = iced::Settings::default();
    settings.initial_surface = InitialSurface::None;
    Locker::run(settings).unwrap();
}

#[derive(Debug, Clone, Default)]
struct Locker {
    exit: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    WaylandEvent(WaylandEvent),
    TimeUp,
    Ignore,
}

impl Application for Locker {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = Theme;

    fn new(_flags: ()) -> (Locker, Command<Self::Message>) {
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

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
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
                        return iced::Command::perform(
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
        Command::none()
    }

    fn view(&self, id: window::Id) -> Element<Self::Message> {
        text(format!("Lock Surface {:?}", id)).into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        listen_raw(|evt, _| {
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
