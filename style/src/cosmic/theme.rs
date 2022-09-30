pub mod palette;

pub use self::palette::Palette;

use crate::application;
use crate::button;
use crate::checkbox;
use crate::container;
use crate::menu;
use crate::pane_grid;
use crate::pick_list;
use crate::progress_bar;
use crate::radio;
use crate::rule;
use crate::scrollable;
use crate::slider;
use crate::text;
use crate::text_input;
use crate::toggler;

use iced_core::{Background, Color};

type CosmicColor = ::palette::rgb::Srgba;
type CosmicComponent = cosmic_theme::Component<CosmicColor>;
type CosmicTheme = cosmic_theme::Theme<CosmicColor>;
type CosmicThemeCss = cosmic_theme::Theme<cosmic_theme::util::CssColor>;

lazy_static::lazy_static! {
    pub static ref COSMIC_DARK: CosmicTheme = CosmicThemeCss::dark_default().into_srgba();
    pub static ref COSMIC_LIGHT: CosmicTheme = CosmicThemeCss::light_default().into_srgba();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn cosmic(self) -> &'static CosmicTheme {
        match self {
            Self::Dark => &COSMIC_DARK,
            Self::Light => &COSMIC_LIGHT,
        }
    }

    pub fn palette(self) -> Palette {
        match self {
            Self::Dark => Palette::DARK,
            Self::Light => Palette::LIGHT,
        }
    }

    pub fn extended_palette(&self) -> &palette::Extended {
        match self {
            Self::Dark => &palette::EXTENDED_DARK,
            Self::Light => &palette::EXTENDED_LIGHT,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::Dark
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Application {
    Default,
    Custom(fn(Theme) -> application::Appearance),
}

impl Default for Application {
    fn default() -> Self {
        Self::Default
    }
}

impl application::StyleSheet for Theme {
    type Style = Application;

    fn appearance(&self, style: Self::Style) -> application::Appearance {
        let cosmic = self.cosmic();

        match style {
            Application::Default => application::Appearance {
                background_color: cosmic.bg_color().into(),
                text_color: cosmic.on_bg_color().into(),
            },
            Application::Custom(f) => f(*self),
        }
    }
}

/*
 * TODO: Button
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Primary,
    Secondary,
    Positive,
    Destructive,
    Text,
}

impl Default for Button {
    fn default() -> Self {
        Self::Primary
    }
}

impl Button {
    fn cosmic(&self, theme: &Theme) -> &'static CosmicComponent {
        let cosmic = theme.cosmic();
        match self {
            Button::Primary => &cosmic.accent,
            Button::Secondary => &cosmic.primary.component,
            Button::Positive => &cosmic.success,
            Button::Destructive => &cosmic.destructive,
            Button::Text => &cosmic.secondary.component,
        }
    }
}

impl button::StyleSheet for Theme {
    type Style = Button;

    fn active(&self, style: Self::Style) -> button::Appearance {
        let cosmic = style.cosmic(self);

        button::Appearance {
            border_radius: 24.0,
            background: match style {
                Button::Text => None,
                _ => Some(Background::from(
                    Color::from(cosmic.base)
                )),
            },
            text_color: cosmic.on.into(),
            ..button::Appearance::default()
        }
    }

    fn hovered(&self, style: Self::Style) -> button::Appearance {
        let active = self.active(style);
        let cosmic = style.cosmic(self);

        button::Appearance {
            background: Some(Background::from(
                Color::from(cosmic.hover)
            )),
            ..active
        }
    }
}

/*
 * TODO: Checkbox
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checkbox {
    Primary,
    Secondary,
    Success,
    Danger,
}

impl Default for Checkbox {
    fn default() -> Self {
        Self::Primary
    }
}

impl checkbox::StyleSheet for Theme {
    type Style = Checkbox;

    fn active(
        &self,
        style: Self::Style,
        is_checked: bool,
    ) -> checkbox::Appearance {
        let palette = self.extended_palette();

        match style {
            Checkbox::Primary => checkbox_appearance(
                palette.primary.strong.text,
                palette.background.base,
                palette.primary.strong,
                is_checked,
            ),
            Checkbox::Secondary => checkbox_appearance(
                palette.background.base.text,
                palette.background.base,
                palette.background.base,
                is_checked,
            ),
            Checkbox::Success => checkbox_appearance(
                palette.success.base.text,
                palette.background.base,
                palette.success.base,
                is_checked,
            ),
            Checkbox::Danger => checkbox_appearance(
                palette.danger.base.text,
                palette.background.base,
                palette.danger.base,
                is_checked,
            ),
        }
    }

    fn hovered(
        &self,
        style: Self::Style,
        is_checked: bool,
    ) -> checkbox::Appearance {
        let palette = self.extended_palette();

        match style {
            Checkbox::Primary => checkbox_appearance(
                palette.primary.strong.text,
                palette.background.weak,
                palette.primary.base,
                is_checked,
            ),
            Checkbox::Secondary => checkbox_appearance(
                palette.background.base.text,
                palette.background.weak,
                palette.background.base,
                is_checked,
            ),
            Checkbox::Success => checkbox_appearance(
                palette.success.base.text,
                palette.background.weak,
                palette.success.base,
                is_checked,
            ),
            Checkbox::Danger => checkbox_appearance(
                palette.danger.base.text,
                palette.background.weak,
                palette.danger.base,
                is_checked,
            ),
        }
    }
}

fn checkbox_appearance(
    checkmark_color: Color,
    base: palette::Pair,
    accent: palette::Pair,
    is_checked: bool,
) -> checkbox::Appearance {
    checkbox::Appearance {
        background: Background::Color(if is_checked {
            accent.color
        } else {
            base.color
        }),
        checkmark_color,
        border_radius: 4.0,
        border_width: if is_checked { 0.0 } else { 1.0 },
        border_color: accent.color,
        text_color: None,
    }
}

/*
 * TODO: Container
 */
#[derive(Clone, Copy)]
pub enum Container {
    Transparent,
    Box,
    Custom(fn(&Theme) -> container::Appearance),
}

impl Default for Container {
    fn default() -> Self {
        Self::Transparent
    }
}

impl From<fn(&Theme) -> container::Appearance> for Container {
    fn from(f: fn(&Theme) -> container::Appearance) -> Self {
        Self::Custom(f)
    }
}

impl container::StyleSheet for Theme {
    type Style = Container;

    fn appearance(&self, style: Self::Style) -> container::Appearance {
        match style {
            Container::Transparent => Default::default(),
            Container::Box => {
                let palette = self.extended_palette();

                container::Appearance {
                    text_color: None,
                    background: palette.background.weak.color.into(),
                    border_radius: 2.0,
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                }
            }
            Container::Custom(f) => f(self),
        }
    }
}

/*
 * Slider
 */
impl slider::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: Self::Style) -> slider::Appearance {
        let palette = self.palette();

        //TODO: no way to set rail thickness
        slider::Appearance {
            rail_colors: (
                palette.primary,
                //TODO: no way to set color before/after slider
                Color::TRANSPARENT,
            ),
            handle: slider::Handle {
                shape: slider::HandleShape::Circle {
                    radius: 10.0,
                },
                color: palette.primary,
                border_color: Color::TRANSPARENT,
                border_width: 0.0,
            }
        }
    }

    fn hovered(&self, style: Self::Style) -> slider::Appearance {
        let mut style = self.active(style);
        style.handle.shape = slider::HandleShape::Circle {
            radius: 16.0
        };
        style.handle.border_width = 6.0;
        style.handle.border_color = match self {
            Theme::Dark => Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.1),
            Theme::Light => Color::from_rgba8(0, 0, 0, 0.1),
        };
        style
    }

    fn dragging(&self, style: Self::Style) -> slider::Appearance {
        let mut style = self.hovered(style);
        style.handle.border_color = match self {
            Theme::Dark => Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.2),
            Theme::Light => Color::from_rgba8(0, 0, 0, 0.2),
        };
        style
    }
}

/*
 * TODO: Menu
 */
impl menu::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: Self::Style) -> menu::Appearance {
        let palette = self.extended_palette();

        menu::Appearance {
            text_color: palette.background.weak.text,
            background: palette.background.base.color.into(),
            border_width: 0.0,
            border_radius: 16.0,
            border_color: palette.background.strong.color,
            selected_text_color: palette.background.weak.text,
            selected_background: palette.background.weak.color.into(),
        }
    }
}

/*
 * TODO: Pick List
 */
impl pick_list::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: ()) -> pick_list::Appearance {
        let palette = self.extended_palette();

        pick_list::Appearance {
            text_color: palette.background.weak.text,
            background: Color::TRANSPARENT.into(),
            placeholder_color: palette.background.strong.color,
            border_radius: 24.0,
            border_width: 0.0,
            border_color: palette.background.strong.color,
            icon_size: 0.7,
        }
    }

    fn hovered(&self, _style: ()) -> pick_list::Appearance {
        let palette = self.extended_palette();

        pick_list::Appearance {
            text_color: palette.background.weak.text,
            background: palette.background.weak.color.into(),
            placeholder_color: palette.background.strong.color,
            border_radius: 24.0,
            border_width: 0.0,
            border_color: palette.primary.strong.color,
            icon_size: 0.7,
        }
    }
}

/*
 * TODO: Radio
 */
impl radio::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: Self::Style) -> radio::Appearance {
        let palette = self.extended_palette();

        radio::Appearance {
            background: Color::TRANSPARENT.into(),
            dot_color: palette.primary.strong.color,
            border_width: 1.0,
            border_color: palette.primary.strong.color,
            text_color: None,
        }
    }

    fn hovered(&self, style: Self::Style) -> radio::Appearance {
        let active = self.active(style);
        let palette = self.extended_palette();

        radio::Appearance {
            dot_color: palette.primary.strong.color,
            background: palette.primary.weak.color.into(),
            ..active
        }
    }
}

/*
 * Toggler
 */
impl toggler::StyleSheet for Theme {
    type Style = ();

    fn active(
        &self,
        _style: Self::Style,
        is_active: bool,
    ) -> toggler::Appearance {
        let palette = self.palette();

        toggler::Appearance {
            background: if is_active {
                palette.primary
            } else {
                //TODO: Grab neutral from palette
                match self {
                    Theme::Dark => Color::from_rgb8(0x78, 0x78, 0x78),
                    Theme::Light => Color::from_rgb8(0x93, 0x93, 0x93),
                }
            },
            background_border: None,
            //TODO: Grab neutral from palette
            foreground: match self {
                Theme::Dark => Color::from_rgb8(0x27, 0x27, 0x27),
                Theme::Light => Color::from_rgb8(0xe4, 0xe4, 0xe4),
            },
            foreground_border: None,
        }
    }

    fn hovered(
        &self,
        style: Self::Style,
        is_active: bool,
    ) -> toggler::Appearance {
        //TODO: grab colors from palette
        match self {
            Theme::Dark  => toggler::Appearance {
                background: if is_active {
                    Color::from_rgb8(0x9f, 0xed, 0xed)
                } else {
                    Color::from_rgb8(0xb6, 0xb6, 0xb6)
                },
                ..self.active(style, is_active)
            },
            Theme::Light  => toggler::Appearance {
                background: if is_active {
                    Color::from_rgb8(0x00, 0x42, 0x62)
                } else {
                    Color::from_rgb8(0x54, 0x54, 0x54)
                },
                ..self.active(style, is_active)
            }
        }
    }
}

/*
 * TODO: Pane Grid
 */
impl pane_grid::StyleSheet for Theme {
    type Style = ();

    fn picked_split(&self, _style: Self::Style) -> Option<pane_grid::Line> {
        let palette = self.extended_palette();

        Some(pane_grid::Line {
            color: palette.primary.strong.color,
            width: 2.0,
        })
    }

    fn hovered_split(&self, _style: Self::Style) -> Option<pane_grid::Line> {
        let palette = self.extended_palette();

        Some(pane_grid::Line {
            color: palette.primary.base.color,
            width: 2.0,
        })
    }
}

/*
 * TODO: Progress Bar
 */
#[derive(Clone, Copy)]
pub enum ProgressBar {
    Primary,
    Success,
    Danger,
    Custom(fn(&Theme) -> progress_bar::Appearance),
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self::Primary
    }
}

impl progress_bar::StyleSheet for Theme {
    type Style = ProgressBar;

    fn appearance(&self, style: Self::Style) -> progress_bar::Appearance {
        let palette = self.extended_palette();

        let from_palette = |bar: Color| progress_bar::Appearance {
            background: palette.background.strong.color.into(),
            bar: bar.into(),
            border_radius: 2.0,
        };

        match style {
            ProgressBar::Primary => from_palette(palette.primary.base.color),
            ProgressBar::Success => from_palette(palette.success.base.color),
            ProgressBar::Danger => from_palette(palette.danger.base.color),
            ProgressBar::Custom(f) => f(self),
        }
    }
}

/*
 * TODO: Rule
 */
#[derive(Clone, Copy)]
pub enum Rule {
    Default,
    Custom(fn(&Theme) -> rule::Appearance),
}

impl Default for Rule {
    fn default() -> Self {
        Self::Default
    }
}

impl rule::StyleSheet for Theme {
    type Style = Rule;

    fn style(&self, style: Self::Style) -> rule::Appearance {
        let palette = self.extended_palette();

        match style {
            Rule::Default => rule::Appearance {
                color: palette.background.strong.color,
                width: 1,
                radius: 0.0,
                fill_mode: rule::FillMode::Full,
            },
            Rule::Custom(f) => f(self),
        }
    }
}

/*
 * TODO: Scrollable
 */
impl scrollable::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: Self::Style) -> scrollable::Scrollbar {
        let palette = self.extended_palette();

        scrollable::Scrollbar {
            background: palette.background.weak.color.into(),
            border_radius: 4.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            scroller: scrollable::Scroller {
                color: palette.background.strong.color,
                border_radius: 4.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
        }
    }

    fn hovered(&self, _style: Self::Style) -> scrollable::Scrollbar {
        let palette = self.extended_palette();

        scrollable::Scrollbar {
            background: palette.background.weak.color.into(),
            border_radius: 4.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            scroller: scrollable::Scroller {
                color: palette.primary.strong.color,
                border_radius: 4.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
        }
    }
}

/*
 * TODO: Text
 */
#[derive(Clone, Copy)]
pub enum Text {
    Default,
    Color(Color),
    Custom(fn(&Theme) -> text::Appearance),
}

impl Default for Text {
    fn default() -> Self {
        Self::Default
    }
}

impl From<Color> for Text {
    fn from(color: Color) -> Self {
        Text::Color(color)
    }
}

impl text::StyleSheet for Theme {
    type Style = Text;

    fn appearance(&self, style: Self::Style) -> text::Appearance {
        match style {
            Text::Default => Default::default(),
            Text::Color(c) => text::Appearance { color: Some(c) },
            Text::Custom(f) => f(self),
        }
    }
}

/*
 * TODO: Text Input
 */
impl text_input::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: Self::Style) -> text_input::Appearance {
        let palette = self.extended_palette();

        text_input::Appearance {
            background: palette.background.base.color.into(),
            border_radius: 2.0,
            border_width: 1.0,
            border_color: palette.background.strong.color,
        }
    }

    fn hovered(&self, _style: Self::Style) -> text_input::Appearance {
        let palette = self.extended_palette();

        text_input::Appearance {
            background: palette.background.base.color.into(),
            border_radius: 2.0,
            border_width: 1.0,
            border_color: palette.background.base.text,
        }
    }

    fn focused(&self, _style: Self::Style) -> text_input::Appearance {
        let palette = self.extended_palette();

        text_input::Appearance {
            background: palette.background.base.color.into(),
            border_radius: 2.0,
            border_width: 1.0,
            border_color: palette.primary.strong.color,
        }
    }

    fn placeholder_color(&self, _style: Self::Style) -> Color {
        let palette = self.extended_palette();

        palette.background.strong.color
    }

    fn value_color(&self, _style: Self::Style) -> Color {
        let palette = self.extended_palette();

        palette.background.base.text
    }

    fn selection_color(&self, _style: Self::Style) -> Color {
        let palette = self.extended_palette();

        palette.primary.weak.color
    }
}