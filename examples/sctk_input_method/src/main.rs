#[macro_use]
extern crate lazy_static;

use iced::{
    event::{self, listen_raw, wayland::InputMethodEvent},
    wayland::{
        actions::{
            input_method::ActionInner,
            input_method_popup::InputMethodPopupSettings,
            virtual_keyboard::ActionInner as VKActionInner,
        },
        input_method::{
            hide_input_method_popup, input_method_action,
            show_input_method_popup,
        },
        virtual_keyboard::virtual_keyboard_action,
        InitialSurface,
    },
    widget::{column, container, row, text},
    window, Alignment, Application, Color, Command, Element, Event, Settings,
    Subscription, Theme,
};
use iced_core::{
    event::wayland::{
        InputMethodKeyboardEvent, KeyEvent, Modifiers, RawModifiers,
    },
    keyboard::Key,
    window::Id,
};
use iced_style::application;
use selection_field::widget::selection_field;
use std::{collections::HashMap, fmt::Debug};
mod selection_field;

lazy_static! {
    static ref ACCENTKEYS: HashMap<char, Vec<char>> = [
        ('A', vec!['À', 'Á', 'Â', 'Ã', 'Ä', 'Å', 'Ā', 'Ă', 'Ą', 'Æ']),
        ('E', vec!['É', 'È', 'Ê', 'Ë']),
        ('I', vec!['Í', 'Ì', 'Î', 'Ï']),
        ('O', vec!['Ó', 'Ò', 'Ô', 'Õ', 'Ö']),
        ('U', vec!['Ú', 'Ù', 'Û', 'Ü']),
        ('a', vec!['à', 'á', 'â', 'ã', 'ä', 'å', 'ā', 'ă', 'ą', 'æ']),
        ('e', vec!['é', 'è', 'ê', 'ë']),
        ('i', vec!['í', 'ì', 'î', 'ï']),
        ('o', vec!['ó', 'ò', 'ô', 'õ', 'ö']),
        ('u', vec!['ú', 'ù', 'û', 'ü']),
    ]
    .into();
}

fn main() -> iced::Result {
    let initial_surface = InputMethodPopupSettings::default();
    let settings = Settings {
        initial_surface: InitialSurface::InputMethodPopup(initial_surface),
        ..Settings::default()
    };
    InputMethod::run(settings)
}

struct InputMethod {
    index: usize,
    popup: bool,
    list: Vec<char>,
}

impl InputMethod {
    fn commit_string(&mut self, character: char) -> Command<Message> {
        Command::batch(vec![
            hide_input_method_popup(),
            input_method_action(ActionInner::CommitString(
                character.to_string(),
            )),
            input_method_action(ActionInner::Commit),
        ])
    }

    fn open_popup(
        &mut self,
        character: char,
        list: &Vec<char>,
    ) -> Command<Message> {
        self.popup = true;
        self.index = 0;
        self.list = list.clone();
        Command::batch(vec![
            input_method_action(ActionInner::SetPreeditString {
                string: character.to_string(),
                cursor_begin: 0,
                cursor_end: 0,
            }),
            input_method_action(ActionInner::Commit),
            show_input_method_popup(),
        ])
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Activate,
    Deactivate,
    KeyPressed(KeyEvent, Key, Modifiers),
    KeyRepeat(KeyEvent, Key, Modifiers),
    KeyReleased(KeyEvent, Key, Modifiers),
    Modifiers(Modifiers, RawModifiers),
    UpdatePopup { index: usize },
    Done,
}

impl Application for InputMethod {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = Theme;

    fn new(_flags: ()) -> (InputMethod, Command<Message>) {
        (
            InputMethod {
                index: 0,
                popup: false,
                list: Vec::new(),
            },
            Command::none(),
        )
    }

    fn title(&self, _: Id) -> String {
        String::from("InputMethod")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Activate => Command::none(),
            Message::Deactivate => Command::none(),
            Message::KeyPressed(_key, key_code, _modifiers) => {
                if self.popup {
                    match key_code {
                        Key::Named(
                            iced_core::keyboard::key::Named::ArrowLeft,
                        ) => {
                            if self.index > 0 {
                                self.index -= 1;
                            }
                            Command::none()
                        }
                        Key::Named(
                            iced_core::keyboard::key::Named::ArrowRight,
                        ) => {
                            if self.index < self.list.len() - 1 {
                                self.index += 1;
                            }
                            Command::none()
                        }
                        Key::Named(iced_core::keyboard::key::Named::Enter) => {
                            self.commit_string(self.list[self.index])
                        }
                        _ => Command::none(),
                    }
                } else {
                    Command::none()
                }
            }
            Message::KeyRepeat(key, key_code, _modifiers) => {
                if !self.popup {
                    if let Some(utf8) = key
                        .utf8
                        .as_ref()
                        .map(|str| str.chars().last().unwrap_or_default())
                    {
                        if let Some(list) = ACCENTKEYS.get(&utf8) {
                            self.open_popup(utf8, list)
                        } else {
                            virtual_keyboard_action(VKActionInner::KeyPressed(
                                key,
                            ))
                        }
                    } else {
                        virtual_keyboard_action(VKActionInner::KeyPressed(key))
                    }
                } else {
                    match key_code {
                        Key::Named(
                            iced_core::keyboard::key::Named::ArrowLeft,
                        ) => {
                            if self.index > 0 {
                                self.index -= 1;
                            }
                            Command::none()
                        }
                        Key::Named(
                            iced_core::keyboard::key::Named::ArrowRight,
                        ) => {
                            if self.index < self.list.len() - 1 {
                                self.index += 1;
                            }
                            Command::none()
                        }
                        _ => Command::none(),
                    }
                }
            }
            Message::KeyReleased(key, key_code, _modifiers) => {
                if !self.popup {
                    Command::batch(vec![
                        virtual_keyboard_action(VKActionInner::KeyPressed(
                            key.clone(),
                        )),
                        virtual_keyboard_action(VKActionInner::KeyReleased(
                            key,
                        )),
                    ])
                } else {
                    match key_code {
                        Key::Named(iced_core::keyboard::key::Named::Enter) => {
                            self.popup = false
                        }
                        _ => {}
                    }
                    Command::none()
                }
            }
            Message::Modifiers(_modifiers, raw_modifiers) => {
                virtual_keyboard_action(VKActionInner::Modifiers(raw_modifiers))
            }
            Message::Done => Command::none(),
            Message::UpdatePopup { index } => {
                self.index = index;
                Command::none()
            }
        }
    }

    fn view(&self, _id: window::Id) -> Element<Message> {
        container(
            row(self
                .list
                .iter()
                .enumerate()
                .map(|(index, char)| {
                    selection_field(
                        column(vec![
                            text((index + 1) % 10)
                                .size(50)
                                .style(iced::theme::Text::Color(Color::WHITE))
                                .into(),
                            text(char)
                                .style(iced::theme::Text::Color(Color::WHITE))
                                .size(50)
                                .into(),
                        ])
                        .align_items(Alignment::Center)
                        .padding(5.0)
                        .spacing(4.0),
                    )
                    .set_indexes(index)
                    .selected(self.index)
                    .on_press(Message::Deactivate)
                    .on_select(Message::UpdatePopup { index })
                    .into()
                })
                .collect::<Vec<_>>())
            .padding(2.0),
        )
        .padding(5.0)
        .style(<iced_style::Theme as container::StyleSheet>::Style::Custom(
            Box::new(CustomTheme),
        ))
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        listen_raw(|event, status| match (event.clone(), status) {
            (
                Event::PlatformSpecific(event::PlatformSpecific::Wayland(
                    event::wayland::Event::InputMethod(event),
                )),
                event::Status::Ignored,
            ) => match event {
                InputMethodEvent::Activate => Some(Message::Activate),
                InputMethodEvent::Deactivate => Some(Message::Deactivate),
                InputMethodEvent::Done => Some(Message::Done),
                _ => None,
            },
            (
                Event::PlatformSpecific(event::PlatformSpecific::Wayland(
                    event::wayland::Event::InputMethodKeyboard(event),
                )),
                event::Status::Ignored,
            ) => match event {
                InputMethodKeyboardEvent::Press(key, key_code, modifiers) => {
                    Some(Message::KeyPressed(key, key_code, modifiers))
                }
                InputMethodKeyboardEvent::Release(key, key_code, modifiers) => {
                    Some(Message::KeyReleased(key, key_code, modifiers))
                }
                InputMethodKeyboardEvent::Repeat(key, key_code, modifiers) => {
                    Some(Message::KeyRepeat(key, key_code, modifiers))
                }
                InputMethodKeyboardEvent::Modifiers(
                    modifiers,
                    raw_modifiers,
                ) => Some(Message::Modifiers(modifiers, raw_modifiers)),
            },
            _ => None,
        })
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

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            border: iced_core::Border {
                color: Color::from_rgb(1.0, 1.0, 1.0),
                width: 3.0,
                radius: 10.0.into(),
            },
            background: Some(Color::from_rgb(0.0, 0.0, 0.0).into()),
            ..container::Appearance::default()
        }
    }
}

impl iced_style::application::StyleSheet for CustomTheme {
    type Style = iced::Theme;

    fn appearance(&self, _style: &Self::Style) -> application::Appearance {
        iced_style::application::Appearance {
            background_color: Color::from_rgba(0.0, 0.0, 0.0, 0.0),
            icon_color: Color::BLACK,
            text_color: Color::BLACK,
        }
    }
}
