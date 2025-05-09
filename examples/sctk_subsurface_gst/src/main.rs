// Shows a subsurface with a 1x1 px red buffer, stretch to window size

use iced::{
    platform_specific::shell::subsurface_widget::{self, SubsurfaceBuffer},
    widget::text,
    window, Element, Length, Subscription, Task,
};
use std::{env, path::Path};

mod pipewire;

fn main() -> iced::Result {
    let args = env::args();
    if args.len() != 2 {
        eprintln!("usage: sctk_subsurface_gst [h264 mp4 path]");
        return Ok(());
    }
    let path = args.skip(1).next().unwrap();
    if !Path::new(&path).exists() {
        eprintln!("File `{path}` not found.");
        return Ok(());
    }
    iced::daemon(
        SubsurfaceApp::title,
        SubsurfaceApp::update,
        SubsurfaceApp::view,
    )
    .subscription(SubsurfaceApp::subscription)
    .run_with(|| SubsurfaceApp::new(path))
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

impl SubsurfaceApp {
    fn new(flags: String) -> (SubsurfaceApp, Task<Message>) {
        (
            SubsurfaceApp {
                path: flags,
                ..SubsurfaceApp::default()
            },
            Task::none(),
        )
    }

    fn title(&self, _id: window::Id) -> String {
        String::from("SubsurfaceApp")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Pipewire(evt) => match evt {
                pipewire::Event::Frame(subsurface_buffer) => {
                    self.buffer = Some(subsurface_buffer);
                }
            },
        }
        Task::none()
    }

    fn view(&self, _id: window::Id) -> Element<Message> {
        if let Some(buffer) = &self.buffer {
            subsurface_widget::Subsurface::new(buffer.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            text("No subsurface").into()
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        pipewire::subscription(&self.path).map(Message::Pipewire)
    }
}
