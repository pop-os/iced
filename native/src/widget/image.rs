//! Display images in your user interface.
pub mod viewer;
use iced_core::Id;
pub use viewer::Viewer;

use crate::image;
use crate::layout;
use crate::renderer;
use crate::widget::Tree;
use crate::{
    ContentFit, Element, Layout, Length, Point, Rectangle, Size, Vector, Widget,
};

use std::hash::Hash;

/// Creates a new [`Viewer`] with the given image `Handle`.
pub fn viewer<Handle>(handle: Handle) -> Viewer<Handle> {
    Viewer::new(handle)
}

/// A frame that displays an image while keeping aspect ratio.
///
/// # Example
///
/// ```
/// # use iced_native::widget::Image;
/// # use iced_native::image;
/// #
/// let image = Image::<image::Handle>::new("resources/ferris.png");
/// ```
///
/// <img src="https://github.com/iced-rs/iced/blob/9712b319bb7a32848001b96bd84977430f14b623/examples/resources/ferris.png?raw=true" width="300">
#[derive(Debug, Hash)]
pub struct Image<Handle> {
    id: Id,
    name: Option<String>,
    description: Option<String>,
    handle: Handle,
    width: Length,
    height: Length,
    content_fit: ContentFit,
}

impl<Handle> Image<Handle> {
    /// Creates a new [`Image`] with the given path.
    pub fn new<T: Into<Handle>>(handle: T) -> Self {
        Image {
            id: Id::unique(),
            name: None,
            description: None,
            handle: handle.into(),
            width: Length::Shrink,
            height: Length::Shrink,
            content_fit: ContentFit::Contain,
        }
    }

    /// Sets the name of the [`Image`].
    pub fn name<T: Into<String>>(mut self, name: T) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the description of the [`Image`].
    pub fn description<T: Into<String>>(mut self, description: T) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the width of the [`Image`] boundaries.
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets the height of the [`Image`] boundaries.
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Sets the [`ContentFit`] of the [`Image`].
    ///
    /// Defaults to [`ContentFit::Contain`]
    pub fn content_fit(self, content_fit: ContentFit) -> Self {
        Self {
            content_fit,
            ..self
        }
    }
}

/// Computes the layout of an [`Image`].
pub fn layout<Renderer, Handle>(
    renderer: &Renderer,
    limits: &layout::Limits,
    handle: &Handle,
    width: Length,
    height: Length,
    content_fit: ContentFit,
) -> layout::Node
where
    Renderer: image::Renderer<Handle = Handle>,
{
    // The raw w/h of the underlying image
    let image_size = {
        let Size { width, height } = renderer.dimensions(handle);

        Size::new(width as f32, height as f32)
    };

    // The size to be available to the widget prior to `Shrink`ing
    let raw_size = limits.width(width).height(height).resolve(image_size);

    // The uncropped size of the image when fit to the bounds above
    let full_size = content_fit.fit(image_size, raw_size);

    // Shrink the widget to fit the resized image, if requested
    let final_size = Size {
        width: match width {
            Length::Shrink => f32::min(raw_size.width, full_size.width),
            _ => raw_size.width,
        },
        height: match height {
            Length::Shrink => f32::min(raw_size.height, full_size.height),
            _ => raw_size.height,
        },
    };

    layout::Node::new(final_size)
}

impl<Message, Renderer, Handle> Widget<Message, Renderer> for Image<Handle>
where
    Renderer: image::Renderer<Handle = Handle>,
    Handle: Clone + Hash,
{
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(
        &self,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout(
            renderer,
            limits,
            &self.handle,
            self.width,
            self.height,
            self.content_fit,
        )
    }

    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        _theme: &Renderer::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor_position: Point,
        _viewport: &Rectangle,
    ) {
        let Size { width, height } = renderer.dimensions(&self.handle);
        let image_size = Size::new(width as f32, height as f32);

        let bounds = layout.bounds();
        let adjusted_fit = self.content_fit.fit(image_size, bounds.size());

        let render = |renderer: &mut Renderer| {
            let offset = Vector::new(
                (bounds.width - adjusted_fit.width).max(0.0) / 2.0,
                (bounds.height - adjusted_fit.height).max(0.0) / 2.0,
            );

            let drawing_bounds = Rectangle {
                width: adjusted_fit.width,
                height: adjusted_fit.height,
                ..bounds
            };

            renderer.draw(self.handle.clone(), drawing_bounds + offset)
        };

        if adjusted_fit.width > bounds.width
            || adjusted_fit.height > bounds.height
        {
            renderer.with_layer(bounds, render);
        } else {
            render(renderer)
        }
    }

    #[cfg(feature = "a11y")]
    fn a11y_nodes(&self, layout: Layout<'_>, state: &Tree, cursor_position: Point) -> iced_accessibility::A11yTree {
        use iced_accessibility::{accesskit::{Rect, NodeBuilder, Role}, A11yTree, A11yNode};

        let bounds = layout.bounds();
        let Rectangle { x, y, width, height } = bounds;
        let bounds = Rect::new(x as f64, y as f64, (x + width) as f64, (y + height) as f64);
        let mut node = NodeBuilder::new(Role::Image);
        node.set_bounds(bounds);
        if let Some(name) = self.name.as_ref() {
            node.set_name(name.clone());
        }
        if let Some(description) = self.description.as_ref() {
            node.set_description(description.clone());
        }

        A11yTree::leaf(A11yNode::new(node, self.id.clone()))
    }
}

impl<'a, Message, Renderer, Handle> From<Image<Handle>>
    for Element<'a, Message, Renderer>
where
    Renderer: image::Renderer<Handle = Handle>,
    Handle: Clone + Hash + 'a,
{
    fn from(image: Image<Handle>) -> Element<'a, Message, Renderer> {
        Element::new(image)
    }
}
