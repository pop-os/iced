use std::process::exit;

use iced::executor;
use iced::widget::canvas::{Cache, Cursor, Geometry, LineCap, Path, Stroke};
use iced::widget::{canvas, container};
use iced::{
    Application, Color, Command, Element, Length, Point, Rectangle, Settings,
    Subscription, Theme, Vector, sctk_settings::InitialSurface
};
use iced_native::command::platform_specific::wayland::layer_surface::IcedLayerSurface;
use iced_native::window::Id;
use iced_sctk::commands::layer_surface::{get_layer_surface, destroy_layer_surface};
use sctk::shell::layer::Anchor;
pub fn main() -> iced::Result {
    Clock::run(Settings {
        antialiasing: true,
        initial_surface: InitialSurface::LayerSurface(IcedLayerSurface {
            size: (None, Some(200)),
            anchor: Anchor::LEFT.union(Anchor::RIGHT).union(Anchor::TOP),
            exclusive_zone: 200,
            ..Default::default()

        }),
        ..Settings::default()
    })
}

struct Clock {
    now: time::OffsetDateTime,
    clock: Cache,
    count: u32,
    to_destroy: Id,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Tick(time::OffsetDateTime),
}

impl Application for Clock {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let to_destroy = Id::new(10);
        (
            Clock {
                now: time::OffsetDateTime::now_local()
                    .unwrap_or_else(|_| time::OffsetDateTime::now_utc()),
                clock: Default::default(),
                count: 0,
                to_destroy
            },
            get_layer_surface(IcedLayerSurface {
                // XXX id must be unique!
                id: to_destroy,
                size: (None, Some(100)),
                anchor: Anchor::LEFT.union(Anchor::RIGHT).union(Anchor::BOTTOM),
                exclusive_zone: 100,
                ..Default::default()

            }),
        )
    }

    fn title(&self) -> String {
        String::from("Clock - Iced")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Tick(local_time) => {
                let now = local_time;

                if now != self.now {
                    self.now = now;
                    self.clock.clear();
                }
                // destroy the second layer surface after counting to 10.
                self.count += 1;
                if self.count == 10 {
                    println!("time to remove the bottom clock!");
                    return destroy_layer_surface::<Message>(self.to_destroy);
                }
            }
        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(500)).map(|_| {
            Message::Tick(
                time::OffsetDateTime::now_local()
                    .unwrap_or_else(|_| time::OffsetDateTime::now_utc()),
            )
        })
    }

    fn view_window(
        &self,
        window: iced_native::window::Id,
    ) -> Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        unimplemented!()
    }

    fn view_popup(
        &self,
        window: iced_native::window::Id,
    ) -> Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        unimplemented!()
    }

    fn view_layer_surface(
        &self,
        window: iced_native::window::Id,
    ) -> Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        let canvas = canvas(self as &Self)
            .width(Length::Fill)
            .height(Length::Fill);

        container(canvas)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }

    fn close_window_requested(
        &self,
        window: iced_native::window::Id,
    ) -> Self::Message {
        unimplemented!()
    }

    fn layer_surface_done(
        &self,
        window: iced_native::window::Id,
    ) -> Self::Message {
        exit(0);
    }

    fn popup_done(&self, window: iced_native::window::Id) -> Self::Message {
        unimplemented!()
    }

    // fn view(&self) -> Element<Message> {
    //     let canvas = canvas(self as &Self)
    //         .width(Length::Fill)
    //         .height(Length::Fill);

    //     container(canvas)
    //         .width(Length::Fill)
    //         .height(Length::Fill)
    //         .padding(20)
    //         .into()
    // }
}

impl<Message> canvas::Program<Message> for Clock {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let clock = self.clock.draw(bounds.size(), |frame| {
            let center = frame.center();
            let radius = frame.width().min(frame.height()) / 2.0;

            let background = Path::circle(center, radius);
            frame.fill(&background, Color::from_rgb8(0x12, 0x93, 0xD8));

            let short_hand =
                Path::line(Point::ORIGIN, Point::new(0.0, -0.5 * radius));

            let long_hand =
                Path::line(Point::ORIGIN, Point::new(0.0, -0.8 * radius));

            let thin_stroke = Stroke {
                width: radius / 100.0,
                color: Color::WHITE,
                line_cap: LineCap::Round,
                ..Stroke::default()
            };

            let wide_stroke = Stroke {
                width: thin_stroke.width * 3.0,
                ..thin_stroke
            };

            frame.translate(Vector::new(center.x, center.y));

            frame.with_save(|frame| {
                frame.rotate(hand_rotation(self.now.hour(), 12));
                frame.stroke(&short_hand, wide_stroke);
            });

            frame.with_save(|frame| {
                frame.rotate(hand_rotation(self.now.minute(), 60));
                frame.stroke(&long_hand, wide_stroke);
            });

            frame.with_save(|frame| {
                frame.rotate(hand_rotation(self.now.second(), 60));
                frame.stroke(&long_hand, thin_stroke);
            })
        });

        vec![clock]
    }
}

fn hand_rotation(n: u8, total: u8) -> f32 {
    let turns = n as f32 / total as f32;

    2.0 * std::f32::consts::PI * turns
}
