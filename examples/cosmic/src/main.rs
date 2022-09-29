use iced::widget::{
    button, checkbox, column, container, horizontal_rule, horizontal_space, progress_bar, radio,
    row, scrollable, slider, svg, text, text_input, toggler, vertical_rule,
    vertical_space,
};
use iced::{theme, Alignment, Element, Length, Sandbox, Settings, Theme};

pub fn main() -> iced::Result {
    Window::run(Settings::default())
}

fn icon(name: &str, size: u16) -> svg::Svg {
    let handle = match freedesktop_icons::lookup(name)
        .with_size(size)
        .with_theme("Pop")
        .with_cache()
        .force_svg()
        .find()
    {
        Some(path) => svg::Handle::from_path(path),
        None => {
            eprintln!("icon '{}' size {} not found", name, size);
            svg::Handle::from_memory(Vec::new())
        },
    };
    svg::Svg::new(handle)
}

#[derive(Default)]
struct Window {
    page: u8,
    debug: bool,
    theme: Theme,
    input_value: String,
    slider_value: f32,
    checkbox_value: bool,
    toggler_value: bool,
}

#[derive(Debug, Clone)]
enum Message {
    Page(u8),
    Debug(bool),
    ThemeChanged(Theme),
    InputChanged(String),
    ButtonPressed,
    SliderChanged(f32),
    CheckboxToggled(bool),
    TogglerToggled(bool),
}

impl Sandbox for Window {
    type Message = Message;

    fn new() -> Self {
        Window::default()
    }

    fn title(&self) -> String {
        String::from("COSMIC Design System - Iced")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Page(page) => self.page = page,
            Message::Debug(debug) => self.debug = debug,
            Message::ThemeChanged(theme) => self.theme = theme,
            Message::InputChanged(value) => self.input_value = value,
            Message::ButtonPressed => {}
            Message::SliderChanged(value) => self.slider_value = value,
            Message::CheckboxToggled(value) => self.checkbox_value = value,
            Message::TogglerToggled(value) => self.toggler_value = value,
        }
    }

    fn view(&self) -> Element<Message> {
        let sidebar: Element<_> = column![
            //TODO: Support symbolic icons
            button(
                row![
                    icon("network-wireless", 16).width(Length::Units(16)),
                    text("Wi-Fi"),
                    horizontal_space(Length::Fill),
                ]
                .spacing(10)
            )
            .on_press(Message::Page(0))
            .style(if self.page == 0 { theme::Button::Primary } else { theme::Button::Text }),

            button(
                row![
                    icon("preferences-desktop", 16).width(Length::Units(16)),
                    text("Desktop"),
                    horizontal_space(Length::Fill),
                ]
                .spacing(10)
            )
            .on_press(Message::Page(1))
            .style(if self.page == 1 { theme::Button::Primary } else { theme::Button::Text }),

            button(
                row![
                    icon("system-software-update", 16).width(Length::Units(16)),
                    text("OS Upgrade & Recovery"),
                    horizontal_space(Length::Fill),
                ]
                .spacing(10)
            )
            .on_press(Message::Page(2))
            .style(if self.page == 2 { theme::Button::Primary } else { theme::Button::Text }),

            toggler(
                String::from("Debug layout"),
                self.debug,
                Message::Debug,
            )
            .width(Length::Shrink)
            .spacing(10),
        ]
        .height(Length::Fill)
        .spacing(20)
        .padding(20)
        .max_width(300)
        .into();

        let choose_theme = [Theme::Light, Theme::Dark].iter().fold(
            row![text("Choose a theme:")].spacing(10),
            |row, theme| {
                row.push(radio(
                    format!("{:?}", theme),
                    *theme,
                    Some(self.theme),
                    Message::ThemeChanged,
                ))
            },
        );

        let text_input = text_input(
            "Type something...",
            &self.input_value,
            Message::InputChanged,
        )
        .padding(10)
        .size(20);

        let btn = button("Submit")
            .padding(10)
            .on_press(Message::ButtonPressed);

        let slider =
            slider(0.0..=100.0, self.slider_value, Message::SliderChanged);

        let progress_bar = progress_bar(0.0..=100.0, self.slider_value);

        let scrollable = scrollable(
            column![
                "Scroll me!",
                vertical_space(Length::Units(800)),
                "You did it!"
            ]
            .width(Length::Fill),
        )
        .height(Length::Units(100));

        let checkbox = checkbox(
            "Check me!",
            self.checkbox_value,
            Message::CheckboxToggled,
        );

        let toggler = toggler(
            String::from("Toggle me!"),
            self.toggler_value,
            Message::TogglerToggled,
        )
        .width(Length::Shrink)
        .spacing(10);

        let content: Element<_> = column![
            icon("pop-os", 64),
            choose_theme,
            horizontal_rule(10),
            text("Buttons"),
            row![
                button("Primary").style(theme::Button::Primary).on_press(Message::ButtonPressed),
                button("Secondary").style(theme::Button::Secondary).on_press(Message::ButtonPressed),
                button("Positive").style(theme::Button::Positive).on_press(Message::ButtonPressed),
                button("Destructive").style(theme::Button::Destructive).on_press(Message::ButtonPressed),
                button("Text").style(theme::Button::Text).on_press(Message::ButtonPressed),
            ].spacing(10),
            row![
                button("Primary").style(theme::Button::Primary),
                button("Secondary").style(theme::Button::Secondary),
                button("Positive").style(theme::Button::Positive),
                button("Destructive").style(theme::Button::Destructive),
                button("Text").style(theme::Button::Text),
            ].spacing(10),
            horizontal_rule(10),
            row![text_input, btn].spacing(10),
            slider,
            progress_bar,
            row![
                scrollable,
                vertical_rule(38),
                column![checkbox, toggler].spacing(20)
            ]
            .spacing(10)
            .height(Length::Units(100))
            .align_items(Alignment::Center),
        ]
        .spacing(20)
        .padding(20)
        .max_width(600)
        .into();

        container(row![
            if self.debug { sidebar.explain(iced::Color::WHITE) } else { sidebar },
            horizontal_space(Length::Fill),
            if self.debug { content.explain(iced::Color::WHITE) } else { content },
            horizontal_space(Length::Fill),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .center_y()
        .into()
    }

    fn theme(&self) -> Theme {
        self.theme
    }
}
