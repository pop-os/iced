use env_logger::Env;
use iced::theme::Theme;

use iced::wayland::actions::window::SctkWindowSettings;
use iced::wayland::InitialSurface;
use iced::widget::button::focus;
use iced::widget::{button, row, text};
use iced::{window, Application, Element};
use iced::{Color, Command, Length, Settings};
use iced_native::layout::Limits;
use iced_style::application;

pub fn main() -> iced::Result {
    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", "debug")
        .write_style_or("MY_LOG_STYLE", "always");

    env_logger::init_from_env(env);
    Window::run(Settings {
        initial_surface: InitialSurface::XdgWindow(SctkWindowSettings {
            app_id: Some("com.system76.SctkWindow".into()),
            title: Some("Accessible Window Test".into()),
            autosize: true,
            size_limits: Limits::NONE
                .min_width(1)
                .min_height(1)
                .max_height(400)
                .max_width(400),
            ..Default::default()
        }),
        ..Default::default()
    })
}

#[derive(Debug, Default)]
struct Window {
    button_1_press_count: u32,
    button_2_press_count: u32,
    id_ctr: u32,
}

#[derive(Debug, Clone)]
enum Message {
    Press1,
    Press2,
    Ignore,
}

impl Application for Window {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Window, Command<Message>) {
        (
            Window {
                id_ctr: 2,
                ..Default::default()
            },
            Command::none(),
        )
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Ignore => {}
            Message::Press1 => {
                self.button_1_press_count += 1;
                return focus(button::Id::new(format!(
                    "button one {}",
                    self.button_1_press_count
                )));
            }
            Message::Press2 => {
                self.button_2_press_count += 1;
                return focus(button::Id::new(format!(
                    "button two {}",
                    self.button_2_press_count
                )));
            }
        }
        Command::none()
    }

    fn view(&self, _id: window::Id) -> Element<Message> {
        row![
            button(text(format!("{}", self.button_1_press_count)))
                .on_press(Message::Press1)
                .width(Length::Shrink)
                .height(Length::Shrink)
                .id(button::Id::new(format!(
                    "button one {}",
                    self.button_1_press_count
                ))),
            button(text(format!("{}", self.button_2_press_count)))
                .on_press(Message::Press2)
                .width(Length::Shrink)
                .height(Length::Shrink)
                .id(button::Id::new(format!(
                    "button two {}",
                    self.button_2_press_count
                ))),
        ]
        .width(Length::Shrink)
        .height(Length::Shrink)
        .into()
    }

    fn close_requested(&self, id: window::Id) -> Self::Message {
        Message::Ignore
    }

    fn style(&self) -> <iced_style::Theme as application::StyleSheet>::Style {
        <iced_style::Theme as application::StyleSheet>::Style::Custom(Box::new(
            CustomTheme,
        ))
    }
    fn title(&self) -> String {
        String::from("Accessible Window Test")
    }
}

pub struct CustomTheme;

impl application::StyleSheet for CustomTheme {
    type Style = iced::Theme;

    fn appearance(&self, style: &Self::Style) -> application::Appearance {
        application::Appearance {
            background_color: Color::from_rgba(1.0, 1.0, 1.0, 0.8),
            text_color: Color::BLACK,
        }
    }
}
