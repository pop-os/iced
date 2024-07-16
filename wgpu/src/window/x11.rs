use std::{
    fs,
    io::{BufRead, BufReader},
    path::Path,
};

use crate::graphics::compositor::Window;

use as_raw_xcb_connection::AsRawXcbConnection;
use raw_window_handle::{
    RawDisplayHandle, XcbDisplayHandle, XlibDisplayHandle,
};
use rustix::fs::{fstat, stat};
use tiny_xlib::Display;
use x11rb::{
    connection::{Connection, RequestConnection},
    protocol::{
        dri3::{ConnectionExt as _, X11_EXTENSION_NAME as DRI3_NAME},
        randr::{
            ConnectionExt as _, ProviderCapability,
            X11_EXTENSION_NAME as RANDR_NAME,
        },
    },
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
    let root = conn.setup().roots[screen as usize].root;

    // The nvidia xorg driver advertises DRI2 and DRI3,
    // but doesn't really return any useful data for either of them.
    // We also can't query EGL, as a display created from an X11 display
    // running on the properietary driver won't return an EGLDevice.
    //
    // So we have to resort to hacks.

    // check for randr
    let _ = conn.extension_information(RANDR_NAME).ok()??;
    // check version, because we need providers to exist
    let version = conn.randr_query_version(1, 4).ok()?.reply().ok()?;
    if version.major_version < 1
        || (version.major_version == 1 && version.minor_version < 4)
    {
        return None;
    }

    // get the name of the first Source Output provider, that will be our main device
    let randr = conn.randr_get_providers(root).ok()?.reply().ok()?;
    let mut name = None;
    for provider in randr.providers {
        let info = conn
            .randr_get_provider_info(provider, randr.timestamp)
            .ok()?
            .reply()
            .ok()?;
        if info
            .capabilities
            .contains(ProviderCapability::SOURCE_OUTPUT)
            || name.is_none()
        {
            name = std::str::from_utf8(&info.name)
                .ok()
                .map(ToString::to_string);
        }
    }

    // if that name is formatted `NVIDIA-x`, then x represents the /dev/nvidiaX number, which we can relate to /dev/dri
    if let Some(number) = name.and_then(|name| {
        name.trim().strip_prefix("NVIDIA-")?.parse::<u32>().ok()
    }) {
        // let it be known, that I hate this "interface"...
        for busid in fs::read_dir("/proc/driver/nvidia/gpus")
            .ok()?
            .map(Result::ok)
            .flatten()
        {
            for line in BufReader::new(
                fs::File::open(busid.path().join("information")).ok()?,
            )
            .lines()
            {
                if let Ok(line) = line {
                    if line.starts_with("Device Minor") {
                        if let Some((_, num)) = line.split_once(":") {
                            let minor = num.trim().parse::<u32>().ok()?;
                            if minor == number {
                                // we found the device
                                for device in fs::read_dir(
                                    Path::new("/sys/module/nvidia/drivers/pci:nvidia/")
                                        .join(busid.file_name())
                                        .join("drm"),
                                )
                                .ok()?
                                .map(Result::ok)
                                .flatten()
                                {
                                    let device = device.file_name();
                                    if device.to_string_lossy().starts_with("card")
                                        || device.to_string_lossy().starts_with("render")
                                    {
                                        let stat =
                                            stat(Path::new("/dev/dri").join(device)).ok()?;
                                        let dev = stat.st_rdev;
                                        return super::ids_from_dev(dev);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    } else {
        // check via DRI3
        let _ = conn.extension_information(DRI3_NAME).ok()??;
        // we have dri3, dri3_open exists on any version, so skip version checks.

        // provider being NONE tells the X server to use the RandR provider.
        let dri3 = conn.dri3_open(root, x11rb::NONE).ok()?.reply().ok()?;
        let device_fd = dri3.device_fd;
        let stat = fstat(device_fd).ok()?;
        let dev = stat.st_rdev;
        super::ids_from_dev(dev)
    }
}
