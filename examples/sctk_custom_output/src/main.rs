use std::env;
use std::time::Duration;

use iced::platform_specific::shell::commands::layer_surface::get_layer_surface;
use iced::platform_specific::shell::commands::output::{
    get_output, get_output_info,
};
use iced::runtime::platform_specific::wayland::layer_surface::SctkLayerSurfaceSettings;
use iced::widget::text;
use iced::{daemon, stream};
use iced::{
    platform_specific::shell::commands::output::OutputInfo,
    runtime::platform_specific::wayland::layer_surface::IcedOutput, Element,
    Task,
};

use iced::window::Id;
use tokio::time::sleep;

fn main() -> iced::Result {
    daemon("Custom Output", App::update, App::view).run_with(App::new)
}

#[derive(Debug)]
enum Message {
    Output(Option<IcedOutput>),
    OutputInfo(Option<OutputInfo>),
}

#[derive(Debug)]
struct App {
    monitor: Option<String>,
    output: IcedOutput,
    logical_size: Option<(i32, i32)>,
}

impl App {
    fn new() -> (App, Task<Message>) {
        let app = App {
            monitor: env::var("WL_OUTPUT").ok(),
            output: IcedOutput::Active,
            logical_size: None,
        };

        let task = match &app.monitor {
            Some(_) => app.try_get_output(),
            None => app.open(),
        };

        (app, task)
    }

    fn try_get_output(&self) -> Task<Message> {
        let monitor = self.monitor.clone();
        get_output(move |output_state| {
            output_state
                .outputs()
                .find(|o| {
                    output_state
                        .info(o)
                        .map(|info| info.name == monitor)
                        .unwrap_or(false)
                })
                .clone()
        })
        .map(|optn| Message::Output(optn.map(IcedOutput::Output)))
    }

    fn try_get_output_info(&self) -> Task<Message> {
        let monitor = self.monitor.clone();
        get_output_info(move |output_state| {
            output_state
                .outputs()
                .find(|o| {
                    output_state
                        .info(o)
                        .map(|info| info.name == monitor)
                        .unwrap_or(false)
                })
                .and_then(|o| output_state.info(&o))
                .clone()
        })
        .map(Message::OutputInfo)
    }

    fn open(&self) -> Task<Message> {
        get_layer_surface(SctkLayerSurfaceSettings {
            output: self.output.clone(),
            size: match self.logical_size {
                Some(size) => {
                    Some((Some((size.0 / 2) as u32), Some((size.1 / 2) as u32)))
                }
                None => Some((Some(800), Some(500))),
            },
            ..Default::default()
        })
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Output(optn) => match optn {
                Some(output) => {
                    self.output = output;
                    self.try_get_output_info()
                }
                None => Task::stream(stream::channel(1, |_| async {
                    sleep(Duration::from_millis(500)).await;
                }))
                .chain(self.try_get_output()),
            },
            Message::OutputInfo(optn) => match optn {
                Some(info) => {
                    self.logical_size = info.logical_size;
                    self.open()
                }
                None => Task::stream(stream::channel(1, |_| async {
                    sleep(Duration::from_millis(500)).await;
                }))
                .chain(self.try_get_output_info()),
            },
        }
    }

    fn view(&self, _window_id: Id) -> Element<Message> {
        match &self.monitor {
            Some(monitor) => text!("Opened on monitor {monitor}\nSize: {:?}", self.logical_size),
            None => text!("No output specified, try setting WL_OUTPUT=YourMonitor\nDefaulting to size 800x500"),
        }.into()
    }
}
