mod dnd_destination;
mod dnd_source;

use std::{borrow::Cow, convert::Infallible};

use dnd_destination::dnd_destination;
use iced::{
    clipboard::mime::{AllowedMimeTypes, AsMimeTypes},
    platform_specific::{
        runtime::wayland::layer_surface::SctkLayerSurfaceSettings,
        shell::commands::layer_surface::get_layer_surface,
    },
    widget::{column, container, text},
    window, Element, Length, Task,
};
use iced_core::{
    widget::{tree, Text},
    Widget,
};

fn main() -> iced::Result {
    iced::daemon(DndTest::title, DndTest::update, DndTest::view)
        .run_with(DndTest::new)
    // iced::application(Todos::title, Todos::update, Todos::view)
    // .subscription(Todos::subscription)
    // .font(include_bytes!("../fonts/icons.ttf").as_slice())
    // .window_size((500.0, 800.0))
    // .run_with(Todos::new)
}

const SUPPORTED_MIME_TYPES: &'static [&'static str; 6] = &[
    "text/plain;charset=utf-8",
    "text/plain;charset=UTF-8",
    "UTF8_STRING",
    "STRING",
    "text/plain",
    "TEXT",
];

#[derive(Debug, Default, Clone)]
pub struct MyDndString(String);

impl AllowedMimeTypes for MyDndString {
    fn allowed() -> std::borrow::Cow<'static, [String]> {
        std::borrow::Cow::Owned(vec![
            "text/plain;charset=utf-8".to_string(),
            "text/plain;charset=UTF-8".to_string(),
            "UTF8_STRING".to_string(),
            "STRING".to_string(),
            "text/plain".to_string(),
            "TEXT".to_string(),
        ])
    }
}

impl TryFrom<(Vec<u8>, String)> for MyDndString {
    type Error = Infallible;

    fn try_from(value: (Vec<u8>, String)) -> Result<Self, Self::Error> {
        Ok(MyDndString(
            String::from_utf8_lossy(value.0.as_slice()).to_string(),
        ))
    }
}

impl AsMimeTypes for MyDndString {
    fn available(&self) -> std::borrow::Cow<'static, [String]> {
        std::borrow::Cow::Owned(vec![
            "text/plain;charset=utf-8".to_string(),
            "text/plain;charset=UTF-8".to_string(),
            "UTF8_STRING".to_string(),
            "STRING".to_string(),
            "text/plain".to_string(),
            "TEXT".to_string(),
        ])
    }

    fn as_bytes(
        &self,
        _mime_type: &str,
    ) -> Option<std::borrow::Cow<'static, [u8]>> {
        Some(Cow::Owned(self.0.clone().into_bytes()))
    }
}

#[derive(Debug, Clone)]
pub struct DndTest {
    /// option with the dragged text
    source: Option<String>,
    /// is the dnd over the target
    current_text: String,
    /// main id
    id: iced_core::window::Id,
}

#[derive(Debug, Clone)]
pub enum Message {
    Drag,
    DndData(MyDndString),
}

impl DndTest {
    fn new() -> (DndTest, Task<Message>) {
        let current_text = String::from("Hello, world!");
        let mut s = SctkLayerSurfaceSettings::default();
        s.size_limits = s.size_limits.min_width(100.0).max_width(400.0);
        s.size = Some((Some(500), Some(600)));
        // s.anchor = Anchor::TOP.union(Anchor::BOTTOM);
        (
            DndTest {
                current_text,
                source: None,
                id: iced_core::window::Id::unique(),
            },
            get_layer_surface(s),
        )
    }

    fn title(&self, _id: window::Id) -> String {
        String::from("DndTest")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DndData(s) => {
                dbg!(&s);
                self.current_text = s.0;
            }
            _ => {}
        }
        Task::none()
    }

    fn view(&self, _id: window::Id) -> Element<Message> {
        let s = self.current_text.chars().rev().collect::<String>();
        let s2 = s.clone();
        column![
            dnd_destination::dnd_destination_for_data::<MyDndString, Message>(
                container(text(format!(
                    "Drag text here: {}",
                    &self.current_text
                )))
                .width(Length::Fill)
                .height(Length::FillPortion(1))
                .padding(20),
                |data, _| {
                    dbg!("got data");
                    Message::DndData(data.unwrap_or_default())
                }
            )
            .drag_id(1)
            .on_enter(|_, _, m| {
                dbg!(m);
                Message::Drag
            })
            .on_action_selected(|a| {
                dbg!(a);
                Message::Drag
            })
            .on_drop(|_, _| {
                dbg!("drop");
                Message::Drag
            })
            .on_motion(|x, y| {
                dbg!(x, y);
                Message::Drag
            }),
            dnd_source::dnd_source(
                container(text(format!(
                    "Drag me: {}",
                    &self.current_text.chars().rev().collect::<String>()
                )))
                .width(Length::Fill)
                .height(Length::FillPortion(1))
                .padding(20)
            )
            .drag_threshold(5.0)
            .drag_icon(move || {
                let t: Text<'static, iced::Theme, iced::Renderer> =
                    text(s.clone());
                let state = <iced_core::widget::Text<
                    'static,
                    iced::Theme,
                    iced::Renderer,
                > as iced_core::Widget<(), iced::Theme, iced::Renderer>>::state(
                    &t,
                );
                (
                    Element::<'static, (), iced::Theme, iced::Renderer>::from(
                        t,
                    ),
                    state,
                )
            })
            .drag_content(move || { MyDndString(s2.clone()) })
        ]
        .width(Length::Fill)
        .into()
    }
}

pub struct CustomTheme;
