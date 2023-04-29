use iced::widget::{button, column, text, Column, scrollable};
use iced::{Alignment, Element, Sandbox, Settings, Length};

pub fn main() -> iced::Result {
    env_logger::init();
    Buttons::run(Settings::default())
}

struct Buttons {
    value: i32,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    IncrementPressed,
    DecrementPressed,
}

impl Sandbox for Buttons {
    type Message = Message;

    fn new() -> Self {
        Self { value: 0 }
    }

    fn title(&self) -> String {
        String::from("Counter - Iced")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::IncrementPressed => {
                self.value += 1;
            }
            Message::DecrementPressed => {
                self.value -= 1;
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let buttons = (0..self.value.max(2))
            .map(|v| 
                 if v % 2 == 0 {button("decrement").on_press(Message::DecrementPressed).into()} else {button("increment").on_press(Message::IncrementPressed).into()})
            .collect::<Vec<_>>();
        scrollable(Column::with_children(buttons)
        .padding(20)
        .align_items(Alignment::Center))
        .into()
    }
}
