mod layer;
mod output;
mod popup;
mod window;

pub use layer::*;
pub use output::*;
pub use popup::*;
pub use window::*;

/// wayland events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// layer surface event
    Layer(LayerEvent),
    /// popup event
    Popup(PopupEvent),
    /// output event
    Output(OutputEvent),
    /// window event
    Window(WindowEvent)
}
