use iced::{
    wayland::{
        data_device::{accept_mime_type, request_dnd_data, set_actions},
        InitialSurface,
    },
    widget::{self, column, container, dnd_listener},
    window, Application, Command, Element, Subscription, Theme,
};
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
    RequestSourceData(String),
    SourceFinished,
    Ignore,
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
                self.current_text = String::from_utf8(data).unwrap();
            }
            Message::RequestSourceData(mime_type) => {
                self.current_text = String::from("RequestSourceData");
            }
            Message::SourceFinished => {
                self.current_text = String::from("SourceFinished");
            }
            Message::Ignore => {}
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::none()
    }

    fn view(&self, _: window::Id) -> Element<Self::Message> {
        container(column![dnd_listener(widget::Text::new(&self.current_text))
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
            })])
        .padding(20)
        .into()
    }

    fn close_requested(&self, id: window::Id) -> Self::Message {
        Message::Ignore
    }
}
