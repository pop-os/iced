//! Svg widgets display vector graphics in your application.
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub use iced_widget::*; } }
//! # pub type State = ();
//! # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
//! use iced::widget::svg;
//!
//! enum Message {
//!     // ...
//! }
//!
//! fn view(state: &State) -> Element<'_, Message> {
//!     svg("tiger.svg").into()
//! }
//! ```
//! Display vector graphics in your application.
use iced_runtime::core::widget::Id;

use crate::core::layout;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::svg;
use crate::core::widget::Tree;
use crate::core::{
    Color, ContentFit, Element, Layout, Length, Point, Rectangle, Rotation,
    Size, Theme, Vector, Widget,
};

#[cfg(feature = "a11y")]
use std::borrow::Cow;
use std::marker::PhantomData;
use std::path::PathBuf;

pub use crate::core::svg::Handle;

/// A vector graphics image.
///
/// An [`Svg`] image resizes smoothly without losing any quality.
///
/// [`Svg`] images can have a considerable rendering cost when resized,
/// specially when they are complex.
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } }
/// # pub type State = ();
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// use iced::widget::svg;
///
/// enum Message {
///     // ...
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     svg("tiger.svg").into()
/// }
/// ```
#[allow(missing_debug_implementations)]
pub struct Svg<'a, Theme = crate::Theme>
where
    Theme: Catalog,
{
    id: Id,
    #[cfg(feature = "a11y")]
    name: Option<Cow<'a, str>>,
    #[cfg(feature = "a11y")]
    description: Option<iced_accessibility::Description<'a>>,
    #[cfg(feature = "a11y")]
    label: Option<Vec<iced_accessibility::accesskit::NodeId>>,
    handle: Handle,
    width: Length,
    height: Length,
    content_fit: ContentFit,
    class: Theme::Class<'a>,
    rotation: Rotation,
    opacity: f32,
    symbolic: bool,
    _phantom_data: PhantomData<&'a ()>,
}

impl<'a, Theme> Svg<'a, Theme>
where
    Theme: Catalog,
{
    /// Creates a new [`Svg`] from the given [`Handle`].
    pub fn new(handle: impl Into<Handle>) -> Self {
        Svg {
            id: Id::unique(),
            #[cfg(feature = "a11y")]
            name: None,
            #[cfg(feature = "a11y")]
            description: None,
            #[cfg(feature = "a11y")]
            label: None,
            handle: handle.into(),
            width: Length::Fill,
            height: Length::Shrink,
            content_fit: ContentFit::Contain,
            class: Theme::default(),
            rotation: Rotation::default(),
            opacity: 1.0,
            symbolic: false,
            _phantom_data: PhantomData::default(),
        }
    }

    /// Creates a new [`Svg`] that will display the contents of the file at the
    /// provided path.
    #[must_use]
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self::new(Handle::from_path(path))
    }

    /// Sets the width of the [`Svg`].
    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Svg`].
    #[must_use]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the [`ContentFit`] of the [`Svg`].
    ///
    /// Defaults to [`ContentFit::Contain`]
    #[must_use]
    pub fn content_fit(self, content_fit: ContentFit) -> Self {
        Self {
            content_fit,
            ..self
        }
    }

    /// Symbolic icons inherit their color from the renderer if a color is not defined.
    #[must_use]
    pub fn symbolic(mut self, symbolic: bool) -> Self {
        self.symbolic = symbolic;
        self
    }

    /// Sets the style of the [`Svg`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`Svg`].
    #[cfg(feature = "advanced")]
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }

    /// Applies the given [`Rotation`] to the [`Svg`].
    pub fn rotation(mut self, rotation: impl Into<Rotation>) -> Self {
        self.rotation = rotation.into();
        self
    }

    /// Sets the opacity of the [`Svg`].
    ///
    /// It should be in the [0.0, 1.0] range—`0.0` meaning completely transparent,
    /// and `1.0` meaning completely opaque.
    pub fn opacity(mut self, opacity: impl Into<f32>) -> Self {
        self.opacity = opacity.into();
        self
    }

    #[cfg(feature = "a11y")]
    /// Sets the name of the [`Button`].
    pub fn name(mut self, name: impl Into<Cow<'a, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[cfg(feature = "a11y")]
    /// Sets the description of the [`Button`].
    pub fn description_widget<T: iced_accessibility::Describes>(
        mut self,
        description: &T,
    ) -> Self {
        self.description = Some(iced_accessibility::Description::Id(
            description.description(),
        ));
        self
    }

    #[cfg(feature = "a11y")]
    /// Sets the description of the [`Button`].
    pub fn description(mut self, description: impl Into<Cow<'a, str>>) -> Self {
        self.description =
            Some(iced_accessibility::Description::Text(description.into()));
        self
    }

    #[cfg(feature = "a11y")]
    /// Sets the label of the [`Button`].
    pub fn label(mut self, label: &dyn iced_accessibility::Labels) -> Self {
        self.label =
            Some(label.label().into_iter().map(|l| l.into()).collect());
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Svg<'a, Theme>
where
    Renderer: svg::Renderer,
    Theme: Catalog,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &self,
        _tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        // The raw w/h of the underlying image
        let Size { width, height } = renderer.measure_svg(&self.handle);
        let image_size = Size::new(width as f32, height as f32);

        // The rotated size of the svg
        let rotated_size = self.rotation.apply(image_size);

        // The size to be available to the widget prior to `Shrink`ing
        let raw_size = limits.resolve(self.width, self.height, rotated_size);

        // The uncropped size of the image when fit to the bounds above
        let full_size = self.content_fit.fit(rotated_size, raw_size);

        // Shrink the widget to fit the resized image, if requested
        let final_size = Size {
            width: match self.width {
                Length::Shrink => f32::min(raw_size.width, full_size.width),
                _ => raw_size.width,
            },
            height: match self.height {
                Length::Shrink => f32::min(raw_size.height, full_size.height),
                _ => raw_size.height,
            },
        };

        layout::Node::new(final_size)
    }

    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let Size { width, height } = renderer.measure_svg(&self.handle);
        let image_size = Size::new(width as f32, height as f32);
        let rotated_size = self.rotation.apply(image_size);

        let bounds = layout.bounds();
        let adjusted_fit = self.content_fit.fit(rotated_size, bounds.size());
        let scale = Vector::new(
            adjusted_fit.width / rotated_size.width,
            adjusted_fit.height / rotated_size.height,
        );

        let final_size = image_size * scale;

        let position = match self.content_fit {
            ContentFit::None => Point::new(
                bounds.x + (rotated_size.width - adjusted_fit.width) / 2.0,
                bounds.y + (rotated_size.height - adjusted_fit.height) / 2.0,
            ),
            _ => Point::new(
                bounds.center_x() - final_size.width / 2.0,
                bounds.center_y() - final_size.height / 2.0,
            ),
        };

        let drawing_bounds = Rectangle::new(position, final_size);

        let is_mouse_over = cursor.is_over(bounds);

        let status = if is_mouse_over {
            Status::Hovered
        } else {
            Status::Idle
        };

        let mut style = theme.style(&self.class, status);
        if self.symbolic && style.color.is_none() {
            style.color = Some(renderer_style.icon_color);
        }

        let render = |renderer: &mut Renderer| {
            renderer.draw_svg(
                svg::Svg {
                    handle: self.handle.clone(),
                    color: style.color,
                    rotation: self.rotation.radians(),
                    opacity: self.opacity,
                    border_radius: [0.0; 4],
                },
                drawing_bounds,
            );
        };

        if adjusted_fit.width > bounds.width
            || adjusted_fit.height > bounds.height
        {
            renderer.with_layer(bounds, render);
        } else {
            render(renderer);
        }
    }

    #[cfg(feature = "a11y")]
    fn a11y_nodes(
        &self,
        layout: Layout<'_>,
        _state: &Tree,
        _cursor: mouse::Cursor,
    ) -> iced_accessibility::A11yTree {
        use iced_accessibility::{
            accesskit::{NodeBuilder, NodeId, Rect, Role},
            A11yTree,
        };

        let bounds = layout.bounds();
        let Rectangle {
            x,
            y,
            width,
            height,
        } = bounds;
        let bounds = Rect::new(
            x as f64,
            y as f64,
            (x + width) as f64,
            (y + height) as f64,
        );
        let mut node = NodeBuilder::new(Role::Image);
        node.set_bounds(bounds);
        if let Some(name) = self.name.as_ref() {
            node.set_name(name.clone());
        }
        match self.description.as_ref() {
            Some(iced_accessibility::Description::Id(id)) => {
                node.set_described_by(
                    id.iter()
                        .cloned()
                        .map(|id| NodeId::from(id))
                        .collect::<Vec<_>>(),
                );
            }
            Some(iced_accessibility::Description::Text(text)) => {
                node.set_description(text.clone());
            }
            None => {}
        }

        if let Some(label) = self.label.as_ref() {
            node.set_labelled_by(label.clone());
        }

        A11yTree::leaf(node, self.id.clone())
    }

    fn id(&self) -> Option<Id> {
        Some(self.id.clone())
    }

    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
}

impl<'a, Message, Theme, Renderer> From<Svg<'a, Theme>>
    for Element<'a, Message, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: svg::Renderer + 'a,
{
    fn from(icon: Svg<'a, Theme>) -> Element<'a, Message, Theme, Renderer> {
        Element::new(icon)
    }
}

/// The possible status of an [`Svg`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The [`Svg`] is idle.
    Idle,
    /// The [`Svg`] is being hovered.
    Hovered,
}

/// The appearance of an [`Svg`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Style {
    /// The [`Color`] filter of an [`Svg`].
    ///
    /// Useful for coloring a symbolic icon.
    ///
    /// `None` keeps the original color.
    pub color: Option<Color>,
}

/// The theme catalog of an [`Svg`].
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
}

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|_theme, _status| Style::default())
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

/// A styling function for an [`Svg`].
///
/// This is just a boxed closure: `Fn(&Theme, Status) -> Style`.
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, Status) -> Style + 'a>;

impl<'a, Theme> From<Style> for StyleFn<'a, Theme> {
    fn from(style: Style) -> Self {
        Box::new(move |_theme, _status| style)
    }
}
