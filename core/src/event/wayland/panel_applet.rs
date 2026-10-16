/// The edge of the output a panel is anchored to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PanelAnchor {
    Left,
    Right,
    #[default]
    Top,
    Bottom,
}

impl PanelAnchor {
    #[must_use]
    pub fn is_horizontal(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }
}

/// The size the panel expects its applets to have.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum AppletSize {
    ExtraSmall,
    Small,
    #[default]
    Medium,
    Large,
    ExtraLarge,
    Custom(u32),
}

/// The background of a panel.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum PanelBackground {
    #[default]
    ThemeDefault,
    Dark,
    Light,
    Color([f32; 3]),
}

/// The settings of the panel an applet is embedded in.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PanelAppletSettings {
    /// Name of the panel profile.
    pub panel_name: String,
    /// Name of the output the panel is displayed on.
    pub output: String,
    pub anchor: PanelAnchor,
    pub applet_size: AppletSize,
    /// Spacing between the applets of the panel in logical pixels.
    pub spacing: u32,
    pub background: PanelBackground,
    /// Ratio of the applet padding that overlaps adjacent applets.
    pub padding_overlap: f32,
}
