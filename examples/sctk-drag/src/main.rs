use iced::{
    event::wayland::DataSourceEvent,
    subscription,
    wayland::{
        actions::data_device::DndIcon,
        data_device::{accept_mime_type, request_dnd_data, set_actions, finish_dnd},
        InitialSurface,
    },
    wayland::{
        data_device::{send_dnd_data, start_drag},
        platform_specific,
    },
    widget::{self, column, container, dnd_listener, mouse_listener, text},
    window, Application, Color, Command, Element, Subscription, Theme,
};
use iced_style::application;
use sctk::reexports::client::protocol::wl_data_device_manager::DndAction;

fn main() {
    DndTest::run(iced::Settings::default());
}

const SUPPORTED_MIME_TYPES: &'static [&'static str; 6] = &[
    "text/plain;charset=utf-8",
    "text/plain;charset=UTF-8",
    "UTF8_STRING",
    "STRING",
    "text/plain",
    "TEXT",
];

#[derive(Debug, Clone, Default)]
enum DndState {
    #[default]
    None,
    Some(Vec<String>),
    Drop,
}

#[derive(Debug, Clone, Default)]
pub struct DndTest {
    /// option with the dragged text
    source: Option<String>,
    /// is the dnd over the target
    target: DndState,
    current_text: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Enter(Vec<String>),
    Leave,
    Drop,
    DndData(Vec<u8>),
    Ignore,
    StartDnd,
    SendSourceData(String),
    SourceFinished,
}

impl Application for DndTest {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = Theme;

    fn new(_flags: ()) -> (DndTest, Command<Self::Message>) {
        let current_text = String::from("Hello, world!");

        (
            DndTest {
                current_text,
                ..DndTest::default()
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("DndTest")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Enter(mut mime_types) => {
                println!("Enter: {:?}", mime_types);
                let mut cmds =
                    vec![set_actions(DndAction::Copy, DndAction::all())];
                mime_types.retain(|mime_type| {
                    SUPPORTED_MIME_TYPES.contains(&mime_type.as_str())
                });
                for m in &mime_types {
                    cmds.push(accept_mime_type(Some(m.clone())));
                }

                self.target = DndState::Some(mime_types);
                return Command::batch(cmds);
            }
            Message::Leave => {
                if let DndState::Drop = &self.target {
                    println!("Leave: {:?}", self.target);
                    return Command::none();
                }
                self.target = DndState::None;
                return Command::batch(vec![
                    accept_mime_type(None),
                    set_actions(DndAction::None, DndAction::empty()),
                ]);
            }
            Message::Drop => {
                if let DndState::Some(m) = &self.target {
                    let m = m[0].clone();
                    println!("Drop: {:?}", self.target);
                    self.target = DndState::Drop;
                    return request_dnd_data(m.clone());
                }
            }
            Message::DndData(data) => {
                println!("DndData: {:?}", data);
                if data.is_empty() {
                    return Command::none();
                }
                if matches!(self.target, DndState::Drop) {
                    self.current_text = String::from_utf8(data).unwrap();
                    self.target = DndState::None;
                    return finish_dnd();
                }
            }
            Message::SendSourceData(mime_type) => {
                println!("Sending source data");
                if let Some(source) = &self.source {
                    return send_dnd_data(
                        source.chars().rev().collect::<String>().into_bytes(),
                    );
                }
                println!("No source");
            }
            Message::SourceFinished => {
                println!("Removing source");
                self.source = None;
                
            }
            Message::StartDnd => {
                println!("Starting DnD");
                self.source = Some(self.current_text.clone());
                return start_drag(
                    SUPPORTED_MIME_TYPES
                        .iter()
                        .map(|t| t.to_string())
                        .collect(),
                    DndAction::Move,
                    window::Id::new(0),
                    Some(DndIcon::Custom(window::Id::new(1))),
                );
            }
            Message::Ignore => {}
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        subscription::events_with(|event, status| {
            if let iced::Event::PlatformSpecific(
                iced::event::PlatformSpecific::Wayland(
                    iced::event::wayland::Event::DataSource(source_event),
                ),
            ) = event
            {
                match source_event {
                    DataSourceEvent::SendDndData(mime_type) => {
                        if SUPPORTED_MIME_TYPES.contains(&mime_type.as_str()) {
                            Some(Message::SendSourceData(mime_type))
                        } else {
                            None
                        }
                    }
                    DataSourceEvent::DndFinished
                    | DataSourceEvent::Cancelled => {
                        Some(Message::SourceFinished)
                    }
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    fn view(&self, id: window::Id) -> Element<Self::Message> {
        if id == window::Id::new(1) {
            return text(&self.current_text).into();
        }
        column![
            dnd_listener(
                container(text(format!(
                    "Drag text here: {}",
                    &self.current_text
                )))
                .style(if matches!(self.target, DndState::Some(_)) {
                    <iced_style::Theme as container::StyleSheet>::Style::Custom(
                        Box::new(CustomTheme),
                    )
                } else {
                    Default::default()
                })
                .padding(20)
            )
            .on_enter(|_, mime_types: Vec<String>, _| {
                if mime_types.iter().any(|mime_type| {
                    SUPPORTED_MIME_TYPES.contains(&mime_type.as_str())
                }) {
                    Message::Enter(mime_types)
                } else {
                    Message::Ignore
                }
            })
            .on_exit(Message::Leave)
            .on_drop(Message::Drop)
            .on_data(|mime_type, data| {
                if matches!(self.target, DndState::Drop) {
                    Message::DndData(data)
                } else {
                    Message::Ignore
                }
            }),
            mouse_listener(
                container(text(format!(
                    "Drag me: {}",
                    &self.current_text.chars().rev().collect::<String>()
                )))
                .style(if self.source.is_some() {
                    <iced_style::Theme as container::StyleSheet>::Style::Custom(
                        Box::new(CustomTheme),
                    )
                } else {
                    Default::default()
                })
                .padding(20)
            )
            .on_press(Message::StartDnd)
        ]
        .into()
    }

    fn close_requested(&self, id: window::Id) -> Self::Message {
        Message::Ignore
    }
}

pub struct CustomTheme;

impl container::StyleSheet for CustomTheme {
    type Style = iced::Theme;

    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        container::Appearance {
            border_color: Color::from_rgb(1.0, 0.0, 0.0),
            border_radius: 2.0,
            border_width: 2.0,
            ..container::Appearance::default()
        }
    }
}
