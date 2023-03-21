use iced::{Application, Subscription, Element, widget, wayland::{InitialSurface}, Theme, Command, window};

fn main() {
}

#[derive(Debug, Clone)]
pub struct DndTest {
    internal_source: (),
    source: (),
    target: (),
    current_text: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Ignore,
}

impl Application for DndTest {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = Theme;

    fn new(_flags: ()) -> (DndTest, Command<Self::Message>) {
        let internal_source = ();
        let source = ();
        let target = ();
        let current_text = String::from("Hello, world!");

        (
            DndTest {
                internal_source,
                source,
                target,
                current_text,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("DndTest")
    }

    fn update(&mut self, _message: Self::Message) -> Command<Self::Message> {
        Command::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::none()
    }

    fn view(&self, _: window::Id) -> Element<Self::Message> {
        widget::Text::new(&self.current_text)
            .into()
    }

    fn close_requested(&self, id: window::Id) -> Self::Message {
        Message::Ignore
    }
}
