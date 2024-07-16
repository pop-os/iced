use crate::graphics::compositor::Window;

use as_raw_xcb_connection::AsRawXcbConnection;
use raw_window_handle::{
    RawDisplayHandle, XcbDisplayHandle, XlibDisplayHandle,
};
use rustix::fs::fstat;
use tiny_xlib::Display;
use x11rb::{
    connection::{Connection, RequestConnection},
    protocol::dri3::{ConnectionExt as _, X11_EXTENSION_NAME as DRI3_NAME},
    xcb_ffi::XCBConnection,
};

pub fn get_x11_device_ids<W: Window>(window: &W) -> Option<(u16, u16)> {
    x11rb::xcb_ffi::load_libxcb().ok()?;

    #[allow(unsafe_code)]
    let (conn, screen) = match window
        .display_handle()
        .map(|handle| handle.as_raw())
    {
        #[allow(unsafe_code)]
        Ok(RawDisplayHandle::Xlib(XlibDisplayHandle {
            display,
            screen,
            ..
        })) => match display {
            Some(ptr) => unsafe {
                let xlib_display = Display::from_ptr(ptr.as_ptr());
                let conn = XCBConnection::from_raw_xcb_connection(
                    xlib_display.as_raw_xcb_connection() as *mut _,
                    false,
                )
                .ok();
                // intentially leak the display, we don't want to close the connection

                (conn?, screen)
            },
            None => (XCBConnection::connect(None).ok()?.0, screen),
        },
        Ok(RawDisplayHandle::Xcb(XcbDisplayHandle {
            connection,
            screen,
            ..
        })) => match connection {
            Some(ptr) => (
                unsafe {
                    XCBConnection::from_raw_xcb_connection(ptr.as_ptr(), false)
                        .ok()?
                },
                screen,
            ),
            None => (XCBConnection::connect(None).ok()?.0, screen),
        },
        _ => {
            return None;
        }
    };

    // check for DRI3
    let _ = conn.extension_information(DRI3_NAME).ok()??;
    // we have dri3, dri3_open exists on any version, so lets skip version checks.

    // provider being NONE tells the X server to use the RandR provider.
    let screen = &conn.setup().roots[screen as usize];
    let dri3 = conn
        .dri3_open(screen.root, x11rb::NONE)
        .ok()?
        .reply()
        .ok()?;
    let device_fd = dri3.device_fd;
    let stat = fstat(device_fd).ok()?;
    let dev = stat.st_rdev;

    super::ids_from_dev(dev)
}
