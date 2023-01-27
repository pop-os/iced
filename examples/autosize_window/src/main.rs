use iced::alignment::{self, Alignment};
use iced::event::{self, Event};
use iced::keyboard;
use iced::subscription;
use iced::theme::{self, Theme};
use iced::wayland::actions::layer_surface::SctkLayerSurfaceSettings;
use iced::wayland::actions::popup::SctkPopupSettings;
use iced::wayland::actions::popup::SctkPositioner;
use iced::wayland::actions::window::SctkWindowSettings;
use iced::wayland::layer_surface::{get_layer_surface, Anchor};
use iced::wayland::popup::destroy_popup;
use iced::wayland::popup::get_popup;
use iced::wayland::window::get_window;
use iced::wayland::InitialSurface;
use iced::wayland::SurfaceIdWrapper;
use iced::widget::{
    self, button, checkbox, column, container, horizontal_space, row,
    scrollable, text, text_input, Column, Row, Text,
};
use iced::Rectangle;
use iced::{window, Application, Element};
use iced::{Color, Command, Font, Length, Settings, Subscription};
use iced_native::layout::Limits;
use iced_style::application;

pub fn main() -> iced::Result {
    Todos::run(Settings {
        initial_surface: InitialSurface::XdgWindow(SctkWindowSettings {
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
struct Todos {
    size: u32,
    id_ctr: u32,
    popup: Option<window::Id>,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    Popup,
    Ignore,
}

impl Application for Todos {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Todos, Command<Message>) {
        (
            Todos {
                size: 1,
                id_ctr: 2,
                ..Default::default()
            },
            Command::none(),
        )
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Tick => {
                self.size = (self.size - 1) % 3;
                if self.size == 0 {
                    self.size = 3;
                }
            }
            Message::Popup => {
                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                } else {
                    self.id_ctr += 1;
                    let new_id = window::Id::new(self.id_ctr);
                    self.popup.replace(new_id);
                    return get_popup(SctkPopupSettings {
                        parent: window::Id::new(0),
                        id: new_id,
                        positioner: SctkPositioner {
                            anchor_rect: Rectangle {
                                x: 20,
                                y: 20,
                                width: 1,
                                height: 1,
                            },
                            ..Default::default()
                        },
                        parent_size: None,
                        grab: true,
                    });
                }
            }
            Message::Ignore => {}
        }
        Command::none()
    }

    fn view(&self, id: SurfaceIdWrapper) -> Element<Message> {
        match id {
            SurfaceIdWrapper::Window(_) => {
                button(horizontal_space(Length::Units(20)))
                    .on_press(Message::Popup)
                    .width(Length::Units(20))
                    .height(Length::Units(20))
                    .into()
            }
            SurfaceIdWrapper::Popup(_) => Column::with_children(
                (0..self.size)
                    .map(|_| {
                        Row::with_children(
                            (0..self.size)
                                .map(|i| {
                                    button(horizontal_space(Length::Units(20)))
                                        .on_press(Message::Popup)
                                        .width(Length::Units(20))
                                        .height(Length::Units(20))
                                        .into()
                                })
                                .collect(),
                        )
                        .spacing(12)
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .into()
                    })
                    .collect(),
            )
            .spacing(12)
            .width(Length::Shrink)
            .height(Length::Shrink)
            .into(),
            SurfaceIdWrapper::LayerSurface(_) => unimplemented!(),
        }
    }

    fn close_requested(&self, id: SurfaceIdWrapper) -> Self::Message {
        Message::Ignore
    }

    fn style(&self) -> <iced_style::Theme as application::StyleSheet>::Style {
        <iced_style::Theme as application::StyleSheet>::Style::Custom(Box::new(
            CustomTheme,
        ))
    }
    fn title(&self) -> String {
        String::from("autosize")
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(1000))
            .map(|_| Message::Tick)
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
