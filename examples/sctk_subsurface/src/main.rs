// Shows a subsurface with a 1x1 px red buffer, stretch to window size

use iced::{
    event::wayland::Event as WaylandEvent, wayland::InitialSurface,
    widget::text, window, Application, Command, Element, Length, Subscription,
    Theme,
};
use iced_sctk::subsurface_widget::SubsurfaceBuffer;
use sctk::reexports::client::{Connection, Proxy};

mod wayland;

fn main() {
    let mut settings = iced::Settings::default();
    settings.initial_surface = InitialSurface::XdgWindow(Default::default());
    SubsurfaceApp::run(settings).unwrap();
}

#[derive(Debug, Clone, Default)]
struct SubsurfaceApp {
    connection: Option<Connection>,
    red_buffer: Option<SubsurfaceBuffer>,
}

#[derive(Debug, Clone)]
pub enum Message {
    WaylandEvent(WaylandEvent),
    Wayland(wayland::Event),
}

impl Application for SubsurfaceApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = Theme;

    fn new(_flags: ()) -> (SubsurfaceApp, Command<Self::Message>) {
        (
            SubsurfaceApp {
                ..SubsurfaceApp::default()
            },
            Command::none(),
        )
    }

    fn title(&self, _id: window::Id) -> String {
        String::from("SubsurfaceApp")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::WaylandEvent(evt) => match evt {
                WaylandEvent::Output(_evt, output) => {
                    if self.connection.is_none() {
                        if let Some(backend) = output.backend().upgrade() {
                            self.connection =
                                Some(Connection::from_backend(backend));
                        }
                    }
                }
                _ => {}
            },
            Message::Wayland(evt) => match evt {
                wayland::Event::RedBuffer(buffer) => {
                    self.red_buffer = Some(buffer);
                }
            },
        }
        Command::none()
    }

    fn view(&self, _id: window::Id) -> Element<Self::Message> {
        if let Some(buffer) = &self.red_buffer {
            iced_sctk::subsurface_widget::Subsurface::new(1, 1, buffer)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            text("No subsurface").into()
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let mut subscriptions = vec![iced::event::listen_with(|evt, _| {
            if let iced::Event::PlatformSpecific(
                iced::event::PlatformSpecific::Wayland(evt),
            ) = evt
            {
                Some(Message::WaylandEvent(evt))
            } else {
                None
            }
        })];
        if let Some(connection) = &self.connection {
            subscriptions
                .push(wayland::subscription(connection).map(Message::Wayland));
        }
        Subscription::batch(subscriptions)
    }
}
