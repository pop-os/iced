use iced::widget::{container, svg, scrollable};
use iced::{Element, Length, Sandbox, Settings};
use iced_sctk::application::SurfaceIdWrapper;
pub fn main() -> iced::Result {
    Tiger::run(Settings::default())
}

struct Tiger;

impl Sandbox for Tiger {
    type Message = ();

    fn new() -> Self {
        Tiger
    }

    fn title(&self) -> String {
        String::from("SVG - Iced")
    }

    fn update(&mut self, _message: ()) {}

    fn view(&self, id: SurfaceIdWrapper) -> Element<()> {
        let svg = scrollable(svg(svg::Handle::from_path(format!(
            "{}/resources/tiger.svg",
            env!("CARGO_MANIFEST_DIR")
        )))
        .width(Length::Units(2000))
        .height(Length::Units(2000))).height(Length::Fill);

        container(svg)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .center_x()
            .center_y()
            .into()
    }
    fn close_requested(&self, _: iced_sctk::application::SurfaceIdWrapper) -> <Self as Sandbox>::Message { todo!() }
}
