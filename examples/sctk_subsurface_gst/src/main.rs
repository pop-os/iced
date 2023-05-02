// Shows a subsurface with a 1x1 px red buffer, stretch to window size

use iced::{
    wayland::InitialSurface, widget::text, window, Application, Command,
    Element, Length, Subscription, Theme,
};
use iced_sctk::subsurface_widget::SubsurfaceBuffer;
use std::{env, path::Path};

mod pipewire;

fn main() {
    let args = env::args();
    if args.len() != 2 {
        eprintln!("usage: sctk_subsurface_gst [h264 mp4 path]");
        return;
    }
    let path = args.skip(1).next().unwrap();
    if !Path::new(&path).exists() {
        eprintln!("File `{path}` not found.");
        return;
    }
    let mut settings = iced::Settings::with_flags(path);
    settings.initial_surface = InitialSurface::XdgWindow(Default::default());
    SubsurfaceApp::run(settings).unwrap();
}

#[derive(Debug, Clone, Default)]
struct SubsurfaceApp {
    path: String,
    buffer: Option<SubsurfaceBuffer>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Pipewire(pipewire::Event),
}

impl Application for SubsurfaceApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = String;
    type Theme = Theme;

    fn new(flags: String) -> (SubsurfaceApp, Command<Self::Message>) {
        (
            SubsurfaceApp {
                path: flags,
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
            Message::Pipewire(evt) => match evt {
                pipewire::Event::Frame(subsurface_buffer) => {
                    self.buffer = Some(subsurface_buffer);
                }
            },
        }
        Command::none()
    }

    fn view(&self, _id: window::Id) -> Element<Self::Message> {
        if let Some(buffer) = &self.buffer {
            iced_sctk::subsurface_widget::Subsurface::new(1, 1, buffer)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            text("No subsurface").into()
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        pipewire::subscription(&self.path).map(Message::Pipewire)
    }
}
