use std::{borrow::Cow, ops::Deref};

use iced_core::{Id, Length, Alignment};
use iced_style::radio;

use crate::text;

use super::{Column, Row, Radio};

enum GroupContainer<'a, Message, Renderer> {
    Row(Row<'a, Message, Renderer>),
    Column(Column<'a, Message, Renderer>),
}

/// A container for a group of radio buttons.
#[allow(missing_debug_implementations)]
pub struct RadioGroup<'a, Message, Renderer, T>
where
    Renderer: text::Renderer,
    Renderer::Theme: radio::StyleSheet,
{
    id: Id,
    container: GroupContainer<'a, Message, Renderer>,
    on_click: Box<dyn Fn(T) -> Message + 'a>,
    options: Vec<Radio<Message, Renderer>>,
    selected: Option<T>,
    width: Length,
    height: Length,
    size: u16,
    button_spacing: u16,
    text_size: Option<u16>,
    font: Renderer::Font,
    style: <Renderer::Theme as radio::StyleSheet>::Style,
}

impl<'a, Message, Renderer, T> RadioGroup<'a, Message, Renderer, T>
where
    Message: Clone,
    Renderer: text::Renderer,
    Renderer::Theme: radio::StyleSheet,
    T: Clone + Copy + Eq,
{
    /// Creates a new horizontal [`RadioGroup`]
    pub fn new_horizontal(on_click: impl Fn(T) -> Message + 'a) -> Self {
        Self {
            id: Id::unique(),
            container: GroupContainer::Row(Row::new()),
            on_click: Box::new(on_click),
            options: Vec::new(),
            selected: None,
            button_spacing: Radio::<Message, Renderer>::DEFAULT_SPACING,
            size: Radio::<Message, Renderer>::DEFAULT_SIZE,
            width: Length::Shrink,
            height: Length::Shrink,
            text_size: None,
            font: Default::default(),
            style: Default::default(),
        }
    }

    /// Creates a new vertical [`RadioGroup`]
    pub fn new_vertical(on_click: impl Fn(T) -> Message + 'a) -> Self {
        Self {
            id: Id::unique(),
            container: GroupContainer::Column(Column::new()),
            on_click: Box::new(on_click),
            options: Vec::new(),
            selected: None,
            button_spacing: Radio::<Message, Renderer>::DEFAULT_SPACING,
            size: Radio::<Message, Renderer>::DEFAULT_SIZE,
            width: Length::Shrink,
            height: Length::Shrink,
            text_size: None,
            font: Default::default(),
            style: Default::default(),
        }
    }

    /// assigns a list of options to the [`RadioGroup`]
    pub fn options(mut self, options: Vec<(T, Cow<'a, str>)>) -> Self {
        self.options = options.into_iter().map(|(value, label)| {
            Radio::new(value, label, self.selected, self.on_click.deref())
        }).collect();
        self
    }

    /// Adds a new [`Radio`] button to the [`RadioGroup`].
    pub fn push(mut self, value: T, label: impl Into<Cow<'a, str>>) -> Self {
        let radio = 
            Radio::new(value, label, self.selected, self.on_click.deref());
        self.options.push(radio);

        self
    }

    /// Sets the selected value of the [`RadioGroup`].
    pub fn selected(mut self, selected: Option<T>) -> Self {
        self.selected = selected;

        self
    }

    /// Sets the width of the [`RadioGroup`].
    pub fn width(mut self, width: Length) -> Self {
        self.container = match self.container {
            GroupContainer::Row(row) => GroupContainer::Row(row.width(width)),
            GroupContainer::Column(column) => GroupContainer::Column(column.width(width)),
        };
        self.width = width;

        self
    }

    /// Sets the height of the [`RadioGroup`].
    pub fn height(mut self, height: Length) -> Self {
        self.container = match self.container {
            GroupContainer::Row(row) => GroupContainer::Row(row.height(height)),
            GroupContainer::Column(column) => GroupContainer::Column(column.height(height)),
        };
        self.height = height;

        self
    }

    /// Sets the spacing between the [`Radio`] button and the text.
    pub fn button_spacing(mut self, spacing: u16) -> Self {
        self.button_spacing = spacing;
        self
    }

    /// Sets the text font of the ['RadioGroup`]
    pub fn font(mut self, font: Renderer::Font) -> Self {
         self.font = font;
         self
    }

    /// Sets the text size of the [`RadioGroup`]
    pub fn text_size(mut self, size: u16) -> Self {
        self.text_size = Some(size);
        self
    }

    /// Sets the size of the [`RadioGroup`] buttons.
    pub fn size(mut self, size: u16) -> Self {
        self.size = size;
        self
    }

    /// Sets the style of the [`RadioGroup`].
    pub fn style(mut self, style: impl Into<<Renderer::Theme as radio::StyleSheet>::Style>) -> Self {
        self.style = style.into();
        self
    }

    /// Sets the [`Alignment`] of the [`RadioGroup`].
    pub fn align_items(mut self, align_items: Alignment) -> Self {
        self.container = match self.container {
            GroupContainer::Row(row) => GroupContainer::Row(row.align_items(align_items)),
            GroupContainer::Column(column) => GroupContainer::Column(column.align_items(align_items)),
        };

        self
    }

    /// Sets the spacing _between_ radio buttons.
    ///
    /// Custom margins per element do not exist in iced. You should use this
    /// method instead! While less flexible, it helps you keep spacing between
    /// elements consistent.
    pub fn spacing(mut self, units: u16) -> Self {
        self.container = match self.container {
            GroupContainer::Row(row) => GroupContainer::Row(row.spacing(units)),
            GroupContainer::Column(column) => GroupContainer::Column(column.spacing(units)),
        };
        self
    }
}
