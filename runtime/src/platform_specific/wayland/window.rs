use std::fmt;
use std::marker::PhantomData;

use iced_core::layout::Limits;
use iced_core::window::Mode;
use iced_core::Size;
use iced_futures::MaybeSend;
use sctk::reexports::protocols::xdg::shell::client::xdg_toplevel::ResizeEdge;

pub use iced_core::window::Id;

use crate::window;

/// window settings
#[derive(Debug, Clone)]
pub struct SctkWindowSettings {
    /// window id
    pub window_id: Id,
    /// optional app id
    pub app_id: Option<String>,
    /// optional window title
    pub title: Option<String>,
    /// optional window parent
    pub parent: Option<Id>,
    /// autosize the window to fit its contents
    pub autosize: bool,
    /// Limits of the window size
    pub size_limits: Limits,

    /// The initial size of the window.
    pub size: (u32, u32),

    /// Whether the window should be resizable or not.
    /// and the size of the window border which can be dragged for a resize
    pub resizable: Option<f64>,

    /// Whether the window should have a border, a title bar, etc. or not.
    pub client_decorations: bool,

    /// Whether the window should be transparent.
    pub transparent: bool,

    /// xdg-activation token
    pub xdg_activation_token: Option<String>,
}

impl Default for SctkWindowSettings {
    fn default() -> Self {
        Self {
            window_id: Id::unique(),
            app_id: Default::default(),
            title: Default::default(),
            parent: Default::default(),
            autosize: Default::default(),
            size_limits: Limits::NONE
                .min_height(1.0)
                .min_width(1.0)
                .max_width(1920.0)
                .max_height(1080.0),
            size: (1024, 768),
            resizable: Some(8.0),
            client_decorations: true,
            transparent: false,
            xdg_activation_token: Default::default(),
        }
    }
}

#[derive(Clone)]
/// Window Action
pub enum Action {
    /// create a window and receive a message with its Id
    Window {
        /// window builder
        builder: SctkWindowSettings,
    },
    /// Destroy the window
    Destroy(Id),
    /// Set size of the window.
    Size {
        /// id of the window
        id: Id,
        /// The new logical width of the window
        width: u32,
        /// The new logical height of the window
        height: u32,
    },
    /// Set min size of the window.
    MinSize {
        /// id of the window
        id: Id,
        /// optional size
        size: Option<(u32, u32)>,
    },
    /// Set max size of the window.
    MaxSize {
        /// id of the window
        id: Id,
        /// optional size
        size: Option<(u32, u32)>,
    },
    /// Set title of the window.
    Title {
        /// id of the window
        id: Id,
        /// The new logical width of the window
        title: String,
    },
    /// Minimize the window.
    /// Start an interactive move of the window.
    InteractiveResize {
        /// id of the window
        id: Id,
        /// edge being resized
        edge: ResizeEdge,
    },
    /// Start an interactive move of the window.
    InteractiveMove {
        /// id of the window
        id: Id,
    },

    /// Set the app id of the window
    AppId {
        /// id of the window
        id: Id,
        /// app id of the window
        app_id: String,
    },
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Window { builder, .. } => write!(
                f,
                "Action::Window::LayerSurface {{ builder: {:?} }}",
                builder
            ),
            Action::Size { id, width, height } => write!(
                f,
                "Action::Window::Size {{ id: {:?}, width: {:?}, height: {:?} }}",
                id, width, height
            ),
            Action::MinSize { id, size } => write!(
                f,
                "Action::Window::MinSize {{ id: {:?}, size: {:?} }}",
                id, size
            ),
            Action::MaxSize { id, size } => write!(
                f,
                "Action::Window::MaxSize {{ id: {:?}, size: {:?} }}",
                id, size
            ),
            Action::Title { id, title } => write!(
                f,
                "Action::Window::Title {{ id: {:?}, title: {:?} }}",
                id, title
            ),
            Action::InteractiveMove { id } => write!(
                f,
                "Action::Window::InteractiveMove {{ id: {:?} }}",
                id
            ),
            Action::InteractiveResize { id, edge } => write!(
                f,
                "Action::Window::InteractiveResize {{ id: {:?}, edge: {:?} }}",
                id, edge
            ),
            Action::Destroy(id) => write!(
                f,
                "Action::Window::Destroy {{ id: {:?} }}",
                id
            ),
            Action::AppId { id, app_id } => write!(
                f,
                "Action::Window::Mode {{ id: {:?}, app_id: {:?} }}",
                id, app_id
            ),
        }
    }
}

/// error type for unsupported actions
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// Not supported
    #[error("Not supported")]
    NotSupported,
}

impl TryFrom<window::Action> for Action {
    type Error = Error;

    fn try_from(value: window::Action) -> Result<Self, Self::Error> {
        match value {
            window::Action::Open(id, settings, tx) => {
                let min = settings.min_size.unwrap_or(Size::new(1., 1.));
                let max = settings.max_size.unwrap_or(Size::INFINITY);
                let builder = SctkWindowSettings {
                    window_id: id,
                    app_id: Some(settings.platform_specific.application_id),
                    title: None,
                    parent: None,
                    autosize: false,
                    size_limits: Limits::NONE
                        .min_width(min.width)
                        .min_height(min.height)
                        .max_width(max.width)
                        .max_height(max.height),
                    size: (
                        settings.size.width.round() as u32,
                        settings.size.height.round() as u32,
                    ),
                    resizable: settings
                        .resizable
                        .then_some(settings.resize_border as f64),
                    client_decorations: !settings.decorations,
                    transparent: settings.transparent,
                    xdg_activation_token: None,
                };
                Ok(Action::Window {
                    builder,
                })
            }
            window::Action::Close(id) => Ok(Action::Destroy(id)),
            window::Action::Resize(id, size) => Ok(Action::Size {
                id,
                width: size.width.round() as u32,
                height: size.height.round() as u32,
            }),
            window::Action::Drag(id) => Ok(Action::InteractiveMove { id }),
            window::Action::GetSize(_, _)
            | window::Action::Maximize(_, _)
            | window::Action::Minimize(_, _)
            | window::Action::GetMaximized(_, _)
            | window::Action::Move(_, _)
            | window::Action::GetMode(_, _)
            | window::Action::ToggleMaximize(_)
            | window::Action::ToggleDecorations(_)
            | window::Action::RequestUserAttention(_, _)
            | window::Action::GainFocus(_)
            | window::Action::ChangeLevel(_, _)
            | window::Action::GetRawId(_, _)
            | window::Action::ChangeIcon(_, _)
            | window::Action::Screenshot(_, _)
            | window::Action::ChangeMode(_, _)
            | window::Action::ShowSystemMenu(_)
            | window::Action::RunWithHandle(_, _) // TODO(POP): Is this supported? Not sure.
            | window::Action::GetPosition(_, _) // TODO(POP): Is this supported? Not sure.
            | window::Action::GetMinimized(_, _) => Err(Error::NotSupported),
            
            window::Action::GetOldest(_) => todo!(),
            window::Action::GetLatest(_) => todo!(),
        }
    }
}
