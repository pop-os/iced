 /// window events
 #[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowEvent {
    WmCapabilities(Vec<u32>),
    State(WindowState)
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// the state of the window
pub enum WindowState {
    Maximized,
    Fullscreen,
    Activated,
    TiledLeft,
    TiledRight,
    TiledTop,
    TiledBottom
}