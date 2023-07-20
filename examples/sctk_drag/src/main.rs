use iced::{
    event::wayland::DataSourceEvent,
    subscription,
    wayland::{
        actions::data_device::DataFromMimeType, data_device::start_drag,
    },
    wayland::{
        actions::data_device::DndIcon,
        data_device::{
            accept_mime_type, finish_dnd, request_dnd_data, set_actions,
        },
        InitialSurface,
    },
    widget::{column, container, dnd_listener, dnd_source, text},
    window, Application, Color, Command, Element, Length, Theme,
};
use iced_style::application;
use sctk::reexports::client::protocol::wl_data_device_manager::DndAction;
use sctk::shell::wlr_layer::Anchor;

fn main() {
    let mut settings = iced::Settings::default();
    match &mut settings.initial_surface {
        InitialSurface::LayerSurface(s) => {
            s.size_limits = s.size_limits.min_width(100.0).max_width(400.0);
            s.size = Some((Some(400), None));
            s.anchor = Anchor::TOP.union(Anchor::BOTTOM);
        }
        _ => {}
    };
    DndTest::run(settings).unwrap();
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

pub struct MyDndString(String);

impl DataFromMimeType for MyDndString {
    fn from_mime_type(&self, mime_type: &str) -> Option<Vec<u8>> {
        if SUPPORTED_MIME_TYPES.contains(&mime_type) {
            Some(self.0.as_bytes().to_vec())
        } else {
            None
        }
    }
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
                    // Sent automatically now after a successful read of data following a drop.
                    // No longer needed here
                    // return finish_dnd();
                }
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
                    window::Id(0),
                    Some(DndIcon::Custom(window::Id(1))),
                    Box::new(MyDndString(
                        self.current_text.chars().rev().collect::<String>(),
                    )),
                );
            }
            Message::Ignore => {}
        }
        Command::none()
    }

    fn view(&self, id: window::Id) -> Element<Self::Message> {
        if id == window::Id(1) {
            return text(&self.current_text).into();
        }
        column![
            dnd_listener(
                container(text(format!(
                    "Drag text here: {}",
                    &self.current_text
                )))
                .width(Length::Fill)
                .height(Length::FillPortion(1))
                .style(if matches!(self.target, DndState::Some(_)) {
                    <iced_style::Theme as container::StyleSheet>::Style::Custom(
                        Box::new(CustomTheme),
                    )
                } else {
                    Default::default()
                })
                .padding(80)
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
            dnd_source(
                container(text(format!(
                    "Drag me: {}",
                    &self.current_text.chars().rev().collect::<String>()
                )))
                .width(Length::Fill)
                .height(Length::FillPortion(1))
                .style(if self.source.is_some() {
                    <iced_style::Theme as container::StyleSheet>::Style::Custom(
                        Box::new(CustomTheme),
                    )
                } else {
                    Default::default()
                })
                .padding(80)
            )
            .drag_threshold(5.0)
            .on_drag(|_| Message::StartDnd)
            .on_finished(Message::SourceFinished)
            .on_cancelled(Message::SourceFinished)
        ]
        .into()
    }

    fn close_requested(&self, id: window::Id) -> Self::Message {
        Message::Ignore
    }

    fn style(&self) -> <Self::Theme as application::StyleSheet>::Style {
        <Self::Theme as application::StyleSheet>::Style::Custom(Box::new(
            CustomTheme,
        ))
    }
}

pub struct CustomTheme;

impl container::StyleSheet for CustomTheme {
    type Style = iced::Theme;

    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        container::Appearance {
            border_color: Color::from_rgb(1.0, 0.0, 0.0),
            border_radius: 2.0.into(),
            border_width: 2.0,
            background: Some(Color::from_rgb(1.0, 0.0, 0.0).into()),
            ..container::Appearance::default()
        }
    }
}

impl iced_style::application::StyleSheet for CustomTheme {
    type Style = iced::Theme;

    fn appearance(&self, style: &Self::Style) -> application::Appearance {
        iced_style::application::Appearance {
            background_color: Color::from_rgba(1.0, 0.0, 1.0, 1.0),
            text_color: Color::from_rgb(0.0, 1.0, 0.0),
        }
    }
}
