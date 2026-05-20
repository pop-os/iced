//! Text widgets display information through writing.
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub fn text<T>(t: T) -> iced_core::widget::Text<'static, iced_core::Theme, ()> { unimplemented!() } }
//! #            pub use iced_core::color; }
//! # pub type State = ();
//! # pub type Element<'a, Message> = iced_core::Element<'a, Message, iced_core::Theme, ()>;
//! use iced::widget::text;
//! use iced::color;
//!
//! enum Message {
//!     // ...
//! }
//!
//! fn view(state: &State) -> Element<'_, Message> {
//!     text("Hello, this is iced!")
//!         .size(20)
//!         .color(color!(0x0000ff))
//!         .into()
//! }
//! ```
use crate::alignment;
use crate::layout;
use crate::mouse::{self, click};
use crate::renderer;
use crate::text;
use crate::text::paragraph::{self, Paragraph};
use crate::widget::tree::{self, Tree};
use crate::{
    Clipboard, Color, Element, Event, Layout, Length, Pixels, Point, Rectangle,
    Shell, Size, Theme, Widget, keyboard, touch,
};

use unicode_segmentation::UnicodeSegmentation;

pub use text::{Alignment, Ellipsize, LineHeight, Shaping, Wrapping};

/// A bunch of text.
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub fn text<T>(t: T) -> iced_core::widget::Text<'static, iced_core::Theme, ()> { unimplemented!() } }
/// #            pub use iced_core::color; }
/// # pub type State = ();
/// # pub type Element<'a, Message> = iced_core::Element<'a, Message, iced_core::Theme, ()>;
/// use iced::widget::text;
/// use iced::color;
///
/// enum Message {
///     // ...
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     text("Hello, this is iced!")
///         .size(20)
///         .color(color!(0x0000ff))
///         .into()
/// }
/// ```
pub struct Text<'a, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    id: crate::widget::Id,
    fragment: text::Fragment<'a>,
    format: Format<Renderer::Font>,
    class: Theme::Class<'a>,
    selectable: bool,
}

impl<'a, Theme, Renderer> Text<'a, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    /// Create a new fragment of [`Text`] with the given contents.
    pub fn new(fragment: impl text::IntoFragment<'a>) -> Self {
        Text {
            id: crate::widget::Id::unique(),
            fragment: fragment.into_fragment(),
            format: Format::default(),
            class: Theme::default(),
            selectable: false,
        }
    }

    /// Sets the size of the [`Text`].
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.format.size = Some(size.into());
        self
    }

    /// Sets the [`LineHeight`] of the [`Text`].
    pub fn line_height(mut self, line_height: impl Into<LineHeight>) -> Self {
        self.format.line_height = line_height.into();
        self
    }

    /// Sets the [`Font`] of the [`Text`].
    ///
    /// [`Font`]: crate::text::Renderer::Font
    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.format.font = Some(font.into());
        self
    }

    /// Sets the [`Font`] of the [`Text`], if `Some`.
    ///
    /// [`Font`]: crate::text::Renderer::Font
    pub fn font_maybe(
        mut self,
        font: Option<impl Into<Renderer::Font>>,
    ) -> Self {
        self.format.font = font.map(Into::into);
        self
    }

    /// Sets the width of the [`Text`] boundaries.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.format.width = width.into();
        self
    }

    /// Sets the height of the [`Text`] boundaries.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.format.height = height.into();
        self
    }

    /// Centers the [`Text`], both horizontally and vertically.
    pub fn center(self) -> Self {
        self.align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
    }

    /// Sets the [`alignment::Horizontal`] of the [`Text`].
    pub fn align_x(mut self, alignment: impl Into<text::Alignment>) -> Self {
        self.format.align_x = alignment.into();
        self
    }

    /// Sets the [`alignment::Vertical`] of the [`Text`].
    pub fn align_y(
        mut self,
        alignment: impl Into<alignment::Vertical>,
    ) -> Self {
        self.format.align_y = alignment.into();
        self
    }

    /// Sets the [`Shaping`] strategy of the [`Text`].
    pub fn shaping(mut self, shaping: Shaping) -> Self {
        self.format.shaping = shaping;
        self
    }

    /// Sets the [`Wrapping`] strategy of the [`Text`].
    pub fn wrapping(mut self, wrapping: Wrapping) -> Self {
        self.format.wrapping = wrapping;
        self
    }

    // Sets the [`Ellipsize`] strategy of the [`Text`].
    pub fn ellipsize(mut self, ellipsize: Ellipsize) -> Self {
        self.format.ellipsize = ellipsize;
        self
    }

    /// Sets the style of the [`Text`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the [`Color`] of the [`Text`].
    pub fn color(self, color: impl Into<Color>) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.color_maybe(Some(color))
    }

    /// Sets the [`Color`] of the [`Text`], if `Some`.
    pub fn color_maybe(self, color: Option<impl Into<Color>>) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        let color = color.map(Into::into);

        self.style(move |_theme| Style {
            color,
            ..Style::default()
        })
    }

    /// Makes the [`Text`] selectable. When enabled, the user can click and
    /// drag to select text, and copy it with Ctrl+C / Cmd+C.
    pub fn selectable(mut self) -> Self {
        self.selectable = true;
        self
    }

    /// Sets the style class of the [`Text`].
    #[cfg(feature = "advanced")]
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }
}

/// The internal state of a [`Text`] widget.
pub struct State<P: Paragraph> {
    /// The cached paragraph layout.
    pub paragraph: paragraph::Plain<P>,
    /// Lazily allocated when text is selectable and first interacted with.
    selection: Option<Box<SelectionState>>,
    focused: bool,
    keyboard_focused: bool,
    context_menu_position: Option<Point>,
}

impl<P: Paragraph> Default for State<P> {
    fn default() -> Self {
        Self {
            paragraph: paragraph::Plain::default(),
            selection: None,
            focused: false,
            keyboard_focused: false,
            context_menu_position: None,
        }
    }
}

impl<P: Paragraph> std::fmt::Debug for State<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("selection", &self.selection)
            .field("focused", &self.focused)
            .finish_non_exhaustive()
    }
}

impl<P: Paragraph> State<P> {
    /// Returns `true` if the widget currently has focus.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Returns `true` if focus was gained via keyboard navigation (Tab).
    pub fn is_keyboard_focused(&self) -> bool {
        self.keyboard_focused
    }

    /// Clears focus, selection, and all interaction state.
    pub fn clear_focus(&mut self) {
        self.focused = false;
        self.keyboard_focused = false;
        self.context_menu_position = None;
        if let Some(sel) = &mut self.selection {
            sel.clear();
        }
    }

    /// Returns the context menu position, if a context menu should be shown.
    pub fn context_menu_position(&self) -> Option<Point> {
        self.context_menu_position
    }

    /// Sets or clears the context menu position.
    pub fn set_context_menu_position(&mut self, pos: Option<Point>) {
        self.context_menu_position = pos;
    }
}

impl<P: Paragraph> std::ops::Deref for State<P> {
    type Target = paragraph::Plain<P>;

    fn deref(&self) -> &paragraph::Plain<P> {
        &self.paragraph
    }
}

impl<P: Paragraph> std::ops::DerefMut for State<P> {
    fn deref_mut(&mut self) -> &mut paragraph::Plain<P> {
        &mut self.paragraph
    }
}

#[derive(Debug, Clone, Default)]
struct SelectionState {
    anchor: usize,
    end: usize,
    dragging: bool,
    modifiers: keyboard::Modifiers,
    last_click: Option<click::Click>,
}

impl SelectionState {
    fn clear(&mut self) {
        self.anchor = 0;
        self.end = 0;
        self.dragging = false;
    }
}

impl<P: Paragraph> crate::widget::operation::Focusable for State<P> {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
        self.keyboard_focused = true;
    }

    fn unfocus(&mut self) {
        self.clear_focus();
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Text<'_, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer::Paragraph>::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.format.width,
            height: self.format.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        layout(
            &mut state.paragraph,
            renderer,
            limits,
            &self.fragment,
            self.format,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor_position: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        let style = theme.style(&self.class);
        let bounds = layout.bounds();
        let paragraph = state.paragraph.raw();
        if let Some(sel) = &state.selection {
            let left = sel.anchor.min(sel.end);
            let right = sel.anchor.max(sel.end);
            let content: &str = self.fragment.as_ref();

            if left != right {
                let lo_byte = grapheme_to_byte(content, left);
                let hi_byte = grapheme_to_byte(content, right);

                let anchor = bounds.anchor(
                    paragraph.min_bounds(),
                    paragraph.align_x(),
                    paragraph.align_y(),
                );

                let rects = paragraph.highlight(
                    0,
                    (lo_byte, text::Affinity::After),
                    (hi_byte, text::Affinity::Before),
                );

                for r in rects {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle {
                                x: anchor.x + r.x,
                                y: anchor.y + r.y,
                                width: r.width,
                                height: r.height,
                            },
                            ..renderer::Quad::default()
                        },
                        style.selected_fill,
                    );
                }
            }
        }

        draw(renderer, defaults, bounds, paragraph, style, viewport);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if !self.selectable {
            return;
        }

        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        let bounds = layout.bounds();
        let content: &str = self.fragment.as_ref();
        let grapheme_count = content.graphemes(true).count();
        let paragraph = state.paragraph.raw();

        // Any click outside this widget clears selection and focus.
        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(_))
                | Event::Touch(touch::Event::FingerPressed { .. })
        ) {
            if cursor.position_over(bounds).is_none() {
                state.focused = false;
                state.keyboard_focused = false;
                if let Some(sel) = &mut state.selection {
                    sel.clear();
                }
            }
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if state.context_menu_position.take().is_some() {
                    shell.capture_event();
                    return;
                }

                if let Some(pos) = cursor.position_over(bounds) {
                    let sel = state.selection.get_or_insert_with(|| {
                        Box::new(SelectionState::default())
                    });

                    let anchor = bounds.anchor(
                        paragraph.min_bounds(),
                        paragraph.align_x(),
                        paragraph.align_y(),
                    );
                    let relative =
                        Point::new(pos.x - anchor.x, pos.y - anchor.y);

                    let grapheme_pos =
                        hit_to_grapheme(paragraph, relative, content);

                    let new_click = click::Click::new(
                        pos,
                        mouse::Button::Left,
                        sel.last_click.take(),
                    );

                    match new_click.kind() {
                        click::Kind::Single => {
                            if sel.modifiers.shift() {
                                sel.end = grapheme_pos;
                            } else {
                                sel.anchor = grapheme_pos;
                                sel.end = grapheme_pos;
                            }
                            sel.dragging = true;
                        }
                        click::Kind::Double => {
                            sel.anchor =
                                previous_start_of_word(content, grapheme_pos);
                            sel.end = next_end_of_word(content, grapheme_pos);
                            sel.dragging = true;
                        }
                        click::Kind::Triple => {
                            sel.anchor = 0;
                            sel.end = grapheme_count;
                            sel.dragging = true;
                        }
                    }

                    sel.last_click = Some(new_click);
                    state.focused = true;
                    state.keyboard_focused = false;
                    shell.capture_event();
                }
            }

            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                if let Some(pos) = cursor.position_over(bounds) {
                    state.context_menu_position = Some(pos);
                    state.focused = true;
                    state.keyboard_focused = false;
                    shell.capture_event();
                }
            }

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. }) => {
                if let Some(sel) = &mut state.selection {
                    sel.dragging = false;
                }
            }

            Event::Mouse(mouse::Event::CursorMoved { position })
            | Event::Touch(touch::Event::FingerMoved { position, .. }) => {
                if let Some(sel) = &mut state.selection {
                    if sel.dragging {
                        let anchor = bounds.anchor(
                            paragraph.min_bounds(),
                            paragraph.align_x(),
                            paragraph.align_y(),
                        );
                        let relative = Point::new(
                            position.x - anchor.x,
                            position.y - anchor.y,
                        );

                        sel.end = hit_to_grapheme(paragraph, relative, content);
                        shell.capture_event();
                    }
                }
            }

            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                physical_key,
                text: _,
                ..
            }) => {
                if !state.focused {
                    return;
                }
                let sel = state
                    .selection
                    .get_or_insert_with(|| Box::new(SelectionState::default()));

                if modifiers.command() {
                    match key.to_latin(*physical_key) {
                        Some('c') => {
                            let left = sel.anchor.min(sel.end);
                            let right = sel.anchor.max(sel.end);
                            if left != right {
                                let selected: String = content
                                    .graphemes(true)
                                    .skip(left)
                                    .take(right - left)
                                    .collect();
                                clipboard.write(
                                    crate::clipboard::Kind::Standard,
                                    selected,
                                );
                            }
                            shell.capture_event();
                            return;
                        }
                        Some('a') => {
                            sel.anchor = 0;
                            sel.end = grapheme_count;
                            shell.capture_event();
                            return;
                        }
                        _ => {}
                    }
                }

                match key {
                    keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => {
                        let by_word = is_jump_modifier(*modifiers);
                        if modifiers.shift() {
                            sel.end = if by_word {
                                previous_start_of_word(content, sel.end)
                            } else {
                                sel.end.saturating_sub(1)
                            };
                        } else {
                            let left = sel.anchor.min(sel.end);
                            let pos = if by_word {
                                previous_start_of_word(content, left)
                            } else {
                                left.saturating_sub(1)
                            };
                            sel.anchor = pos;
                            sel.end = pos;
                        }
                        shell.capture_event();
                    }
                    keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
                        let by_word = is_jump_modifier(*modifiers);
                        if modifiers.shift() {
                            sel.end = if by_word {
                                next_end_of_word(content, sel.end)
                            } else {
                                (sel.end + 1).min(grapheme_count)
                            };
                        } else {
                            let right = sel.anchor.max(sel.end);
                            let pos = if by_word {
                                next_end_of_word(content, right)
                            } else {
                                (right + 1).min(grapheme_count)
                            };
                            sel.anchor = pos;
                            sel.end = pos;
                        }
                        shell.capture_event();
                    }
                    keyboard::Key::Named(keyboard::key::Named::Home) => {
                        if modifiers.shift() {
                            sel.end = 0;
                        } else {
                            sel.anchor = 0;
                            sel.end = 0;
                        }
                        shell.capture_event();
                    }
                    keyboard::Key::Named(keyboard::key::Named::End) => {
                        if modifiers.shift() {
                            sel.end = grapheme_count;
                        } else {
                            sel.anchor = grapheme_count;
                            sel.end = grapheme_count;
                        }
                        shell.capture_event();
                    }
                    _ => {}
                }
            }

            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                if let Some(sel) = &mut state.selection {
                    sel.modifiers = *modifiers;
                }
            }

            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.selectable && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::None
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn super::Operation,
    ) {
        operation.text(None, layout.bounds(), &self.fragment);

        if self.selectable {
            let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
            operation.focusable(None, layout.bounds(), state);
        }
    }

    #[cfg(feature = "a11y")]
    fn a11y_nodes(
        &self,
        layout: Layout<'_>,
        _state: &Tree,
        _: mouse::Cursor,
    ) -> iced_accessibility::A11yTree {
        use iced_accessibility::{
            A11yTree,
            accesskit::{Live, Node, Rect, Role},
        };

        let Rectangle {
            x,
            y,
            width,
            height,
        } = layout.bounds();
        let bounds = Rect::new(
            x as f64,
            y as f64,
            (x + width) as f64,
            (y + height) as f64,
        );

        let mut node = Node::new(Role::Paragraph);

        // TODO is the name likely different from the content?
        node.set_label(self.fragment.to_string().into_boxed_str());
        node.set_bounds(bounds);

        // TODO make this configurable
        node.set_live(Live::Polite);
        A11yTree::leaf(node, self.id.clone())
    }

    fn id(&self) -> Option<crate::widget::Id> {
        Some(self.id.clone())
    }

    fn set_id(&mut self, id: crate::widget::Id) {
        self.id = id;
    }
}

/// The format of some [`Text`].
///
/// Check out the methods of the [`Text`] widget
/// to learn more about each field.
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct Format<Font> {
    pub width: Length,
    pub height: Length,
    pub size: Option<Pixels>,
    pub font: Option<Font>,
    pub line_height: LineHeight,
    pub align_x: text::Alignment,
    pub align_y: alignment::Vertical,
    pub shaping: Shaping,
    pub wrapping: Wrapping,
    pub ellipsize: Ellipsize,
}

impl<Font> Default for Format<Font> {
    fn default() -> Self {
        Self {
            size: None,
            line_height: LineHeight::default(),
            font: None,
            width: Length::Shrink,
            height: Length::Shrink,
            align_x: text::Alignment::Default,
            align_y: alignment::Vertical::Top,
            shaping: Shaping::default(),
            wrapping: Wrapping::default(),
            ellipsize: Ellipsize::default(),
        }
    }
}

/// Produces the [`layout::Node`] of a [`Text`] widget.
pub fn layout<Renderer>(
    paragraph: &mut paragraph::Plain<Renderer::Paragraph>,
    renderer: &Renderer,
    limits: &layout::Limits,
    content: &str,
    format: Format<Renderer::Font>,
) -> layout::Node
where
    Renderer: text::Renderer,
{
    layout::sized(limits, format.width, format.height, |limits| {
        let bounds = limits.max();

        let size = format.size.unwrap_or_else(|| renderer.default_size());
        let font = format.font.unwrap_or_else(|| renderer.default_font());

        let _ = paragraph.update(text::Text {
            content,
            bounds,
            size,
            line_height: format.line_height,
            font,
            align_x: format.align_x,
            align_y: format.align_y,
            shaping: format.shaping,
            wrapping: format.wrapping,
            ellipsize: format.ellipsize,
        });

        paragraph.min_bounds()
    })
}

/// Draws text using the same logic as the [`Text`] widget.
pub fn draw<Renderer>(
    renderer: &mut Renderer,
    style: &renderer::Style,
    bounds: Rectangle,
    paragraph: &Renderer::Paragraph,
    appearance: Style,
    viewport: &Rectangle,
) where
    Renderer: text::Renderer,
{
    let anchor = bounds.anchor(
        paragraph.min_bounds(),
        paragraph.align_x(),
        paragraph.align_y(),
    );

    renderer.fill_paragraph(
        paragraph,
        anchor,
        appearance.color.unwrap_or(style.text_color),
        *viewport,
    );
}

impl<'a, Message, Theme, Renderer> From<Text<'a, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(
        text: Text<'a, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(text)
    }
}

// impl<'a, Theme, Renderer> Clone for Text<'a, Theme, Renderer>
// where
//     Renderer: text::Renderer,
// {
//     fn clone(&self) -> Self {
//         Self {
//             id: self.id.clone(),
//             content: self.content.clone(),
//             size: self.size,
//             line_height: self.line_height,
//             width: self.width,
//             height: self.height,
//             horizontal_alignment: self.horizontal_alignment,
//             vertical_alignment: self.vertical_alignment,
//             font: self.font,
//             style: self.style,
//             shaping: self.shaping,
//             wrap: self.wrap,
//         }
//     }
// }
// TODO(POP): Clone no longer can be implemented because of style being a Box(style)

impl<'a, Theme, Renderer> From<&'a str> for Text<'a, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: text::Renderer,
{
    fn from(content: &'a str) -> Self {
        Self::new(content)
    }
}

impl<'a, Message, Theme, Renderer> From<&'a str>
    for Element<'a, Message, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(content: &'a str) -> Self {
        Text::from(content).into()
    }
}

/// The appearance of some text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The [`Color`] of the text.
    ///
    /// The default, `None`, means using the inherited color.
    pub color: Option<Color>,
    /// The fill [`Color`] of the selection highlight.
    pub selected_fill: Color,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            color: None,
            selected_fill: DEFAULT_SELECTION_COLOR,
        }
    }
}

/// The theme catalog of a [`Text`].
pub trait Catalog: Sized {
    /// The item class of this [`Catalog`].
    type Class<'a>;

    /// The default class produced by this [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, item: &Self::Class<'_>) -> Style;
}

/// A styling function for a [`Text`].
///
/// This is just a boxed closure: `Fn(&Theme, Status) -> Style`.
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|_theme| Style::default())
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

/// The default text styling; color is inherited.
pub fn default(_theme: &Theme) -> Style {
    Style::default()
}

/// Text with the default base color.
pub fn base(theme: &Theme) -> Style {
    Style {
        color: Some(theme.palette().text),
        ..Style::default()
    }
}

/// Text conveying some important information, like an action.
pub fn primary(theme: &Theme) -> Style {
    Style {
        color: Some(theme.palette().primary),
        ..Style::default()
    }
}

/// Text conveying some secondary information, like a footnote.
pub fn secondary(theme: &Theme) -> Style {
    Style {
        color: Some(theme.extended_palette().secondary.base.color),
        ..Style::default()
    }
}

/// Text conveying some positive information, like a successful event.
pub fn success(theme: &Theme) -> Style {
    Style {
        color: Some(theme.palette().success),
        ..Style::default()
    }
}

/// Text conveying some mildly negative information, like a warning.
pub fn warning(theme: &Theme) -> Style {
    Style {
        color: Some(theme.palette().warning),
        ..Style::default()
    }
}

/// Text conveying some negative information, like an error.
pub fn danger(theme: &Theme) -> Style {
    Style {
        color: Some(theme.palette().danger),
        ..Style::default()
    }
}

use crate::widget::tree::Tree as WidgetTree;

/// Implement this on a **widget** to enable context menu support for
/// text selection (Copy, Select All, and optionally Cut / Paste) in libcosmic
pub trait HasSelectableText {
    /// Returns the currently selected text, if any.
    fn selected_text(&self, tree: &WidgetTree) -> Option<String>;

    /// Selects all text.
    fn select_all(&self, tree: &mut WidgetTree);

    /// Returns `true` if the widget is editable (enables Cut / Paste).
    fn is_editable(&self) -> bool {
        false
    }

    /// Returns `true` if the widget is currently focused.
    fn is_focused(&self, tree: &WidgetTree) -> bool;

    /// Returns the position where the context menu should appear
    fn context_menu_position(&self, tree: &WidgetTree) -> Option<Point>;

    /// Sets or clears the context menu position.
    fn set_context_menu_position(
        &self,
        tree: &mut WidgetTree,
        pos: Option<Point>,
    );

    /// Copies the selection to the clipboard.
    fn copy_to_clipboard(
        &self,
        tree: &WidgetTree,
        clipboard: &mut dyn Clipboard,
    ) {
        if let Some(text) = self.selected_text(tree) {
            clipboard.write(crate::clipboard::Kind::Standard, text);
        }
    }

    /// Deletes the selected text and returns the new full content.
    /// Only called when [`is_editable`](Self::is_editable) is `true`.
    fn delete_selection(&self, _tree: &mut WidgetTree) -> Option<String> {
        None
    }

    /// Inserts `text` at the cursor (replacing any selection) and returns
    /// the new full content.
    /// Only called when [`is_editable`](Self::is_editable) is `true`.
    fn paste_text(
        &self,
        _tree: &mut WidgetTree,
        _text: &str,
    ) -> Option<String> {
        None
    }
}

impl<Theme: Catalog, Renderer: text::Renderer> HasSelectableText
    for Text<'_, Theme, Renderer>
{
    fn selected_text(&self, tree: &WidgetTree) -> Option<String> {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        let sel = state.selection.as_ref()?;
        let left = sel.anchor.min(sel.end);
        let right = sel.anchor.max(sel.end);
        if left == right {
            return None;
        }
        let content = state.paragraph.content();
        let lo = grapheme_to_byte(content, left);
        let hi = grapheme_to_byte(content, right);
        content.get(lo..hi).map(|s| s.to_owned())
    }

    fn select_all(&self, tree: &mut WidgetTree) {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        let count = state.paragraph.content().graphemes(true).count();
        let sel = state
            .selection
            .get_or_insert_with(|| Box::new(SelectionState::default()));
        sel.anchor = 0;
        sel.end = count;
    }

    fn is_focused(&self, tree: &WidgetTree) -> bool {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        state.is_focused()
    }

    fn context_menu_position(&self, tree: &WidgetTree) -> Option<Point> {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        state.context_menu_position()
    }

    fn set_context_menu_position(
        &self,
        tree: &mut WidgetTree,
        pos: Option<Point>,
    ) {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        state.set_context_menu_position(pos);
    }
}

const DEFAULT_SELECTION_COLOR: Color = Color {
    r: 0.0,
    g: 0.47,
    b: 0.84,
    a: 0.3,
};

fn grapheme_to_byte(content: &str, grapheme_index: usize) -> usize {
    content
        .graphemes(true)
        .take(grapheme_index)
        .map(|g| g.len())
        .sum()
}

fn hit_to_grapheme<P: Paragraph>(
    paragraph: &P,
    point: Point,
    content: &str,
) -> usize {
    match paragraph.hit_test(point) {
        Some(hit) => {
            let byte_offset = hit.cursor().min(content.len());
            content[..byte_offset].graphemes(true).count()
        }
        None => content.graphemes(true).count(),
    }
}

fn previous_start_of_word(content: &str, grapheme_index: usize) -> usize {
    let graphemes: Vec<&str> = content.graphemes(true).collect();
    let clamped = grapheme_index.min(graphemes.len());
    let before: String = graphemes[..clamped].concat();

    UnicodeSegmentation::split_word_bound_indices(&*before)
        .filter(|(_, word)| !word.trim_start().is_empty())
        .next_back()
        .map_or(0, |(i, prev_word)| {
            clamped
                - prev_word.graphemes(true).count()
                - before[i + prev_word.len()..].graphemes(true).count()
        })
}

fn next_end_of_word(content: &str, grapheme_index: usize) -> usize {
    let graphemes: Vec<&str> = content.graphemes(true).collect();
    let clamped = grapheme_index.min(graphemes.len());
    let after: String = graphemes[clamped..].concat();

    UnicodeSegmentation::split_word_bound_indices(&*after)
        .find(|(_, word)| !word.trim_start().is_empty())
        .map_or(graphemes.len(), |(i, next_word)| {
            clamped
                + next_word.graphemes(true).count()
                + after[..i].graphemes(true).count()
        })
}

fn is_jump_modifier(modifiers: keyboard::Modifiers) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.alt()
    } else {
        modifiers.control()
    }
}
