// Shows a subsurface with a 1x1 px red buffer, stretch to window size

use iced::{
    event::wayland::Event as WaylandEvent,
    platform_specific::shell::subsurface_widget::{self, SubsurfaceBuffer},
    widget::text,
    window::{self, Id, Settings},
    Element, Length, Subscription, Task,
};
use sctk::reexports::client::{Connection, Proxy};

mod wayland;

fn main() -> iced::Result {
    iced::daemon(
        SubsurfaceApp::title,
        SubsurfaceApp::update,
        SubsurfaceApp::view,
    )
    .subscription(SubsurfaceApp::subscription)
    .run_with(SubsurfaceApp::new)
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
    Pressed(&'static str),
    Id(Id),
}

impl SubsurfaceApp {
    fn new() -> (SubsurfaceApp, Task<Message>) {
        (
            SubsurfaceApp {
                ..SubsurfaceApp::default()
            },
            iced::window::open(Settings {
                ..Default::default()
            })
            .1
            .map(Message::Id),
        )
    }

    fn title(&self, _id: window::Id) -> String {
        String::from("SubsurfaceApp")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
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
            Message::Pressed(side) => println!("{side} surface pressed"),
            Message::Id(_) => {}
        }
        Task::none()
    }

    fn view(&self, _id: window::Id) -> Element<Message> {
        if let Some(buffer) = &self.red_buffer {
            iced::widget::row![
                iced::widget::button(
                    subsurface_widget::Subsurface::new(1, 1, buffer)
                        .width(Length::Fill)
                        .height(Length::Fill)
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .on_press(Message::Pressed("left")),
                iced::widget::button(
                    subsurface_widget::Subsurface::new(1, 1, buffer)
                        .width(Length::Fill)
                        .height(Length::Fill)
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .on_press(Message::Pressed("right"))
            ]
            .into()
        } else {
            text("No subsurface").into()
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![iced::event::listen_with(|evt, _, _| {
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
