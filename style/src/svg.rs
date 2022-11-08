use iced_core::Color;

#[derive(Debug, Default, Clone, Copy)]
pub struct Appearance {
    pub fill: Option<Color>,
}

pub trait StyleSheet {
    type Style: Default + Copy;

    fn appearance(&self, style: Self::Style) -> Appearance;
}
