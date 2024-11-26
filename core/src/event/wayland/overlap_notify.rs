use cctk::{sctk::shell::wlr_layer::Layer, wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1};

#[derive(Debug, Clone, PartialEq)]
pub enum OverlapNotifyEvent {
    OverlapToplevelAdd {
        toplevel: ExtForeignToplevelHandleV1,
        logical_rect: crate::Rectangle,
    },
    OverlapToplevelRemove {
        toplevel: ExtForeignToplevelHandleV1,
    },
    OverlapLayerAdd {
        identifier: String,
        exclusive: u32,
        layer: Option<Layer>,
        logical_rect: crate::Rectangle,
    },
    OverlapLayerRemove {
        identifier: String,
    },
}
