//! Display information and interactive controls in your application.
pub use iced_native::widget::helpers::*;

pub use iced_core::Id;
pub use iced_native::{column, row};

/// A container that distributes its contents vertically.
#[cfg(any(
    feature = "softbuffer",
    feature = "glow",
    feature = "wgpu",
    feature = "dyrend"
))]
pub type Column<'a, Message, Renderer = crate::Renderer> =
    iced_native::widget::Column<'a, Message, Renderer>;
#[cfg(not(any(
    feature = "softbuffer",
    feature = "glow",
    feature = "wgpu",
    feature = "dyrend"
)))]
pub use iced_native::widget::Column;

/// A container that distributes its contents horizontally.
#[cfg(any(
    feature = "softbuffer",
    feature = "glow",
    feature = "wgpu",
    feature = "dyrend"
))]
pub type Row<'a, Message, Renderer = crate::Renderer> =
    iced_native::widget::Row<'a, Message, Renderer>;
#[cfg(not(any(
    feature = "softbuffer",
    feature = "glow",
    feature = "wgpu",
    feature = "dyrend"
)))]
pub use iced_native::widget::Row;

pub mod text {
    //! Write some text for your users to read.
    pub use iced_native::widget::text::{Appearance, StyleSheet};

    /// A paragraph of text.
    #[cfg(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    ))]
    pub type Text<'a, Renderer = crate::Renderer> =
        iced_native::widget::Text<'a, Renderer>;
    #[cfg(not(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    )))]
    pub use iced_native::widget::Text;
}

pub mod button {
    //! Allow your users to perform actions by pressing a button.
    pub use iced_native::widget::button::{focus, Appearance, Id, StyleSheet};

    /// A widget that produces a message when clicked.
    #[cfg(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    ))]
    pub type Button<'a, Message, Renderer = crate::Renderer> =
        iced_native::widget::Button<'a, Message, Renderer>;
    #[cfg(not(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    )))]
    pub use iced_native::widget::Button;
}

pub mod checkbox {
    //! Show toggle controls using checkboxes.
    pub use iced_native::widget::checkbox::{Appearance, StyleSheet};

    /// A box that can be checked.
    #[cfg(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    ))]
    pub type Checkbox<'a, Message, Renderer = crate::Renderer> =
        iced_native::widget::Checkbox<'a, Message, Renderer>;
    #[cfg(not(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    )))]
    pub use iced_native::widget::Checkbox;
}

pub mod container {
    //! Decorate content and apply alignment.
    pub use iced_native::widget::container::{Appearance, StyleSheet};

    /// An element decorating some content.
    #[cfg(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    ))]
    pub type Container<'a, Message, Renderer = crate::Renderer> =
        iced_native::widget::Container<'a, Message, Renderer>;
    #[cfg(not(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    )))]
    pub use iced_native::widget::Container;
}

pub mod mouse_listener {
    //! Intercept mouse events on a widget.

    /// A container intercepting mouse events.
    #[cfg(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    ))]
    pub type MouseListener<'a, Message, Renderer = crate::Renderer> =
        iced_native::widget::MouseListener<'a, Message, Renderer>;
    #[cfg(not(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    )))]
    pub use iced_native::widget::MouseListener;
}

pub mod pane_grid {
    //! Let your users split regions of your application and organize layout dynamically.
    //!
    //! [![Pane grid - Iced](https://thumbs.gfycat.com/MixedFlatJellyfish-small.gif)](https://gfycat.com/mixedflatjellyfish)
    //!
    //! # Example
    //! The [`pane_grid` example] showcases how to use a [`PaneGrid`] with resizing,
    //! drag and drop, and hotkey support.
    //!
    //! [`pane_grid` example]: https://github.com/iced-rs/iced/tree/0.6/examples/pane_grid
    pub use iced_native::widget::pane_grid::{
        Axis, Configuration, Direction, DragEvent, Line, Node, Pane,
        ResizeEvent, Split, State, StyleSheet,
    };

    /// A collection of panes distributed using either vertical or horizontal splits
    /// to completely fill the space available.
    ///
    /// [![Pane grid - Iced](https://thumbs.gfycat.com/MixedFlatJellyfish-small.gif)](https://gfycat.com/mixedflatjellyfish)
    #[cfg(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    ))]
    pub type PaneGrid<'a, Message, Renderer = crate::Renderer> =
        iced_native::widget::PaneGrid<'a, Message, Renderer>;
    #[cfg(not(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    )))]
    pub use iced_native::widget::PaneGrid;

    /// The content of a [`Pane`].
    #[cfg(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    ))]
    pub type Content<'a, Message, Renderer = crate::Renderer> =
        iced_native::widget::pane_grid::Content<'a, Message, Renderer>;
    #[cfg(not(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    )))]
    pub use iced_native::widget::pane_grid::Content;

    /// The title bar of a [`Pane`].
    #[cfg(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    ))]
    pub type TitleBar<'a, Message, Renderer = crate::Renderer> =
        iced_native::widget::pane_grid::TitleBar<'a, Message, Renderer>;
    #[cfg(not(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    )))]
    pub use iced_native::widget::pane_grid::TitleBar;
}

pub mod pick_list {
    //! Display a dropdown list of selectable values.
    pub use iced_native::widget::pick_list::{Appearance, StyleSheet};

    /// A widget allowing the selection of a single value from a list of options.
    #[cfg(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    ))]
    pub type PickList<'a, T, Message, Renderer = crate::Renderer> =
        iced_native::widget::PickList<'a, T, Message, Renderer>;
    #[cfg(not(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    )))]
    pub use iced_native::widget::PickList;
}

pub mod radio {
    //! Create choices using radio buttons.
    pub use iced_native::widget::radio::{Appearance, StyleSheet};

    /// A circular button representing a choice.
    #[cfg(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    ))]
    pub type Radio<Message, Renderer = crate::Renderer> =
        iced_native::widget::Radio<Message, Renderer>;
    #[cfg(not(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    )))]
    pub use iced_native::widget::Radio;
}

pub mod scrollable {
    //! Navigate an endless amount of content with a scrollbar.
    pub use iced_native::widget::scrollable::{
        snap_to, style::Scrollbar, style::Scroller, Id, StyleSheet,
    };

    /// A widget that can vertically display an infinite amount of content
    /// with a scrollbar.
    #[cfg(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    ))]
    pub type Scrollable<'a, Message, Renderer = crate::Renderer> =
        iced_native::widget::Scrollable<'a, Message, Renderer>;
    #[cfg(not(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    )))]
    pub use iced_native::widget::Scrollable;
}

pub mod toggler {
    //! Show toggle controls using togglers.
    pub use iced_native::widget::toggler::{Appearance, StyleSheet};

    /// A toggler widget.
    #[cfg(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    ))]
    pub type Toggler<'a, Message, Renderer = crate::Renderer> =
        iced_native::widget::Toggler<'a, Message, Renderer>;
    #[cfg(not(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    )))]
    pub use iced_native::widget::Toggler;
}

pub mod text_input {
    //! Display fields that can be filled with text.
    pub use iced_native::widget::text_input::{
        focus, move_cursor_to, move_cursor_to_end, move_cursor_to_front,
        select_all, Appearance, Id, StyleSheet, State
    };

    /// A field that can be filled with text.
    #[cfg(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    ))]
    pub type TextInput<'a, Message, Renderer = crate::Renderer> =
        iced_native::widget::TextInput<'a, Message, Renderer>;
    #[cfg(not(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    )))]
    pub use iced_native::widget::TextInput;
}

pub mod tooltip {
    //! Display a widget over another.
    pub use iced_native::widget::tooltip::Position;

    /// A widget allowing the selection of a single value from a list of options.
    #[cfg(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    ))]
    pub type Tooltip<'a, Message, Renderer = crate::Renderer> =
        iced_native::widget::Tooltip<'a, Message, Renderer>;
    #[cfg(not(any(
        feature = "softbuffer",
        feature = "glow",
        feature = "wgpu",
        feature = "dyrend"
    )))]
    pub use iced_native::widget::Tooltip;
}

pub use iced_native::widget::progress_bar;
pub use iced_native::widget::rule;
pub use iced_native::widget::slider;
pub use iced_native::widget::vertical_slider;
pub use iced_native::widget::Space;

pub use button::Button;
pub use checkbox::Checkbox;
pub use container::Container;
pub use pane_grid::PaneGrid;
pub use pick_list::PickList;
pub use progress_bar::ProgressBar;
pub use radio::Radio;
pub use rule::Rule;
pub use scrollable::Scrollable;
pub use slider::Slider;
pub use text::Text;
pub use text_input::TextInput;
pub use toggler::Toggler;
pub use tooltip::Tooltip;
pub use vertical_slider::VerticalSlider;

#[cfg(feature = "canvas")]
#[cfg_attr(docsrs, doc(cfg(feature = "canvas")))]
pub use iced_graphics::widget::canvas;

#[cfg(feature = "canvas")]
#[cfg_attr(docsrs, doc(cfg(feature = "canvas")))]
/// Creates a new [`Canvas`].
pub fn canvas<P, Message, Theme>(program: P) -> Canvas<Message, Theme, P>
where
    P: canvas::Program<Message, Theme>,
{
    Canvas::new(program)
}

#[cfg(feature = "image")]
#[cfg_attr(docsrs, doc(cfg(feature = "image")))]
pub mod image {
    //! Display images in your user interface.
    pub use iced_native::image::Handle;

    /// A frame that displays an image.
    pub type Image = iced_native::widget::Image<Handle>;

    pub use iced_native::widget::image::viewer;
    pub use viewer::Viewer;
}

#[cfg(feature = "qr_code")]
#[cfg_attr(docsrs, doc(cfg(feature = "qr_code")))]
pub use iced_graphics::widget::qr_code;

#[cfg(feature = "svg")]
#[cfg_attr(docsrs, doc(cfg(feature = "svg")))]
pub mod svg {
    //! Display vector graphics in your application.
    pub use iced_native::svg::Handle;
    pub use iced_native::widget::svg::{Appearance, StyleSheet, Svg};
}

#[cfg(feature = "canvas")]
#[cfg_attr(docsrs, doc(cfg(feature = "canvas")))]
pub use canvas::Canvas;

#[cfg(feature = "image")]
#[cfg_attr(docsrs, doc(cfg(feature = "image")))]
pub use image::Image;

#[cfg(feature = "qr_code")]
#[cfg_attr(docsrs, doc(cfg(feature = "qr_code")))]
pub use qr_code::QRCode;

#[cfg(feature = "svg")]
#[cfg_attr(docsrs, doc(cfg(feature = "svg")))]
pub use svg::Svg;

#[cfg(any(feature = "winit", feature = "wayland"))]
use crate::Command;
#[cfg(any(feature = "winit", feature = "wayland"))]
use iced_native::widget::operation;

/// Focuses the previous focusable widget.
#[cfg(any(feature = "winit", feature = "wayland"))]
pub fn focus_previous<Message>() -> Command<Message>
where
    Message: 'static,
{
    Command::widget(operation::focusable::focus_previous())
}

/// Focuses the next focusable widget.
#[cfg(any(feature = "winit", feature = "wayland"))]
pub fn focus_next<Message>() -> Command<Message>
where
    Message: 'static,
{
    Command::widget(operation::focusable::focus_next())
}
